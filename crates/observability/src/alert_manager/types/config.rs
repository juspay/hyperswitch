//! The wire contract for the alert configuration resources.

use diesel_models::observability::{
    alerts_info::{
        AlertsInfo, AlertsInfoNew, AlertsInfoUpdate, Blacklist, BlacklistEntry, Snooze,
        SnoozeEntry, ThresholdEntry, Thresholds,
    },
    merchants_alert_external_config::{
        effective_is_enabled, MerchantsAlertExternalConfig, MerchantsAlertExternalConfigNew,
    },
};
use serde::{Deserialize, Deserializer, Serialize};
use time::PrimitiveDateTime;

/// Tell "the caller did not mention this field" apart from "the caller set it to null".
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

/// The body of `POST /alerts/config/definitions`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertDefinitionCreateRequest {
    /// The detector this configures.
    pub name: String,

    /// The family the alert belongs to.
    pub product: String,

    /// Whether the alert fires.
    pub is_enabled: bool,

    /// Who is creating this definition.
    pub author: String,

    /// Who signed it off, when someone has.
    #[serde(default)]
    pub approver: Option<String>,

    /// The dimensions the detector groups by.
    #[serde(default)]
    pub dimensions: Option<String>,

    /// How long a window the detector evaluates, in minutes.
    #[serde(default)]
    pub period: Option<i32>,

    /// Where announcements go when nothing overrides it.
    #[serde(default)]
    pub default_channel: Option<String>,

    /// Whether announcements are critical when nothing overrides it.
    #[serde(default)]
    pub default_critical: Option<bool>,

    /// Suppression rules:
    #[serde(default)]
    pub blacklist: Option<Vec<BlacklistEntry>>,

    /// Snooze windows:
    #[serde(default)]
    pub snooze: Option<Vec<SnoozeEntry>>,

    /// How much history the detector compares against, in days.
    #[serde(default)]
    pub history_window: Option<i32>,

    /// Per-merchant and per-profile overrides of the detector's gates.
    #[serde(default)]
    pub thresholds: Option<Vec<ThresholdEntry>>,

    /// Anything the dashboard wants to keep alongside the definition.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,

    /// Operator notes.
    #[serde(default)]
    pub comments: Option<serde_json::Value>,

    /// How often the detector runs, in minutes.
    #[serde(default)]
    pub call_period: Option<i32>,
}

