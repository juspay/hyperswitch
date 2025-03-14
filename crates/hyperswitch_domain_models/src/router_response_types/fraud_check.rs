use serde::Serialize;

use crate::router_response_types::ResponseId;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum FraudCheckResponseData {
    TransactionResponse {
        resource_id: ResponseId,
        status: diesel_models::enums::FraudCheckStatus,
        connector_metadata: Option<serde_json::Value>,
        reason: Option<serde_json::Value>,
        score: Option<i32>,
    },
    FulfillmentResponse {
        order_id: String,
        shipment_ids: Vec<String>,
    },
    RecordReturnResponse {
        resource_id: ResponseId,
        connector_metadata: Option<serde_json::Value>,
        return_id: Option<String>,
    },
}

impl common_utils::events::ApiEventMetric for FraudCheckResponseData {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::FraudCheck)
    }
}
