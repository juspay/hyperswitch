use common_utils::{events::ApiEventMetric, pii};
use masking::Secret;
use utoipa::ToSchema;

use crate::{
    customers::{CustomerRequest, CustomerResponse},
    payments::CustomerDetailsResponse,
};

/// Prefix used when generating subscription IDs.
pub const SUBSCRIPTION_ID_PREFIX: &str = "sub";

/// Request payload for creating a subscription.
///
/// This struct captures details required to create a subscription,
/// including plan, profile, merchant connector, and optional customer info.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    /// Unique identifier for the subscription (optional, generated if missing).
    pub subscription_id: Option<String>,

    /// Associated profile ID for this subscription.
    pub profile_id: common_utils::id_type::ProfileId,

    /// Identifier for the subscription plan.
    pub plan_id: Option<String>,

    /// Optional coupon code applied to the subscription.
    pub coupon_code: Option<String>,

    /// Optional merchant connector account ID for routing payments.
    pub merchant_connector_account_id: Option<common_utils::id_type::MerchantConnectorAccountId>,

    /// Whether to immediately confirm the subscription on creation.
    pub confirm: Option<bool>,

    /// Optional customer ID associated with this subscription.
    pub customer_id: Option<common_utils::id_type::CustomerId>,

    /// Optional full customer request data.
    pub customer: Option<CustomerRequest>,
}

impl CreateSubscriptionRequest {
    /// Retrieves the `customer_id` either from the top-level field
    /// or from the nested [`CustomerRequest`].
    pub fn get_customer_id(&self) -> Option<&common_utils::id_type::CustomerId> {
        self.customer_id
            .as_ref()
            .or_else(|| self.customer.as_ref()?.customer_id.as_ref())
    }
}

/// Response payload returned after successfully creating a subscription.
///
/// Includes details such as subscription ID, status, plan, merchant, and customer info.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CreateSubscriptionResponse {
    /// Unique identifier for the subscription.
    pub id: String,

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

    /// Optional merchant connector account ID for routing payments.
    pub merchant_connector_account_id: Option<common_utils::id_type::MerchantConnectorAccountId>,

    /// Optional coupon code applied to this subscription.
    pub coupon_code: Option<String>,

    /// Optional customer details response.
    pub customer: Option<CustomerDetailsResponse>,
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
    pub fn new(
        id: String,
        status: SubscriptionStatus,
        plan_id: Option<String>,
        profile_id: common_utils::id_type::ProfileId,
        merchant_id: common_utils::id_type::MerchantId,
        merchant_connector_account_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    ) -> Self {
        Self {
            id,
            status,
            plan_id,
            profile_id,
            client_secret: None,
            merchant_id,
            merchant_connector_account_id,
            coupon_code: None,
            customer: None,
        }
    }
}

/// Maps a [`CustomerResponse`] into a [`CustomerDetailsResponse`].
///
/// This is used to transform customer information returned by the customer API
/// into the response format expected by the subscription API.
pub fn map_customer_resp_to_details(response: &CustomerResponse) -> CustomerDetailsResponse {
    CustomerDetailsResponse {
        id: Some(response.customer_id.clone()),
        name: response.name.as_ref().map(|name| name.clone().into_inner()),
        email: response
            .email
            .as_ref()
            .map(|email| pii::Email::from(email.clone())),
        phone: response
            .phone
            .as_ref()
            .map(|phone| phone.clone().into_inner()),
        phone_country_code: response.phone_country_code.clone(),
    }
}

impl ApiEventMetric for CreateSubscriptionResponse {}
impl ApiEventMetric for CreateSubscriptionRequest {}
