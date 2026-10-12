//! Sending email through AWS SES or an SMTP server.
//!
//! The clients themselves are the shared [`cloud_services::email`] ones; this module adds
//! Hyperswitch's templating ([`EmailData`]), settings and error reporting.

use common_utils::{errors::CustomResult, pii};
use serde::Deserialize;

/// Implementation of aws ses client
pub mod ses;

/// Implementation of SMTP server client
pub mod smtp;

/// Implementation of Email client when email support is disabled
pub mod no_email;

/// Custom Result type alias for Email operations.
pub type EmailResult<T> = CustomResult<T, EmailError>;

/// A trait that defines the methods that must be implemented to send email.
#[async_trait::async_trait]
pub trait EmailClient: Sync + Send + dyn_clone::DynClone {
    /// The rich text type of the email client
    type RichText;

    /// Sends one email to `recipients` with the given subject and body.
    async fn send_email(
        &self,
        recipients: EmailRecipients,
        subject: String,
        body: Self::RichText,
        proxy_url: Option<&String>,
    ) -> EmailResult<()>;

    /// Convert Stringified HTML to client native rich text format
    /// This has to be done because not all clients may format html as the same
    fn convert_to_rich_text(
        &self,
        intermediate_string: IntermediateString,
    ) -> CustomResult<Self::RichText, EmailError>
    where
        Self::RichText: Send;
}

/// A super trait which is automatically implemented for all EmailClients
#[async_trait::async_trait]
pub trait EmailService: Sync + Send + dyn_clone::DynClone {
    /// Compose and send email using the email data
    async fn compose_and_send_email(
        &self,
        base_url: &str,
        email_data: Box<dyn EmailData + Send>,
        proxy_url: Option<&String>,
    ) -> EmailResult<()>;
}

#[async_trait::async_trait]
impl<T> EmailService for T
where
    T: EmailClient,
    <Self as EmailClient>::RichText: Send,
{
    async fn compose_and_send_email(
        &self,
        base_url: &str,
        email_data: Box<dyn EmailData + Send>,
        proxy_url: Option<&String>,
    ) -> EmailResult<()> {
        let email_data = email_data.get_email_data(base_url);
        let email_data = email_data.await?;

        let EmailContents {
            subject,
            body,
            recipients,
        } = email_data;

        let rich_text_string = self.convert_to_rich_text(body)?;

        self.send_email(recipients, subject, rich_text_string, proxy_url)
            .await
    }
}

/// This is a struct used to create Intermediate String for rich text ( html )
#[derive(Debug)]
pub struct IntermediateString(String);

impl IntermediateString {
    /// Create a new Instance of IntermediateString using a string
    pub fn new(inner: String) -> Self {
        Self(inner)
    }

    /// Get the inner String
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// Temporary output for the email subject
#[derive(Debug)]
pub struct EmailContents {
    /// The subject of email
    pub subject: String,

    /// This will be the intermediate representation of the email body in a generic format.
    /// The email clients can convert this intermediate representation to their client specific rich text format
    pub body: IntermediateString,

    /// Who the email is sent to; a single address converts into `to`.
    pub recipients: EmailRecipients,
}

/// The recipients of one email.
#[derive(Debug, Clone, Default)]
pub struct EmailRecipients {
    /// Recipients
    pub to: Vec<pii::Email>,
    /// Copied recipients
    pub cc: Vec<pii::Email>,
    /// Blind-copied recipients
    pub bcc: Vec<pii::Email>,
}

impl From<pii::Email> for EmailRecipients {
    fn from(recipient: pii::Email) -> Self {
        Self {
            to: vec![recipient],
            ..Self::default()
        }
    }
}

impl EmailRecipients {
    fn addresses(emails: Vec<pii::Email>) -> Vec<String> {
        use hyperswitch_masking::PeekInterface;
        emails
            .into_iter()
            .map(|email| email.peek().to_owned())
            .collect()
    }

    /// The recipients as plain addresses, for the shared clients.
    pub(crate) fn into_addresses(self) -> (Vec<String>, Vec<String>, Vec<String>) {
        (
            Self::addresses(self.to),
            Self::addresses(self.cc),
            Self::addresses(self.bcc),
        )
    }
}

/// A trait which will contain the logic of generating the email subject and body
#[async_trait::async_trait]
pub trait EmailData {
    /// Get the email contents
    async fn get_email_data(&self, base_url: &str) -> CustomResult<EmailContents, EmailError>;
}

dyn_clone::clone_trait_object!(EmailClient<RichText = String>);

/// List of available email clients to choose from
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(tag = "active_email_client")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EmailClientConfigs {
    #[default]
    /// Default Email client to use when no client is specified
    NoEmailClient,
    /// AWS ses email client
    Ses {
        /// AWS SES client configuration
        aws_ses: ses::SESConfig,
    },
    /// Other Simple SMTP server
    Smtp {
        /// SMTP server configuration
        smtp: smtp::SmtpServerConfig,
    },
}

/// Struct that contains the settings required to construct an EmailClient.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct EmailSettings {
    /// The AWS region to send SES requests to.
    pub aws_region: String,

    /// Number of days for verification of the email
    pub allowed_unverified_days: i64,

    /// Sender email
    pub sender_email: pii::Email,

    #[serde(flatten)]
    /// The client specific configurations
    pub client_config: EmailClientConfigs,

    /// Recipient email for recon emails
    pub recon_recipient_email: pii::Email,

    /// Recipient email for recon emails
    pub prod_intent_recipient_email: pii::Email,
}

impl EmailSettings {
    /// Validation for the Email client specific configurations
    pub fn validate(&self) -> Result<(), &'static str> {
        match &self.client_config {
            EmailClientConfigs::Ses { ref aws_ses } => aws_ses.validate(),
            EmailClientConfigs::Smtp { ref smtp } => smtp.validate(),
            EmailClientConfigs::NoEmailClient => Ok(()),
        }
    }
}

/// Errors that could occur from EmailClient.
#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    /// An error occurred when building email client.
    #[error("Error building email client")]
    ClientBuildingFailure,

    /// An error occurred when sending email
    #[error("Error sending email to recipient")]
    EmailSendingFailure,

    /// Failed to generate the email token
    #[error("Failed to generate email token")]
    TokenGenerationFailure,

    /// The expected feature is not implemented
    #[error("Feature not implemented")]
    NotImplemented,

    /// An error occurred when building email content.
    #[error("Error building email content")]
    ContentBuildFailure,
}
