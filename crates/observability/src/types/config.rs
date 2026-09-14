use std::collections::BTreeMap;

use diesel_models::observability::{
    alerts_info::{AlertsInfo, AlertsInfoNew, AlertsInfoUpdate},
    merchants_alert_external_config::{
        MerchantsAlertExternalConfig, MerchantsAlertExternalConfigNew,
        MerchantsAlertExternalConfigUpdate,
    },
    raw_json::RawJson,
};
use error_stack::report;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

const SNOOZE_ENTRY_PREFIXES: [&str; 2] = ["snooze_entry_", "custom_snooze_entry_"];

const SNOOZE_TIME_FORMAT: &[time::format_description::FormatItem<'static>] =
    time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct Blacklist(RawJson);

impl Blacklist {
    pub fn validate(&self) -> Result<(), String> {
        let groups = match serde_json::from_str(self.0.get()) {
            Ok(serde_json::Value::Array(groups)) => Some(groups),
            Ok(serde_json::Value::Object(groups)) => Some(groups.into_values().collect()),
            _ => None,
        };

        groups
            .filter(|groups| groups.iter().all(is_blacklist_group))
            .map(|_| ())
            .ok_or_else(|| {
                "blacklist must be a list or an object of groups, each mapping a dimension to a value or a list of values".to_owned()
            })
    }
}

impl From<Blacklist> for RawJson {
    fn from(blacklist: Blacklist) -> Self {
        blacklist.0
    }
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct Snooze(RawJson);

#[derive(Deserialize)]
struct SnoozeEntry {
    snooze_start_time: Option<String>,
    snooze_end_time: String,
}

impl Snooze {
    pub fn validate(&self) -> Result<(), String> {
        serde_json::from_str::<BTreeMap<String, SnoozeEntry>>(self.0.get())
            .ok()
            .filter(|entries| {
                entries.iter().all(|(key, entry)| {
                    SNOOZE_ENTRY_PREFIXES
                        .iter()
                        .any(|prefix| key.starts_with(prefix))
                        && is_snooze_time(&entry.snooze_end_time)
                        && entry
                            .snooze_start_time
                            .as_deref()
                            .is_none_or(is_snooze_time)
                })
            })
            .map(|_| ())
            .ok_or_else(|| {
                "snooze entries must be keyed snooze_entry_<time> or custom_snooze_entry_<time> and carry snooze_end_time as YYYY-MM-DD HH:MM:SS".to_owned()
            })
    }
}

impl From<Snooze> for RawJson {
    fn from(snooze: Snooze) -> Self {
        snooze.0
    }
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct Thresholds(RawJson);

impl Thresholds {
    pub fn validate(&self) -> Result<(), String> {
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(self.0.get())
            .map(|_| ())
            .map_err(|_| "thresholds must be an object of threshold names to values".to_owned())
    }
}

impl From<Thresholds> for RawJson {
    fn from(thresholds: Thresholds) -> Self {
        thresholds.0
    }
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
    pub blacklist: Option<Blacklist>,
    pub snooze: Option<Snooze>,
    pub history_window: Option<i32>,
    pub thresholds: Option<Thresholds>,
    pub metadata: Option<serde_json::Value>,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
}

impl AlertDefinitionCreateRequest {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        valid_documents([
            self.blacklist.as_ref().map(Blacklist::validate),
            self.snooze.as_ref().map(Snooze::validate),
            self.thresholds.as_ref().map(Thresholds::validate),
        ])
    }

    pub fn into_insertable(self, id: uuid::Uuid, now: PrimitiveDateTime) -> AlertsInfoNew {
        AlertsInfoNew {
            id,
            name: self.name,
            product: self.product,
            dimensions: self.dimensions,
            period: self.period,
            default_channel: self.default_channel,
            default_critical: self.default_critical,
            blacklist: self.blacklist.map(RawJson::from),
            snooze: self.snooze.map(RawJson::from),
            history_window: self.history_window,
            thresholds: self.thresholds.map(RawJson::from),
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
    pub blacklist: Option<Option<Blacklist>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub snooze: Option<Option<Snooze>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub history_window: Option<Option<i32>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub thresholds: Option<Option<Thresholds>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub metadata: Option<Option<serde_json::Value>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub comments: Option<Option<serde_json::Value>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub call_period: Option<Option<i32>>,
}

impl AlertDefinitionUpdateRequest {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        valid_documents([
            self.blacklist
                .as_ref()
                .and_then(Option::as_ref)
                .map(Blacklist::validate),
            self.snooze
                .as_ref()
                .and_then(Option::as_ref)
                .map(Snooze::validate),
            self.thresholds
                .as_ref()
                .and_then(Option::as_ref)
                .map(Thresholds::validate),
        ])
    }
}

impl From<AlertDefinitionUpdateRequest> for AlertsInfoUpdate {
    fn from(request: AlertDefinitionUpdateRequest) -> Self {
        Self::Update {
            dimensions: request.dimensions,
            period: request.period,
            default_channel: request.default_channel,
            default_critical: request.default_critical,
            blacklist: request.blacklist.map(|value| value.map(RawJson::from)),
            snooze: request.snooze.map(|value| value.map(RawJson::from)),
            history_window: request.history_window,
            thresholds: request.thresholds.map(|value| value.map(RawJson::from)),
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
    pub blacklist: Option<RawJson>,
    pub snooze: Option<RawJson>,
    pub history_window: Option<i32>,
    pub thresholds: Option<RawJson>,
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
        Self {
            id: definition.id,
            name: definition.name,
            product: definition.product,
            is_enabled: definition.is_enabled.unwrap_or(false),
            dimensions: definition.dimensions,
            period: definition.period,
            default_channel: definition.default_channel,
            default_critical: definition.default_critical,
            blacklist: definition.blacklist,
            snooze: definition.snooze,
            history_window: definition.history_window,
            thresholds: definition.thresholds,
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
    #[serde(default, with = "serde_with::rust::double_option")]
    pub category: Option<Option<String>>,
    #[serde(default, with = "serde_with::rust::double_option")]
    pub metadata: Option<Option<serde_json::Value>>,
}

impl AlertEnablementUpsertRequest {
    pub fn to_insertable(
        &self,
        name: String,
        product: String,
        now: PrimitiveDateTime,
    ) -> MerchantsAlertExternalConfigNew {
        MerchantsAlertExternalConfigNew {
            name,
            product,
            category: self.category.clone().flatten(),
            is_enabled: Some(self.is_enabled),
            metadata: self.metadata.clone().flatten(),
            last_updated_at: now,
        }
    }
}

impl From<AlertEnablementUpsertRequest> for MerchantsAlertExternalConfigUpdate {
    fn from(request: AlertEnablementUpsertRequest) -> Self {
        Self::Update {
            category: request.category,
            is_enabled: request.is_enabled,
            metadata: request.metadata,
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

fn valid_documents(validations: [Option<Result<(), String>>; 3]) -> ObservabilityApiResult<()> {
    validations
        .into_iter()
        .flatten()
        .collect::<Result<(), String>>()
        .map_err(|message| report!(ObservabilityError::InvalidRequestData { message }))
}

fn is_blacklist_group(group: &serde_json::Value) -> bool {
    group.as_object().is_some_and(|dimensions| {
        !dimensions.is_empty()
            && dimensions.values().all(|values| match values {
                serde_json::Value::Array(values) => values.iter().all(is_blacklist_value),
                value => is_blacklist_value(value),
            })
    })
}

fn is_blacklist_value(value: &serde_json::Value) -> bool {
    value.is_string() || value.is_number() || value.is_boolean()
}

fn is_snooze_time(value: &str) -> bool {
    PrimitiveDateTime::parse(value, SNOOZE_TIME_FORMAT).is_ok()
}
