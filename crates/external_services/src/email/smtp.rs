use std::time::Duration;

use common_utils::{errors::CustomResult, pii};
use error_stack::ResultExt;
use hyperswitch_masking::{PeekInterface, Secret};
use lettre::{
    address::AddressError,
    error,
    message::{header::ContentType, Mailbox},
    transport::smtp::{
        self,
        authentication::{Credentials, Mechanism},
        client::{AsyncSmtpConnection, AsyncTokioStream, TlsParameters},
        extension::ClientId,
    },
    Message,
};
use router_env::logger;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

use crate::email::{EmailClient, EmailError, EmailResult, EmailSettings, IntermediateString};

async fn connect_direct(host: &str, port: u16) -> Result<TcpStream, SmtpError> {
    TcpStream::connect((host, port))
        .await
        .map_err(SmtpError::DirectConnectionFailed)
}

/// SOCKS5 proxy to tunnel the SMTP connection through (absent means connect directly); username/password auth (RFC 1929) is sent to the proxy in cleartext.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Socks5Config {
    /// SOCKS5 proxy hostname or IP.
    pub host: String,
    /// SOCKS5 proxy port.
    pub port: u16,
    /// SOCKS5 username, if the proxy requires auth.
    pub username: Option<Secret<String>>,
    /// SOCKS5 password, if the proxy requires auth.
    pub password: Option<Secret<String>>,
}

async fn connect_via_socks5(
    socks5: &Socks5Config,
    smtp_host: &str,
    smtp_port: u16,
) -> Result<TcpStream, SmtpError> {
    let proxy_addr = tokio::net::lookup_host((socks5.host.as_str(), socks5.port))
        .await
        .map_err(SmtpError::Socks5ProxyResolutionFailed)?
        .next()
        .ok_or_else(|| {
            SmtpError::Socks5ProxyResolutionFailed(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "proxy hostname resolved to no addresses",
            ))
        })?;

    let target = (smtp_host, smtp_port);

    let stream = match socks5.username.as_ref().zip(socks5.password.as_ref()) {
        Some((username, password)) => {
            Socks5Stream::connect_with_password(
                proxy_addr,
                target,
                username.peek().as_str(),
                password.peek().as_str(),
            )
            .await
        }
        None => Socks5Stream::connect(proxy_addr, target).await,
    }
    .map_err(|error| {
        logger::warn!(
            ?error,
            proxy_host = %socks5.host,
            proxy_port = socks5.port,
            "SOCKS5 proxy connection failed"
        );
        SmtpError::Socks5ConnectionFailed(error)
    })?;

    Ok(stream.into_inner())
}

/// Client for SMTP server operation
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct SmtpServer {
    /// sender email id
    pub sender: pii::Email,
    /// SMTP server specific configs
    pub smtp_config: SmtpServerConfig,
}

impl SmtpServer {
    pub(crate) async fn create_client(&self) -> Result<AsyncSmtpConnection, SmtpError> {
        let host = &self.smtp_config.host;
        let port = self.smtp_config.port;
        let client_id = ClientId::default();
        let timeout = Duration::from_secs(self.smtp_config.timeout);

        let stream = tokio::time::timeout(timeout, async {
            match self.smtp_config.socks5.as_ref() {
                Some(socks5) => connect_via_socks5(socks5, host, port).await,
                None => connect_direct(host, port).await,
            }
        })
        .await
        .map_err(|_| SmtpError::Timeout)??;

        let stream: Box<dyn AsyncTokioStream> = Box::new(stream);

        let mut conn = tokio::time::timeout(
            timeout,
            AsyncSmtpConnection::connect_with_transport(stream, &client_id),
        )
        .await
        .map_err(|_| SmtpError::Timeout)?
        .map_err(SmtpError::ConnectionFailure)?;

        if matches!(self.smtp_config.connection, SmtpConnection::StartTls) {
            let tls_parameters = TlsParameters::builder(host.clone())
                .build()
                .map_err(SmtpError::ConnectionFailure)?;
            tokio::time::timeout(timeout, conn.starttls(tls_parameters, &client_id))
                .await
                .map_err(|_| SmtpError::Timeout)?
                .map_err(SmtpError::ConnectionFailure)?;
        }

        Ok(conn)
    }
    /// Constructs a new SMTP client
    pub async fn create(conf: &EmailSettings, smtp_config: SmtpServerConfig) -> Self {
        Self {
            sender: conf.sender_email.clone(),
            smtp_config: smtp_config.clone(),
        }
    }
    fn to_mail_box(email: String) -> EmailResult<Mailbox> {
        Ok(Mailbox::new(
            None,
            email
                .parse()
                .map_err(SmtpError::EmailParsingFailed)
                .change_context(EmailError::EmailSendingFailure)?,
        ))
    }
}
/// Struct that contains the SMTP server specific configs required
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SmtpServerConfig {
    /// hostname of the SMTP server eg: smtp.gmail.com
    pub host: String,
    /// portname of the SMTP server eg: 25
    pub port: u16,
    /// timeout for the SMTP server connection in seconds eg: 10
    pub timeout: u64,
    /// Username name of the SMTP server
    pub username: Option<Secret<String>>,
    /// Password of the SMTP server
    pub password: Option<Secret<String>>,
    /// Connection type of the SMTP server
    #[serde(default)]
    pub connection: SmtpConnection,
    /// SOCKS5 proxy config; absent means connect directly.
    pub socks5: Option<Socks5Config>,
}

/// Enum that contains the connection types of the SMTP server
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmtpConnection {
    #[default]
    /// Plaintext connection which MUST then successfully upgrade to TLS via STARTTLS
    StartTls,
    /// Plaintext connection (very insecure)
    Plaintext,
}

