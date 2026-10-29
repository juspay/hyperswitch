//! SMTP email client.
//!
//! Hyperswitch adapter over the shared [`cloud_services::email::smtp`] client. The configuration
//! stays here so its credentials remain masked [`Secret`]s.

pub use cloud_services::email::smtp::{SmtpConnection, SmtpError};
use cloud_services::email::{smtp as shared, EmailMessage};
use common_utils::{errors::CustomResult, ext_traits::AsyncExt, pii};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::{
    email::{
        EmailClient, EmailError, EmailRecipients, EmailResult, EmailSettings, IntermediateString,
    },
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
    pub primary_password: Option<Secret<String>>,
    /// Connection type of the SMTP server
    #[serde(default)]
    pub connection: SmtpConnection,
    /// SOCKS5 proxy config; absent means connect directly.
    pub socks5: Option<Socks5Config>,
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
    pub primary_password: Option<Secret<String>>,
}

impl SmtpServerConfig {
    #[allow(missing_docs)]
    pub async fn decrypt_credentials(
        self,
        client: &dyn hyperswitch_interfaces::secrets_interface::SecretManagementInterface,
    ) -> CustomResult<Self, hyperswitch_interfaces::secrets_interface::SecretsManagementError> {
        let primary_password = self
            .primary_password
            .async_map(|secret| async move {
                client
                    .get_secret(secret)
                    .await
                    .map_err(|error| error.attach_printable("email.smtp.primary_password"))
            })
            .await
            .transpose()?;
        let socks5 = self
            .socks5
            .async_map(|socks5| async move {
                let primary_password = socks5
                    .primary_password
                    .async_map(|secret| async move {
                        client.get_secret(secret).await.map_err(|error| {
                            error.attach_printable("email.smtp.socks5.primary_password")
                        })
                    })
                    .await
                    .transpose()?;
                Ok::<
                    _,
                    error_stack::Report<
                        hyperswitch_interfaces::secrets_interface::SecretsManagementError,
                    >,
                >(Socks5Config {
                    primary_password,
                    ..socks5
                })
            })
            .await
            .transpose()?;
        Ok(Self {
            primary_password,
            socks5,
            ..self
        })
    }

    /// Validation for the SMTP server client specific configs
    pub fn validate(&self) -> Result<(), &'static str> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};
        when(self.host.is_default_or_empty(), || {
            Err("email.smtp.host must not be empty")
        })?;
        self.username
            .clone()
            .zip(self.primary_password.clone())
            .map_or(Ok(()), |(username, password)| {
                when(username.peek().is_default_or_empty(), || {
                    Err("email.smtp.username must not be empty")
                })?;
                when(password.peek().is_default_or_empty(), || {
                    Err("email.smtp.primary_password must not be empty")
                })
            })?;
        self.socks5.as_ref().map_or(Ok(()), Socks5Config::validate)
    }

    fn to_shared(&self) -> shared::SmtpConfig {
        shared::SmtpConfig {
            host: self.host.clone(),
            port: self.port,
            timeout: self.timeout,
            username: self.username.as_ref().map(|value| value.peek().to_owned()),
            password: self
                .primary_password
                .as_ref()
                .map(|value| value.peek().to_owned()),
            connection: self.connection.clone(),
            socks5: self.socks5.as_ref().map(|socks5| shared::Socks5Config {
                host: socks5.host.clone(),
                port: socks5.port,
                username: socks5
                    .username
                    .as_ref()
                    .map(|value| value.peek().to_owned()),
                password: socks5
                    .primary_password
                    .as_ref()
                    .map(|value| value.peek().to_owned()),
            }),
        }
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
        match (self.username.as_ref(), self.primary_password.as_ref()) {
            (Some(username), Some(password)) => {
                when(username.peek().is_default_or_empty(), || {
                    Err("email.smtp.socks5.username must not be empty")
                })?;
                when(password.peek().is_default_or_empty(), || {
                    Err("email.smtp.socks5.primary_password must not be empty")
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
        recipients: EmailRecipients,
        subject: String,
        body: Self::RichText,
        // Unused here — SES uses this; SMTP's proxy lives in smtp_config.socks5.
        _proxy_url: Option<&String>,
    ) -> EmailResult<()> {
        // A new connection for every email
        let (to, cc, bcc) = recipients.into_addresses();
        let message = EmailMessage {
            from: self.sender.peek().to_owned(),
            to,
            cc,
            bcc,
            subject,
            html_body: body,
        };
        shared::SmtpClient::new(self.smtp_config.to_shared())
            .send(message)
            .await
            .map_err(|error| {
                report_with_cause(error).change_context(EmailError::EmailSendingFailure)
            })
    }
}
