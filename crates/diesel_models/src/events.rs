use common_utils::custom_serde;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, enums as storage_enums, schema::events};

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = events)]
pub struct EventNew {
    pub event_id: String,
    pub event_type: storage_enums::EventType,
    pub event_class: storage_enums::EventClass,
    pub is_webhook_notified: bool,
    pub primary_object_id: String,
    pub primary_object_type: storage_enums::EventObjectType,
    pub idempotent_event_id: Option<String>,
    pub initial_attempt_id: Option<String>,
    pub request: Option<Encryption>,
    pub response: Option<Encryption>,
}

#[derive(Debug)]
pub enum EventUpdate {
    UpdateWebhookNotified { is_webhook_notified: Option<bool> },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = events)]
pub struct EventUpdateInternal {
    pub is_webhook_notified: Option<bool>,
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
    pub idempotent_event_id: Option<String>,
    pub initial_attempt_id: Option<String>,
    pub request: Option<Encryption>,
    pub response: Option<Encryption>,
}

impl From<EventUpdate> for EventUpdateInternal {
    fn from(event_update: EventUpdate) -> Self {
        match event_update {
            EventUpdate::UpdateWebhookNotified {
                is_webhook_notified,
            } => Self {
                is_webhook_notified,
            },
        }
    }
}
