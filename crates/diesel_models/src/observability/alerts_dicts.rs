use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use hyperswitch_masking::Secret;
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
    pub username: Option<Secret<String>>,
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
    pub username: Option<Secret<String>>,
    pub metadata: Option<RawJson>,
}

#[derive(Debug)]
pub enum AlertsDictUpdate {
    Retire,
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = alerts_dicts)]
pub struct AlertsDictUpdateInternal {
    pub is_enabled: Option<bool>,
}

impl From<AlertsDictUpdate> for AlertsDictUpdateInternal {
    fn from(update: AlertsDictUpdate) -> Self {
        match update {
            AlertsDictUpdate::Retire => Self {
                is_enabled: Some(false),
            },
        }
    }
}
