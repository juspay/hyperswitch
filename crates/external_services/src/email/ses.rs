//! AWS SES email client.
//!
//! Hyperswitch adapter over the shared [`cloud_services::email::ses`] client: it adds logging and
//! `error-stack` reports on top of the plain client.

pub use cloud_services::email::ses::{SesConfig as SESConfig, SesError as AwsSesError};
use cloud_services::email::{ses as shared, EmailMessage};
use common_utils::{errors::CustomResult, pii};
use hyperswitch_masking::PeekInterface;
use router_env::logger;

use crate::{
    email::{
        EmailClient, EmailError, EmailRecipients, EmailResult, EmailSettings, IntermediateString,
    },
    report_with_cause,
};

/// Client for AWS SES operation
#[derive(Debug, Clone)]
pub struct AwsSes {
    sender: pii::Email,
    inner: shared::SesClient,
}

impl AwsSes {
    /// Constructs a new AwsSes client
    pub async fn create(
        conf: &EmailSettings,
        ses_config: &SESConfig,
        proxy_url: Option<impl AsRef<str>>,
    ) -> Self {
        let inner = shared::SesClient::new(conf.aws_region.to_owned(), ses_config.clone());

        // Build the client initially which will help us know if the email configuration is correct
        if let Err(error) = inner
            .sdk_client(proxy_url.as_ref().map(AsRef::as_ref))
            .await
        {
            logger::error!(error = ?report_with_cause(error), "Failed to initialize SES Client");
        }

        Self {
            sender: conf.sender_email.clone(),
            inner,
        }
    }
}

#[async_trait::async_trait]
impl EmailClient for AwsSes {
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
        proxy_url: Option<&String>,
    ) -> EmailResult<()> {
        // A new role session for every email, as the one created at startup would expire
        let (to, cc, bcc) = recipients.into_addresses();
        let message = EmailMessage {
            from: self.sender.peek().to_owned(),
            to,
            cc,
            bcc,
            subject,
            html_body: body,
        };
        self.inner
            .send(message, proxy_url.map(String::as_str))
            .await
            .map_err(|error| {
                let context = match &error {
                    error if error.is_client_building_failure() => {
                        EmailError::ClientBuildingFailure
                    }
                    AwsSesError::ContentBuildFailure(_) => EmailError::ContentBuildFailure,
                    _ => EmailError::EmailSendingFailure,
                };
                report_with_cause(error).change_context(context)
            })
    }
}
