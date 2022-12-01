use common_utils::custom_serde;
#[cfg(feature = "diesel")]
use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[cfg(feature = "diesel")]
use crate::schema::events;
use crate::types::storage::enums;

#[derive(Clone, Debug, Deserialize, Serialize, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = events))]
#[serde(deny_unknown_fields)]
pub struct EventNew {
    pub event_id: String,
    pub event_type: enums::EventType,
    pub event_class: enums::EventClass,
    pub is_webhook_notified: bool,
    pub intent_reference_id: Option<String>,
    pub primary_object_id: String,
    pub primary_object_type: enums::EventObjectType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "diesel", derive(Identifiable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = events))]
pub struct Event {
    #[serde(skip_serializing)]
    pub id: i32,
    pub event_id: String,
    pub event_type: enums::EventType,
    pub event_class: enums::EventClass,
    pub is_webhook_notified: bool,
    pub intent_reference_id: Option<String>,
    pub primary_object_id: String,
    pub primary_object_type: enums::EventObjectType,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}
