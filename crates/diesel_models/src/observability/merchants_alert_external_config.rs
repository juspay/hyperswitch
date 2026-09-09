//! Per-alert enablement, keyed on `(name, product)`.
//!
//! The alert manager reads this table to decide whether an alert runs for a name and product,
//! separately from the definition itself. Two switches therefore exist, and which wins is written
//! down in [`effective_is_enabled`] rather than left to whichever reader gets there first.
//!
//! `(name, product)` is the primary key, which is the difference between this table and the one
//! r-apps has. Keyless, an upsert naming one column as its conflict target inserts a second row,
//! and two rows can then disagree about whether the same alert is on — with no rule saying which
//! of them is read.

use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::merchants_alert_external_config;

/// Whether an alert runs, given both switches.
///
/// **The definition is the master switch; this table can only narrow it.** An alert runs when its
/// definition is enabled *and* its config row does not say otherwise; a missing config row does
/// not narrow anything, matching the column's `DEFAULT TRUE`.
///
/// The rule follows from what each row is for. `alerts_info.is_enabled` decides whether a detector
/// runs at all — with it off there is no result to publish, so a config row saying `true` would be
/// asking to externalise something that was never computed. Letting this table win would mean an
/// operator disabling a definition could be silently overridden by a row on a screen they were not
/// looking at, which is the failure mode a master switch exists to prevent.
///
/// The alternative — most-recently-updated wins — was rejected: it makes the answer depend on
/// clock skew between two writers, and gives no way to express "off, and stay off".
pub fn effective_is_enabled(definition_is_enabled: bool, config_is_enabled: Option<bool>) -> bool {
    definition_is_enabled && config_is_enabled.unwrap_or(true)
}

/// One enablement row, as stored.
#[derive(Clone, Debug, PartialEq, Identifiable, Queryable, Selectable)]
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

/// An enablement row being written.
///
/// Insert and update carry the same fields, because the key is supplied by the caller and the row
/// has nothing the database fills in. That is what lets the upsert be a single statement rather
/// than a read followed by a write that races it.
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

    /// The decision this module exists to record: neither switch alone is the answer, and the
    /// narrower one cannot turn anything on.
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

    /// Adding a definition is enough to make it run. Requiring a config row as well would make
    /// every new alert silently dead until somebody noticed the second table.
    #[test]
    fn an_alert_with_no_config_row_is_not_narrowed() {
        assert!(effective_is_enabled(true, None));
    }
}
