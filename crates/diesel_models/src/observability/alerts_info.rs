use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::{raw_json::RawJson, schema::alerts_info};

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Deserialize, Serialize)]
#[diesel(table_name = alerts_info, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AlertsInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub product: String,
    pub dimensions: String,
    pub period: i32,
    pub default_channel: Option<String>,
    pub default_critical: bool,
    pub blacklist: Option<RawJson>,
    pub snooze: Option<RawJson>,
    pub history_window: Option<i32>,
    pub thresholds: Option<RawJson>,
    pub metadata: Option<serde_json::Value>,
    pub is_enabled: bool,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
    pub author: String,
    pub approver: Option<String>,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alerts_info)]
pub struct AlertsInfoNew {
    pub id: uuid::Uuid,
    pub name: String,
    pub product: String,
    pub dimensions: String,
    pub period: i32,
    pub default_channel: Option<String>,
    pub default_critical: bool,
    pub blacklist: Option<RawJson>,
    pub snooze: Option<RawJson>,
    pub history_window: Option<i32>,
    pub thresholds: Option<RawJson>,
    pub metadata: Option<serde_json::Value>,
    pub is_enabled: bool,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
    pub author: String,
    pub approver: Option<String>,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub enum AlertsInfoUpdate {
    Update {
        dimensions: Option<String>,
        period: Option<i32>,
        default_channel: Option<Option<String>>,
        default_critical: Option<bool>,
        blacklist: Option<Option<RawJson>>,
        snooze: Option<Option<RawJson>>,
        history_window: Option<Option<i32>>,
        thresholds: Option<Option<RawJson>>,
        metadata: Option<Option<serde_json::Value>>,
        is_enabled: Option<bool>,
        comments: Option<Option<serde_json::Value>>,
        call_period: Option<Option<i32>>,
        approver: Option<Option<String>>,
    },
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = alerts_info)]
pub struct AlertsInfoUpdateInternal {
    pub dimensions: Option<String>,
    pub period: Option<i32>,
    pub default_channel: Option<Option<String>>,
    pub default_critical: Option<bool>,
    pub blacklist: Option<Option<RawJson>>,
    pub snooze: Option<Option<RawJson>>,
    pub history_window: Option<Option<i32>>,
    pub thresholds: Option<Option<RawJson>>,
    pub metadata: Option<Option<serde_json::Value>>,
    pub is_enabled: Option<bool>,
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
                last_updated_at: common_utils::date_time::now(),
            },
        }
    }
}
