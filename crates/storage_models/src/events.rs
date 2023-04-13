use common_utils::custom_serde;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::events};

#[derive(Clone, Debug, Deserialize, Insertable, Serialize, router_derive::DebugAsDisplay)]
#[diesel(table_name = events)]
#[serde(deny_unknown_fields)]
pub struct EventNew {
    pub event_id: String,
    pub event_type: storage_enums::EventType,
    pub event_class: storage_enums::EventClass,
    pub is_webhook_notified: bool,
    pub intent_reference_id: Option<String>,
    pub primary_object_id: String,
    pub primary_object_type: storage_enums::EventObjectType,
}

#[derive(Debug)]
pub struct EventUpdate {
    pub event_type: Option<storage_enums::EventType>,
    pub event_class: Option<storage_enums::EventClass>,
    pub is_webhook_notified: Option<bool>,
    pub intent_reference_id: Option<String>,
    pub primary_object_id: Option<String>,
    pub primary_object_type: Option<storage_enums::EventObjectType>,
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = events)]
pub struct EventUpdateInternal {
    pub event_type: Option<storage_enums::EventType>,
    pub event_class: Option<storage_enums::EventClass>,
    pub is_webhook_notified: Option<bool>,
    pub intent_reference_id: Option<String>,
    pub primary_object_id: Option<String>,
    pub primary_object_type: Option<storage_enums::EventObjectType>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable)]
#[diesel(table_name = events)]
pub struct Event {
    #[serde(skip_serializing)]
    pub id: i32,
    pub event_id: String,
    pub event_type: storage_enums::EventType,
    pub event_class: storage_enums::EventClass,
    pub is_webhook_notified: bool,
    pub intent_reference_id: Option<String>,
    pub primary_object_id: String,
    pub primary_object_type: storage_enums::EventObjectType,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}

impl From<EventUpdate> for EventUpdateInternal {
    fn from(event_update: EventUpdate) -> Self {
        Self {
            event_type: event_update.event_type,
            event_class: event_update.event_class,
            is_webhook_notified: event_update.is_webhook_notified,
            intent_reference_id: event_update.intent_reference_id,
            primary_object_id: event_update.primary_object_id,
            primary_object_type: event_update.primary_object_type,
        }
    }
}
