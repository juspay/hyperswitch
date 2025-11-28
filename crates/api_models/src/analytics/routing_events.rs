#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RoutingEventsRequest {
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub payout_id: Option<common_utils::id_type::PayoutId>,
    pub refund_id: Option<String>,
    pub dispute_id: Option<String>,
}
