//! The per (alert name, product) switch for merchant-facing delivery: merchant Slack/email
//! senders join against this table with `is_enabled = true` before sending anything.

use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::merchants_alert_external_config;

/// A new merchant alert delivery switch.
///
/// A `None` field is left out of the insert entirely, so `category`, `is_enabled`, and `metadata`
/// fall back to their column defaults, and `last_updated_at` stays `NULL`.
#[derive(Clone, Debug, Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = merchants_alert_external_config)]
pub struct MerchantsAlertExternalConfigNew {
    pub name: String,
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<serde_json::Value>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// A stored merchant alert delivery switch.
#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(
    table_name = merchants_alert_external_config,
    primary_key(name, product),
    check_for_backend(diesel::pg::Pg)
)]
pub struct MerchantsAlertExternalConfig {
    pub name: String,
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<serde_json::Value>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// What an update may change.
///
/// Not `AsChangeset`: `metadata` needs `||` rather than `=`, so the query layer builds its own
/// changeset from this.
#[derive(Clone, Debug)]
pub struct MerchantsAlertExternalConfigUpdate {
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<serde_json::Value>,
    pub last_updated_at: PrimitiveDateTime,
}

/// The filters `list_by_filter` accepts.
#[derive(Clone, Debug, Default)]
pub struct MerchantsAlertExternalConfigListFilter {
    pub names: Option<Vec<String>>,
    pub products: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub is_enabled: Option<bool>,
    pub metadata: Vec<(String, Vec<String>)>,
    pub last_updated_at_start: Option<PrimitiveDateTime>,
    pub last_updated_at_end: Option<PrimitiveDateTime>,
}
