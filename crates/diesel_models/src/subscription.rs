use common_utils::pii::SecretSerdeValue;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::subscription;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = subscription)]
pub struct SubscriptionNew {
    subscription_id: String,
    status: String,
    billing_processor: Option<String>,
    payment_method_id: Option<String>,
    mca_id: Option<String>,
    client_secret: Option<String>,
    connector_subscription_id: Option<String>,
    merchant_id: common_utils::id_type::MerchantId,
    customer_id: common_utils::id_type::CustomerId,
    metadata: Option<SecretSerdeValue>,
    created_at: time::PrimitiveDateTime,
    modified_at: time::PrimitiveDateTime,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize,
)]
#[diesel(table_name = subscription, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct Subscription {
    #[serde(skip_serializing, skip_deserializing)]
    pub id: i32,
    pub subscription_id: String,
    pub status: String,
    pub billing_processor: Option<String>,
    pub payment_method_id: Option<String>,
    pub mca_id: Option<String>,
    pub client_secret: Option<String>,
    pub connector_subscription_id: Option<String>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::CustomerId,
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
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
        subscription_id: String,
        status: String,
        billing_processor: Option<String>,
        payment_method_id: Option<String>,
        mca_id: Option<String>,
        client_secret: Option<String>,
        connector_subscription_id: Option<String>,
        merchant_id: common_utils::id_type::MerchantId,
        customer_id: common_utils::id_type::CustomerId,
        metadata: Option<SecretSerdeValue>,
    ) -> Self {
        let now = common_utils::date_time::now();
        Self {
            subscription_id,
            status,
            billing_processor,
            payment_method_id,
            mca_id,
            client_secret,
            connector_subscription_id,
            merchant_id,
            customer_id,
            metadata,
            created_at: now,
            modified_at: now,
        }
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
