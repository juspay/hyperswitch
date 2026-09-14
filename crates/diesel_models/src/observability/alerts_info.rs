use diesel::{AsChangeset, AsExpression, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::alerts_info;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlacklistEntry {
    pub merchant_id: String,
    #[serde(default)]
    pub profile_id: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub created_by: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SnoozeEntry {
    pub merchant_id: String,
    #[serde(default)]
    pub profile_id: String,
    #[serde(default)]
    pub connector: Option<String>,
    #[serde(default)]
    pub payment_method: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub starts_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ends_at: PrimitiveDateTime,
    #[serde(default)]
    pub created_by: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ThresholdEntry {
    pub merchant_id: String,
    #[serde(default)]
    pub profile_id: String,
    #[serde(default)]
    pub min_volume: Option<f64>,
    #[serde(default)]
    pub min_impacted_volume: Option<f64>,
    #[serde(default)]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub diff_threshold: Option<f64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct Blacklist(pub Vec<BlacklistEntry>);

common_utils::impl_to_sql_from_sql_json!(Blacklist);

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct Snooze(pub Vec<SnoozeEntry>);

common_utils::impl_to_sql_from_sql_json!(Snooze);

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct Thresholds(pub Vec<ThresholdEntry>);

common_utils::impl_to_sql_from_sql_json!(Thresholds);

#[derive(Clone, Debug, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize)]
#[diesel(table_name = alerts_info, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AlertsInfo {
    pub id: uuid::Uuid,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimensions: Option<String>,
    pub period: Option<i32>,
    pub default_channel: Option<String>,
    pub default_critical: Option<bool>,
    pub blacklist: Option<Blacklist>,
    pub snooze: Option<Snooze>,
    pub history_window: Option<i32>,
    pub thresholds: Option<Thresholds>,
    pub metadata: Option<serde_json::Value>,
    pub is_enabled: Option<bool>,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
    pub author: Option<String>,
    pub approver: Option<String>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug, PartialEq, Insertable)]
#[diesel(table_name = alerts_info)]
pub struct AlertsInfoNew {
    pub id: uuid::Uuid,
    pub name: String,
    pub product: String,
    pub dimensions: Option<String>,
    pub period: Option<i32>,
    pub default_channel: Option<String>,
    pub default_critical: Option<bool>,
    pub blacklist: Option<Blacklist>,
    pub snooze: Option<Snooze>,
    pub history_window: Option<i32>,
    pub thresholds: Option<Thresholds>,
    pub metadata: Option<serde_json::Value>,
    pub is_enabled: Option<bool>,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
    pub author: Option<String>,
    pub approver: Option<String>,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub enum AlertsInfoUpdate {
    Update {
        dimensions: Option<Option<String>>,
        period: Option<Option<i32>>,
        default_channel: Option<Option<String>>,
        default_critical: Option<Option<bool>>,
        blacklist: Option<Option<Blacklist>>,
        snooze: Option<Option<Snooze>>,
        history_window: Option<Option<i32>>,
        thresholds: Option<Option<Thresholds>>,
        metadata: Option<Option<serde_json::Value>>,
        is_enabled: Option<Option<bool>>,
        comments: Option<Option<serde_json::Value>>,
        call_period: Option<Option<i32>>,
        approver: Option<Option<String>>,
        last_updated_at: PrimitiveDateTime,
    },
}

#[derive(Clone, Debug, PartialEq, AsChangeset)]
#[diesel(table_name = alerts_info)]
pub struct AlertsInfoUpdateInternal {
    pub dimensions: Option<Option<String>>,
    pub period: Option<Option<i32>>,
    pub default_channel: Option<Option<String>>,
    pub default_critical: Option<Option<bool>>,
    pub blacklist: Option<Option<Blacklist>>,
    pub snooze: Option<Option<Snooze>>,
    pub history_window: Option<Option<i32>>,
    pub thresholds: Option<Option<Thresholds>>,
    pub metadata: Option<Option<serde_json::Value>>,
    pub is_enabled: Option<Option<bool>>,
    pub comments: Option<Option<serde_json::Value>>,
    pub call_period: Option<Option<i32>>,
    pub approver: Option<Option<String>>,
    pub last_updated_at: PrimitiveDateTime,
}

impl From<AlertsInfoUpdate> for AlertsInfoUpdateInternal {
    fn from(update: AlertsInfoUpdate) -> Self {
        match update {
            AlertsInfoUpdate::Update {
                dimensions,
                period,
                default_channel,
                default_critical,
                blacklist,
                snooze,
                history_window,
                thresholds,
                metadata,
                is_enabled,
                comments,
                call_period,
                approver,
                last_updated_at,
            } => Self {
                dimensions,
                period,
                default_channel,
                default_critical,
                blacklist,
                snooze,
                history_window,
                thresholds,
                metadata,
                is_enabled,
                comments,
                call_period,
                approver,
                last_updated_at,
            },
        }
    }
}
