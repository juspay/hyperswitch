use common_enums::connector_enums::InvoiceStatus;
#[derive(Debug, Clone)]
pub struct SubscriptionCreate;
#[derive(Debug, Clone)]
pub struct SubscriptionPause;
#[derive(Debug, Clone)]
pub struct SubscriptionResume;
#[derive(Debug, Clone)]
pub struct SubscriptionCancel;
#[derive(Debug, Clone)]
pub struct GetSubscriptionPlans;

#[derive(Debug, Clone)]
pub struct GetSubscriptionPlanPrices;

#[derive(Debug, Clone)]
pub struct GetSubscriptionEstimate;

/// Generic structure for subscription MIT (Merchant Initiated Transaction) payment data
#[derive(Debug, Clone)]
pub struct SubscriptionMitPaymentData {
    pub invoice_id: common_utils::id_type::InvoiceId,
    pub amount_due: common_utils::types::MinorUnit,
    pub currency_code: common_enums::enums::Currency,
    pub status: Option<InvoiceStatus>,
    pub customer_id: common_utils::id_type::CustomerId,
    pub subscription_id: common_utils::id_type::SubscriptionId,
    pub first_invoice: bool,
}
