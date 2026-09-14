use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::merchant_thresholds;

#[derive(Clone, Debug, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize)]
#[diesel(table_name = merchant_thresholds, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct MerchantThreshold {
    pub id: uuid::Uuid,
    pub name: String,
    pub product: String,
    pub merchant_id: String,
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
    pub author: String,
    pub is_enabled: bool,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, PartialEq, Insertable)]
#[diesel(table_name = merchant_thresholds)]
pub struct MerchantThresholdNew {
    pub id: uuid::Uuid,
    pub name: String,
    pub product: String,
    pub merchant_id: String,
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
    pub author: String,
    pub is_enabled: bool,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub enum MerchantThresholdUpdate {
    Update {
        thresholds_min_volume: Option<Option<f64>>,
        thresholds_min_impacted_volume: Option<Option<f64>>,
        thresholds_tolerance: Option<Option<f64>>,
        thresholds_diff_threshold: Option<Option<f64>>,
        thresholds_merchant_impact: Option<Option<f64>>,
        thresholds_alert_period: Option<Option<f64>>,
        thresholds_min_observations: Option<Option<f64>>,
        thresholds_min_history_volume: Option<Option<f64>>,
        thresholds_filter_percentile: Option<Option<f64>>,
        thresholds_current_min_volume: Option<Option<f64>>,
        metadata: Option<Option<serde_json::Value>>,
        author: Option<String>,
        is_enabled: Option<bool>,
    },
}

#[derive(Clone, Debug, PartialEq, AsChangeset)]
#[diesel(table_name = merchant_thresholds)]
pub struct MerchantThresholdUpdateInternal {
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
    pub author: Option<String>,
    pub is_enabled: Option<bool>,
    pub last_updated_at: PrimitiveDateTime,
}

impl From<MerchantThresholdUpdate> for MerchantThresholdUpdateInternal {
    fn from(update: MerchantThresholdUpdate) -> Self {
        match update {
            MerchantThresholdUpdate::Update {
                thresholds_min_volume,
                thresholds_min_impacted_volume,
                thresholds_tolerance,
                thresholds_diff_threshold,
                thresholds_merchant_impact,
                thresholds_alert_period,
                thresholds_min_observations,
                thresholds_min_history_volume,
                thresholds_filter_percentile,
                thresholds_current_min_volume,
                metadata,
                author,
                is_enabled,
            } => Self {
                thresholds_min_volume,
                thresholds_min_impacted_volume,
                thresholds_tolerance,
                thresholds_diff_threshold,
                thresholds_merchant_impact,
                thresholds_alert_period,
                thresholds_min_observations,
                thresholds_min_history_volume,
                thresholds_filter_percentile,
                thresholds_current_min_volume,
                metadata,
                author,
                is_enabled,
                last_updated_at: common_utils::date_time::now(),
            },
        }
    }
}
