use std::time::Duration;

use common_utils::{errors::CustomResult, pii};
use error_stack::ResultExt;
use lettre::{
    address::AddressError,
    error,
    message::{header::ContentType, Mailbox},
    transport::smtp::{self, authentication::Credentials},
    Message, SmtpTransport, Transport,
};
use masking::{PeekInterface, Secret};

use crate::email::{EmailClient, EmailError, EmailResult, EmailSettings, IntermediateString};

/// Client for SMTP server operation
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct SmtpServer {
    /// sender email id
    pub sender: String,
    /// SMTP server specific configs
    pub smtp_config: SmtpServerConfig,
}

impl SmtpServer {
    /// A helper function to create SMTP server client
    pub fn create_client(&self) -> Result<SmtpTransport, SmtpError> {
        let host = self.smtp_config.host.clone();
        let port = self.smtp_config.port;
        let timeout = Some(Duration::from_secs(self.smtp_config.timeout));
        let credentials = self
            .smtp_config
            .username
            .clone()
            .zip(self.smtp_config.password.clone())
            .map(|(username, password)| {
                Credentials::new(username.peek().to_owned(), password.peek().to_owned())
            });
        match &self.smtp_config.connection {
            SmtpConnection::StartTls => match credentials {
                Some(credentials) => Ok(SmtpTransport::starttls_relay(&host)
                    .map_err(SmtpError::ConnectionFailure)?
                    .port(port)
                    .timeout(timeout)
                    .credentials(credentials)
                    .build()),
                None => Ok(SmtpTransport::starttls_relay(&host)
                    .map_err(SmtpError::ConnectionFailure)?
                    .port(port)
                    .timeout(timeout)
                    .build()),
            },
            SmtpConnection::Plaintext => match credentials {
                Some(credentials) => Ok(SmtpTransport::builder_dangerous(&host)
                    .port(port)
                    .timeout(timeout)
                    .credentials(credentials)
                    .build()),
                None => Ok(SmtpTransport::builder_dangerous(&host)
                    .port(port)
                    .timeout(timeout)
                    .build()),
            },
        }
    }
    /// Constructs a new SMTP client
    pub async fn create(conf: &EmailSettings, smtp_config: SmtpServerConfig) -> Self {
        Self {
            sender: conf.sender_email.clone(),
            smtp_config: smtp_config.clone(),
        }
    }
    /// helper function to convert email id into Mailbox
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
        _proxy_url: Option<&String>,
    ) -> EmailResult<()> {
        // Create a client every time when the email is being sent
        let email_client =
            Self::create_client(self).change_context(EmailError::EmailSendingFailure)?;

        let email = Message::builder()
            .to(Self::to_mail_box(recipient.peek().to_string())?)
            .from(Self::to_mail_box(self.sender.clone())?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body)
            .map_err(SmtpError::MessageBuildingFailed)
            .change_context(EmailError::EmailSendingFailure)?;

        email_client
            .send(&email)
            .map_err(SmtpError::SendingFailure)
            .change_context(EmailError::EmailSendingFailure)?;
        Ok(())
    }
}

/// Errors that could occur during SES operations.
#[derive(Debug, thiserror::Error)]
pub enum SmtpError {
    /// An error occurred in the SMTP while sending email.
    #[error("Failed to Send Email {0:?}")]
    SendingFailure(smtp::Error),
    /// An error occurred in the SMTP while building the message content.
    #[error("Failed to create connection {0:?}")]
    ConnectionFailure(smtp::Error),
    /// An error occurred in the SMTP while building the message content.
    #[error("Failed to Build Email content {0:?}")]
    MessageBuildingFailed(error::Error),
    /// An error occurred in the SMTP while building the message content.
    #[error("Failed to parse given email {0:?}")]
    EmailParsingFailed(AddressError),
}
