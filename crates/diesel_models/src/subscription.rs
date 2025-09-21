use common_utils::{generate_id_with_default_len, pii::SecretSerdeValue};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::schema::subscription;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = subscription)]
pub struct SubscriptionNew {
    id: common_utils::id_type::SubscriptionId,
    status: String,
    billing_processor: Option<String>,
    payment_method_id: Option<String>,
    merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
    client_secret: Option<String>,
    connector_subscription_id: Option<String>,
    merchant_id: common_utils::id_type::MerchantId,
    customer_id: common_utils::id_type::CustomerId,
    metadata: Option<SecretSerdeValue>,
    created_at: time::PrimitiveDateTime,
    modified_at: time::PrimitiveDateTime,
    profile_id: common_utils::id_type::ProfileId,
    merchant_reference_id: Option<String>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize,
)]
#[diesel(table_name = subscription, primary_key(id), check_for_backend(diesel::pg::Pg))]
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
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_reference_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, AsChangeset, router_derive::DebugAsDisplay, Deserialize)]
#[diesel(table_name = subscription)]
pub struct SubscriptionUpdate {
    pub payment_method_id: Option<String>,
    pub status: Option<String>,
    pub modified_at: time::PrimitiveDateTime,
}

impl SubscriptionNew {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: common_utils::id_type::SubscriptionId,
        status: String,
        billing_processor: Option<String>,
        payment_method_id: Option<String>,
        merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
        client_secret: Option<String>,
        connector_subscription_id: Option<String>,
        merchant_id: common_utils::id_type::MerchantId,
        customer_id: common_utils::id_type::CustomerId,
        metadata: Option<SecretSerdeValue>,
        profile_id: common_utils::id_type::ProfileId,
        merchant_reference_id: Option<String>,
    ) -> Self {
        let now = common_utils::date_time::now();
        Self {
            id,
            status,
            billing_processor,
            payment_method_id,
            merchant_connector_id,
            client_secret,
            connector_subscription_id,
            merchant_id,
            customer_id,
            metadata,
            created_at: now,
            modified_at: now,
            profile_id,
            merchant_reference_id,
        }
    }

    pub fn generate_and_set_client_secret(&mut self) -> Secret<String> {
        let client_secret =
            generate_id_with_default_len(&format!("{}_secret", self.id.get_string_repr()));
        self.client_secret = Some(client_secret.clone());
        Secret::new(client_secret)
    }
}

impl SubscriptionUpdate {
    pub fn new(payment_method_id: Option<String>, status: Option<String>) -> Self {
        Self {
            payment_method_id,
            status,
            modified_at: common_utils::date_time::now(),
        }
    }
}
