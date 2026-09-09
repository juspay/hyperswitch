use common_utils::{encryption::Encryption, id_type};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::schema::resources;

#[derive(
    Clone,
    Debug,
    Identifiable,
    Queryable,
    Selectable,
    router_derive::DebugAsDisplay,
    serde::Serialize,
    serde::Deserialize,
)]
#[diesel(table_name = resources, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct Resource {
    pub id: id_type::ResourceId,
    pub resource_type: String,
    pub scope: String,
    pub scope_id: String,
    pub data: serde_json::Value,
    pub encrypted_data: Option<Encryption>,
    pub created_by: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = resources)]
pub struct ResourceNew {
    pub id: id_type::ResourceId,
    pub resource_type: String,
    pub scope: String,
    pub scope_id: String,
    pub data: serde_json::Value,
    pub encrypted_data: Option<Encryption>,
    pub created_by: String,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = resources)]
pub struct ResourceUpdateInternal {
    pub data: Option<serde_json::Value>,
    pub modified_at: time::PrimitiveDateTime,
}
