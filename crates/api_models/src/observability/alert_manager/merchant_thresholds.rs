//! Per-merchant threshold overrides, as the observability service's
//! `/alerts/alerts_manager/merchant_thresholds` routes accept and return them.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use time::PrimitiveDateTime;

/// One value, or a list of values to match any of. An empty list means no filter on this field.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum MerchantThresholdsTextFilter {
    One(String),
    Many(Vec<String>),
}

/// A range of `last_updated_at`, or a bare timestamp meaning "at or after".
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum MerchantThresholdsTimeFilter {
    Range {
        #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
        start: Option<PrimitiveDateTime>,
        #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
        end: Option<PrimitiveDateTime>,
    },
    From(#[serde(with = "common_utils::custom_serde::iso8601")] PrimitiveDateTime),
}

/// The body of `POST /alerts/alerts_manager/merchant_thresholds/list`.
///
/// Every field is optional; `{}` matches every row.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantThresholdsListRequest {
    pub id: Option<MerchantThresholdsTextFilter>,
    pub name: Option<MerchantThresholdsTextFilter>,
    pub product: Option<MerchantThresholdsTextFilter>,
    pub merchant_id: Option<MerchantThresholdsTextFilter>,
    pub profile_id: Option<MerchantThresholdsTextFilter>,
    pub author: Option<MerchantThresholdsTextFilter>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<BTreeMap<String, MerchantThresholdsTextFilter>>,
    pub last_updated_at: Option<MerchantThresholdsTimeFilter>,
}

/// The body of `POST /alerts/alerts_manager/merchant_thresholds`.
///
/// `name`, `product`, `merchant_id` and `profile_id` are required strings, empty allowed. On a
/// conflict of `(name, product, merchant_id, profile_id, is_enabled, author)`, only the non-key
/// columns actually sent here are overwritten.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantThresholdsUpsertRequest {
    pub name: String,
    pub product: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub thresholds_min_volume: Option<f64>,
    pub thresholds_min_impacted_volume: Option<f64>,
    pub thresholds_tolerance: Option<f64>,
    pub thresholds_diff_threshold: Option<f64>,
    pub thresholds_merchant_impact: Option<f64>,
    pub thresholds_alert_period: Option<f64>,
    pub thresholds_min_observations: Option<f64>,
    pub thresholds_min_history_volume: Option<f64>,
    pub thresholds_filter_percentile: Option<f64>,
    pub thresholds_current_min_volume: Option<f64>,
    pub metadata: Option<Value>,
    pub author: Option<String>,
    pub is_enabled: Option<bool>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// Deserialize a field that distinguishes "absent" from "sent as `null`": absent leaves the
/// `#[serde(default)]` outer `None`, `null` becomes `Some(None)`, and a value becomes `Some(Some(v))`.
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// The [`present`] of an ISO 8601 timestamp.
fn present_iso8601<'de, D>(deserializer: D) -> Result<Option<Option<PrimitiveDateTime>>, D::Error>
where
    D: Deserializer<'de>,
{
    common_utils::custom_serde::iso8601::option::deserialize(deserializer).map(Some)
}

/// The body of `POST /alerts/alerts_manager/merchant_thresholds/update`.
///
/// `name`, `product`, `merchant_id` and `profile_id` are each optional keys, but at least one is
/// required; only the keys sent filter which rows are touched. Every other field is absent to keep
/// a column, `null` to clear it, or a value to set it.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantThresholdsUpdateRequest {
    pub name: Option<String>,
    pub product: Option<String>,
    pub merchant_id: Option<String>,
    pub profile_id: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_min_volume: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_min_impacted_volume: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_tolerance: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_diff_threshold: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_merchant_impact: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_alert_period: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_min_observations: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_min_history_volume: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_filter_percentile: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub thresholds_current_min_volume: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present")]
    pub metadata: Option<Option<Value>>,
    #[serde(default, deserialize_with = "present")]
    pub author: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    pub is_enabled: Option<Option<bool>>,
    #[serde(default, deserialize_with = "present_iso8601")]
    pub last_updated_at: Option<Option<PrimitiveDateTime>>,
}

/// The body of `POST /alerts/alerts_manager/merchant_thresholds/delete`.
///
/// `name` and `product` are required; `merchant_id` and `profile_id` narrow further when sent.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantThresholdsDeleteByFilterRequest {
    pub name: String,
    pub product: String,
    pub merchant_id: Option<String>,
    pub profile_id: Option<String>,
}

/// Built from the `{id}` path of `DELETE /alerts/alerts_manager/merchant_thresholds/{id}`.
#[derive(Clone, Debug)]
pub struct MerchantThresholdsDeleteRequest {
    pub id: String,
}

/// One stored merchant threshold override.
#[derive(Clone, Debug, Serialize)]
pub struct MerchantThresholdsResponse {
    pub id: String,
    pub name: Option<String>,
    pub product: Option<String>,
    pub merchant_id: Option<String>,
    pub profile_id: String,
    pub thresholds_min_volume: Option<f64>,
    pub thresholds_min_impacted_volume: Option<f64>,
    pub thresholds_tolerance: Option<f64>,
    pub thresholds_diff_threshold: Option<f64>,
    pub thresholds_merchant_impact: Option<f64>,
    pub thresholds_alert_period: Option<f64>,
    pub thresholds_min_observations: Option<f64>,
    pub thresholds_min_history_volume: Option<f64>,
    pub thresholds_filter_percentile: Option<f64>,
    pub thresholds_current_min_volume: Option<f64>,
    pub metadata: Option<Value>,
    pub author: Option<String>,
    pub is_enabled: Option<bool>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// The body of every merchant threshold route that returns more than one row.
#[derive(Clone, Debug, Serialize)]
pub struct MerchantThresholdsListResponse {
    pub count: usize,
    pub data: Vec<MerchantThresholdsResponse>,
}
