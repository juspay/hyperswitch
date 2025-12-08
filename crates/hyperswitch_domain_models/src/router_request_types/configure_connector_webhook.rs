use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    pii::Email,
};
use diesel_models::types::OrderDetailsWithAmount;
use masking::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::router_request_types;

#[derive(Debug, Clone)]
pub struct ConnectorWebhookRegisterData {
    pub event_type: common_enums::ConnectorWebhookEventType,
    pub webhook_url: Option<String>,
}