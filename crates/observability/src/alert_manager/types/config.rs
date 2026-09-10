//! The wire contract for the alert configuration resources.
//!
//! Two resources, both under `/alerts/config`:
//!
//! * **definitions** — [`alerts_info`](diesel_models::observability::alerts_info), everything that
//!   says what an alert *is*. Suppression, snooze and thresholds are columns of that one row, so
//!   they are fields of this one resource rather than three of their own.
//! * **enablement** — [`merchants_alert_external_config`](diesel_models::observability::merchants_alert_external_config),
//!   the per-`(name, product)` switch. Which of the two switches wins is decided in
//!   [`effective_is_enabled`](diesel_models::observability::merchants_alert_external_config::effective_is_enabled)
//!   and reported here as [`AlertEnablementResponse::effective_is_enabled`].
//!
//! ## Definitions are addressed by id, enablement by its natural key
//!
//! A definition's `(name, product)` is a unique index rather than its primary key, and a caller
//! creating one cannot know the id it will be given, so creation posts to the collection and
//! everything afterwards addresses `/{id}`. Addressing definitions by name instead was rejected:
//! renaming would then be indistinguishable from creating, and the id is what
//! `merchants_alert_external` rows already carry.
//!
//! Enablement has no id of its own — `(name, product)` *is* its primary key — so it is addressed
//! by the pair, and its write is a real upsert rather than a create and an update.
//!
//! ## An update mentions only what it changes
//!
//! Three portal screens edit different parts of a definition. A whole-row `PUT` from any of them
//! would silently discard what the other two had just saved, so [`AlertDefinitionUpdateRequest`]
//! distinguishes three states per field with [`Option<Option<T>>`]: absent leaves the value alone,
//! an explicit `null` clears it, and a value sets it. Same idiom as `RefundUpdateInternal`.
//!
//! Optimistic concurrency — a version column the caller echoes back — was the alternative. It was
//! rejected because it solves a problem this resource does not have: two screens editing
//! *different* columns are not in conflict, and making them retry against each other would be a
//! worse experience than the lost update it prevents. Two screens editing the *same* column still
//! race, and last-writer-wins is the honest answer for a config row.
//!
//! ## No `status` envelope, and no delete
//!
//! The notify routes answer `{ "status": ... }` because "the request succeeded" and "the message
//! arrived" are genuinely different questions there. A config write has no such gap: a `200` means
//! the row is written, so a `status` field would be a constant. These routes answer with the row
//! instead, and errors keep the crate's existing envelope.
//!
//! There is no delete route on either resource. `is_enabled` is how an alert is turned off, and it
//! is reversible; deleting an `alerts_info` row cascades to every `alerts_main` row referencing it,
//! which destroys the record of what was announced in order to stop announcing it.

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
///
/// `#[serde(default)]` alone collapses both into `None`, which is why an absent field and an
/// explicit `null` would otherwise be the same request. Applied to an `Option<Option<T>>` field,
/// this runs only when the key is present, so the outer option answers "was it mentioned" and the
/// inner one "what to". Diesel's `AsChangeset` reads the same shape the same way, so the wire
/// meaning and the SQL meaning cannot drift apart.
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

