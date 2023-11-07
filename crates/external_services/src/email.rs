//! Interactions with the AWS SES SDK

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_sesv2::{
    config::Region,
    operation::send_email::SendEmailError,
    types::{Body, Content, Destination, EmailContent, Message},
    Client,
};
use common_utils::{errors::CustomResult, pii};
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use serde::Deserialize;

/// Custom Result type alias for Email operations.
pub type EmailResult<T> = CustomResult<T, EmailError>;

/// A trait that defines the methods that must be implemented to send email.
#[async_trait::async_trait]
pub trait EmailClient: Sync + Send + dyn_clone::DynClone {
    /// Sends an email to the specified recipient with the given subject and body.
    async fn send_email(
        &self,
        recipient: pii::Email,
        subject: String,
        body: String,
    ) -> EmailResult<()>;
}

dyn_clone::clone_trait_object!(EmailClient);

/// Struct that contains the settings required to construct an EmailClient.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EmailSettings {
    /// Sender email.
    pub from_email: String,

    /// The AWS region to send SES requests to.
    pub aws_region: String,

    /// Base-url used when adding links that should redirect to self
    pub base_url: String,
}

/// Client for AWS SES operation
#[derive(Debug, Clone)]
pub struct AwsSes {
    ses_client: Client,
    from_email: String,
}

impl AwsSes {
    /// Constructs a new AwsSes client
    pub async fn new(conf: &EmailSettings) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(conf.aws_region.clone()));
        let sdk_config = aws_config::from_env().region(region_provider).load().await;

        Self {
            ses_client: Client::new(&sdk_config),
            from_email: conf.from_email.clone(),
        }
    }
}

#[async_trait::async_trait]
impl EmailClient for AwsSes {
    async fn send_email(
        &self,
        recipient: pii::Email,
        subject: String,
        body: String,
    ) -> EmailResult<()> {
        self.ses_client
            .send_email()
            .from_email_address(self.from_email.to_owned())
            .destination(
                Destination::builder()
                    .to_addresses(recipient.peek())
                    .build(),
            )
            .content(
                EmailContent::builder()
                    .simple(
                        Message::builder()
                            .subject(Content::builder().data(subject).build())
                            .body(
                                Body::builder()
                                    .text(Content::builder().data(body).charset("UTF-8").build())
                                    .build(),
                            )
                            .build(),
                    )
                    .build(),
            )
            .send()
            .await
            .map_err(AwsSesError::SendingFailure)
            .into_report()
            .change_context(EmailError::EmailSendingFailure)?;

        Ok(())
    }
}

#[allow(missing_docs)]
/// Errors that could occur from EmailClient.
#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    /// An error occurred when building email client.
    #[error("Error building email client")]
    ClientBuildingFailure,
    /// An error occurred when sending email
    #[error("Error sending email to recipient")]
    EmailSendingFailure,
    #[error("Failed to generate email token")]
    TokenGenerationFailure,
    #[error("Feature not implemented")]
    NotImplemented,
}

/// Errors that could occur during SES operations.
#[derive(Debug, thiserror::Error)]
pub enum AwsSesError {
    /// An error occurred in the SDK while sending email.
    #[error("Failed to Send Email {0:?}")]
    SendingFailure(aws_smithy_client::SdkError<SendEmailError>),
}
