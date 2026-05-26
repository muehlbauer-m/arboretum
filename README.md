# Arboretum

A desktop app that grows daily research digests from OpenAlex and arXiv, curated by AI.

Type a research interest in plain language. Arboretum extracts academic search keywords, queries OpenAlex and arXiv, then asks an LLM to pick the 10 most relevant recent papers and write a short editorial summary. The result is saved as Markdown to your machine and (optionally) emailed to you on a schedule.

![screenshot placeholder](docs/screenshot.png)

---

## Features

- **Plain-language topics** — write what you care about; the model converts it into optimised academic search keywords and a newsletter title.
- **Three AI providers** — Anthropic Claude (API key), Google Gemini (API key), or a local model via Ollama (offline, no key, runs on your machine).
- **Two paper sources** — OpenAlex (250M+ cross-disciplinary works) and arXiv, toggleable per run.
- **Multi-topic parallelism** — generate several digests in one click; they run concurrently.
- **Conflict scanner** — define a research profile, and Arboretum scans recent literature for potentially overlapping or competing work, scoring each paper 0–100.
- **Live progress** — per-step status (and per-token throughput, for the local provider) streams into the UI while a generation runs.
- **History** — browse, re-read, and re-open past newsletters; outbound paper links open in your browser.
- **Email delivery** — optional SMTP dispatch (HTML + plain-text multipart) after each generation, with a provider-led setup wizard for Gmail / Outlook / iCloud.
- **Scheduled runs** — Arboretum installs a native scheduled task (Windows Task Scheduler / macOS launchd) so the app can wake up, run headlessly, send the email, and exit — no background daemon required.
- **Editorial theme** — Paper (ivory) / Forest (dark) palettes, Cormorant Garamond + Inter, designed to feel like a quiet reading room rather than another dashboard.

---

## Tech stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | [Tauri v2](https://tauri.app) (Rust backend + system WebView) |
| Frontend | React 18 + TypeScript + Tailwind CSS |
| HTTP | reqwest 0.12 (rustls-tls, no OpenSSL) |
| AI — Claude | Anthropic Messages API (`claude-sonnet-4-6`) |
| AI — Gemini | Gemini 2.5 Flash via REST |
| AI — Local | Ollama (default: `qwen3:4b`), with auto hardware-tiering |
| Paper sources | OpenAlex REST API, arXiv Atom feed |
| Scheduling | Windows Task Scheduler (`schtasks.exe`) / macOS launchd plist |
| Email | lettre 0.11 (STARTTLS / SMTP, multipart HTML + plain text) |
| Secrets | DPAPI sidecar (Windows) / Keychain (macOS) / plaintext fallback (Linux) |

---

## Prerequisites

- [Rust](https://rustup.rs) (stable; MSVC toolchain on Windows)
- [Node.js](https://nodejs.org) 18+
- **Windows only:** [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the *Desktop development with C++* workload (for the MSVC linker)
- At least one of:
  - An [Anthropic API key](https://console.anthropic.com/settings/keys), or
  - A [Gemini API key](https://aistudio.google.com/apikey), or
  - [Ollama](https://ollama.com) installed locally with a downloaded model

---

## Getting started

```bash
# Install frontend dependencies
npm install

# Run in development mode (hot-reload, port 1420)
npm run tauri dev

# Build a standalone bundle
npm run tauri build
# Windows: src-tauri/target/release/bundle/{msi,nsis}/
# macOS:   src-tauri/target/release/bundle/{dmg,macos}/
```

---

## Configuration

Open **Settings** inside the app after first launch.

| Setting | Description |
|---------|-------------|
| AI Provider | Claude (Anthropic API), Google Gemini, or Local (Ollama) |
| API Key | Required when using Claude or Gemini. Stored in the OS credential store, not on disk. |
| Local model | Auto-recommended based on your RAM/CPU; downloadable from inside the app |
| Output Directory | Where `.md` newsletters are saved (default: `Documents/newsletters`) |
| Default Sources | OpenAlex, arXiv, or both |
| Max Papers | How many papers per source to retrieve (10–100) |
| Date Range | How far back to search (7 days – 1 year) |
| Email | SMTP host, credentials, and recipient for auto-delivery (provider wizard auto-fills Gmail/Outlook/iCloud) |
| Schedule | Frequency, days, and time for unattended runs (installs a Windows scheduled task or macOS launchd plist) |

### Where secrets live

API keys and the SMTP password are **never** stored in `config.json`. On first save:

- **Windows** — values are encrypted with DPAPI (`CryptProtectData`) and written to `%APPDATA%\com.research.newsletter\secrets.json`. The ciphertext is bound to your Windows user, so an exfiltrated file cannot be decrypted elsewhere.
- **macOS** — values go into the login keychain as a generic password under service `com.research.newsletter`.
- **Linux** — plaintext sidecar with a stderr warning. (libsecret/Secret Service integration is on the to-do list.)

Legacy plaintext configs are auto-migrated to the secret store on first read; the keys are then stripped from `config.json`.

---

## How it works

1. **Keyword extraction** — the LLM rewrites your natural-language topic into an academic search string and a short newsletter title.
2. **Paper retrieval** — OpenAlex and/or arXiv are queried in parallel with that string, fetching titles, authors, abstracts, and links.
3. **Curation & summarisation** — the LLM ranks the candidates, picks the ten most relevant, and writes a 2–3 sentence plain-English summary plus a relevance tag for each. Output is Markdown.
4. **Save** — written to `{output_dir}/newsletter-YYYY-MM-DD.md`, with `-2`, `-3`, … suffixes on same-day collisions.
5. **(Optional) Email** — the same Markdown is converted to multipart HTML + plain-text via `pulldown-cmark` and sent over SMTP.

The model only ever sees abstracts (truncated to ~600 characters); full-text PDFs are never fetched.

---

## Project structure

```
├── src/                          # React frontend
│   ├── pages/                    # Home, History, Scanner, Settings, Help
│   ├── components/               # Topic cards, progress log, sidebar, wizards, primitives
│   └── lib/                      # Tauri API wrappers, theme, generation/conflict contexts
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── lib.rs                # Tauri commands & app entry
│   │   ├── main.rs               # Headless --scheduled-run entry point
│   │   ├── pipeline.rs           # Generation pipeline, filename collision handling
│   │   ├── gemini.rs             # Gemini REST client
│   │   ├── claude_api.rs         # Anthropic Messages API client
│   │   ├── local_llm.rs          # Ollama client + token streaming
│   │   ├── hardware.rs           # RAM/CPU-aware local-model recommendations
│   │   ├── conflict.rs           # Multi-query conflict scanner pipeline
│   │   ├── scheduler.rs          # Windows Task Scheduler / launchd glue
│   │   ├── email.rs              # SMTP via lettre, Markdown → HTML
│   │   ├── secrets.rs            # DPAPI / Keychain credential storage
│   │   ├── config.rs             # Config struct, load/save, secret migration
│   │   └── sources/              # OpenAlex + arXiv API clients
│   └── capabilities/             # Tauri permission declarations
└── scripts/                      # Dev-only quality-comparison harnesses
```

---

## Development

```bash
npm run test                           # Vitest (frontend)
npx tsc --noEmit                       # Type-check the frontend
cargo check --manifest-path src-tauri/Cargo.toml
cargo test  --manifest-path src-tauri/Cargo.toml   # MSVC linker needed on Windows
npm run generate:icons                 # Re-rasterise the desktop icon from src/assets/icon-master.svg
```

---

## License

MIT
