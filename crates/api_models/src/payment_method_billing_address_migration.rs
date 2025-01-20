#[derive(Debug, Clone, serde::Serialize)]
pub struct PaymentMethodBillingAddressMigrationResponse {
    pub payment_method_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub migration_successful: MigrationStatus,
    pub failure_reason: Option<String>,
}

impl common_utils::events::ApiEventMetric for PaymentMethodBillingAddressMigrationResponse {}

#[derive(Debug, Clone, serde::Serialize)]
pub enum MigrationStatus {
    Success,
    Failed,
}
