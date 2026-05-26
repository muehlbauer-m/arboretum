use crate::config::EmailConfig;
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use pulldown_cmark::{Parser, html};

/// Convert Markdown text to a complete HTML document with inline styles
/// suitable for rendering in email clients.
fn markdown_to_html(md: &str) -> String {
    let parser = Parser::new(md);
    let mut html_body = String::new();
    html::push_html(&mut html_body, parser);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"></head>
<body style="margin:0;padding:0;background-color:#f4f4f7;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#1a1a2e;line-height:1.6;">
  <div style="max-width:640px;margin:24px auto;background-color:#ffffff;border-radius:8px;padding:32px 40px;border:1px solid #e0e0e6;">
    <div style="font-size:16px;color:#1a1a2e;">
      {html_body}
    </div>
    <hr style="border:none;border-top:1px solid #e0e0e6;margin:32px 0 16px;">
    <p style="font-size:13px;color:#6b7280;margin:0;text-align:center;">Sent by Arboretum</p>
  </div>
</body>
</html>"#
    )
}

/// Send `content` (Markdown text) as an email using the given SMTP config.
pub fn send_email(
    content: &str,
    subject: &str,
    email_cfg: &EmailConfig,
) -> Result<(), String> {
    if !email_cfg.enabled {
        return Err("Email sending is disabled in settings.".to_string());
    }
    if email_cfg.smtp_host.is_empty() {
        return Err("SMTP host is not configured.".to_string());
    }
    if email_cfg.recipient.is_empty() {
        return Err("Recipient email is not configured.".to_string());
    }
    if email_cfg.smtp_user.is_empty() {
        return Err("SMTP username is not configured.".to_string());
    }
    if email_cfg.smtp_password.is_empty() {
        return Err(
            "SMTP password is not configured. Enter it in Settings → Email and click Save changes."
                .to_string(),
        );
    }

    let from = email_cfg
        .smtp_user
        .parse::<lettre::message::Mailbox>()
        .map_err(|e| format!("Invalid 'from' address '{}': {e}", email_cfg.smtp_user))?;

    let to = email_cfg
        .recipient
        .parse::<lettre::message::Mailbox>()
        .map_err(|e| format!("Invalid recipient address '{}': {e}", email_cfg.recipient))?;

    let html_body = markdown_to_html(content);
    let plain_body = format!("{content}\n\n---\nSent by Arboretum");

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(plain_body),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body),
                ),
        )
        .map_err(|e| format!("Failed to build email message: {e}"))?;

    let creds = Credentials::new(
        email_cfg.smtp_user.clone(),
        email_cfg.smtp_password.clone(),
    );

    let mailer = SmtpTransport::starttls_relay(&email_cfg.smtp_host)
        .map_err(|e| format!("Failed to connect to SMTP host '{}': {e}", email_cfg.smtp_host))?
        .port(email_cfg.smtp_port)
        .credentials(creds)
        .build();

    mailer
        .send(&email)
        .map(|_| ())
        .map_err(|e| format!("Failed to send email: {e}"))
}

/// Open an SMTP connection with the given credentials, perform the full
/// EHLO → STARTTLS → AUTH → QUIT handshake, and close it — without sending
/// any mail.
///
/// We use the low-level `SmtpConnection` API instead of
/// `SmtpTransport::test_connection()` because the high-level version only
/// issues NOOP, which most servers answer without ever authenticating. NOOP
/// tells you "reachable", not "credentials work". The auth step here is
/// what actually verifies the password — its failure is the same error the
/// eventual `send_email` call would hit.
pub fn test_connection(email_cfg: &EmailConfig) -> Result<(), String> {
    use lettre::transport::smtp::authentication::Mechanism;
    use lettre::transport::smtp::client::{SmtpConnection, TlsParameters};
    use lettre::transport::smtp::extension::ClientId;
    use std::time::Duration;

    if email_cfg.smtp_host.is_empty() {
        return Err("SMTP host is not configured.".to_string());
    }
    if email_cfg.smtp_user.is_empty() {
        return Err("SMTP username is not configured.".to_string());
    }
    if email_cfg.smtp_password.is_empty() {
        return Err("SMTP password is not configured.".to_string());
    }

    let host = email_cfg.smtp_host.as_str();
    let port = email_cfg.smtp_port;
    let hello_name = ClientId::default();
    let timeout = Some(Duration::from_secs(10));

    let tls_params = TlsParameters::new(host.to_string())
        .map_err(|e| format!("TLS setup for '{host}' failed: {e}"))?;

    // 465 = implicit TLS (SMTPS). 587 (and most others) = plaintext connect
    // then STARTTLS upgrade.
    let mut conn = if port == 465 {
        SmtpConnection::connect(
            (host, port),
            timeout,
            &hello_name,
            Some(&tls_params),
            None,
        )
        .map_err(|e| format!("Connect to {host}:{port} failed: {e}"))?
    } else {
        let mut c = SmtpConnection::connect(
            (host, port),
            timeout,
            &hello_name,
            None,
            None,
        )
        .map_err(|e| format!("Connect to {host}:{port} failed: {e}"))?;
        c.starttls(&tls_params, &hello_name)
            .map_err(|e| format!("STARTTLS upgrade failed: {e}"))?;
        c
    };

    let creds = Credentials::new(
        email_cfg.smtp_user.clone(),
        email_cfg.smtp_password.clone(),
    );

    conn.auth(&[Mechanism::Plain, Mechanism::Login], &creds)
        .map_err(|e| format!("Authentication failed: {e}"))?;

    let _ = conn.quit();
    Ok(())
}
