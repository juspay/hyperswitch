//! Alert definitions: what an alert is, and whether it should fire.
//!
//! Suppression, snooze and thresholds are JSON columns of this row rather than side tables, so
//! everything about an alert is in one place.
//!
//! Three consequences follow from that arrangement.
//!
//! **Contention**, which is why every field of [`AlertsInfoUpdate`] is optional: three portal
//! screens edit different parts of this row, and a whole-row write from any of them would discard
//! what the other two had just saved.
//!
//! **The reserved [`ALL_DEFINITIONS`] row**, which carries suppression applying to every detector
//! rather than to one, because a rule that spans alerts has no other row to live on.
//!
//! **Typed columns rather than opaque JSON.** The three columns holding lists are modelled as
//! [`Blacklist`], [`Snooze`] and [`Thresholds`], so a caller cannot store a shape the alert
//! manager will fail to read. The cost is that the bytes are not preserved: a value read back has
//! been through `serde_json` twice and has this crate's field order and no whitespace. Storing the
//! columns as strings would preserve them exactly, and was rejected — the failure it avoids
//! (whitespace changing) is cosmetic, and the one it introduces (a dashboard writing a key nothing
//! reads, discovered when an alert silently stops being suppressed) is not. `metadata` and
//! `comments` stay [`serde_json::Value`] because nothing in this plane interprets them.

use diesel::{
    AsChangeset, AsExpression, FromSqlRow, Identifiable, Insertable, Queryable, Selectable,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::alerts_info;

/// The definition holding suppression that applies to every detector.
///
/// Not a detector itself: nothing runs it, and it has no thresholds or period worth setting. It
/// exists because the alert manager's suppression is keyed by rule id and accepts `all`, and a
/// blacklist entry scoped to every rule has nowhere else to be stored now that suppression is a
/// column rather than a table.
pub const ALL_DEFINITIONS: &str = "all";

/// One suppression rule: an alert that is correct and not worth seeing.
///
/// A dead test merchant fires forever — the detector is right every time — and suppression is how
/// an operator says "yes, we know" without turning the detector off for everyone.
///
/// `merchant_id` is exact; an empty `profile_id` means every profile of that merchant. An entry on
/// the [`ALL_DEFINITIONS`] row applies to every detector, which is the only way to express "mute
/// this merchant everywhere".
///
/// The `rule_id` of the ClickHouse original is absent: the row this list hangs off *is* the rule,
/// so carrying it in the entry would let a definition hold suppression addressed to a different
/// one. `is_deleted` is absent for the same class of reason — a tombstone is how a
/// `ReplacingMergeTree` deletes, and here a removed entry is a shorter list.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlacklistEntry {
    /// The merchant to suppress. Matched exactly.
    pub merchant_id: String,

    /// The profile within that merchant, or empty for all of them.
    #[serde(default)]
    pub profile_id: String,

    /// Why this is suppressed. Free text, and the only thing that will explain a silent alert to
    /// whoever finds it in six months.
    #[serde(default)]
    pub reason: String,

    /// Who added it.
    #[serde(default)]
    pub created_by: Option<String>,
}

/// One snooze window: suppression that ends by itself.
///
/// Matched on merchant and profile like a suppression rule, and additionally on connector and
/// payment method when those are set. An unset dimension matches anything, so a window with no
/// connector silences the alert for every connector.
///
/// Renamed from the original `snooze_start_time` / `snooze_end_time`, which the dashboard wrote as
/// local-time strings and the alert manager parsed in the portal's timezone. These are
/// [`PrimitiveDateTime`] read and written as ISO 8601 in UTC, matching every other timestamp in
/// this database, so an operator in one timezone cannot schedule a window that begins five and a
/// half hours after they meant it to.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SnoozeEntry {
    /// The merchant to silence. Matched exactly.
    pub merchant_id: String,

    /// The profile within that merchant, or empty for all of them.
    #[serde(default)]
    pub profile_id: String,

    /// Only silence this connector. Unset silences every connector.
    #[serde(default)]
    pub connector: Option<String>,

    /// Only silence this payment method. Unset silences every payment method.
    #[serde(default)]
    pub payment_method: Option<String>,

    /// When the window opens. Unset means it opened when it was created, which is what an
    /// operator snoozing something in front of them means.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub starts_at: Option<PrimitiveDateTime>,

    /// When the window closes. Required, because a snooze with no end is a blacklist entry that
    /// nobody will remember to remove.
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ends_at: PrimitiveDateTime,

    /// Who opened it.
    #[serde(default)]
    pub created_by: Option<String>,
}

