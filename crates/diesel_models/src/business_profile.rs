use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};

use crate::schema::business_profile;

#[derive(
    Clone,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Identifiable,
    Queryable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = business_profile, primary_key(profile_id))]
pub struct BusinessProfile {
    pub profile_id: String,
    pub merchant_id: String,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<serde_json::Value>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub is_recon_enabled: bool,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile, primary_key(profile_id))]
pub struct BusinessProfileNew {
    pub profile_id: String,
    pub merchant_id: String,
    pub profile_name: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: bool,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: bool,
    pub webhook_details: Option<serde_json::Value>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub is_recon_enabled: bool,
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = business_profile)]
pub struct BusinessProfileUpdateInternal {
    pub profile_name: Option<String>,
    pub modified_at: Option<time::PrimitiveDateTime>,
    pub return_url: Option<String>,
    pub enable_payment_response_hash: Option<bool>,
    pub payment_response_hash_key: Option<String>,
    pub redirect_to_merchant_with_http_post: Option<bool>,
    pub webhook_details: Option<serde_json::Value>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub routing_algorithm: Option<serde_json::Value>,
    pub intent_fulfillment_time: Option<i64>,
    pub frm_routing_algorithm: Option<serde_json::Value>,
    pub payout_routing_algorithm: Option<serde_json::Value>,
    pub is_recon_enabled: Option<bool>,
}

impl From<BusinessProfileNew> for BusinessProfile {
    fn from(new: BusinessProfileNew) -> Self {
        BusinessProfile {
            profile_id: new.profile_id,
            merchant_id: new.merchant_id,
            profile_name: new.profile_name,
            created_at: new.created_at,
            modified_at: new.modified_at,
            return_url: new.return_url,
            enable_payment_response_hash: new.enable_payment_response_hash,
            payment_response_hash_key: new.payment_response_hash_key,
            redirect_to_merchant_with_http_post: new.redirect_to_merchant_with_http_post,
            webhook_details: new.webhook_details,
            metadata: new.metadata,
            routing_algorithm: new.routing_algorithm,
            intent_fulfillment_time: new.intent_fulfillment_time,
            frm_routing_algorithm: new.frm_routing_algorithm,
            payout_routing_algorithm: new.payout_routing_algorithm,
            is_recon_enabled: new.is_recon_enabled,
        }
    }
}

impl BusinessProfileUpdateInternal {
    pub fn apply_changeset(self, source: BusinessProfile) -> BusinessProfile {
        BusinessProfile {
            profile_name: self.profile_name.unwrap_or(source.profile_name),
            modified_at: self.modified_at.unwrap_or(source.modified_at),
            return_url: self.return_url,
            enable_payment_response_hash: self
                .enable_payment_response_hash
                .unwrap_or(source.enable_payment_response_hash),
            payment_response_hash_key: self.payment_response_hash_key,
            redirect_to_merchant_with_http_post: self
                .redirect_to_merchant_with_http_post
                .unwrap_or(source.redirect_to_merchant_with_http_post),
            webhook_details: self.webhook_details,
            metadata: self.metadata,
            routing_algorithm: self.routing_algorithm,
            intent_fulfillment_time: self.intent_fulfillment_time,
            frm_routing_algorithm: self.frm_routing_algorithm,
            payout_routing_algorithm: self.payout_routing_algorithm,
            is_recon_enabled: self.is_recon_enabled.unwrap_or(source.is_recon_enabled),
            ..source
        }
    }
}
