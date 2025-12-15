use diesel_models::types::OrderDetailsWithAmount;
use masking::Secret;
use utoipa::ToSchema;

use crate::router_request_types;

#[derive(Debug, Clone)]
pub struct ConnectorWebhookRegisterData {
    pub event_type: common_enums::ConnectorWebhookEventType,
    pub webhook_url: String,
}