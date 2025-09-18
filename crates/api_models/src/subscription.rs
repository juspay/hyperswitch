use common_types::payments::CustomerAcceptance;
use common_utils::{events::ApiEventMetric, pii, types::MinorUnit};
use masking::Secret;
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::{
    customers::{CustomerRequest, CustomerResponse},
    enums as api_enums,
    payments::{Address, CustomerDetails, CustomerDetailsResponse, PaymentMethodDataRequest},
};

// use crate::{
//     customers::{CustomerRequest, CustomerResponse},
//     payments::CustomerDetailsResponse,
// };

/// Request payload for creating a subscription.
///
/// This struct captures details required to create a subscription,
/// including plan, profile, merchant connector, and optional customer info.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,

    /// Identifier for the subscription plan.
    pub plan_id: Option<String>,

    /// Optional coupon code applied to the subscription.
    pub coupon_code: Option<String>,

    /// customer ID associated with this subscription.
    pub customer_id: common_utils::id_type::CustomerId,
}

/// Response payload returned after successfully creating a subscription.
///
/// Includes details such as subscription ID, status, plan, merchant, and customer info.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CreateSubscriptionResponse {
    /// Unique identifier for the subscription.
    pub id: common_utils::id_type::SubscriptionId,

    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,

    /// Current status of the subscription.
    pub status: SubscriptionStatus,

    /// Identifier for the associated subscription plan.
    pub plan_id: Option<String>,

    /// Associated profile ID.
    pub profile_id: common_utils::id_type::ProfileId,

    /// Optional client secret used for secure client-side interactions.
    pub client_secret: Option<Secret<String>>,

    /// Merchant identifier owning this subscription.
    pub merchant_id: common_utils::id_type::MerchantId,

    /// Optional coupon code applied to this subscription.
    pub coupon_code: Option<String>,

    /// Optional customer ID associated with this subscription.
    pub customer_id: common_utils::id_type::CustomerId,
}

/// Possible states of a subscription lifecycle.
///
/// - `Created`: Subscription was created but not yet activated.
/// - `Active`: Subscription is currently active.
/// - `InActive`: Subscription is inactive (e.g., cancelled or expired).
#[derive(Debug, Clone, serde::Serialize, strum::EnumString, strum::Display, ToSchema)]
pub enum SubscriptionStatus {
    /// Subscription is active.
    Active,
    /// Subscription is created but not yet active.
    Created,
    /// Subscription is inactive.
    InActive,
    /// Subscription is in pending state.
    Pending,
}

impl CreateSubscriptionResponse {
    /// Creates a new [`CreateSubscriptionResponse`] with the given identifiers.
    ///
    /// By default, `client_secret`, `coupon_code`, and `customer` fields are `None`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: common_utils::id_type::SubscriptionId,
        merchant_reference_id: Option<String>,
        status: SubscriptionStatus,
        plan_id: Option<String>,
        profile_id: common_utils::id_type::ProfileId,
        merchant_id: common_utils::id_type::MerchantId,
        client_secret: Option<Secret<String>>,
        customer_id: common_utils::id_type::CustomerId,
    ) -> Self {
        Self {
            id,
            merchant_reference_id,
            status,
            plan_id,
            profile_id,
            client_secret,
            merchant_id,
            coupon_code: None,
            customer_id,
        }
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
    pub payment_id: common_utils::id_type::PaymentId,
    pub status: api_enums::IntentStatus,
    pub amount: MinorUnit,
    pub currency: api_enums::Currency,
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
    /// Unique identifier for the subscription.
    pub id: common_utils::id_type::SubscriptionId,

    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,

    /// Current status of the subscription.
    pub status: SubscriptionStatus,

    /// Identifier for the associated subscription plan.
    pub plan_id: Option<String>,

    /// Associated profile ID.
    pub profile_id: common_utils::id_type::ProfileId,
    pub payment: Option<PaymentResponseData>,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    // pub invoice: Option<Invoice>,
}

impl ApiEventMetric for ConfirmSubscriptionResponse {}

// Dummy types from here onwards, to be replaced after connector integration
#[derive(Debug, Clone)]
pub struct SubscriptionCreateResponse {
    pub subscription_id: String,
    pub status: String,
    pub customer_id: String,
    pub currency_code: api_enums::Currency,
    pub total_amount: MinorUnit,
    pub next_billing_at: Option<PrimitiveDateTime>,
    pub created_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct CreateCustomer;

#[derive(Debug, Clone)]
pub struct CreateCustomerRequest {
    pub id: common_utils::id_type::CustomerId,
    // More fields can be added as needed
}

#[derive(Debug, Clone)]
pub struct CreateCustomerResponse {
    pub id: common_utils::id_type::CustomerId,
    // More fields can be added as needed
}

#[derive(Debug, Clone)]
pub struct CreateSubscription;

#[derive(Debug, Clone)]
pub struct SubscriptionCreateRequest {
    pub id: String, // More fields can be added as needed
}

#[allow(clippy::todo)]
impl Default for SubscriptionCreateResponse {
    fn default() -> Self {
        // TODO: Replace with a proper default implementation after connector integration
        todo!()
    }
}
