use common_utils::{events::ApiEventMetric, pii};

use crate::{
    customers::{CustomerRequest, CustomerResponse},
    enums as api_enums,
    payments::{
        Address, CustomerDetails, CustomerDetailsResponse, PaymentMethodDataRequest,
        PaymentsResponse,
    },
};
use common_types::payments::CustomerAcceptance;

pub const SUBSCRIPTION_ID_PREFIX: &str = "sub";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateSubscriptionRequest {
    pub subscription_id: Option<String>,
    pub profile_id: common_utils::id_type::ProfileId,
    pub plan_id: Option<String>,
    pub coupon_code: Option<String>,
    pub mca_id: Option<String>,
    pub confirm: bool,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub customer: Option<CustomerRequest>,
}

impl CreateSubscriptionRequest {
    pub fn get_customer_id(&self) -> Option<&common_utils::id_type::CustomerId> {
        self.customer_id
            .as_ref()
            .or_else(|| self.customer.as_ref()?.customer_id.as_ref())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateSubscriptionResponse {
    pub subscription: Subscription,
    pub profile_id: common_utils::id_type::ProfileId,
    pub client_secret: Option<String>,
    pub merchant_id: String,
    pub mca_id: Option<String>,
    pub coupon_code: Option<String>,
    pub customer: Option<CustomerDetailsResponse>,
    pub invoice: Option<Invoice>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Subscription {
    pub id: String,
    pub status: SubscriptionStatus,
    pub plan_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, strum::EnumString, strum::Display)]
pub enum SubscriptionStatus {
    Created,
    Active,
    InActive,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Invoice {
    pub id: String,
    pub total_amount: u64,
    pub currency: common_enums::Currency,
}

impl Subscription {
    pub fn new(id: impl Into<String>, status: SubscriptionStatus, plan_id: Option<String>) -> Self {
        Self {
            id: id.into(),
            status,
            plan_id,
        }
    }
}

impl Invoice {
    pub fn new(id: impl Into<String>, total_amount: u64, currency: common_enums::Currency) -> Self {
        Self {
            id: id.into(),
            total_amount,
            currency,
        }
    }
}
impl CreateSubscriptionResponse {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subscription: Subscription,
        profile_id: common_utils::id_type::ProfileId,
        merchant_id: impl Into<String>,
        mca_id: Option<String>,
    ) -> Self {
        Self {
            subscription,
            profile_id,
            client_secret: None,
            merchant_id: merchant_id.into(),
            mca_id,
            coupon_code: None,
            customer: None,
            invoice: None,
        }
    }
}

pub fn map_customer_resp_to_details(r: &CustomerResponse) -> CustomerDetailsResponse {
    CustomerDetailsResponse {
        id: Some(r.customer_id.clone()),
        name: r.name.as_ref().map(|n| n.clone().into_inner()),
        email: r.email.as_ref().map(|e| pii::Email::from(e.clone())),
        phone: r.phone.as_ref().map(|p| p.clone().into_inner()),
        phone_country_code: r.phone_country_code.clone(),
    }
}

impl ApiEventMetric for CreateSubscriptionResponse {}
impl ApiEventMetric for CreateSubscriptionRequest {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentData {
    pub payment_method: api_enums::PaymentMethod,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub payment_method_data: PaymentMethodDataRequest,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub customer_acceptance: Option<CustomerAcceptance>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentResponseData {
    pub payment_id: String,
    pub status: api_enums::IntentStatus,
    pub amount: common_utils::types::MinorUnit,
    pub currency: String,
    pub connector: Option<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfirmSubscriptionRequest {
    pub client_secret: Option<String>,
    pub amount: i64,
    pub currency: api_enums::Currency,
    pub plan_id: Option<String>,
    pub item_price_id: Option<String>,
    pub coupon_code: Option<String>,
    pub customer: Option<CustomerDetails>,
    pub billing_address: Option<Address>,
    pub payment_data: PaymentData,
}

impl ApiEventMetric for ConfirmSubscriptionRequest {}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfirmSubscriptionResponse {
    pub subscription: Subscription,
    pub payment: PaymentResponseData,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub invoice: Option<Invoice>,
}

impl ApiEventMetric for ConfirmSubscriptionResponse {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubscriptionPaymentsRequest {
    pub payment_id: Option<String>,
    pub amount: Option<i64>,
    pub currency: Option<api_enums::Currency>,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub merchant_id: Option<String>,
    pub confirm: Option<bool>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub payment_method: Option<api_enums::PaymentMethod>,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub payment_method_data: Option<PaymentMethodDataRequest>,
    pub customer_acceptance: Option<CustomerAcceptance>,
}
