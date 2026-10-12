//! SMTP client, optionally tunnelled through a SOCKS5 proxy.

use std::time::Duration;

use lettre::{
    address::AddressError,
    message::{header::ContentType, Mailbox},
    transport::smtp::{
        self,
        authentication::{Credentials, Mechanism},
        client::{AsyncSmtpConnection, AsyncTokioStream, TlsParameters},
        extension::ClientId,
    },
};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

use super::EmailMessage;

/// Redacts a credential in `Debug` output.
fn redacted(value: &Option<String>) -> Option<&'static str> {
    value.as_ref().map(|_| "***")
}

/// How to reach the SMTP server.
#[derive(Clone, Default, serde::Deserialize)]
pub struct SmtpConfig {
    /// The server's host, e.g. `smtp.gmail.com`.
    pub host: String,
    /// The server's port, e.g. `25`.
    pub port: u16,
    /// Timeout of each network step (connect, handshake, STARTTLS, AUTH, send, QUIT), in seconds.
    pub timeout: u64,
    /// Login username; with `password`, the client authenticates.
    pub username: Option<String>,
    /// Login password.
    pub password: Option<String>,
    /// Whether the connection upgrades to TLS.
    #[serde(default)]
    pub connection: SmtpConnection,
    /// SOCKS5 proxy to tunnel the connection through; absent means connect directly.
    #[serde(default)]
    pub socks5: Option<Socks5Config>,
}

impl std::fmt::Debug for SmtpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmtpConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("timeout", &self.timeout)
            .field("username", &redacted(&self.username))
            .field("password", &redacted(&self.password))
            .field("connection", &self.connection)
            .field("socks5", &self.socks5)
            .finish()
    }
}

/// A SOCKS5 proxy. Username/password auth (RFC 1929) is sent to the proxy in cleartext.
#[derive(Clone, Default, serde::Deserialize)]
pub struct Socks5Config {
    /// The proxy's hostname or IP.
    pub host: String,
    /// The proxy's port.
    pub port: u16,
    /// Proxy username, if the proxy requires auth; with `password`.
    pub username: Option<String>,
    /// Proxy password.
    pub password: Option<String>,
}

