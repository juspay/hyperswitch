use diesel::{
    AsChangeset, AsExpression, FromSqlRow, Identifiable, Insertable, Queryable, Selectable,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::alerts_info;

pub const ALL_DEFINITIONS: &str = "all";

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

#[derive(
    Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct Blacklist(pub Vec<BlacklistEntry>);

common_utils::impl_to_sql_from_sql_json!(Blacklist, diesel::sql_types::Json);

#[derive(
    Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct Snooze(pub Vec<SnoozeEntry>);

common_utils::impl_to_sql_from_sql_json!(Snooze, diesel::sql_types::Json);

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct Thresholds(pub Vec<ThresholdEntry>);

common_utils::impl_to_sql_from_sql_json!(Thresholds, diesel::sql_types::Json);

// Serialize/Deserialize satisfy `DejaQueryResult`, which the query helpers require under `deja`.
#[derive(Clone, Debug, PartialEq, Identifiable, Queryable, Selectable, Deserialize, Serialize)]
#[diesel(table_name = alerts_info, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AlertsInfo {
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
    pub last_updated_at: Option<PrimitiveDateTime>,
}

impl AlertsInfo {
    pub fn is_enabled(&self) -> bool {
        self.is_enabled.unwrap_or(false)
    }

    pub fn is_all_definitions(&self) -> bool {
        self.name == ALL_DEFINITIONS
    }
}

#[derive(Clone, Debug, PartialEq, Insertable)]
#[diesel(table_name = alerts_info)]
pub struct AlertsInfoNew {
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

// `Option<Option<T>>`: an absent field leaves the column alone, an explicit null clears it.
#[derive(Clone, Debug, PartialEq, AsChangeset)]
#[diesel(table_name = alerts_info)]
pub struct AlertsInfoUpdate {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn entry() -> BlacklistEntry {
        BlacklistEntry {
            merchant_id: "merchant_1234".to_owned(),
            profile_id: String::new(),
            reason: "dead test merchant".to_owned(),
            created_by: Some("reliability_team".to_owned()),
        }
    }

    #[test]
    fn a_blacklist_stores_as_a_bare_array() {
        let stored = serde_json::to_value(Blacklist(vec![entry()])).unwrap();

        assert!(stored.is_array());
        assert_eq!(stored[0]["merchant_id"], "merchant_1234");
    }

    #[test]
    fn a_blacklist_entry_needs_only_a_merchant() {
        let parsed: Blacklist =
            serde_json::from_value(serde_json::json!([{ "merchant_id": "merchant_1234" }]))
                .unwrap();

        assert_eq!(parsed.0[0].profile_id, "");
        assert_eq!(parsed.0[0].created_by, None);
    }

    #[test]
    fn a_snooze_window_without_an_end_is_rejected() {
        let error = serde_json::from_value::<Snooze>(serde_json::json!([{
            "merchant_id": "merchant_1234",
            "starts_at": "2026-09-09T10:00:00.000Z",
        }]))
        .unwrap_err();

        assert!(error.to_string().contains("ends_at"));
    }

    #[test]
    fn a_snooze_window_reads_and_writes_utc_iso8601() {
        let parsed: Snooze = serde_json::from_value(serde_json::json!([{
            "merchant_id": "merchant_1234",
            "ends_at": "2026-09-09T10:00:00.000Z",
        }]))
        .unwrap();

        assert_eq!(parsed.0[0].starts_at, None);
        assert_eq!(
            serde_json::to_value(&parsed).unwrap()[0]["ends_at"],
            "2026-09-09T10:00:00.000Z"
        );
    }

    #[test]
    fn a_threshold_left_out_is_not_the_same_as_a_threshold_of_zero() {
        let parsed: Thresholds = serde_json::from_value(serde_json::json!([{
            "merchant_id": "merchant_1234",
            "min_volume": 0,
        }]))
        .unwrap();

        assert_eq!(parsed.0[0].min_volume, Some(0.0));
        assert_eq!(parsed.0[0].tolerance, None);
    }

    #[test]
    fn a_definition_with_no_enablement_recorded_is_off() {
        let mut definition = AlertsInfo {
            id: uuid::Uuid::nil(),
            name: ALL_DEFINITIONS.to_owned(),
            product: "payments".to_owned(),
            dimensions: None,
            period: None,
            default_channel: None,
            default_critical: None,
            blacklist: None,
            snooze: None,
            history_window: None,
            thresholds: None,
            metadata: None,
            is_enabled: None,
            comments: None,
            call_period: None,
            author: None,
            approver: None,
            last_updated_at: None,
        };

        assert!(!definition.is_enabled());
        assert!(definition.is_all_definitions());

        definition.is_enabled = Some(true);
        assert!(definition.is_enabled());
    }
}
