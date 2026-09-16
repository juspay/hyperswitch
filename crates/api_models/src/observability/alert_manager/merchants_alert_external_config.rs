//! Merchant alert delivery switches, as the observability service's
//! `/alerts/alerts_manager/external_config` routes accept and return them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::PrimitiveDateTime;

/// The body of `POST /alerts/alerts_manager/external_config`.
///
/// Only `name` and `product` are required. An omitted `category`, `is_enabled`, or `metadata`
/// takes the database default rather than being stored as `null`, and an omitted
/// `last_updated_at` stays `NULL`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantsAlertExternalConfigCreateRequest {
    pub name: String,
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<Value>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// The body of `POST /alerts/alerts_manager/external_config/{name}/{product}`.
///
/// `name` and `product` come from the path, not the body: the handler fills them in after the
/// body is parsed, so a body that carries either fails to deserialize instead of being ignored.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantsAlertExternalConfigUpdateRequest {
    #[serde(skip_deserializing)]
    pub name: String,
    #[serde(skip_deserializing)]
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<Value>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// Built from the `{name}/{product}` path of retrieve, update, and delete.
#[derive(Clone, Debug, Deserialize)]
pub struct MerchantsAlertExternalConfigKey {
    pub name: String,
    pub product: String,
}

/// A list filter value: a single string, or several to match with `= ANY(...)`.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum MerchantsAlertExternalConfigFilterValues {
    One(String),
    Many(Vec<String>),
}

impl MerchantsAlertExternalConfigFilterValues {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values,
        }
    }
}

/// r-apps' `{start, end}` shape for a `last_updated_at` filter.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantsAlertExternalConfigTimeRange {
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end: Option<PrimitiveDateTime>,
}

/// The body of `POST /alerts/alerts_manager/external_config/list`.
///
/// Every field is optional; `{}` matches every row.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantsAlertExternalConfigListRequest {
    pub name: Option<MerchantsAlertExternalConfigFilterValues>,
    pub product: Option<MerchantsAlertExternalConfigFilterValues>,
    pub category: Option<MerchantsAlertExternalConfigFilterValues>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<BTreeMap<String, MerchantsAlertExternalConfigFilterValues>>,
    pub last_updated_at: Option<MerchantsAlertExternalConfigTimeRange>,
}

/// One stored merchant alert delivery switch.
#[derive(Clone, Debug, Serialize)]
pub struct MerchantsAlertExternalConfigResponse {
    pub name: String,
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<Value>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// The body of `POST /alerts/alerts_manager/external_config/list`'s response.
#[derive(Clone, Debug, Serialize)]
pub struct MerchantsAlertExternalConfigListResponse {
    pub count: usize,
    pub data: Vec<MerchantsAlertExternalConfigResponse>,
}