/// The body of `POST /alerts/config/definitions`.
///
/// `is_enabled` is **required**, and that is load-bearing. The column defaults to false, so a
/// definition created without it is off — and an alert that is off without anyone deciding it
/// should be reads as "the alert is broken" rather than "nobody enabled it". Making the caller say
/// which they meant is the same trick [`crate::types`] plays with `status`: the failure mode of
/// the shape is a caller that does not confront the question.
///
/// `author` is required for a different reason: the internal API key identifies the calling
/// service, not a person, so if the body does not say who is asking then nothing does.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertDefinitionCreateRequest {
    /// The detector this configures. `all` is reserved for the row carrying suppression that
    /// applies to every detector.
    pub name: String,

    /// The family the alert belongs to.
    pub product: String,

    /// Whether the alert fires. No default — see the type's documentation.
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

    /// Suppression rules: alerts that are correct and not worth seeing.
    #[serde(default)]
    pub blacklist: Option<Vec<BlacklistEntry>>,

    /// Snooze windows: suppression that ends by itself.
    #[serde(default)]
    pub snooze: Option<Vec<SnoozeEntry>>,

    /// How much history the detector compares against, in days.
    #[serde(default)]
    pub history_window: Option<i32>,

    /// Per-merchant and per-profile overrides of the detector's gates.
    #[serde(default)]
    pub thresholds: Option<Vec<ThresholdEntry>>,

    /// Anything the dashboard wants to keep alongside the definition. Not interpreted here.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,

    /// Operator notes. Not interpreted here.
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
///
/// Every field is absent-able. `name`, `product` and `author` are not here at all: the first two
/// are the alert's identity, referenced by the enablement table and matched by name in the alert
/// manager, and the third records who introduced the definition rather than who last touched it.
///
/// `is_enabled` is a plain `Option<bool>` where the rest are nested. Clearing a switch is the same
/// as turning it off, and offering two spellings for off would mean two paths to test and two ways
/// for a screen to express one intent.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertDefinitionUpdateRequest {
    /// Whether the alert fires. Absent leaves it as it is.
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
    ///
    /// `last_updated_at` is set unconditionally, which also guarantees the changeset is never
    /// empty — diesel refuses an update with nothing to set, so a request that mentioned no field
    /// would otherwise fail rather than be the no-op it reads as.
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
///
/// Nothing is skipped when it is null. The notify responses drop absent fields because a caller
/// reads them once and throws them away; a config row is read in order to be edited and sent back,
/// and a field that disappears when it is null is a field a round-tripping caller will drop.
///
/// `is_enabled` is a plain `bool` even though the column is nullable, because `NULL` and `false`
/// both mean the alert is off and a caller should not have to know that.
///
/// The three list columns render as arrays, empty when the column is `NULL`: a definition with no
/// suppression and one whose suppression column was never written are the same alert.
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
///
/// An object rather than a bare array. `count` costs nothing and gives a caller something to
/// assert on, and an object leaves room for a cursor the day this list stops fitting on a screen —
/// which a top-level array would not.
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
///
/// The key is in the path, not the body. Carrying it in both would give a request two authorities
/// on which alert it addresses, and nothing useful to do when they disagree.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertEnablementUpsertRequest {
    /// Whether this alert runs for this product. Required, for the reason
    /// [`AlertDefinitionCreateRequest::is_enabled`] is.
    pub is_enabled: bool,

    /// How the dashboard groups this alert.
    #[serde(default)]
    pub category: Option<String>,

    /// Anything the dashboard wants to keep alongside the switch. Not interpreted here.
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
///
/// `is_enabled` is this table's switch; `effective_is_enabled` is the answer after the definition's
/// own switch is applied. Both are reported, because a caller that saw only the stored value would
/// have no way to tell "on" from "on, but the definition is off", and the ticket that added this
/// resource exists because those two were silently the same.
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
    ///
    /// `definition_is_enabled` is `None` when no definition exists for the pair. That is only
    /// reachable on a read of a row written before this API started checking, and it resolves to
    /// off: an enablement row naming an alert nobody defined cannot switch anything on.
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

    /// The property the whole update shape exists for: three states, not two. Without it a screen
    /// editing the snooze list would clear the suppression list it never mentioned.
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

    /// Diesel refuses an update with nothing to set, so the timestamp is what makes an update
    /// mentioning no field a no-op rather than an error.
    #[test]
    fn an_update_that_changes_nothing_still_moves_the_timestamp() {
        let stamp = now();
        let changeset = update_from(serde_json::json!({})).into_changeset(stamp);

        assert_eq!(changeset.last_updated_at, stamp);
        assert_eq!(changeset.dimensions, None);
    }

    /// Renaming a definition would orphan the enablement rows and the announcements that reference
    /// it by name, so the field is not there to be sent at all.
    #[test]
    fn an_update_cannot_rename_a_definition() {
        let error = serde_json::from_value::<AlertDefinitionUpdateRequest>(
            serde_json::json!({ "name": "sr_drop_v2" }),
        )
        .unwrap_err();

        assert!(error.to_string().contains("name"));
    }

    /// A definition created without saying so is off, and reads to whoever finds it as broken
    /// rather than as never switched on. Requiring the field makes that a rejection instead.
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

    /// The internal API key says which service called, never which person, so the body is the only
    /// place an author can come from.
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

    /// Structured rather than opaque, which is the point of typing the column: a suppression rule
    /// the alert manager cannot read is refused at the edge instead of being stored and ignored.
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

    /// A caller reads a definition in order to edit it and send it back. A field that vanishes
    /// when it is null is a field that round trip would drop.
    #[test]
    fn a_definition_response_names_every_field_even_when_it_is_null() {
        let body =
            serde_json::to_value(AlertDefinitionResponse::from(definition(Some(true)))).unwrap();

        for field in ["dimensions", "period", "metadata", "author", "approver"] {
            assert!(body.get(field).is_some(), "{field} was skipped");
            assert!(body[field].is_null());
        }
    }

    /// `NULL` and `false` both mean off, and a caller should not have to know the column is
    /// nullable to work that out. Likewise a missing list is an empty list.
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

    /// Reporting only the stored switch would leave a caller unable to tell "on" from "on, but the
    /// definition is off" — which is the confusion this resource was ticketed to end.
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