impl AlertDefinitionCreateRequest {
    /// Turn the request into the row to insert, stamped at `now`.
    pub fn into_insertable(self, now: PrimitiveDateTime) -> AlertsInfoNew {
        AlertsInfoNew {
            name: self.name,
            product: self.product,
            dimensions: self.dimensions,
            period: self.period,
            default_channel: self.default_channel,
            default_critical: self.default_critical,
            blacklist: self.blacklist.map(Blacklist),
            snooze: self.snooze.map(Snooze),
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

/// The body of `POST /alerts/config/definitions/{id}`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertDefinitionUpdateRequest {
    /// Whether the alert fires.
    #[serde(default)]
    pub is_enabled: Option<bool>,

    /// Who signed off the current state.
    #[serde(default, deserialize_with = "double_option")]
    pub approver: Option<Option<String>>,

    #[serde(default, deserialize_with = "double_option")]
    pub dimensions: Option<Option<String>>,

    #[serde(default, deserialize_with = "double_option")]
    pub period: Option<Option<i32>>,

    #[serde(default, deserialize_with = "double_option")]
    pub default_channel: Option<Option<String>>,

    #[serde(default, deserialize_with = "double_option")]
    pub default_critical: Option<Option<bool>>,

    #[serde(default, deserialize_with = "double_option")]
    pub blacklist: Option<Option<Vec<BlacklistEntry>>>,

    #[serde(default, deserialize_with = "double_option")]
    pub snooze: Option<Option<Vec<SnoozeEntry>>>,

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
    /// Turn the request into the changeset to apply, stamped at `now`.
    pub fn into_changeset(self, now: PrimitiveDateTime) -> AlertsInfoUpdate {
        AlertsInfoUpdate {
            dimensions: self.dimensions,
            period: self.period,
            default_channel: self.default_channel,
            default_critical: self.default_critical,
            blacklist: self.blacklist.map(|entries| entries.map(Blacklist)),
            snooze: self.snooze.map(|entries| entries.map(Snooze)),
            history_window: self.history_window,
            thresholds: self.thresholds.map(|entries| entries.map(Thresholds)),
            metadata: self.metadata,
            is_enabled: self.is_enabled.map(Some),
            comments: self.comments,
            call_period: self.call_period,
            approver: self.approver,
            last_updated_at: now,
        }
    }
}

/// One alert definition, as a caller sees it.
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
    pub snooze: Vec<SnoozeEntry>,
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
        let is_enabled = definition.is_enabled();

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

/// What `GET /alerts/config/definitions` returns.
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

/// The body of `POST /alerts/config/enablement/{name}/{product}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertEnablementUpsertRequest {
    /// Whether this alert runs for this product.
    pub is_enabled: bool,

    /// How the dashboard groups this alert.
    #[serde(default)]
    pub category: Option<String>,

    /// Anything the dashboard wants to keep alongside the switch.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

impl AlertEnablementUpsertRequest {
    /// Turn the request into the row to upsert, stamped at `now`.
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

/// One enablement row, as a caller sees it.
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
    /// Build the response, resolving the two switches against each other.
    pub fn new(row: MerchantsAlertExternalConfig, definition_is_enabled: Option<bool>) -> Self {
        Self {
            effective_is_enabled: effective_is_enabled(
                definition_is_enabled.unwrap_or(false),
                row.is_enabled,
            ),
            name: row.name,
            product: row.product,
            category: row.category,
            is_enabled: row.is_enabled.unwrap_or(true),
            metadata: row.metadata,
            last_updated_at: row.last_updated_at,
        }
    }
}

/// What `GET /alerts/config/enablement` returns.
#[derive(Debug, Serialize)]
pub struct AlertEnablementListResponse {
    pub count: usize,
    pub enablements: Vec<AlertEnablementResponse>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn update_from(value: serde_json::Value) -> AlertDefinitionUpdateRequest {
        serde_json::from_value(value).unwrap()
    }

    fn now() -> PrimitiveDateTime {
        common_utils::date_time::now()
    }

    /// The property the whole update shape exists for:
    #[test]
    fn an_absent_field_and_an_explicit_null_are_different_requests() {
        let mentioned = update_from(serde_json::json!({ "default_channel": null }));
        let unmentioned = update_from(serde_json::json!({}));

        assert_eq!(mentioned.default_channel, Some(None));
        assert_eq!(unmentioned.default_channel, None);
    }

    #[test]
    fn a_field_that_is_set_carries_its_value() {
        let update = update_from(serde_json::json!({ "period": 15 }));

        assert_eq!(update.period, Some(Some(15)));
    }

    /// Diesel refuses an update with nothing to set, so the timestamp is what makes an update mentioning no field a no-op rather than an error.
    #[test]
    fn an_update_that_changes_nothing_still_moves_the_timestamp() {
        let stamp = now();
        let changeset = update_from(serde_json::json!({})).into_changeset(stamp);

        assert_eq!(changeset.last_updated_at, stamp);
        assert_eq!(changeset.dimensions, None);
    }

    /// Renaming a definition would orphan the enablement rows and the announcements that reference it by name, so the field is not there to be sent at all.
    #[test]
    fn an_update_cannot_rename_a_definition() {
        let error = serde_json::from_value::<AlertDefinitionUpdateRequest>(
            serde_json::json!({ "name": "sr_drop_v2" }),
        )
        .unwrap_err();

        assert!(error.to_string().contains("name"));
    }

    /// A definition created without saying so is off, and reads to whoever finds it as broken rather than as never switched on.
    #[test]
    fn creating_a_definition_requires_saying_whether_it_is_on() {
        let error = serde_json::from_value::<AlertDefinitionCreateRequest>(serde_json::json!({
            "name": "sr_drop",
            "product": "payments",
            "author": "reliability_team",
        }))
        .unwrap_err();

        assert!(error.to_string().contains("is_enabled"));
    }

    /// The internal API key says which service called, never which person, so the body is the only place an author can come from.
    #[test]
    fn creating_a_definition_requires_an_author() {
        let error = serde_json::from_value::<AlertDefinitionCreateRequest>(serde_json::json!({
            "name": "sr_drop",
            "product": "payments",
            "is_enabled": true,
        }))
        .unwrap_err();

        assert!(error.to_string().contains("author"));
    }

    /// Structured rather than opaque, which is the point of typing the column:
    #[test]
    fn a_malformed_suppression_rule_is_refused_rather_than_stored() {
        let error = serde_json::from_value::<AlertDefinitionCreateRequest>(serde_json::json!({
            "name": "sr_drop",
            "product": "payments",
            "is_enabled": true,
            "author": "reliability_team",
            "blacklist": [{ "merchant": "merchant_1234" }],
        }))
        .unwrap_err();

        assert!(error.to_string().contains("merchant_id"));
    }

    #[test]
    fn a_created_definition_carries_its_lists_into_the_row() {
        let request: AlertDefinitionCreateRequest = serde_json::from_value(serde_json::json!({
            "name": "sr_drop",
            "product": "payments",
            "is_enabled": true,
            "author": "reliability_team",
            "blacklist": [{ "merchant_id": "merchant_1234", "reason": "dead test merchant" }],
        }))
        .unwrap();

        let row = request.into_insertable(now());

        assert_eq!(row.is_enabled, Some(true));
        assert_eq!(row.author.as_deref(), Some("reliability_team"));
        assert_eq!(row.blacklist.unwrap().0[0].merchant_id, "merchant_1234");
        assert_eq!(row.snooze, None);
    }

    fn definition(is_enabled: Option<bool>) -> AlertsInfo {
        AlertsInfo {
            id: uuid::Uuid::nil(),
            name: "sr_drop".to_owned(),
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
            is_enabled,
            comments: None,
            call_period: None,
            author: None,
            approver: None,
            last_updated_at: None,
        }
    }

    /// A caller reads a definition in order to edit it and send it back.
    #[test]
    fn a_definition_response_names_every_field_even_when_it_is_null() {
        let body =
            serde_json::to_value(AlertDefinitionResponse::from(definition(Some(true)))).unwrap();

        for field in ["dimensions", "period", "metadata", "author", "approver"] {
            assert!(body.get(field).is_some(), "{field} was skipped");
            assert!(body[field].is_null());
        }
    }

    /// `NULL` and `false` both mean off, and a caller should not have to know the column is nullable to work that out.
    #[test]
    fn a_definition_response_resolves_nulls_that_have_only_one_meaning() {
        let body = serde_json::to_value(AlertDefinitionResponse::from(definition(None))).unwrap();

        assert_eq!(body["is_enabled"], false);
        assert_eq!(body["blacklist"], serde_json::json!([]));
        assert_eq!(body["snooze"], serde_json::json!([]));
        assert_eq!(body["thresholds"], serde_json::json!([]));
    }

    #[test]
    fn a_definition_list_reports_how_many_it_found() {
        let body = serde_json::to_value(
            [definition(Some(true)), definition(None)]
                .into_iter()
                .collect::<AlertDefinitionListResponse>(),
        )
        .unwrap();

        assert_eq!(body["count"], 2);
        assert_eq!(body["definitions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn an_empty_definition_list_is_an_object_with_a_zero_count() {
        let body = serde_json::to_value(
            std::iter::empty::<AlertsInfo>().collect::<AlertDefinitionListResponse>(),
        )
        .unwrap();

        assert_eq!(body["count"], 0);
        assert_eq!(body["definitions"], serde_json::json!([]));
    }

    fn enablement(is_enabled: Option<bool>) -> MerchantsAlertExternalConfig {
        MerchantsAlertExternalConfig {
            name: "sr_drop".to_owned(),
            product: "payments".to_owned(),
            category: None,
            is_enabled,
            metadata: None,
            last_updated_at: None,
        }
    }

    /// Reporting only the stored switch would leave a caller unable to tell "on" from "on, but the definition is off" — which is the confusion this resource was ticketed to end.
    #[test]
    fn an_enablement_response_reports_both_switches() {
        let body = serde_json::to_value(AlertEnablementResponse::new(
            enablement(Some(true)),
            Some(false),
        ))
        .unwrap();

        assert_eq!(body["is_enabled"], true);
        assert_eq!(body["effective_is_enabled"], false);
    }

    /// An enablement row naming an alert nobody defined switches nothing on.
    #[test]
    fn an_enablement_row_without_a_definition_is_not_effective() {
        let response = AlertEnablementResponse::new(enablement(Some(true)), None);

        assert!(!response.effective_is_enabled);
    }

    #[test]
    fn an_enablement_upsert_takes_its_key_from_the_path() {
        let error = serde_json::from_value::<AlertEnablementUpsertRequest>(
            serde_json::json!({ "is_enabled": true, "name": "sr_drop" }),
        )
        .unwrap_err();

        assert!(error.to_string().contains("name"));
    }
}
