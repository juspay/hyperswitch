//! Typed API models for alert dictionary state.

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

fn default_product() -> String {
    "[]".to_owned()
}

fn default_values() -> String {
    "[]".to_owned()
}

fn default_metadata() -> String {
    "{}".to_owned()
}

/// Full replacement body for one dictionary entry.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictionaryUpsertRequest {
    pub name: String,
    #[serde(rename = "key_")]
    pub key: String,
    #[serde(default = "default_product")]
    pub product: String,
    #[serde(default = "default_values", rename = "values_")]
    pub values: String,
    #[serde(default = "default_metadata")]
    pub metadata: String,
    pub updated_by: String,
}

/// A stored dictionary entry. JSON payloads intentionally remain encoded strings.
#[derive(Clone, Debug, Serialize)]
pub struct DictionaryEntryResponse {
    pub name: String,
    #[serde(rename = "key_")]
    pub key: String,
    pub product: String,
    #[serde(rename = "values_")]
    pub values: String,
    pub metadata: String,
    pub updated_by: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct DictionaryListResponse {
    pub entries: Vec<DictionaryEntryResponse>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DictionaryUpsertResponse {
    pub ok: bool,
    pub name: String,
    #[serde(rename = "key_")]
    pub key: String,
}
