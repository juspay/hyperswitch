use diesel_models::observability::{
    alerts_info::{
        AlertsInfo, AlertsInfoNew, AlertsInfoUpdate, Blacklist, BlacklistEntry, Snooze,
        SnoozeEntry, ThresholdEntry, Thresholds,
    },
    merchants_alert_external_config::{
        MerchantsAlertExternalConfig, MerchantsAlertExternalConfigNew,
    },
};
use serde::{Deserialize, Deserializer, Serialize};
use time::PrimitiveDateTime;

fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

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
    pub fn into_insertable(self, id: uuid::Uuid, now: PrimitiveDateTime) -> AlertsInfoNew {
        AlertsInfoNew {
            id,
            name: self.name,
            product: self.product,
            dimensions: self.dimensions.unwrap_or_default(),
            period: self.period.unwrap_or_default(),
            default_channel: self.default_channel,
            default_critical: self.default_critical.unwrap_or_default(),
            blacklist: self.blacklist.map(Blacklist),
            snooze: self.snooze,
            history_window: self.history_window,
            thresholds: self.thresholds.map(Thresholds),
            metadata: self.metadata,
            is_enabled: self.is_enabled,
            comments: self.comments,
            call_period: self.call_period,
            author: self.author,
            approver: self.approver,
            last_updated_at: now,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertDefinitionUpdateRequest {
    pub is_enabled: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    pub approver: Option<Option<String>>,
    pub dimensions: Option<String>,
    pub period: Option<i32>,
    #[serde(default, deserialize_with = "double_option")]
    pub default_channel: Option<Option<String>>,
    pub default_critical: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    pub blacklist: Option<Option<Vec<BlacklistEntry>>>,
    #[serde(default, deserialize_with = "double_option")]
    pub snooze: Option<Option<Snooze>>,
    #[serde(default, deserialize_with = "double_option")]
    pub history_window: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub thresholds: Option<Option<Vec<ThresholdEntry>>>,
    #[serde(default, deserialize_with = "double_option")]
    pub metadata: Option<Option<serde_json::Value>>,
    #[serde(default, deserialize_with = "double_option")]
    pub comments: Option<Option<serde_json::Value>>,
    #[serde(default, deserialize_with = "double_option")]
    pub call_period: Option<Option<i32>>,
}

impl AlertDefinitionUpdateRequest {
    pub fn into_changeset(self, now: PrimitiveDateTime) -> AlertsInfoUpdate {
        AlertsInfoUpdate::Update {
            dimensions: self.dimensions,
            period: self.period,
            default_channel: self.default_channel,
            default_critical: self.default_critical,
            blacklist: self.blacklist.map(|entries| entries.map(Blacklist)),
            snooze: self.snooze,
            history_window: self.history_window,
            thresholds: self.thresholds.map(|entries| entries.map(Thresholds)),
            metadata: self.metadata,
            is_enabled: self.is_enabled,
            comments: self.comments,
            call_period: self.call_period,
            approver: self.approver,
            last_updated_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AlertDefinitionResponse {
    pub id: uuid::Uuid,
    pub name: String,
    pub product: String,
    pub is_enabled: bool,
    pub dimensions: String,
    pub period: i32,
    pub default_channel: Option<String>,
    pub default_critical: bool,
    pub blacklist: Vec<BlacklistEntry>,
    pub snooze: std::collections::BTreeMap<String, SnoozeEntry>,
    pub history_window: Option<i32>,
    pub thresholds: Vec<ThresholdEntry>,
    pub metadata: Option<serde_json::Value>,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
    pub author: String,
    pub approver: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

impl From<AlertsInfo> for AlertDefinitionResponse {
    fn from(definition: AlertsInfo) -> Self {
        Self {
            id: definition.id,
            name: definition.name,
            product: definition.product,
            is_enabled: definition.is_enabled,
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

impl FromIterator<AlertsInfo> for AlertDefinitionListResponse {
    fn from_iter<I: IntoIterator<Item = AlertsInfo>>(definitions: I) -> Self {
        let definitions = definitions
            .into_iter()
            .map(AlertDefinitionResponse::from)
            .collect::<Vec<_>>();

        Self {
            count: definitions.len(),
            definitions,
        }
    }
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
            category: self.category.unwrap_or_default(),
            is_enabled: self.is_enabled,
            metadata: self
                .metadata
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
            last_updated_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AlertEnablementResponse {
    pub name: String,
    pub product: String,
    pub category: String,
    pub is_enabled: bool,
    pub effective_is_enabled: bool,
    pub metadata: serde_json::Value,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

impl AlertEnablementResponse {
    pub fn new(row: MerchantsAlertExternalConfig, definition_is_enabled: Option<bool>) -> Self {
        Self {
            effective_is_enabled: definition_is_enabled.unwrap_or(false) && row.is_enabled,
            name: row.name,
            product: row.product,
            category: row.category,
            is_enabled: row.is_enabled,
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
