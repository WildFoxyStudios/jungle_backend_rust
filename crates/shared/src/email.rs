use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use once_cell::sync::OnceCell;
use std::sync::Arc;

static EMAIL_SENDER: OnceCell<Arc<EmailSender>> = OnceCell::new();

pub struct EmailSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_mailbox: Mailbox,
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub encryption: String,
    pub from_email: String,
    pub from_name: String,
}

impl SmtpConfig {
    pub fn from_env() -> Option<Self> {
        let host = std::env::var("SMTP_HOST").ok()?;
        if host.is_empty() {
            return None;
        }
        Some(Self {
            host,
            port: std::env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            username: std::env::var("SMTP_USERNAME").unwrap_or_default(),
            password: std::env::var("SMTP_PASSWORD").unwrap_or_default(),
            encryption: std::env::var("SMTP_ENCRYPTION").unwrap_or_else(|_| "tls".into()),
            from_email: std::env::var("SMTP_FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@example.com".into()),
            from_name: std::env::var("SMTP_FROM_NAME")
                .unwrap_or_else(|_| "Jungle".into()),
        })
    }
}

impl EmailSender {
    pub fn new(config: SmtpConfig) -> Result<Self, String> {
        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let transport = match config.encryption.as_str() {
            "ssl" => {
                let builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                    .map_err(|e| format!("SMTP relay error: {}", e))?;
                builder.credentials(creds).port(config.port).build()
            }
            "starttls" | "tls" => {
                let builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                    .map_err(|e| format!("SMTP starttls error: {}", e))?;
                builder.credentials(creds).port(config.port).build()
            }
            _ => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
                .port(config.port)
                .build(),
        };

        let from_mailbox = format!("{} <{}>", config.from_name, config.from_email)
            .parse()
            .map_err(|e| format!("Invalid from address: {}", e))?;

        Ok(Self {
            transport,
            from_mailbox,
        })
    }

    pub fn init_global(config: SmtpConfig) -> Result<(), String> {
        let sender = Self::new(config)?;
        EMAIL_SENDER
            .set(Arc::new(sender))
            .map_err(|_| "EmailSender already initialized".to_string())
    }

    pub fn global() -> Option<&'static Arc<EmailSender>> {
        EMAIL_SENDER.get()
    }

    pub async fn send(
        &self,
        to_email: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), String> {
        let to_mailbox: Mailbox = to_email
            .parse()
            .map_err(|e| format!("Invalid recipient: {}", e))?;

        let message = Message::builder()
            .from(self.from_mailbox.clone())
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| format!("Message build error: {}", e))?;

        self.transport
            .send(message)
            .await
            .map_err(|e| format!("SMTP send error: {}", e))?;

        Ok(())
    }
}

pub async fn send_email(to: &str, subject: &str, html_body: &str) -> Result<(), String> {
    match EmailSender::global() {
        Some(sender) => sender.send(to, subject, html_body).await,
        None => {
            tracing::warn!(to, subject, "Email not sent — SMTP not configured");
            Ok(())
        }
    }
}
