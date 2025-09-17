use common_utils::events::ApiEventMetric;
use masking::Secret;
use utoipa::ToSchema;

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
