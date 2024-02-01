//! Interactions with the AWS SES SDK

use aws_sdk_sesv2::types::Body;
use common_utils::{errors::CustomResult, pii};
use serde::Deserialize;

/// Implementation of aws ses client
pub mod ses;

/// Custom Result type alias for Email operations.
pub type EmailResult<T> = CustomResult<T, EmailError>;

/// A trait that defines the methods that must be implemented to send email.
#[async_trait::async_trait]
pub trait EmailClient: Sync + Send + dyn_clone::DynClone {
    /// The rich text type of the email client
    type RichText;

    /// Sends an email to the specified recipient with the given subject and body.
    async fn send_email(
        &self,
        recipient: pii::Email,
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
        /// Composes an email using the provided email data and sends it to the specified recipient.
    /// If a proxy URL is provided, the email will be sent through the proxy.
    async fn compose_and_send_email(
        &self,
        email_data: Box<dyn EmailData + Send>,
        proxy_url: Option<&String>,
    ) -> EmailResult<()> {
        let email_data = email_data.get_email_data();
        let email_data = email_data.await?;

        let EmailContents {
            subject,
            body,
            recipient,
        } = email_data;

        let rich_text_string = self.convert_to_rich_text(body)?;

        self.send_email(recipient, subject, rich_text_string, proxy_url)
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

    /// This will be the intermediate representation of the the email body in a generic format.
    /// The email clients can convert this intermediate representation to their client specific rich text format
    pub body: IntermediateString,

    /// The email of the recipient to whom the email has to be sent
    pub recipient: pii::Email,
}

/// A trait which will contain the logic of generating the email subject and body
#[async_trait::async_trait]
pub trait EmailData {
    /// Get the email contents
    async fn get_email_data(&self) -> CustomResult<EmailContents, EmailError>;
}

dyn_clone::clone_trait_object!(EmailClient<RichText = Body>);

/// List of available email clients to choose from
#[derive(Debug, Clone, Default, Deserialize)]
pub enum AvailableEmailClients {
    #[default]
    /// AWS ses email client
    SES,
}

/// Struct that contains the settings required to construct an EmailClient.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EmailSettings {
    /// The AWS region to send SES requests to.
    pub aws_region: String,

    /// Base-url used when adding links that should redirect to self
    pub base_url: String,

    /// Number of days for verification of the email
    pub allowed_unverified_days: i64,

    /// Sender email
    pub sender_email: String,

    /// Configs related to AWS Simple Email Service
    pub aws_ses: Option<ses::SESConfig>,

    /// The active email client to use
    pub active_email_client: AvailableEmailClients,
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
}
