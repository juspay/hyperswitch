//! A/B routing for Revenue Recovery (e.g. Smart vs Cascading).
//!
//! The routing *decision* is made by **Superposition's experiment engine**: the
//! config key resolves — under an experiment, keyed on the invoice id (the
//! targeting key) — to the assigned variant's value, which embeds both the
//! `variant` label and the `algorithm` to run. This module maps that resolved
//! decision into the record we persist on the payment intent and reuse for the
//! invoice's whole recovery life.
//!
//! Stickiness comes from **persist-and-reuse** (the assignment is written to
//! `feature_metadata` once and reused), not from recomputation — so a later
//! config/split change never re-routes an in-flight invoice.
//!
//! Design reference: `revenue-recovery/ab-routing-approach.md`.

use std::str::FromStr;

use common_enums::enums::RevenueRecoveryAlgorithmType;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// The routing decision resolved from Superposition. Under an experiment this is
/// the assigned variant's value (which embeds the variant label + algorithm);
/// otherwise it is the base/default value.
///
/// `Default` is the safe/disabled state, so a missing or unparseable value falls
/// back to legacy routing.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RecoveryRoutingDecision {
    /// `false` (default) → not enrolled in an experiment; keep legacy routing.
    #[serde(default)]
    pub enabled: bool,
    /// The experiment this decision belongs to.
    #[serde(default)]
    pub experiment_name: String,
    /// The assigned variant label (e.g. `treatment_smart`).
    #[serde(default)]
    pub variant: String,
    /// The algorithm to run (`smart` / `cascading`).
    #[serde(default)]
    pub algorithm: String,
}

/// The persisted assignment record. Written once onto the payment intent's
/// `feature_metadata` and reused for the rest of the invoice's recovery life.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevenueRecoveryRoutingData {
    /// The experiment this invoice was assigned under.
    pub experiment_name: String,
    /// The assigned variant label.
    pub variant: String,
    /// The algorithm the experiment intended to run.
    pub assigned_algorithm: RevenueRecoveryAlgorithmType,
    /// When the assignment was made.
    pub assigned_at: PrimitiveDateTime,
}

// NOTE: the algorithm that *actually* ran (after dispatch/fallbacks — e.g.
// smart-with-retry vs smart-no-retry vs smart-error) is deliberately not stored
// yet; capturing it needs the decider proto and is out of v1 scope. It is left
// out rather than kept as an always-null field, since a null would be ambiguous
// (`no fallback occurred` vs `never captured`). This record is opaque JSON with
// serde defaults, so the field can be added later without a migration.

// NOTE: the recovery outcome is intentionally NOT stored on the assignment.
// Whether an invoice recovered is authoritative on `payment_intent.status`, and
// the amount/timing are on the payment intent too; analytics derives the
// per-variant outcome by joining those with `assigned_algorithm`/`variant`. So
// there is nothing to persist on the psync path (which also avoids updating a
// `Succeeded` intent, an operation the payments layer rejects).

/// The outcome of the assignment chokepoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbRoutingResolution {
    /// The invoice already had an assignment; reuse it (sticky, never recomputed).
    Reused(RevenueRecoveryRoutingData),
    /// A fresh assignment was produced from the Superposition decision.
    Created(RevenueRecoveryRoutingData),
    /// No active experiment for this invoice; the caller keeps legacy routing.
    DisabledOrUnavailable,
    /// The decision was present but its algorithm is unsupported (holds the value).
    InvalidConfig(String),
}

fn parse_algorithm(value: &str) -> Option<RevenueRecoveryAlgorithmType> {
    RevenueRecoveryAlgorithmType::from_str(value).ok()
}

/// Map a Superposition routing decision into a fresh assignment.
///
/// - disabled → `DisabledOrUnavailable` (legacy routing)
/// - unsupported `algorithm` → `InvalidConfig`
/// - otherwise → `Created` (the assignment to persist and route by)
///
/// Reuse of an existing assignment is handled by the caller (it must not query
/// Superposition when the invoice is already assigned) — that is what makes the
/// assignment sticky.
pub fn from_decision(
    decision: &RecoveryRoutingDecision,
    assigned_at: PrimitiveDateTime,
) -> AbRoutingResolution {
    if !decision.enabled {
        return AbRoutingResolution::DisabledOrUnavailable;
    }
    match parse_algorithm(&decision.algorithm) {
        Some(assigned_algorithm) => AbRoutingResolution::Created(RevenueRecoveryRoutingData {
            experiment_name: decision.experiment_name.clone(),
            variant: decision.variant.clone(),
            assigned_algorithm,
            assigned_at,
        }),
        None => AbRoutingResolution::InvalidConfig(decision.algorithm.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_timestamp() -> PrimitiveDateTime {
        PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::MIDNIGHT,
        )
    }

    fn decision(enabled: bool, variant: &str, algorithm: &str) -> RecoveryRoutingDecision {
        RecoveryRoutingDecision {
            enabled,
            experiment_name: "rr_smart_vs_cascading_2026_q3".to_string(),
            variant: variant.to_string(),
            algorithm: algorithm.to_string(),
        }
    }

    #[test]
    fn disabled_decision_is_disabled_or_unavailable() {
        let resolution = from_decision(&decision(false, "", "cascading"), fixed_timestamp());
        assert_eq!(resolution, AbRoutingResolution::DisabledOrUnavailable);
    }

    #[test]
    fn default_decision_is_disabled() {
        let resolution = from_decision(&RecoveryRoutingDecision::default(), fixed_timestamp());
        assert_eq!(resolution, AbRoutingResolution::DisabledOrUnavailable);
    }

    #[test]
    fn enabled_decision_creates_assignment() {
        let resolution =
            from_decision(&decision(true, "treatment_smart", "smart"), fixed_timestamp());
        let expected = RevenueRecoveryRoutingData {
            experiment_name: "rr_smart_vs_cascading_2026_q3".to_string(),
            variant: "treatment_smart".to_string(),
            assigned_algorithm: RevenueRecoveryAlgorithmType::Smart,
            assigned_at: fixed_timestamp(),
        };
        assert_eq!(resolution, AbRoutingResolution::Created(expected));
    }

    #[test]
    fn cascading_decision_creates_cascading_assignment() {
        let resolution = from_decision(
            &decision(true, "control_cascading", "cascading"),
            fixed_timestamp(),
        );
        assert!(matches!(
            resolution,
            AbRoutingResolution::Created(data) if data.assigned_algorithm == RevenueRecoveryAlgorithmType::Cascading
        ));
    }

    #[test]
    fn unsupported_algorithm_is_invalid() {
        let resolution =
            from_decision(&decision(true, "treatment", "nonexistent"), fixed_timestamp());
        assert_eq!(
            resolution,
            AbRoutingResolution::InvalidConfig("nonexistent".to_string())
        );
    }
}
