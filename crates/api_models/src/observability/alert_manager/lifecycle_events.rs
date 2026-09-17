//! Typed API models for alert lifecycle episode state.

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// Optional overlap bounds for lifecycle reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleEventsQuery {
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub from: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub to: Option<PrimitiveDateTime>,
}

/// A complete replacement for one lifecycle episode.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleEventRequest {
    pub alert_key: String,
    pub detector: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub state: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub first_seen: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_seen: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub recovered_at: PrimitiveDateTime,
    pub runs: i64,
    pub severity: String,
    pub sr: f64,
    pub failed: i64,
    pub total: i64,
    pub connector: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub notified_at: PrimitiveDateTime,
    pub ts_slack: String,
    pub sent: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleEventsBatchRequest {
    pub events: Vec<LifecycleEventRequest>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub snapshot_at: PrimitiveDateTime,
    pub version_bump_seconds: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LifecycleEventResponse {
    pub alert_key: String,
    pub detector: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub state: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub first_seen: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_seen: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub recovered_at: PrimitiveDateTime,
    pub runs: i64,
    pub severity: String,
    pub sr: f64,
    pub failed: i64,
    pub total: i64,
    pub connector: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub notified_at: PrimitiveDateTime,
    pub ts_slack: String,
    pub sent: bool,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct LifecycleEventsListResponse {
    pub events: Vec<LifecycleEventResponse>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub server_time: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct LifecycleEventsBatchResponse {
    pub ok: bool,
    pub persisted: usize,
}
