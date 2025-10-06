use std::str::FromStr;

use api_models::{enums as api_enums, payments::Address};
use common_utils::{
    errors::{CustomResult, ValidationError},
    generate_id_with_default_len,
    pii::SecretSerdeValue,
    types::{
        keymanager::{self, KeyManagerState},
        MinorUnit,
    },
};
use masking::{ExposeInterface, PeekInterface, Secret};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::merchant_key_store::MerchantKeyStore;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    pub amount: MinorUnit,
    pub currency: api_enums::Currency,
    pub merchant_reference_id: Option<String>,
    pub plan_id: Option<String>,
    pub coupon_code: Option<String>,
    pub customer_id: common_utils::id_type::CustomerId,
    pub payment_details: CreateSubscriptionPaymentDetails,
    pub billing: Option<Address>,
    pub shipping: Option<Address>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateSubscriptionPaymentDetails {
    pub return_url: common_utils::types::Url,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    pub authentication_type: Option<api_enums::AuthenticationType>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CreateSubscriptionResponse {
    pub id: common_utils::id_type::SubscriptionId,
    pub merchant_reference_id: Option<String>,
    pub status: SubscriptionStatus,
    pub plan_id: Option<String>,
    pub profile_id: common_utils::id_type::ProfileId,
    pub client_secret: Option<Secret<String>>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub coupon_code: Option<String>,
    pub customer_id: common_utils::id_type::CustomerId,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub enum SubscriptionStatus {
    Active,
    Created,
    InActive,
    Pending,
    Trial,
    Paused,
    Unpaid,
    Onetime,
    Cancelled,
    Failed,
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Created => write!(f, "Created"),
            Self::InActive => write!(f, "InActive"),
            Self::Pending => write!(f, "Pending"),
            Self::Trial => write!(f, "Trial"),
            Self::Paused => write!(f, "Paused"),
            Self::Unpaid => write!(f, "Unpaid"),
            Self::Onetime => write!(f, "Onetime"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

// Type conversions between API models and domain models

/// Convert from API model `CreateSubscriptionRequest` to domain model `CreateSubscriptionRequest`
#[cfg(feature = "v1")]
impl From<api_models::subscription::CreateSubscriptionRequest> for CreateSubscriptionRequest {
    fn from(api_request: api_models::subscription::CreateSubscriptionRequest) -> Self {
        Self {
            amount: api_request.amount,
            currency: api_request.currency,
            merchant_reference_id: api_request.merchant_reference_id,
            plan_id: api_request.plan_id,
            coupon_code: api_request.coupon_code,
            customer_id: api_request.customer_id,
            payment_details: api_request.payment_details.into(),
            billing: api_request.billing,
            shipping: api_request.shipping,
        }
    }
}

#[cfg(feature = "v1")]
impl From<api_models::subscription::CreateSubscriptionPaymentDetails>
    for CreateSubscriptionPaymentDetails
{
    fn from(api_details: api_models::subscription::CreateSubscriptionPaymentDetails) -> Self {
        Self {
            return_url: api_details.return_url,
            setup_future_usage: api_details.setup_future_usage,
            capture_method: api_details.capture_method,
            authentication_type: api_details.authentication_type,
        }
    }
}

/// Convert from domain model `CreateSubscriptionResponse` to API model `CreateSubscriptionResponse`
#[cfg(feature = "v1")]
impl From<CreateSubscriptionResponse> for api_models::subscription::SubscriptionResponse {
    fn from(domain_response: CreateSubscriptionResponse) -> Self {
        Self {
            id: domain_response.id,
            merchant_reference_id: domain_response.merchant_reference_id,
            status: api_models::subscription::SubscriptionStatus::from(domain_response.status),
            plan_id: domain_response.plan_id,
            profile_id: domain_response.profile_id,
            client_secret: domain_response.client_secret,
            merchant_id: domain_response.merchant_id,
            coupon_code: domain_response.coupon_code,
            customer_id: domain_response.customer_id,
        }
    }
}

/// Convert from domain model `SubscriptionStatus` to API model `SubscriptionStatus`
#[cfg(feature = "v1")]
impl From<SubscriptionStatus> for api_models::subscription::SubscriptionStatus {
    fn from(domain_status: SubscriptionStatus) -> Self {
        match domain_status {
            SubscriptionStatus::Active => Self::Active,
            SubscriptionStatus::Created => Self::Created,
            SubscriptionStatus::InActive => Self::InActive,
            SubscriptionStatus::Pending => Self::Pending,
            SubscriptionStatus::Trial => Self::Active,
            SubscriptionStatus::Paused => Self::InActive,
            SubscriptionStatus::Unpaid => Self::Pending,
            SubscriptionStatus::Onetime => Self::Onetime,
            SubscriptionStatus::Cancelled => Self::InActive,
            SubscriptionStatus::Failed => Self::InActive,
        }
    }
}

impl CreateSubscriptionRequest {
    pub fn to_subscription(
        self,
        id: common_utils::id_type::SubscriptionId,
        profile_id: common_utils::id_type::ProfileId,
        merchant_id: common_utils::id_type::MerchantId,
    ) -> Subscription {
        Subscription {
            id,
            status: SubscriptionStatus::Created.to_string(),
            billing_processor: None,
            payment_method_id: None,
            merchant_connector_id: None,
            client_secret: None,
            connector_subscription_id: None,
            merchant_id,
            customer_id: self.customer_id,
            metadata: None,
            profile_id,
            merchant_reference_id: self.merchant_reference_id,
        }
    }
}

impl CreateSubscriptionResponse {
    pub fn from_subscription_db(
        subscription: Subscription,
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

/// Add FromStr implementation for SubscriptionStatus
impl FromStr for SubscriptionStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Active" => Ok(Self::Active),
            "Created" => Ok(Self::Created),
            "InActive" => Ok(Self::InActive),
            "Pending" => Ok(Self::Pending),
            "Trail" => Ok(Self::Trial),
            "Paused" => Ok(Self::Paused),
            "Unpaid" => Ok(Self::Unpaid),
            "Onetime" => Ok(Self::Onetime),
            "Cancelled" => Ok(Self::Cancelled),
            "Failed" => Ok(Self::Failed),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Subscription {
    pub id: common_utils::id_type::SubscriptionId,
    pub status: String,
    pub billing_processor: Option<String>,
    pub payment_method_id: Option<String>,
    pub merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub client_secret: Option<String>,
    pub connector_subscription_id: Option<String>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub metadata: Option<SecretSerdeValue>,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_reference_id: Option<String>,
}

impl Subscription {
    pub fn generate_and_set_client_secret(&mut self) -> Secret<String> {
        let client_secret =
            generate_id_with_default_len(&format!("{}_secret", self.id.get_string_repr()));
        self.client_secret = Some(client_secret.clone());
        Secret::new(client_secret)
    }
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for Subscription {
    type DstType = diesel_models::subscription::Subscription;
    type NewDstType = diesel_models::subscription::SubscriptionNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        let now = common_utils::date_time::now();
        Ok(diesel_models::subscription::Subscription {
            id: self.id,
            status: self.status,
            billing_processor: self.billing_processor,
            payment_method_id: self.payment_method_id,
            merchant_connector_id: self.merchant_connector_id,
            client_secret: self.client_secret,
            connector_subscription_id: self.connector_subscription_id,
            merchant_id: self.merchant_id,
            customer_id: self.customer_id,
            metadata: self.metadata.map(|m| m.expose()),
            created_at: now,
            modified_at: now,
            profile_id: self.profile_id,
            merchant_reference_id: self.merchant_reference_id,
        })
    }

    async fn convert_back(
        _state: &KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: item.id,
            status: item.status,
            billing_processor: item.billing_processor,
            payment_method_id: item.payment_method_id,
            merchant_connector_id: item.merchant_connector_id,
            client_secret: item.client_secret,
            connector_subscription_id: item.connector_subscription_id,
            merchant_id: item.merchant_id,
            customer_id: item.customer_id,
            metadata: item.metadata.map(SecretSerdeValue::new),
            profile_id: item.profile_id,
            merchant_reference_id: item.merchant_reference_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::subscription::SubscriptionNew::new(
            self.id,
            self.status,
            self.billing_processor,
            self.payment_method_id,
            self.merchant_connector_id,
            self.client_secret,
            self.connector_subscription_id,
            self.merchant_id,
            self.customer_id,
            self.metadata,
            self.profile_id,
            self.merchant_reference_id,
        ))
    }
}

#[async_trait::async_trait]
pub trait SubscriptionInterface {
    type Error;
    async fn insert_subscription_entry(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        subscription_new: Subscription,
    ) -> CustomResult<Subscription, Self::Error>;

    async fn find_by_merchant_id_subscription_id(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
    ) -> CustomResult<Subscription, Self::Error>;

    async fn update_subscription_entry(
        &self,
        state: &KeyManagerState,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
        data: SubscriptionUpdate,
    ) -> CustomResult<Subscription, Self::Error>;
}

pub struct SubscriptionUpdate {
    pub connector_subscription_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub status: Option<String>,
    pub modified_at: PrimitiveDateTime,
}

impl SubscriptionUpdate {
    pub fn new(
        payment_method_id: Option<Secret<String>>,
        status: Option<String>,
        connector_subscription_id: Option<String>,
    ) -> Self {
        Self {
            payment_method_id: payment_method_id.map(|pmid| pmid.peek().clone()),
            status,
            connector_subscription_id,
            modified_at: common_utils::date_time::now(),
        }
    }
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for SubscriptionUpdate {
    type DstType = diesel_models::subscription::SubscriptionUpdate;
    type NewDstType = diesel_models::subscription::SubscriptionUpdate;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::subscription::SubscriptionUpdate {
            connector_subscription_id: self.connector_subscription_id,
            payment_method_id: self.payment_method_id,
            status: self.status,
            modified_at: self.modified_at,
        })
    }

    async fn convert_back(
        _state: &KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            connector_subscription_id: item.connector_subscription_id,
            payment_method_id: item.payment_method_id,
            status: item.status,
            modified_at: item.modified_at,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::subscription::SubscriptionUpdate {
            connector_subscription_id: self.connector_subscription_id,
            payment_method_id: self.payment_method_id,
            status: self.status,
            modified_at: self.modified_at,
        })
    }
}
