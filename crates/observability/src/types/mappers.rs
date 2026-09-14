use diesel_models::observability::{
    alerts_dicts::{AlertsDict, AlertsDictNew},
    raw_json::RawJson,
};
use error_stack::report;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::{not_blank, within_width};
use crate::{
    auth::UserName,
    errors::{ObservabilityApiResult, ObservabilityError},
};

const NAME_MAX_CHARS: usize = 64;

const KEY_MAX_CHARS: usize = 255;

const MAX_ENTRY_BYTES: usize = 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapperEntrySaveRequest {
    pub name: String,
    pub key: String,
    pub product: Option<RawJson>,
    pub values: Option<RawJson>,
    pub metadata: Option<RawJson>,
}

impl MapperEntrySaveRequest {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        not_blank("name", &self.name)?;
        not_blank("key", &self.key)?;
        within_width("name", Some(&self.name), NAME_MAX_CHARS)?;
        within_width("key", Some(&self.key), KEY_MAX_CHARS)?;

        let bytes = [&self.product, &self.values, &self.metadata]
            .into_iter()
            .flatten()
            .map(|column| column.get().len())
            .sum::<usize>();
        if bytes > MAX_ENTRY_BYTES {
            Err(report!(ObservabilityError::EntryTooLarge {
                bytes,
                limit: MAX_ENTRY_BYTES,
            }))?;
        }

        Ok(())
    }

    pub fn into_insertable(
        self,
        id: uuid::Uuid,
        user_name: Option<UserName>,
        now: PrimitiveDateTime,
    ) -> AlertsDictNew {
        AlertsDictNew {
            id,
            name: self.name,
            key_: self.key,
            product: self.product,
            values_: self.values,
            ts_created: now,
            is_enabled: true,
            username: user_name.map(UserName::get_secret),
            metadata: self.metadata,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MapperEntryResponse {
    pub id: uuid::Uuid,
    pub name: String,
    pub key: String,
    pub product: Option<RawJson>,
    pub values: Option<RawJson>,
    pub metadata: Option<RawJson>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_created: Option<PrimitiveDateTime>,
    pub username: Option<Secret<String>>,
}

impl From<AlertsDict> for MapperEntryResponse {
    fn from(entry: AlertsDict) -> Self {
        Self {
            id: entry.id,
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

#[derive(Debug, Serialize)]
pub struct MapperEntryListResponse {
    pub count: usize,
    pub entries: Vec<MapperEntryResponse>,
}

#[derive(Debug, Serialize)]
pub struct MapperEntryDeleteResponse {
    pub name: String,
    pub key: String,
    pub deleted: bool,
}
