#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InvoiceRecordBackTrackingData {
    pub payment_id: common_utils::id_type::PaymentId,
    pub subscription_id: String,
    pub billing_processor_mca_id: common_utils::id_type::MerchantConnectorAccountId,
    pub invoice_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub amount: common_utils::types::MinorUnit,
    pub currency: crate::enums::Currency,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub attempt_status: common_enums::AttemptStatus,
}

impl InvoiceRecordBackTrackingData {
    pub fn new(
        payment_id: common_utils::id_type::PaymentId,
        subscription_id: String,
        billing_processor_mca_id: common_utils::id_type::MerchantConnectorAccountId,
        invoice_id: String,
        merchant_id: common_utils::id_type::MerchantId,
        profile_id: common_utils::id_type::ProfileId,
        customer_id: common_utils::id_type::CustomerId,
        amount: common_utils::types::MinorUnit,
        currency: crate::enums::Currency,
        payment_method_type: Option<common_enums::PaymentMethodType>,
        attempt_status: common_enums::AttemptStatus,
    ) -> Self {
        Self {
            payment_id,
            subscription_id,
            billing_processor_mca_id,
            invoice_id,
            merchant_id,
            profile_id,
            customer_id,
            amount,
            currency,
            payment_method_type,
            attempt_status,
        }
    }
}
