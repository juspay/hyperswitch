use common_utils::events::ApiEventMetric;
use error_stack::ResultExt;

use crate::errors::api_error_response::ApiErrorResponse;

const SECRET_SPLIT: &str = "_secret";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClientSecret(String);

impl ClientSecret {
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    pub fn get_subscription_id(&self) -> error_stack::Result<String, ApiErrorResponse> {
        let sub_id = self
            .0
            .split(SECRET_SPLIT)
            .next()
            .ok_or(ApiErrorResponse::MissingRequiredField {
                field_name: "client_secret",
            })
            .attach_printable("Failed to extract subscription_id from client_secret")?;

        Ok(sub_id.to_string())
    }
}

impl std::fmt::Display for ClientSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ApiEventMetric for ClientSecret {}

#[cfg(feature = "v1")]
impl From<api_models::subscription::ClientSecret> for ClientSecret {
    fn from(api_secret: api_models::subscription::ClientSecret) -> Self {
        Self::new(api_secret.as_str().to_string())
    }
}

#[cfg(feature = "v1")]
impl From<ClientSecret> for api_models::subscription::ClientSecret {
    fn from(domain_secret: ClientSecret) -> Self {
        Self::new(domain_secret.to_string())
    }
}
