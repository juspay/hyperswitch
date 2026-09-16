//! Per-merchant threshold overrides: a row overrides one alert's thresholds for one merchant
//! (and, optionally, one profile), and a job coalesces it with the alert's own definition.

use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::merchant_thresholds;

/// A new merchant threshold override.
///
/// The application supplies the ID. A `None` `author` or `is_enabled` falls back to its column
/// default rather than being stored as `null`.
#[derive(Clone, Debug, Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = merchant_thresholds)]
pub struct MerchantThresholdsNew {
    pub id: String,
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
    pub metadata: Option<serde_json::Value>,
    pub author: Option<String>,
    pub is_enabled: Option<bool>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// A stored merchant threshold override.
#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = merchant_thresholds, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct MerchantThresholds {
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
    pub metadata: Option<serde_json::Value>,
    pub author: Option<String>,
    pub is_enabled: Option<bool>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// What `update_by_key_filter` may set or clear. `None` skips a column; `Some(None)` writes NULL.
#[derive(Clone, Debug, Default, AsChangeset)]
#[diesel(table_name = merchant_thresholds)]
pub struct MerchantThresholdsUpdate {
    pub thresholds_min_volume: Option<Option<f64>>,
    pub thresholds_min_impacted_volume: Option<Option<f64>>,
    pub thresholds_tolerance: Option<Option<f64>>,
    pub thresholds_diff_threshold: Option<Option<f64>>,
    pub thresholds_merchant_impact: Option<Option<f64>>,
    pub thresholds_alert_period: Option<Option<f64>>,
    pub thresholds_min_observations: Option<Option<f64>>,
    pub thresholds_min_history_volume: Option<Option<f64>>,
    pub thresholds_filter_percentile: Option<Option<f64>>,
    pub thresholds_current_min_volume: Option<Option<f64>>,
    pub metadata: Option<Option<serde_json::Value>>,
    pub author: Option<Option<String>>,
    pub is_enabled: Option<Option<bool>>,
    pub last_updated_at: Option<Option<PrimitiveDateTime>>,
}

/// The filters `list_by_filter` accepts. `metadata` pairs match `metadata ->> key = ANY(values)`.
#[derive(Clone, Debug, Default)]
pub struct MerchantThresholdsFilter {
    pub ids: Option<Vec<String>>,
    pub names: Option<Vec<String>>,
    pub products: Option<Vec<String>>,
    pub merchant_ids: Option<Vec<String>>,
    pub profile_ids: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
    pub is_enabled: Option<bool>,
    pub metadata: Vec<(String, Vec<String>)>,
    pub updated_from: Option<PrimitiveDateTime>,
    pub updated_to: Option<PrimitiveDateTime>,
}

/// The keys `update_by_key_filter` and `delete_by_key_filter` select rows by. Only the keys sent
/// filter; at least one must be sent.
#[derive(Clone, Debug, Default)]
pub struct MerchantThresholdsKeyFilter {
    pub name: Option<String>,
    pub product: Option<String>,
    pub merchant_id: Option<String>,
    pub profile_id: Option<String>,
}
