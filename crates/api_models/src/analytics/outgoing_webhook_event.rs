#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct OutgoingWebhookLogsRequest {
    pub payment_id: common_utils::id_type::PaymentId,
    pub event_id: Option<String>,
    pub refund_id: Option<String>,
    pub dispute_id: Option<String>,
    pub mandate_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub attempt_id: Option<String>,
}
