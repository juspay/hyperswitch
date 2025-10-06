use common_enums::Currency;
use common_utils::{id_type, types::MinorUnit};
use time::PrimitiveDateTime;

#[derive(Debug, Clone)]
pub struct SubscriptionCreateResponse {
    pub subscription_id: id_type::SubscriptionId,
    pub status: SubscriptionStatus,
    pub customer_id: id_type::CustomerId,
    pub currency_code: Currency,
    pub total_amount: MinorUnit,
    pub next_billing_at: Option<PrimitiveDateTime>,
    pub created_at: Option<PrimitiveDateTime>,
    pub invoice_details: Option<SubscriptionInvoiceData>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SubscriptionInvoiceData {
    pub id: id_type::InvoiceId,
    pub total: MinorUnit,
    pub currency_code: Currency,
    pub status: Option<common_enums::connector_enums::InvoiceStatus>,
    pub billing_address: Option<api_models::payments::Address>,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum SubscriptionStatus {
    Pending,
    Trial,
    Active,
    Paused,
    Unpaid,
    Onetime,
    Cancelled,
    Failed,
}

#[cfg(feature = "v1")]
impl From<SubscriptionStatus> for api_models::subscription::SubscriptionStatus {
    fn from(status: SubscriptionStatus) -> Self {
        match status {
            SubscriptionStatus::Pending => Self::Pending,
            SubscriptionStatus::Trial => Self::Trial,
            SubscriptionStatus::Active => Self::Active,
            SubscriptionStatus::Paused => Self::Paused,
            SubscriptionStatus::Unpaid => Self::Unpaid,
            SubscriptionStatus::Onetime => Self::Onetime,
            SubscriptionStatus::Cancelled => Self::Cancelled,
            SubscriptionStatus::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetSubscriptionPlansResponse {
    pub list: Vec<SubscriptionPlans>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionPlans {
    pub subscription_provider_plan_id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GetSubscriptionPlanPricesResponse {
    pub list: Vec<SubscriptionPlanPrices>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionPlanPrices {
    pub price_id: String,
    pub plan_id: Option<String>,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub interval: PeriodUnit,
    pub interval_count: i64,
    pub trial_period: Option<i64>,
    pub trial_period_unit: Option<PeriodUnit>,
}

#[derive(Debug, Clone)]
pub enum PeriodUnit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone)]
pub struct GetSubscriptionEstimateResponse {
    pub sub_total: MinorUnit,
    pub total: MinorUnit,
    pub credits_applied: Option<MinorUnit>,
    pub amount_paid: Option<MinorUnit>,
    pub amount_due: Option<MinorUnit>,
    pub currency: Currency,
    pub next_billing_at: Option<PrimitiveDateTime>,
    pub line_items: Vec<SubscriptionLineItem>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionLineItem {
    pub item_id: String,
    pub item_type: String,
    pub description: String,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub unit_amount: Option<MinorUnit>,
    pub quantity: i64,
    pub pricing_model: Option<String>,
}
