use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{schema::events, types::storage::enums, utils::custom_serde};

#[derive(Clone, Debug, Deserialize, Serialize, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = events)]
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

#[derive(Clone, Debug, Identifiable, Queryable, Deserialize, Serialize)]
#[diesel(table_name = events)]
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
