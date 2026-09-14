use diesel_models::observability::{
    alerts_dicts::{AlertsDict, AlertsDictNew},
    raw_json::RawJson,
};
use error_stack::{report, ResultExt};
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

const DEFAULT_USER_NAME: &str = "reliability_team";

const EMPTY_LIST: &str = "[]";

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
    ) -> ObservabilityApiResult<AlertsDictNew> {
        Ok(AlertsDictNew {
            id,
            name: self.name,
            key_: self.key,
            product: or_empty_list(self.product)?,
            values_: or_empty_list(self.values)?,
            ts_created: now,
            is_enabled: true,
            username: user_name.map_or_else(
                || Secret::new(DEFAULT_USER_NAME.to_owned()),
                UserName::get_secret,
            ),
            metadata: or_empty_list(self.metadata)?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct MapperEntryResponse {
    pub id: uuid::Uuid,
    pub name: String,
    pub key: String,
    pub product: RawJson,
    pub values: RawJson,
    pub metadata: RawJson,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_created: PrimitiveDateTime,
    pub username: Secret<String>,
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

fn or_empty_list(column: Option<RawJson>) -> ObservabilityApiResult<RawJson> {
    column.map_or_else(
        || {
            serde_json::from_str(EMPTY_LIST)
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to build an empty JSON list")
        },
        Ok,
    )
}
