use common_utils::custom_serde;
use diesel::{Identifiable, Insertable, Queryable};
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
