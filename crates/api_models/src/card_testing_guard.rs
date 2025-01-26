use common_utils::events::ApiEventMetric;
use utoipa::ToSchema;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpdateCardTestingGuardRequest {
    pub card_ip_blocking: bool,
    pub guest_user_card_blocking: bool,
    pub customer_id_blocking: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpdateCardTestingGuardResponse {
    pub card_ip_blocking_status: String,
    pub guest_user_card_blocking_status: String,
    pub customer_id_blocking_status: String,
}

impl ApiEventMetric for UpdateCardTestingGuardRequest {}
impl ApiEventMetric for UpdateCardTestingGuardResponse {}
