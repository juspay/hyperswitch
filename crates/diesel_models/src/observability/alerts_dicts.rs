use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::{raw_json::RawJson, schema::alerts_dicts};

#[derive(
    Clone, Debug, Queryable, Identifiable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = alerts_dicts, check_for_backend(diesel::pg::Pg))]
pub struct AlertsDict {
    pub id: uuid::Uuid,
    pub name: String,
    pub key_: String,
    pub product: Option<RawJson>,
    pub values_: Option<RawJson>,
    pub ts_created: Option<PrimitiveDateTime>,
    pub is_enabled: Option<bool>,
    pub username: Option<String>,
    pub metadata: Option<RawJson>,
}

#[derive(Clone, Debug, Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = alerts_dicts)]
pub struct AlertsDictNew {
    pub name: String,
    pub key_: String,
    pub product: Option<RawJson>,
    pub values_: Option<RawJson>,
    pub ts_created: PrimitiveDateTime,
    pub username: Option<String>,
    pub metadata: Option<RawJson>,
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = alerts_dicts)]
pub struct AlertsDictRetire {
    pub is_enabled: Option<bool>,
}
