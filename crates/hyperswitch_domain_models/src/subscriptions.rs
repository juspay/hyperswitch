use masking::Secret;
use std::str::FromStr;
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
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
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

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Created => write!(f, "Created"),
            Self::InActive => write!(f, "InActive"),
            Self::Pending => write!(f, "Pending"),
        }
    }
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

// Type conversions between API models and domain models

/// Convert from API model `CreateSubscriptionRequest` to domain model `CreateSubscriptionRequest`
impl From<api_models::subscription::CreateSubscriptionRequest> for CreateSubscriptionRequest {
    fn from(api_request: api_models::subscription::CreateSubscriptionRequest) -> Self {
        Self {
            merchant_reference_id: api_request.merchant_reference_id,
            plan_id: api_request.plan_id,
            coupon_code: api_request.coupon_code,
            customer_id: api_request.customer_id,
        }
    }
}

/// Convert from domain model `CreateSubscriptionRequest` to API model `CreateSubscriptionRequest`
impl From<CreateSubscriptionRequest> for api_models::subscription::CreateSubscriptionRequest {
    fn from(domain_request: CreateSubscriptionRequest) -> Self {
        Self {
            merchant_reference_id: domain_request.merchant_reference_id,
            plan_id: domain_request.plan_id,
            coupon_code: domain_request.coupon_code,
            customer_id: domain_request.customer_id,
        }
    }
}

/// Convert from domain model `CreateSubscriptionResponse` to API model `CreateSubscriptionResponse`
impl From<CreateSubscriptionResponse> for api_models::subscription::CreateSubscriptionResponse {
    fn from(domain_response: CreateSubscriptionResponse) -> Self {
        Self {
            id: domain_response.id,
            merchant_reference_id: domain_response.merchant_reference_id,
            status: domain_response.status.into(),
            plan_id: domain_response.plan_id,
            profile_id: domain_response.profile_id,
            client_secret: domain_response.client_secret,
            merchant_id: domain_response.merchant_id,
            coupon_code: domain_response.coupon_code,
            customer_id: domain_response.customer_id,
        }
    }
}

/// Convert from API model `CreateSubscriptionResponse` to domain model `CreateSubscriptionResponse`
impl From<api_models::subscription::CreateSubscriptionResponse> for CreateSubscriptionResponse {
    fn from(api_response: api_models::subscription::CreateSubscriptionResponse) -> Self {
        Self {
            id: api_response.id,
            merchant_reference_id: api_response.merchant_reference_id,
            status: api_response.status.into(),
            plan_id: api_response.plan_id,
            profile_id: api_response.profile_id,
            client_secret: api_response.client_secret,
            merchant_id: api_response.merchant_id,
            coupon_code: api_response.coupon_code,
            customer_id: api_response.customer_id,
        }
    }
}

/// Convert from domain model `SubscriptionStatus` to API model `SubscriptionStatus`
impl From<SubscriptionStatus> for api_models::subscription::SubscriptionStatus {
    fn from(domain_status: SubscriptionStatus) -> Self {
        match domain_status {
            SubscriptionStatus::Active => Self::Active,
            SubscriptionStatus::Created => Self::Created,
            SubscriptionStatus::InActive => Self::InActive,
            SubscriptionStatus::Pending => Self::Pending,
        }
    }
}

/// Convert from API model `SubscriptionStatus` to domain model `SubscriptionStatus`
impl From<api_models::subscription::SubscriptionStatus> for SubscriptionStatus {
    fn from(api_status: api_models::subscription::SubscriptionStatus) -> Self {
        match api_status {
            api_models::subscription::SubscriptionStatus::Active => Self::Active,
            api_models::subscription::SubscriptionStatus::Created => Self::Created,
            api_models::subscription::SubscriptionStatus::InActive => Self::InActive,
            api_models::subscription::SubscriptionStatus::Pending => Self::Pending,
        }
    }
}

impl CreateSubscriptionRequest {
    /// Convert domain request with context to SubscriptionNew for database operations
    pub fn to_subscription_new(
        self,
        id: common_utils::id_type::SubscriptionId,
        profile_id: common_utils::id_type::ProfileId,
        merchant_id: common_utils::id_type::MerchantId,
    ) -> diesel_models::subscription::SubscriptionNew {
        diesel_models::subscription::SubscriptionNew::new(
            id,
            SubscriptionStatus::Created.to_string(),
            None, // billing_processor
            None, // payment_method_id
            None, // merchant_connector_id
            None, // client_secret
            None, // connector_subscription_id
            merchant_id,
            self.customer_id,
            None, // metadata
            profile_id,
            self.merchant_reference_id,
        )
    }
}

impl CreateSubscriptionResponse {
    /// Convert from database subscription model to domain response
    pub fn from_subscription_db(
        subscription: diesel_models::subscription::Subscription,
        customer_id: common_utils::id_type::CustomerId,
    ) -> Self {
        Self {
            id: subscription.id,
            merchant_reference_id: subscription.merchant_reference_id,
            status: SubscriptionStatus::from_str(&subscription.status)
                .unwrap_or(SubscriptionStatus::Created),
            plan_id: None, // Not stored in the current DB schema
            profile_id: subscription.profile_id,
            client_secret: subscription.client_secret.map(Secret::new),
            merchant_id: subscription.merchant_id,
            coupon_code: None, // Not stored in the current DB schema
            customer_id,
        }
    }
}

/// Add FromStr implementation for SubscriptionStatus if not already present
impl FromStr for SubscriptionStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Active" => Ok(Self::Active),
            "Created" => Ok(Self::Created),
            "InActive" => Ok(Self::InActive),
            "Pending" => Ok(Self::Pending),
            _ => Err(()),
        }
    }
}