/// One threshold override, for a merchant and optionally a profile.
///
/// Narrows the gates a detector ships with, without a redeploy. Resolution is most specific first:
/// an entry naming a profile, then one naming only the merchant, then the detector's own defaults.
///
/// Every value is optional and independently so: absent means the detector keeps its default, and
/// zero is an operator deliberately setting zero. Collapsing the two would make "no floor" and "a
/// floor of nothing" the same request.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ThresholdEntry {
    /// The merchant this override applies to.
    pub merchant_id: String,

    /// The profile within that merchant, or empty for all of them.
    #[serde(default)]
    pub profile_id: String,

    /// Ignore slices with less traffic than this.
    #[serde(default)]
    pub min_volume: Option<f64>,

    /// Ignore slices where less than this much traffic is actually affected.
    #[serde(default)]
    pub min_impacted_volume: Option<f64>,

    /// How far from expected is still normal.
    #[serde(default)]
    pub tolerance: Option<f64>,

    /// How large the gap from expected must be before this is an alert.
    #[serde(default)]
    pub diff_threshold: Option<f64>,
}

/// The `blacklist` column: every suppression rule on this definition.
///
/// A newtype rather than a bare `Vec`, because the `FromSql`/`ToSql` pair that maps it onto a
/// `json` column cannot be written for a type this crate does not own. It serializes as the array
/// it wraps, so the stored JSON is what a reader would expect to find.
#[derive(
    Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct Blacklist(pub Vec<BlacklistEntry>);

common_utils::impl_to_sql_from_sql_json!(Blacklist, diesel::sql_types::Json);

/// The `snooze` column: every open or scheduled window on this definition.
#[derive(
    Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct Snooze(pub Vec<SnoozeEntry>);

common_utils::impl_to_sql_from_sql_json!(Snooze, diesel::sql_types::Json);

/// The `thresholds` column: every override narrowing this definition's gates.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct Thresholds(pub Vec<ThresholdEntry>);

common_utils::impl_to_sql_from_sql_json!(Thresholds, diesel::sql_types::Json);

/// An alert definition, as stored.
#[derive(Clone, Debug, PartialEq, Identifiable, Queryable, Selectable)]
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
    /// Whether this definition is on.
    ///
    /// The column is nullable and defaults to false, so a row can say `NULL`. That is read as off:
    /// an alert nobody has enabled has not been enabled, and guessing the other way would start a
    /// detector nobody asked for.
    pub fn is_enabled(&self) -> bool {
        self.is_enabled.unwrap_or(false)
    }

    /// Whether this is the reserved row carrying suppression for every detector.
    pub fn is_all_definitions(&self) -> bool {
        self.name == ALL_DEFINITIONS
    }
}

/// A definition being created.
///
/// `id` is absent: the column defaults to `gen_random_uuid()`, so the database assigns it and a
/// caller cannot collide with an existing definition by guessing one.
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

/// A partial change to a definition.
///
/// The nested option is the whole point. The outer says whether the caller mentioned the field at
/// all, the inner what they set it to — so an absent field is left alone and an explicit null
/// clears it, and two screens editing different columns do not overwrite each other. Same idiom as
/// `RefundUpdateInternal`.
///
/// `last_updated_at` is unconditional, and is what makes this changeset always non-empty: diesel
/// refuses an update with nothing to set, and an update that did not move the timestamp would
/// leave the row claiming it had not changed.
///
/// `name` and `product` are absent. They are the alert's identity, referenced by
/// `merchants_alert_external_config` and matched by name in the alert manager, so renaming through
/// an update would orphan those references rather than failing.
///
/// `author` is absent for a different reason. The column is single-valued and records who
/// introduced the alert; treating it as "last edited by" would overwrite that on every save, so
/// the API would forget who wrote a definition in exchange for a change history it still would not
/// have. `approver` is updatable, because signing off is a thing that happens repeatedly.
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

    /// The column holds the array, not an object wrapping it, so the shape on disk is the shape a
    /// reader would guess from the column name.
    #[test]
    fn a_blacklist_stores_as_a_bare_array() {
        let stored = serde_json::to_value(Blacklist(vec![entry()])).unwrap();

        assert!(stored.is_array());
        assert_eq!(stored[0]["merchant_id"], "merchant_1234");
    }

    /// Only `merchant_id` is load-bearing; the rest of an entry written by hand may be missing.
    #[test]
    fn a_blacklist_entry_needs_only_a_merchant() {
        let parsed: Blacklist =
            serde_json::from_value(serde_json::json!([{ "merchant_id": "merchant_1234" }]))
                .unwrap();

        assert_eq!(parsed.0[0].profile_id, "");
        assert_eq!(parsed.0[0].created_by, None);
    }

    /// The reason a snooze is typed at all: a window with no end never closes, which is the one
    /// mistake a snooze must not permit.
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

    /// Absent and zero are different requests, and a threshold store that conflated them would
    /// silently ignore an operator asking for no floor at all.
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

    /// A definition nobody enabled is off. The column is nullable and defaults to false, and both
    /// of those have to read the same way or an alert would start firing because a row was written
    /// without mentioning it.
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
