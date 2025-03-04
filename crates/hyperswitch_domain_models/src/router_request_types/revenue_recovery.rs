#[derive(Debug, Clone)]
pub struct GetAdditionalRevenueRecoveryRequestData {
    // stripe charge id for additional call
    pub charge_id: Option<String>,
}
