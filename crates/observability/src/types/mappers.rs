use diesel_models::observability::{alerts_dicts::AlertsDict, raw_json::RawJson};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};

#[derive(Debug, Serialize)]
pub struct MapperEntry {
    pub name: String,
    pub key: String,
    pub product: Option<RawJson>,
    pub values: Option<RawJson>,
    pub metadata: Option<RawJson>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_created: Option<PrimitiveDateTime>,
    pub username: Option<Secret<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapperUpsertRequest {
    pub name: String,
    pub key: String,
    pub product: Option<RawJson>,
    pub values: Option<RawJson>,
    pub metadata: Option<RawJson>,
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
            product: entry.product,
            values: entry.values_,
            metadata: entry.metadata,
            ts_created: entry.ts_created,
            username: entry.username,
        }
    }
}
