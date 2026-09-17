//! Typed API models for alert blacklist state.

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// Replacement body for one blacklist entry.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlacklistUpsertRequest {
    pub rule_id: String,
    pub merchant_id: String,
    #[serde(default)]
    pub profile_id: String,
    #[serde(default)]
    pub reason: String,
    pub created_by: String,
}

/// Natural key and attribution for an idempotent tombstone.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlacklistDeleteRequest {
    pub rule_id: String,
    pub merchant_id: String,
    #[serde(default)]
    pub profile_id: String,
    pub created_by: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct BlacklistEntry {
    pub rule_id: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub reason: String,
    pub created_by: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct BlacklistListResponse {
    pub entries: Vec<BlacklistEntry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BlacklistUpsertResponse {
    pub status: &'static str,
    pub rule_id: String,
    pub merchant_id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct BlacklistDeleteResponse {
    pub status: &'static str,
}
