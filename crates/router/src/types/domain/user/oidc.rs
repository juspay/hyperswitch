use api_models::oidc::Scope;
use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    pii,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCodeData {
    pub sub: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Vec<Scope>,
    pub nonce: Option<String>,
    pub email: pii::Email,
}

impl ApiEventMetric for AuthCodeData {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Oidc)
    }
}
