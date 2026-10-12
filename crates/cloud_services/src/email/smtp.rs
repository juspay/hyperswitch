//! SMTP client.

use std::time::Duration;

use lettre::{
    address::AddressError,
    message::{header::ContentType, Mailbox},
    transport::smtp::{self, authentication::Credentials},
    SmtpTransport, Transport,
};

use super::EmailMessage;

/// How to reach the SMTP server.
#[derive(Clone, Default, serde::Deserialize)]
pub struct SmtpConfig {
    /// The server's host, e.g. `smtp.gmail.com`.
    pub host: String,
    /// The server's port, e.g. `25`.
    pub port: u16,
    /// Connection timeout, in seconds.
    pub timeout: u64,
    /// Login username; with `password`, the client authenticates.
    pub username: Option<String>,
    /// Login password.
    pub password: Option<String>,
    /// Whether the connection upgrades to TLS.
    #[serde(default)]
    pub connection: SmtpConnection,
}

impl std::fmt::Debug for SmtpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let redacted = |value: &Option<String>| value.as_ref().map(|_| "***");
        f.debug_struct("SmtpConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("timeout", &self.timeout)
            .field("username", &redacted(&self.username))
            .field("password", &redacted(&self.password))
            .field("connection", &self.connection)
            .finish()
    }
}

/// The SMTP connection's security.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmtpConnection {
    /// Plaintext connection which MUST then successfully upgrade to TLS via STARTTLS.
    #[default]
    StartTls,
    /// Plaintext connection (very insecure).
    Plaintext,
}

/// Errors from sending through SMTP.
#[derive(Debug, thiserror::Error)]
pub enum SmtpError {
    /// The server refused or failed to send.
    #[error("Failed to send email")]
    SendingFailure(#[source] smtp::Error),
    /// The connection could not be set up.
    #[error("Failed to create the SMTP connection")]
    ConnectionFailure(#[source] smtp::Error),
    /// The message could not be built.
    #[error("Failed to build the email content")]
    MessageBuildingFailed(#[source] lettre::error::Error),
    /// An address could not be parsed.
    #[error("Failed to parse the email address {address:?}")]
    EmailParsingFailed {
        /// The address.
        address: String,
        /// The parse error.
        #[source]
        source: AddressError,
    },
}

/// Sends email through an SMTP server.
#[derive(Clone, Debug)]
pub struct SmtpClient {
    config: SmtpConfig,
}

impl SmtpClient {
    /// Creates the client; nothing is contacted until [`Self::send`].
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    /// Sends `message` over a new connection. Blocks while the server responds.
    pub fn send(&self, message: EmailMessage) -> Result<(), SmtpError> {
        let mut builder = lettre::Message::builder()
            .from(mailbox(message.from)?)
            .subject(message.subject)
            .header(ContentType::TEXT_HTML);
        for address in message.to {
            builder = builder.to(mailbox(address)?);
        }
        for address in message.cc {
            builder = builder.cc(mailbox(address)?);
        }
        for address in message.bcc {
            builder = builder.bcc(mailbox(address)?);
        }
        let email = builder
            .body(message.html_body)
            .map_err(SmtpError::MessageBuildingFailed)?;

        self.transport()?
            .send(&email)
            .map_err(SmtpError::SendingFailure)?;
        Ok(())
    }

    fn transport(&self) -> Result<SmtpTransport, SmtpError> {
        let config = &self.config;
        let builder = match config.connection {
            SmtpConnection::StartTls => {
                SmtpTransport::starttls_relay(&config.host).map_err(SmtpError::ConnectionFailure)?
            }
            SmtpConnection::Plaintext => SmtpTransport::builder_dangerous(&config.host),
        }
        .port(config.port)
        .timeout(Some(Duration::from_secs(config.timeout)));

        let credentials = config
            .username
            .clone()
            .zip(config.password.clone())
            .map(|(username, password)| Credentials::new(username, password));
        Ok(match credentials {
            Some(credentials) => builder.credentials(credentials).build(),
            None => builder.build(),
        })
    }
}

fn mailbox(address: String) -> Result<Mailbox, SmtpError> {
    match address.parse() {
        Ok(address) => Ok(Mailbox::new(None, address)),
        Err(source) => Err(SmtpError::EmailParsingFailed { address, source }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_credentials() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_owned(),
            username: Some("user".to_owned()),
            password: Some("hunter2".to_owned()),
            ..SmtpConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("smtp.example.com"));
        assert!(!debug.contains("user\"") && !debug.contains("hunter2"));
    }

    #[test]
    fn an_unparsable_address_names_itself() {
        let client = SmtpClient::new(SmtpConfig::default());
        let message = EmailMessage {
            from: "not an address".to_owned(),
            to: vec!["to@example.com".to_owned()],
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: "s".to_owned(),
            html_body: "<p>b</p>".to_owned(),
        };
        match client.send(message) {
            Err(SmtpError::EmailParsingFailed { address, .. }) => {
                assert_eq!(address, "not an address")
            }
            other => panic!("expected a parse failure, got {other:?}"),
        }
    }
}
