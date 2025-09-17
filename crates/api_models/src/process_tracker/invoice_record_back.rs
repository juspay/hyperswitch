#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InvoiceRecordBackTrackingData {
    pub payment_id: common_utils::id_type::PaymentId,
    pub subscription_id: String,
    pub billing_processor_mca_id: common_utils::id_type::MerchantConnectorAccountId,
    pub invoice_id: String,
    pub should_refund: bool,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub amount: common_utils::types::MinorUnit,
    pub currency: crate::enums::Currency,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub attempt_status: common_enums::AttemptStatus,
}
