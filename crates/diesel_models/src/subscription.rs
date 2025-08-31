use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::subscription;

#[derive(Clone, Debug, Eq, Insertable, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = subscription)]
pub struct SubscriptionNew {
    pub id: String,
    pub subscription_id: Option<String>,
    pub billing_processor: Option<String>,
    pub payment_method_id: Option<String>,
    pub mca_id: Option<String>,
    pub client_secret: Option<String>,
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize,
)]
#[diesel(table_name = subscription, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct Subscription {
    pub id: String,
    pub subscription_id: Option<String>,
    pub billing_processor: Option<String>,
    pub payment_method_id: Option<String>,
    pub mca_id: Option<String>,
    pub client_secret: Option<String>,
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub metadata: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Eq, PartialEq, AsChangeset, router_derive::DebugAsDisplay, Deserialize)]
#[diesel(table_name = subscription)]
pub struct SubscriptionUpdate {
    pub subscription_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub modified_at: time::PrimitiveDateTime,
}

impl SubscriptionNew {
    pub fn new(
        id: String,
        subscription_id: Option<String>,
        billing_processor: Option<String>,
        payment_method_id: Option<String>,
        mca_id: Option<String>,
        client_secret: Option<String>,
        customer_id: common_utils::id_type::CustomerId,
        merchant_id: common_utils::id_type::MerchantId,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        let now = common_utils::date_time::now();
        Self {
            id,
            subscription_id,
            billing_processor,
            payment_method_id,
            mca_id,
            client_secret,
            customer_id,
            merchant_id,
            metadata,
            created_at: now,
            modified_at: now,
        }
    }
}

impl SubscriptionUpdate {
    pub fn new(subscription_id: Option<String>, payment_method_id: Option<String>) -> Self {
        Self {
            subscription_id,
            payment_method_id,
            modified_at: common_utils::date_time::now(),
        }
    }
}
