use common_utils::{
    errors::CustomResult, events::ApiEventMetric, generate_id_with_default_len,
    pii::SecretSerdeValue,
};
use error_stack::ResultExt;
use hyperswitch_masking::{PeekInterface, Secret};
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

    async fn list_by_merchant_id_profile_id(
        &self,
        key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<Subscription>, Self::Error>;
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
