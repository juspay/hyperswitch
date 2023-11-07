pub use diesel_models::payout_attempt::{
    PayoutAttempt, PayoutAttemptNew, PayoutAttemptUpdate, PayoutAttemptUpdateInternal,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PayoutRoutingData {
    pub routed_through: Option<String>,
    pub algorithm: Option<api_models::admin::PayoutStraightThroughAlgorithm>,
}
