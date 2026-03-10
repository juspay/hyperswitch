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
    Created,
}

impl From<SubscriptionStatus> for common_enums::SubscriptionStatus {
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
            SubscriptionStatus::Created => Self::Created,
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
pub struct SubscriptionPauseResponse {
    pub subscription_id: id_type::SubscriptionId,
    pub status: SubscriptionStatus,
    pub paused_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionResumeResponse {
    pub subscription_id: id_type::SubscriptionId,
    pub status: SubscriptionStatus,
    pub next_billing_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionCancelResponse {
    pub subscription_id: id_type::SubscriptionId,
    pub status: SubscriptionStatus,
    pub cancelled_at: Option<PrimitiveDateTime>,
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

impl From<SubscriptionPlanPrices> for api_models::subscription::SubscriptionPlanPrices {
    fn from(item: SubscriptionPlanPrices) -> Self {
        Self {
            price_id: item.price_id,
            plan_id: item.plan_id,
            amount: item.amount,
            currency: item.currency,
            interval: item.interval.into(),
            interval_count: item.interval_count,
            trial_period: item.trial_period,
            trial_period_unit: item.trial_period_unit.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PeriodUnit {
    Day,
    Week,
    Month,
    Year,
}

impl From<PeriodUnit> for api_models::subscription::PeriodUnit {
    fn from(unit: PeriodUnit) -> Self {
        match unit {
            PeriodUnit::Day => Self::Day,
            PeriodUnit::Week => Self::Week,
            PeriodUnit::Month => Self::Month,
            PeriodUnit::Year => Self::Year,
        }
    }
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
    pub customer_id: Option<id_type::CustomerId>,
}

impl From<GetSubscriptionEstimateResponse>
    for api_models::subscription::EstimateSubscriptionResponse
{
    fn from(value: GetSubscriptionEstimateResponse) -> Self {
        Self {
            amount: value.total,
            currency: value.currency,
            plan_id: None,
            item_price_id: None,
            coupon_code: None,
            customer_id: value.customer_id,
            line_items: value
                .line_items
                .into_iter()
                .map(api_models::subscription::SubscriptionLineItem::from)
                .collect(),
        }
    }
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

impl From<SubscriptionLineItem> for api_models::subscription::SubscriptionLineItem {
    fn from(value: SubscriptionLineItem) -> Self {
        Self {
            item_id: value.item_id,
            description: value.description,
            item_type: value.item_type,
            amount: value.amount,
            currency: value.currency,
            quantity: value.quantity,
        }
    }
}
