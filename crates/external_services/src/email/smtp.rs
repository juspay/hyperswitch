//! SMTP email client.
//!
//! Hyperswitch adapter over the shared [`cloud_services::email::smtp`] client. The configuration
//! stays here so its credentials remain masked [`Secret`]s.

pub use cloud_services::email::smtp::{SmtpConnection, SmtpError};
use cloud_services::email::{smtp as shared, EmailMessage};
use common_utils::{errors::CustomResult, pii};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::{
    email::{EmailClient, EmailError, EmailResult, EmailSettings, IntermediateString},
    report_with_cause,
};

/// Client for SMTP server operation
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct SmtpServer {
    /// sender email id
    pub sender: pii::Email,
    /// SMTP server specific configs
    pub smtp_config: SmtpServerConfig,
}

impl SmtpServer {
    /// Constructs a new SMTP client
    pub async fn create(conf: &EmailSettings, smtp_config: SmtpServerConfig) -> Self {
        Self {
            sender: conf.sender_email.clone(),
            smtp_config,
        }
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

    fn to_shared(&self) -> shared::SmtpConfig {
        shared::SmtpConfig {
            host: self.host.clone(),
            port: self.port,
            timeout: self.timeout,
            username: self.username.as_ref().map(|value| value.peek().to_owned()),
            password: self.password.as_ref().map(|value| value.peek().to_owned()),
            connection: self.connection.clone(),
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
        _proxy_url: Option<&String>,
    ) -> EmailResult<()> {
        // A new connection for every email
        let message = EmailMessage {
            from: self.sender.peek().to_owned(),
            to: vec![recipient.peek().to_owned()],
            cc: Vec::new(),
            bcc: Vec::new(),
            subject,
            html_body: body,
        };
        shared::SmtpClient::new(self.smtp_config.to_shared())
            .send(message)
            .map_err(|error| {
                report_with_cause(error).change_context(EmailError::EmailSendingFailure)
            })
    }
}
