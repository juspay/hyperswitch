//! Success-rate threshold overrides owned by the observability plane.

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// Natural key shared by upsert and delete requests.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdDeleteRequest {
    pub name: String,
    pub product: String,
    pub merchant_id: String,
    #[serde(default)]
    pub profile_id: String,
    pub updated_by: String,
}

/// Replacement body for one threshold override.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdUpsertRequest {
    pub name: String,
    pub product: String,
    pub merchant_id: String,
    #[serde(default)]
    pub profile_id: String,
    pub min_volume: Option<f64>,
    pub min_impacted_volume: Option<f64>,
    pub tolerance: Option<f64>,
    pub diff_threshold: Option<f64>,
    pub updated_by: String,
}

/// A stored threshold override.
#[derive(Clone, Debug, Serialize)]
pub struct ThresholdResponse {
    pub name: String,
    pub product: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub min_volume: Option<f64>,
    pub min_impacted_volume: Option<f64>,
    pub tolerance: Option<f64>,
    pub diff_threshold: Option<f64>,
    pub updated_by: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
    pub is_deleted: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ThresholdListResponse {
    pub overrides: Vec<ThresholdResponse>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ThresholdUpsertResponse {
    pub threshold: ThresholdResponse,
}

#[derive(Clone, Debug, Serialize)]
pub struct ThresholdDeleteResponse {
    pub status: &'static str,
}
