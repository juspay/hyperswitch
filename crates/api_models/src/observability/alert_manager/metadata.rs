//! Typed API models for per-alert metadata and snooze state.

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// Partial replacement body for one alert identity.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertMetadataPatchRequest {
    pub metadata: Option<String>,
    pub snooze: Option<String>,
    pub updated_by: String,
}

/// Stored metadata and snooze strings for one alert identity.
#[derive(Clone, Debug, Serialize)]
pub struct AlertMetadataEntryResponse {
    pub id: String,
    pub metadata: String,
    pub snooze: String,
    pub updated_by: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct AlertMetadataListResponse {
    pub entries: Vec<AlertMetadataEntryResponse>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AlertMetadataPatchResponse {
    pub ok: bool,
    pub id: String,
}
