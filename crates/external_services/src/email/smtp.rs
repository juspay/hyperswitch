use std::time::Duration;

use base64::Engine;
use common_utils::{consts::BASE64_ENGINE, errors::CustomResult, pii};
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
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::email::{EmailClient, EmailError, EmailResult, EmailSettings, IntermediateString};

fn is_bypassed(bypass_list: Option<&str>, host: &str) -> bool {
    let Some(list) = bypass_list else {
        return false;
    };
    list.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .any(|entry| entry == "*" || host == entry || host.ends_with(&format!(".{entry}")))
}

const MAX_CONNECT_RESPONSE_BYTES: usize = 8192;

async fn connect_direct(host: &str, port: u16) -> Result<TcpStream, SmtpError> {
    TcpStream::connect((host, port))
        .await
        .map_err(SmtpError::DirectConnectionFailed)
}

async fn connect_via_proxy(
    proxy_url: &str,
    smtp_host: &str,
    smtp_port: u16,
) -> Result<TcpStream, SmtpError> {
    let uri = proxy_url
        .parse::<http::Uri>()
        .map_err(SmtpError::ProxyUrlParsingFailed)?;
    if !matches!(uri.scheme_str(), Some("http") | None) {
        return Err(SmtpError::UnsupportedProxyScheme(
            uri.scheme_str().unwrap_or("<none>").to_owned(),
        ));
    }
    let proxy_host = uri.host().ok_or(SmtpError::InvalidProxyUrl)?;
    let proxy_port = uri.port_u16().unwrap_or(80);

    let mut stream = TcpStream::connect((proxy_host, proxy_port))
        .await
        .map_err(SmtpError::ProxyConnectionFailed)?;

    let auth_header = uri
        .authority()
        .and_then(|a| a.as_str().rsplit_once('@').map(|(userinfo, _)| userinfo))
        .map(|userinfo| {
            let decoded = urlencoding::decode(userinfo).unwrap_or_else(|_| userinfo.into());
            format!(
                "Proxy-Authorization: Basic {}\r\n",
                BASE64_ENGINE.encode(decoded.as_bytes())
            )
        })
        .unwrap_or_default();

    let connect_request = format!(
        "CONNECT {smtp_host}:{smtp_port} HTTP/1.1\r\nHost: {smtp_host}:{smtp_port}\r\n{auth_header}\r\n"
    );
    stream
        .write_all(connect_request.as_bytes())
        .await
        .map_err(SmtpError::ProxyConnectionFailed)?;

    read_connect_response(&mut stream).await?;

    Ok(stream)
}

async fn read_connect_response(stream: &mut TcpStream) -> Result<(), SmtpError> {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = stream
            .read(&mut byte)
            .await
            .map_err(SmtpError::ProxyConnectionFailed)?;
        if n == 0 {
            return Err(SmtpError::ProxyConnectFailed(
                "connection closed before CONNECT response completed".to_owned(),
            ));
        }
        header.push(byte[0]);
        if header.len() > MAX_CONNECT_RESPONSE_BYTES {
            return Err(SmtpError::ProxyConnectFailed(
                "CONNECT response headers too large".to_owned(),
            ));
        }
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header = String::from_utf8_lossy(&header);
    let status_line = header.lines().next().unwrap_or_default();
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok());
    if !matches!(status_code, Some(200..=299)) {
        return Err(SmtpError::ProxyConnectFailed(status_line.to_owned()));
    }
    Ok(())
}

/// Client for SMTP server operation
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct SmtpServer {
    /// sender email id
    pub sender: pii::Email,
    /// SMTP server specific configs
    pub smtp_config: SmtpServerConfig,
    /// Comma-separated list of hosts that should bypass the configured proxy.
    pub bypass_proxy_hosts: Option<String>,
}

impl SmtpServer {
    pub(crate) async fn create_client(
        &self,
        proxy_url: Option<&str>,
    ) -> Result<AsyncSmtpConnection, SmtpError> {
        let host = &self.smtp_config.host;
        let port = self.smtp_config.port;
        let client_id = ClientId::default();
        let timeout = Duration::from_secs(self.smtp_config.timeout);

        let use_proxy = proxy_url
            .filter(|url| !url.trim().is_empty())
            .filter(|_| !is_bypassed(self.bypass_proxy_hosts.as_deref(), host));

        let stream = tokio::time::timeout(timeout, async {
            match use_proxy {
                Some(proxy_url) => connect_via_proxy(proxy_url, host, port).await,
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
    pub async fn create(
        conf: &EmailSettings,
        smtp_config: SmtpServerConfig,
        bypass_proxy_hosts: Option<String>,
    ) -> Self {
        Self {
            sender: conf.sender_email.clone(),
            smtp_config: smtp_config.clone(),
            bypass_proxy_hosts,
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
        Ok(())
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
        proxy_url: Option<&String>,
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
            .create_client(proxy_url.map(String::as_str))
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
    /// The configured proxy URL could not be parsed.
    #[error("Failed to parse proxy URL {0:?}")]
    ProxyUrlParsingFailed(http::uri::InvalidUri),
    /// The proxy URL was parsed but has no valid host.
    #[error("Proxy URL has no host")]
    InvalidProxyUrl,
    /// The proxy URL uses a scheme this dialer can't reach (only plain TCP today).
    #[error(
        "Unsupported proxy URL scheme {0:?}: the proxy listener must be reachable over plain TCP"
    )]
    UnsupportedProxyScheme(String),
    /// TCP connection to the proxy itself failed.
    #[error("Failed to connect to proxy: {0:?}")]
    ProxyConnectionFailed(std::io::Error),
    /// The proxy rejected or mishandled the CONNECT request.
    #[error("Proxy CONNECT failed: {0}")]
    ProxyConnectFailed(String),
    /// TCP connection to the SMTP host itself failed (non-proxied direct connect).
    #[error("Failed to connect to SMTP server: {0:?}")]
    DirectConnectionFailed(std::io::Error),
    /// A network step exceeded `smtp_config.timeout`.
    #[error("SMTP operation timed out")]
    Timeout,
}
