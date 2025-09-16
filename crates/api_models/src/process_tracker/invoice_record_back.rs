#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InvoiceRecordBackTrackingData {
    pub payment_id: common_utils::id_type::PaymentId,
    pub subscription_id: String,
    pub billing_processor_mca_id: common_utils::id_type::MerchantConnectorAccountId,
    pub invoice_id: String,
    pub should_refund: bool,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub profile_id: common_utils::id_type::ProfileId,
}
