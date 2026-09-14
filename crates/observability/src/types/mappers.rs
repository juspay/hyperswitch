use diesel_models::observability::alerts_dicts::AlertsDict;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};

#[derive(Debug, Serialize)]
pub struct MapperEntry {
    pub name: String,
    pub key: String,
    pub product: Box<RawValue>,
    pub values: Box<RawValue>,
    pub metadata: Box<RawValue>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_created: PrimitiveDateTime,
    pub username: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapperUpsertRequest {
    pub name: String,
    pub key: String,
    #[serde(default)]
    pub product: Option<Box<RawValue>>,
    #[serde(default)]
    pub values: Option<Box<RawValue>>,
    #[serde(default)]
    pub metadata: Option<Box<RawValue>>,
}

#[derive(Debug, Serialize)]
pub struct MapperListResponse {
    pub status: ReadStatus,
    pub entries: Vec<MapperEntry>,
}

#[derive(Debug, Serialize)]
pub struct MapperSaveResponse {
    pub status: WriteStatus,
    pub entry: MapperEntry,
}

#[derive(Debug, Serialize)]
pub struct MapperRetireResponse {
    pub status: WriteStatus,
}

impl From<AlertsDict> for MapperEntry {
    fn from(entry: AlertsDict) -> Self {
        Self {
            name: entry.name,
            key: entry.key_,
            product: entry.product.into_raw(),
            values: entry.values_.into_raw(),
            metadata: entry.metadata.into_raw(),
            ts_created: entry.ts_created,
            username: entry.username,
        }
    }
}
