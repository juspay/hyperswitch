use diesel_models::observability::{
    alerts_info::{
        AlertsInfo, AlertsInfoNew, AlertsInfoUpdate, Blacklist, BlacklistEntry, Snooze,
        SnoozeEntry, ThresholdEntry, Thresholds,
    },
    merchants_alert_external_config::{
        MerchantsAlertExternalConfig, MerchantsAlertExternalConfigNew,
    },
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertDefinitionCreateRequest {
    pub name: String,
    pub product: String,
    pub is_enabled: bool,
    pub author: String,
    pub approver: Option<String>,
    pub dimensions: Option<String>,
    pub period: Option<i32>,
    pub default_channel: Option<String>,
    pub default_critical: Option<bool>,
    pub blacklist: Option<Vec<BlacklistEntry>>,
    pub snooze: Option<Snooze>,
    pub history_window: Option<i32>,
    pub thresholds: Option<Vec<ThresholdEntry>>,
    pub metadata: Option<serde_json::Value>,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
}

impl AlertDefinitionCreateRequest {
    pub fn into_insertable(self, now: PrimitiveDateTime) -> AlertsInfoNew {
        AlertsInfoNew {
            name: self.name,
            product: self.product,
            dimensions: self.dimensions,
            period: self.period,
            default_channel: self.default_channel,
            default_critical: self.default_critical,
            blacklist: self.blacklist.map(Blacklist),
            snooze: self.snooze,
            history_window: self.history_window,
            thresholds: self.thresholds.map(Thresholds),
            metadata: self.metadata,
            is_enabled: Some(self.is_enabled),
            comments: self.comments,
            call_period: self.call_period,
            author: Some(self.author),
            approver: self.approver,
            last_updated_at: now,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertDefinitionUpdateRequest {
    pub is_enabled: Option<bool>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub approver: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub dimensions: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub period: Option<Option<i32>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub default_channel: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub default_critical: Option<Option<bool>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub blacklist: Option<Option<Vec<BlacklistEntry>>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub snooze: Option<Option<Snooze>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub history_window: Option<Option<i32>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub thresholds: Option<Option<Vec<ThresholdEntry>>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub metadata: Option<Option<serde_json::Value>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub comments: Option<Option<serde_json::Value>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub call_period: Option<Option<i32>>,
}

impl From<AlertDefinitionUpdateRequest> for AlertsInfoUpdate {
    fn from(request: AlertDefinitionUpdateRequest) -> Self {
        Self::Update {
            dimensions: request.dimensions,
            period: request.period,
            default_channel: request.default_channel,
            default_critical: request.default_critical,
            blacklist: request.blacklist.map(|entries| entries.map(Blacklist)),
            snooze: request.snooze,
            history_window: request.history_window,
            thresholds: request.thresholds.map(|entries| entries.map(Thresholds)),
            metadata: request.metadata,
            is_enabled: request.is_enabled,
            comments: request.comments,
            call_period: request.call_period,
            approver: request.approver,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AlertDefinitionResponse {
    pub id: uuid::Uuid,
    pub name: String,
    pub product: String,
    pub is_enabled: bool,
    pub dimensions: Option<String>,
    pub period: Option<i32>,
    pub default_channel: Option<String>,
    pub default_critical: Option<bool>,
    pub blacklist: Vec<BlacklistEntry>,
    pub snooze: std::collections::BTreeMap<String, SnoozeEntry>,
    pub history_window: Option<i32>,
    pub thresholds: Vec<ThresholdEntry>,
    pub metadata: Option<serde_json::Value>,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
    pub author: Option<String>,
    pub approver: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}

impl From<AlertsInfo> for AlertDefinitionResponse {
    fn from(definition: AlertsInfo) -> Self {
        let is_enabled = definition.is_enabled.unwrap_or(false);

        Self {
            id: definition.id,
            name: definition.name,
            product: definition.product,
            is_enabled,
            dimensions: definition.dimensions,
            period: definition.period,
            default_channel: definition.default_channel,
            default_critical: definition.default_critical,
            blacklist: definition
                .blacklist
                .map(|value| value.0)
                .unwrap_or_default(),
            snooze: definition.snooze.map(|value| value.0).unwrap_or_default(),
            history_window: definition.history_window,
            thresholds: definition
                .thresholds
                .map(|value| value.0)
                .unwrap_or_default(),
            metadata: definition.metadata,
            comments: definition.comments,
            call_period: definition.call_period,
            author: definition.author,
            approver: definition.approver,
            last_updated_at: definition.last_updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AlertDefinitionListResponse {
    pub count: usize,
    pub definitions: Vec<AlertDefinitionResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertEnablementUpsertRequest {
    pub is_enabled: bool,
    pub category: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl AlertEnablementUpsertRequest {
    pub fn into_upsertable(
        self,
        name: String,
        product: String,
        now: PrimitiveDateTime,
    ) -> MerchantsAlertExternalConfigNew {
        MerchantsAlertExternalConfigNew {
            name,
            product,
            category: self.category,
            is_enabled: Some(self.is_enabled),
            metadata: self.metadata,
            last_updated_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AlertEnablementResponse {
    pub name: String,
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: bool,
    pub effective_is_enabled: bool,
    pub metadata: Option<serde_json::Value>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}

impl AlertEnablementResponse {
    pub fn new(row: MerchantsAlertExternalConfig, definition_is_enabled: Option<bool>) -> Self {
        Self {
            effective_is_enabled: effective_is_enabled(
                definition_is_enabled.unwrap_or(false),
                row.is_enabled,
            ),
            name: row.name,
            product: row.product,
            category: row.category,
            is_enabled: row.is_enabled.unwrap_or(false),
            metadata: row.metadata,
            last_updated_at: row.last_updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AlertEnablementListResponse {
    pub count: usize,
    pub enablements: Vec<AlertEnablementResponse>,
}

fn effective_is_enabled(definition_is_enabled: bool, config_is_enabled: Option<bool>) -> bool {
    definition_is_enabled && config_is_enabled.unwrap_or(false)
}
