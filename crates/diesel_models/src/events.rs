use common_utils::{
    crypto::OptionalEncryptableSecretString, custom_serde, encryption::Encryption,
    types::keymanager::ToEncryptable,
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::events};

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = events)]
pub struct EventNew {
    pub event_id: String,
    pub event_type: storage_enums::EventType,
    pub event_class: storage_enums::EventClass,
    pub is_webhook_notified: bool,
    pub primary_object_id: String,
    pub primary_object_type: storage_enums::EventObjectType,
    pub created_at: PrimitiveDateTime,
    pub merchant_id: Option<String>,
    pub business_profile_id: Option<String>,
    pub primary_object_created_at: Option<PrimitiveDateTime>,
    pub idempotent_event_id: Option<String>,
    pub initial_attempt_id: Option<String>,
    pub request: Option<Encryption>,
    pub response: Option<Encryption>,
    pub delivery_attempt: Option<storage_enums::WebhookDeliveryAttempt>,
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = events)]
pub struct EventUpdateInternal {
    pub is_webhook_notified: Option<bool>,
    pub response: Option<Encryption>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable)]
#[diesel(table_name = events, primary_key(event_id))]
pub struct Event {
    pub event_id: String,
    pub event_type: storage_enums::EventType,
    pub event_class: storage_enums::EventClass,
    pub is_webhook_notified: bool,
    pub primary_object_id: String,
    pub primary_object_type: storage_enums::EventObjectType,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub merchant_id: Option<String>,
    pub business_profile_id: Option<String>,
    // This column can be used to partition the database table, so that all events related to a
    // single object would reside in the same partition
    pub primary_object_created_at: Option<PrimitiveDateTime>,
    pub idempotent_event_id: Option<String>,
    pub initial_attempt_id: Option<String>,
    pub request: Option<Encryption>,
    pub response: Option<Encryption>,
    pub delivery_attempt: Option<storage_enums::WebhookDeliveryAttempt>,
}

pub struct EventWithEncryption {
    pub request: Option<Encryption>,
    pub response: Option<Encryption>,
}

pub struct EncryptableEvent {
    pub request: OptionalEncryptableSecretString,
    pub response: OptionalEncryptableSecretString,
}

impl ToEncryptable<EncryptableEvent, Secret<String>, Encryption> for EventWithEncryption {
    fn to_encryptable(self) -> rustc_hash::FxHashMap<String, Encryption> {
        let mut map = rustc_hash::FxHashMap::default();
        self.request.map(|x| map.insert("request".to_string(), x));
        self.response.map(|x| map.insert("response".to_string(), x));
        map
    }

    fn from_encryptable(
        mut hashmap: rustc_hash::FxHashMap<
            String,
            common_utils::crypto::Encryptable<Secret<String>>,
        >,
    ) -> common_utils::errors::CustomResult<EncryptableEvent, common_utils::errors::ParsingError>
    {
        Ok(EncryptableEvent {
            request: hashmap.remove("request"),
            response: hashmap.remove("response"),
        })
    }
}