impl std::fmt::Debug for Socks5Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Socks5Config")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &redacted(&self.username))
            .field("password", &redacted(&self.password))
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
    /// The server refused or failed the `MAIL FROM`/`RCPT TO`/`DATA` exchange.
    #[error("Failed to send email")]
    SendingFailure(#[source] smtp::Error),
    /// The server rejected the login.
    #[error("SMTP authentication failed")]
    AuthenticationFailure(#[source] smtp::Error),
    /// The SMTP handshake or STARTTLS upgrade failed.
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
    /// The SOCKS5 proxy's hostname could not be resolved.
    #[error("Failed to resolve the SOCKS5 proxy address")]
    Socks5ProxyResolutionFailed(#[source] std::io::Error),
    /// The SOCKS5 tunnel could not be set up (proxy unreachable, auth rejected, protocol error).
    #[error("SOCKS5 proxy connection failed")]
    Socks5ConnectionFailed(#[source] tokio_socks::Error),
    /// The TCP connection to the SMTP server failed.
    #[error("Failed to connect to the SMTP server")]
    DirectConnectionFailed(#[source] std::io::Error),
    /// A network step exceeded the configured timeout.
    #[error("SMTP operation timed out: {0}")]
    Timeout(&'static str),
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

    /// Sends `message` over a new connection, through the SOCKS5 proxy when one is configured.
    pub async fn send(&self, message: EmailMessage) -> Result<(), SmtpError> {
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

        let config = &self.config;
        let mut conn = self.connect().await?;
        let credentials = config
            .username
            .clone()
            .zip(config.password.clone())
            .map(|(username, password)| Credentials::new(username, password));
        if let Some(credentials) = credentials {
            self.timed(
                "AUTH",
                conn.auth(&[Mechanism::Plain, Mechanism::Login], &credentials),
            )
            .await?
            .map_err(SmtpError::AuthenticationFailure)?;
        }
        self.timed("SEND", conn.send(email.envelope(), &email.formatted()))
            .await?
            .map_err(SmtpError::SendingFailure)?;

        // The message is already accepted, so a failed QUIT is only logged.
        match self.timed("QUIT", conn.quit()).await {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => tracing::warn!(?error, "SMTP QUIT failed after the message was sent"),
            Err(error) => tracing::warn!(%error, "SMTP QUIT failed after the message was sent"),
        }
        Ok(())
    }

    /// Runs one network step within the configured timeout.
    async fn timed<T>(
        &self,
        step: &'static str,
        future: impl std::future::Future<Output = T>,
    ) -> Result<T, SmtpError> {
        tokio::time::timeout(Duration::from_secs(self.config.timeout), future)
            .await
            .map_err(|_| SmtpError::Timeout(step))
    }

    /// Opens the TCP stream (directly or through the proxy), then the SMTP session, upgrading it
    /// with STARTTLS when configured.
    async fn connect(&self) -> Result<AsyncSmtpConnection, SmtpError> {
        let config = &self.config;
        let client_id = ClientId::default();
        let stream = self
            .timed("connect", async {
                match config.socks5.as_ref() {
                    Some(socks5) => connect_via_socks5(socks5, &config.host, config.port).await,
                    None => TcpStream::connect((config.host.as_str(), config.port))
                        .await
                        .map_err(SmtpError::DirectConnectionFailed),
                }
            })
            .await??;
        let stream: Box<dyn AsyncTokioStream> = Box::new(stream);

        let mut conn = self
            .timed(
                "connect handshake",
                AsyncSmtpConnection::connect_with_transport(stream, &client_id),
            )
            .await?
            .map_err(SmtpError::ConnectionFailure)?;
        if matches!(config.connection, SmtpConnection::StartTls) {
            let tls_parameters = TlsParameters::builder(config.host.clone())
                .build()
                .map_err(SmtpError::ConnectionFailure)?;
            self.timed("STARTTLS", conn.starttls(tls_parameters, &client_id))
                .await?
                .map_err(SmtpError::ConnectionFailure)?;
        }
        Ok(conn)
    }
}

/// Tunnels to the SMTP server through the proxy, trying each address the proxy resolves to.
async fn connect_via_socks5(
    socks5: &Socks5Config,
    smtp_host: &str,
    smtp_port: u16,
) -> Result<TcpStream, SmtpError> {
    let proxy_addrs = tokio::net::lookup_host((socks5.host.as_str(), socks5.port))
        .await
        .map_err(SmtpError::Socks5ProxyResolutionFailed)?;
    let target = (smtp_host, smtp_port);

    let mut last_error = SmtpError::Socks5ProxyResolutionFailed(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "proxy hostname resolved to no addresses",
    ));
    for proxy_addr in proxy_addrs {
        let result = match socks5.username.as_ref().zip(socks5.password.as_ref()) {
            Some((username, password)) => {
                Socks5Stream::connect_with_password(proxy_addr, target, username, password).await
            }
            None => Socks5Stream::connect(proxy_addr, target).await,
        };
        match result {
            Ok(stream) => return Ok(stream.into_inner()),
            Err(error) => {
                tracing::warn!(
                    ?error,
                    proxy_host = %socks5.host,
                    proxy_port = socks5.port,
                    %proxy_addr,
                    "SOCKS5 connection attempt failed, trying the next resolved address if any"
                );
                last_error = SmtpError::Socks5ConnectionFailed(error);
            }
        }
    }
    Err(last_error)
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
        let config = SmtpConfig {
            socks5: Some(Socks5Config {
                host: "proxy.example.com".to_owned(),
                username: Some("proxy-user".to_owned()),
                password: Some("proxy-pass".to_owned()),
                ..Socks5Config::default()
            }),
            ..config
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("smtp.example.com") && debug.contains("proxy.example.com"));
        assert!(!debug.contains("user\"") && !debug.contains("hunter2"));
        assert!(!debug.contains("proxy-user") && !debug.contains("proxy-pass"));
    }

    #[tokio::test]
    async fn an_unparsable_address_names_itself() {
        let client = SmtpClient::new(SmtpConfig::default());
        let message = EmailMessage {
            from: "not an address".to_owned(),
            to: vec!["to@example.com".to_owned()],
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: "s".to_owned(),
            html_body: "<p>b</p>".to_owned(),
        };
        match client.send(message).await {
            Err(SmtpError::EmailParsingFailed { address, .. }) => {
                assert_eq!(address, "not an address")
            }
            other => panic!("expected a parse failure, got {other:?}"),
        }
    }
}
