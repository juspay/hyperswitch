use common_utils::{
    errors::{CustomResult, ValidationError},
    events::ApiEventMetric,
    generate_id_with_default_len,
    pii::SecretSerdeValue,
    types::keymanager::{self, KeyManagerState},
};
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret};
use time::PrimitiveDateTime;

use crate::{errors::api_error_response::ApiErrorResponse, merchant_key_store::MerchantKeyStore};

const SECRET_SPLIT: &str = "_secret";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClientSecret(String);

impl ClientSecret {
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    pub fn get_subscription_id(&self) -> error_stack::Result<String, ApiErrorResponse> {
        let sub_id = self
            .0
            .split(SECRET_SPLIT)
            .next()
            .ok_or(ApiErrorResponse::MissingRequiredField {
                field_name: "client_secret",
            })
            .attach_printable("Failed to extract subscription_id from client_secret")?;

        Ok(sub_id.to_string())
    }
}

impl std::fmt::Display for ClientSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ApiEventMetric for ClientSecret {}

impl From<api_models::subscription::ClientSecret> for ClientSecret {
    fn from(api_secret: api_models::subscription::ClientSecret) -> Self {
        Self::new(api_secret.as_str().to_string())
    }
}

impl From<ClientSecret> for api_models::subscription::ClientSecret {
    fn from(domain_secret: ClientSecret) -> Self {
        Self::new(domain_secret.to_string())
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
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_reference_id: Option<String>,
    pub plan_id: Option<String>,
    pub item_price_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
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
            plan_id: self.plan_id,
            item_price_id: self.item_price_id,
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
            created_at: item.created_at,
            modified_at: item.modified_at,
            profile_id: item.profile_id,
            merchant_reference_id: item.merchant_reference_id,
            plan_id: item.plan_id,
            item_price_id: item.item_price_id,
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
            self.plan_id,
            self.item_price_id,
        ))
    }
}

#[async_trait::async_trait]
pub trait SubscriptionInterface {
    type Error;
    async fn insert_subscription_entry(
        &self,
        key_store: &MerchantKeyStore,
        subscription_new: Subscription,
    ) -> CustomResult<Subscription, Self::Error>;

    async fn find_by_merchant_id_subscription_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        subscription_id: String,
    ) -> CustomResult<Subscription, Self::Error>;

    async fn update_subscription_entry(
        &self,
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
    pub plan_id: Option<String>,
    pub item_price_id: Option<String>,
}

impl SubscriptionUpdate {
    pub fn new(
        connector_subscription_id: Option<String>,
        payment_method_id: Option<Secret<String>>,
        status: Option<String>,
        plan_id: Option<String>,
        item_price_id: Option<String>,
    ) -> Self {
        Self {
            connector_subscription_id,
            payment_method_id: payment_method_id.map(|pmid| pmid.peek().clone()),
            status,
            modified_at: common_utils::date_time::now(),
            plan_id,
            item_price_id,
        }
    }

    pub fn update_status(status: String) -> Self {
        Self::new(None, None, Some(status), None, None)
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
            plan_id: self.plan_id,
            item_price_id: self.item_price_id,
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
            plan_id: item.plan_id,
            item_price_id: item.item_price_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::subscription::SubscriptionUpdate {
            connector_subscription_id: self.connector_subscription_id,
            payment_method_id: self.payment_method_id,
            status: self.status,
            modified_at: self.modified_at,
            plan_id: self.plan_id,
            item_price_id: self.item_price_id,
        })
    }
}
