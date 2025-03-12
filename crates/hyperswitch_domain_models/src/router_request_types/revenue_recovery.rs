use common_enums::enums;

#[derive(Debug, Clone)]
pub struct GetAdditionalRevenueRecoveryRequestData {
    /// unique id for making additional revenue recovery call
    pub additional_revenue_recovery_id: String,
}

#[derive(Debug, Clone)]
pub struct RevenueRecoveryRecordBackRequest {
    pub merchant_reference_id: common_utils::id_type::PaymentReferenceId,
    pub amount: common_utils::types::MinorUnit,
    pub currency: enums::Currency,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub attempt_status: common_enums::AttemptStatus,
    pub connector_transaction_id: Option<common_utils::types::ConnectorTransactionId>,
}
