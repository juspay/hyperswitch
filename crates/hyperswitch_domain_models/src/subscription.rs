use common_utils::events::ApiEventMetric;
use error_stack::ResultExt;

use crate::errors::api_error_response::ApiErrorResponse;

const SECRET_SPLIT: &str = "_secret";

#[derive(Clone, Debug, serde::Serialize)]
pub struct ClientSecret(String);

impl ClientSecret {
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
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

impl ApiEventMetric for ClientSecret {}
