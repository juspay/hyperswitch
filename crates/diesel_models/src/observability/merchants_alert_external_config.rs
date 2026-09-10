use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::merchants_alert_external_config;

pub fn effective_is_enabled(definition_is_enabled: bool, config_is_enabled: Option<bool>) -> bool {
    definition_is_enabled && config_is_enabled.unwrap_or(true)
}

// Serialize/Deserialize satisfy `DejaQueryResult`, which the query helpers require under `deja`.
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

#[derive(Clone, Debug, PartialEq, Insertable, AsChangeset)]
#[diesel(table_name = merchants_alert_external_config)]
pub struct MerchantsAlertExternalConfigNew {
    pub name: String,
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<serde_json::Value>,
    pub last_updated_at: PrimitiveDateTime,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn a_disabled_definition_stays_off_however_the_config_row_reads() {
        assert!(!effective_is_enabled(false, Some(true)));
        assert!(!effective_is_enabled(false, None));
    }

    #[test]
    fn a_config_row_can_turn_an_enabled_definition_off() {
        assert!(effective_is_enabled(true, Some(true)));
        assert!(!effective_is_enabled(true, Some(false)));
    }

    #[test]
    fn an_alert_with_no_config_row_is_not_narrowed() {
        assert!(effective_is_enabled(true, None));
    }
}
