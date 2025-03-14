use common_utils::{errors::CustomResult, pii};
use router_env::logger;

use crate::email::{EmailClient, EmailError, EmailResult, IntermediateString};

/// Client when email support is disabled
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct NoEmailClient {}

impl NoEmailClient {
    /// Constructs a new client when email is disabled
    pub async fn create() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl EmailClient for NoEmailClient {
    type RichText = String;
    fn convert_to_rich_text(
        &self,
        intermediate_string: IntermediateString,
    ) -> CustomResult<Self::RichText, EmailError> {
        Ok(intermediate_string.into_inner())
    }

    async fn send_email(
        &self,
        _recipient: pii::Email,
        _subject: String,
        _body: Self::RichText,
        _proxy_url: Option<&String>,
    ) -> EmailResult<()> {
        logger::info!("Email not sent as email support is disabled, please enable any of the supported email clients to send emails");
        Ok(())
    }
}
