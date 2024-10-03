#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ConnectorEventsRequest {
    pub payment_id: common_utils::id_type::PaymentId,
    pub refund_id: Option<String>,
    pub dispute_id: Option<String>,
}
