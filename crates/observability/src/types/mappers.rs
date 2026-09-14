use common_utils::ext_traits::OptionExt;
use diesel_models::observability::{alerts_dicts::AlertsDict, raw_json::RawJson};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};
use crate::errors::ObservabilityError;

#[derive(Debug, Serialize)]
pub struct MapperEntry {
    pub name: String,
    pub key: String,
    pub product: Option<Box<RawValue>>,
    pub values: Option<Box<RawValue>>,
    pub metadata: Option<Box<RawValue>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_created: Option<PrimitiveDateTime>,
    pub username: Option<String>,
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

impl TryFrom<AlertsDict> for MapperEntry {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(entry: AlertsDict) -> Result<Self, Self::Error> {
        let id = entry.id;

        Ok(Self {
            name: entry
                .name
                .get_required_value("name")
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable_lazy(|| format!("Mapper entry {id} has no name"))?,
            key: entry
                .key_
                .get_required_value("key_")
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable_lazy(|| format!("Mapper entry {id} has no key"))?,
            product: entry.product.map(RawJson::into_raw),
            values: entry.values_.map(RawJson::into_raw),
            metadata: entry.metadata.map(RawJson::into_raw),
            ts_created: entry.ts_created,
            username: entry.username,
        })
    }
}
