//! Platform-native secret storage for the SMTP password and Gemini API key.
//!
//! Storage strategy per platform:
//!
//! * **Windows** — DPAPI (`CryptProtectData` / `CryptUnprotectData`) encrypts
//!   each secret. Ciphertext is base64-encoded and stored in `secrets.json`
//!   next to `config.json` in `%APPDATA%\com.research.newsletter\`. DPAPI
//!   binds the ciphertext to the current Windows user, so an exfiltrated
//!   file is unreadable on a different user account or machine.
//!
//! * **macOS** — Login keychain via `security-framework`'s generic-password
//!   API. No sidecar file. The OS handles the encryption.
//!
//! * **Linux + everything else** — Plaintext sidecar file with a
//!   stderr warning. (TODO: integrate `secret-service` / libsecret.)
//!
//! Public API is the same on every platform:
//!
//! ```ignore
//! secrets::get(secrets::KEY_SMTP_PASSWORD) -> Result<Option<String>, String>
//! secrets::set(secrets::KEY_SMTP_PASSWORD, "abc")  -> Result<(), String>
//! ```
//!
//! We deliberately do **not** depend on the `keyring` crate. v3.6.3's
//! Windows backend has a known failure mode where `set_password()` returns
//! `Ok(())` while the credential never lands in Credential Manager
//! (verified empty with `cmdkey /list`).

pub const KEY_SMTP_PASSWORD: &str = "smtp_password";
pub const KEY_GEMINI_API_KEY: &str = "gemini_api_key";
pub const KEY_CLAUDE_API_KEY: &str = "claude_api_key";

const SERVICE: &str = "com.research.newsletter";

// ════════════════════════════════════════════════════════════════════════════
//  Windows — DPAPI + base64 sidecar JSON
// ════════════════════════════════════════════════════════════════════════════

#[cfg(windows)]
mod imp {
    use super::SERVICE;
    use base64::engine::{general_purpose::STANDARD, Engine};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    fn storage_path() -> PathBuf {
        let dir = if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join(SERVICE)
        } else if let Ok(profile) = std::env::var("USERPROFILE") {
            PathBuf::from(profile)
                .join("AppData")
                .join("Roaming")
                .join(SERVICE)
        } else {
            PathBuf::from(".")
        };
        dir.join("secrets.json")
    }

    fn dpapi_encrypt(plain: &[u8]) -> Result<Vec<u8>, String> {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: plain.len() as u32,
            pbData: plain.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(
                &mut input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
            .map_err(|e| format!("CryptProtectData: {e}"))?;
            let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(out.pbData as _)));
            Ok(v)
        }
    }

    fn dpapi_decrypt(cipher: &[u8]) -> Result<Vec<u8>, String> {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: cipher.len() as u32,
            pbData: cipher.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(
                &mut input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
            .map_err(|e| format!("CryptUnprotectData: {e}"))?;
            let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(out.pbData as _)));
            Ok(v)
        }
    }

    fn read_store(path: &Path) -> BTreeMap<String, String> {
        if !path.exists() {
            return BTreeMap::new();
        }
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => BTreeMap::new(),
        }
    }

    fn write_store(path: &Path, map: &BTreeMap<String, String>) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir secrets dir: {e}"))?;
        }
        let text =
            serde_json::to_string_pretty(map).map_err(|e| format!("serialize secrets: {e}"))?;
        fs::write(path, text).map_err(|e| format!("write secrets file: {e}"))
    }

    pub fn get(name: &str) -> Result<Option<String>, String> {
        let path = storage_path();
        let store = read_store(&path);
        let entry = match store.get(name) {
            Some(s) => s,
            None => return Ok(None),
        };
        let cipher = STANDARD
            .decode(entry)
            .map_err(|e| format!("base64 decode ({name}): {e}"))?;
        let plain = dpapi_decrypt(&cipher)?;
        let s = String::from_utf8(plain).map_err(|e| format!("utf8 ({name}): {e}"))?;
        Ok(Some(s))
    }

    pub fn set(name: &str, value: &str) -> Result<(), String> {
        let path = storage_path();
        let mut store = read_store(&path);
        if value.is_empty() {
            store.remove(name);
        } else {
            let cipher = dpapi_encrypt(value.as_bytes())?;
            store.insert(name.to_string(), STANDARD.encode(cipher));
        }
        if store.is_empty() && path.exists() {
            // Remove the file once everything has been cleared.
            let _ = fs::remove_file(&path);
            return Ok(());
        }
        write_store(&path, &store)
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  macOS — Keychain via security-framework
// ════════════════════════════════════════════════════════════════════════════

#[cfg(target_os = "macos")]
mod imp {
    use super::SERVICE;
    use security_framework::passwords::{
        delete_generic_password, get_generic_password, set_generic_password,
    };

    pub fn get(name: &str) -> Result<Option<String>, String> {
        match get_generic_password(SERVICE, name) {
            Ok(bytes) => {
                let s = String::from_utf8(bytes)
                    .map_err(|e| format!("utf8 ({name}): {e}"))?;
                Ok(Some(s))
            }
            // -25300 == errSecItemNotFound
            Err(e) if e.code() == -25300 => Ok(None),
            Err(e) => Err(format!("Keychain read ({name}): {e}")),
        }
    }

    pub fn set(name: &str, value: &str) -> Result<(), String> {
        if value.is_empty() {
            // Best-effort delete; ignore "not found" results.
            let _ = delete_generic_password(SERVICE, name);
            Ok(())
        } else {
            set_generic_password(SERVICE, name, value.as_bytes())
                .map_err(|e| format!("Keychain write ({name}): {e}"))
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Linux + other — plaintext sidecar (TODO: integrate libsecret)
// ════════════════════════════════════════════════════════════════════════════

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    use super::SERVICE;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};

    static WARNED: AtomicBool = AtomicBool::new(false);

    fn warn_once() {
        if !WARNED.swap(true, Ordering::Relaxed) {
            eprintln!(
                "[secrets] WARNING: storing secrets in plaintext on this platform. \
                 libsecret/Secret Service integration is not yet implemented."
            );
        }
    }

    fn storage_path() -> PathBuf {
        let dir = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg).join(SERVICE)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config").join(SERVICE)
        } else {
            PathBuf::from(".")
        };
        dir.join("secrets.json")
    }

    fn read_store(path: &Path) -> BTreeMap<String, String> {
        if !path.exists() {
            return BTreeMap::new();
        }
        fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn write_store(path: &Path, map: &BTreeMap<String, String>) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        let text = serde_json::to_string_pretty(map).map_err(|e| format!("serialize: {e}"))?;
        fs::write(path, text).map_err(|e| format!("write: {e}"))
    }

    pub fn get(name: &str) -> Result<Option<String>, String> {
        let path = storage_path();
        Ok(read_store(&path).remove(name))
    }

    pub fn set(name: &str, value: &str) -> Result<(), String> {
        warn_once();
        let path = storage_path();
        let mut store = read_store(&path);
        if value.is_empty() {
            store.remove(name);
        } else {
            store.insert(name.to_string(), value.to_string());
        }
        write_store(&path, &store)
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Public API
// ════════════════════════════════════════════════════════════════════════════

pub fn get(name: &str) -> Result<Option<String>, String> {
    imp::get(name)
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    imp::set(name, value)
}

pub fn get_or_none(name: &str) -> Option<String> {
    get(name).ok().flatten()
}