impl SmtpServerConfig {
    /// Validation for the SMTP server client specific configs
    pub fn validate(&self) -> Result<(), &'static str> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};
        when(self.host.is_default_or_empty(), || {
            Err("email.smtp.host must not be empty")
        })?;
        self.username.clone().zip(self.password.clone()).map_or(
            Ok(()),
            |(username, password)| {
                when(username.peek().is_default_or_empty(), || {
                    Err("email.smtp.username must not be empty")
                })?;
                when(password.peek().is_default_or_empty(), || {
                    Err("email.smtp.password must not be empty")
                })
            },
        )?;
        self.socks5.as_ref().map_or(Ok(()), Socks5Config::validate)
    }
}

impl Socks5Config {
    fn validate(&self) -> Result<(), &'static str> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};
        when(self.host.is_default_or_empty(), || {
            Err("email.smtp.socks5.host must not be empty")
        })?;
        when(self.port == 0, || {
            Err("email.smtp.socks5.port must not be zero")
        })?;
        match (self.username.as_ref(), self.password.as_ref()) {
            (Some(username), Some(password)) => {
                when(username.peek().is_default_or_empty(), || {
                    Err("email.smtp.socks5.username must not be empty")
                })?;
                when(password.peek().is_default_or_empty(), || {
                    Err("email.smtp.socks5.password must not be empty")
                })
            }
            (None, None) => Ok(()),
            _ => Err("email.smtp.socks5 username and password must both be set, or neither"),
        }
    }
}

#[async_trait::async_trait]
impl EmailClient for SmtpServer {
    type RichText = String;
    fn convert_to_rich_text(
        &self,
        intermediate_string: IntermediateString,
    ) -> CustomResult<Self::RichText, EmailError> {
        Ok(intermediate_string.into_inner())
    }

    async fn send_email(
        &self,
        recipient: pii::Email,
        subject: String,
        body: Self::RichText,
        // Unused here — SES uses this; SMTP's proxy lives in smtp_config.socks5.
        _proxy_url: Option<&String>,
    ) -> EmailResult<()> {
        let email = Message::builder()
            .to(Self::to_mail_box(recipient.peek().to_string())?)
            .from(Self::to_mail_box(self.sender.peek().to_string())?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body)
            .map_err(SmtpError::MessageBuildingFailed)
            .change_context(EmailError::EmailSendingFailure)?;

        let mut conn = self
            .create_client()
            .await
            .change_context(EmailError::EmailSendingFailure)?;

        let timeout = Duration::from_secs(self.smtp_config.timeout);

        let credentials = self
            .smtp_config
            .username
            .clone()
            .zip(self.smtp_config.password.clone())
            .map(|(username, password)| {
                Credentials::new(username.peek().to_owned(), password.peek().to_owned())
            });

        if let Some(credentials) = credentials {
            tokio::time::timeout(
                timeout,
                conn.auth(&[Mechanism::Plain, Mechanism::Login], &credentials),
            )
            .await
            .map_err(|_| SmtpError::Timeout)
            .change_context(EmailError::EmailSendingFailure)?
            .map_err(SmtpError::AuthenticationFailure)
            .change_context(EmailError::EmailSendingFailure)?;
        }

        tokio::time::timeout(timeout, conn.send(email.envelope(), &email.formatted()))
            .await
            .map_err(|_| SmtpError::Timeout)
            .change_context(EmailError::EmailSendingFailure)?
            .map_err(SmtpError::SendingFailure)
            .change_context(EmailError::EmailSendingFailure)?;

        match tokio::time::timeout(timeout, conn.quit()).await {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                logger::warn!(?error, "SMTP QUIT failed after message was already sent");
            }
            Err(_) => {
                logger::warn!("SMTP QUIT timed out after message was already sent");
            }
        }

        Ok(())
    }
}

/// Errors that could occur during SMTP operations.
#[derive(Debug, thiserror::Error)]
pub enum SmtpError {
    /// An error occurred in the SMTP client while sending the email (the SMTP `MAIL
    /// FROM`/`RCPT TO`/`DATA` exchange).
    #[error("Failed to Send Email {0:?}")]
    SendingFailure(smtp::Error),
    /// SMTP authentication (AUTH command) failed.
    #[error("SMTP authentication failed: {0:?}")]
    AuthenticationFailure(smtp::Error),
    /// The SMTP connect handshake or STARTTLS upgrade failed.
    #[error("Failed to create connection {0:?}")]
    ConnectionFailure(smtp::Error),
    /// The outgoing email's message content (headers/body) could not be built.
    #[error("Failed to Build Email content {0:?}")]
    MessageBuildingFailed(error::Error),
    /// The sender or recipient email address could not be parsed.
    #[error("Failed to parse given email {0:?}")]
    EmailParsingFailed(AddressError),
    /// The configured SOCKS5 proxy hostname could not be resolved.
    #[error("Failed to resolve SOCKS5 proxy address: {0:?}")]
    Socks5ProxyResolutionFailed(std::io::Error),
    /// Establishing the SOCKS5 tunnel failed (proxy unreachable, auth rejected, or protocol error).
    #[error("SOCKS5 proxy connection failed: {0}")]
    Socks5ConnectionFailed(tokio_socks::Error),
    /// TCP connection to the SMTP host itself failed (non-proxied direct connect).
    #[error("Failed to connect to SMTP server: {0:?}")]
    DirectConnectionFailed(std::io::Error),
    /// A network step exceeded `smtp_config.timeout`.
    #[error("SMTP operation timed out")]
    Timeout,
}
