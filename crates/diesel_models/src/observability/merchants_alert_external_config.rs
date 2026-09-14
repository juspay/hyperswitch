use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::merchants_alert_external_config;

#[derive(Clone, Debug, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Insertable)]
#[diesel(table_name = merchants_alert_external_config)]
pub struct MerchantsAlertExternalConfigNew {
    pub name: String,
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<serde_json::Value>,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub enum MerchantsAlertExternalConfigUpdate {
    Update {
        category: Option<Option<String>>,
        is_enabled: bool,
        metadata: Option<Option<serde_json::Value>>,
    },
}

#[derive(Clone, Debug, PartialEq, AsChangeset)]
#[diesel(table_name = merchants_alert_external_config)]
pub struct MerchantsAlertExternalConfigUpdateInternal {
    pub category: Option<Option<String>>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<Option<serde_json::Value>>,
    pub last_updated_at: PrimitiveDateTime,
}

impl From<MerchantsAlertExternalConfigUpdate> for MerchantsAlertExternalConfigUpdateInternal {
    fn from(update: MerchantsAlertExternalConfigUpdate) -> Self {
        match update {
            MerchantsAlertExternalConfigUpdate::Update {
                category,
                is_enabled,
                metadata,
            } => Self {
                category,
                is_enabled: Some(is_enabled),
                metadata,
                last_updated_at: common_utils::date_time::now(),
            },
        }
    }
}
