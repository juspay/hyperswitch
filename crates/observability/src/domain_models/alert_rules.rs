//! Alert rules: the rule types that readers and writers exchange, and the checks a rule passes
//! before it is stored.
//!
//! The typed columns (the status, and the versioned configuration with its own checks) live in
//! [`common_types::alert_rules::stored`], so the database models can store them as their
//! types.

use common_types::alert_rules::stored::{AlertConfiguration, AlertRuleStatus};
use diesel_models::observability::alert_manager::alert_rules as storage;
use time::PrimitiveDateTime;

const MAX_ID_LENGTH: usize = 64;
const MAX_NAME_LENGTH: usize = 255;

/// A stored rule.
#[derive(Clone, Debug, PartialEq)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub status: AlertRuleStatus,
    pub alerts_configuration: AlertConfiguration,
    /// When the rule is next due for evaluation (UTC).
    pub next_run_at: PrimitiveDateTime,
    pub created_at: PrimitiveDateTime,
    pub last_updated_at: PrimitiveDateTime,
}

/// A rule ready to store (create or replace).
///
/// Built only through [`AlertRuleNew::new`], which runs every check, so an invalid rule cannot
/// reach the table.
#[derive(Clone, Debug, PartialEq)]
pub struct AlertRuleNew {
    id: String,
    name: String,
    status: AlertRuleStatus,
    alerts_configuration: AlertConfiguration,
    next_run_at: PrimitiveDateTime,
}

impl AlertRuleNew {
    /// Check a rule and prepare it for storage, due at `now`. The error names the failing field.
    pub fn new(
        id: String,
        name: String,
        status: AlertRuleStatus,
        alerts_configuration: AlertConfiguration,
        now: PrimitiveDateTime,
    ) -> Result<Self, String> {
        validate_id(&id)?;

        let name = name.trim().to_owned();
        if name.is_empty() || name.chars().count() > MAX_NAME_LENGTH {
            return Err(format!("name must be 1 to {MAX_NAME_LENGTH} characters"));
        }

        alerts_configuration.validate()?;

        Ok(Self {
            id,
            name,
            status,
            alerts_configuration,
            next_run_at: now,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    let leading = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    let valid = (1..=MAX_ID_LENGTH).contains(&id.len())
        && id.starts_with(leading)
        && id.chars().all(|c| leading(c) || c == '-' || c == '_');

    if valid {
        Ok(())
    } else {
        Err(format!(
            "id must be 1 to {MAX_ID_LENGTH} lowercase letters, digits, `-` or `_`, starting with \
             a letter or digit"
        ))
    }
}

impl From<AlertRuleNew> for storage::AlertRuleNew {
    fn from(rule: AlertRuleNew) -> Self {
        Self {
            id: rule.id,
            name: rule.name,
            status: rule.status,
            alerts_configuration: rule.alerts_configuration,
            next_run_at: rule.next_run_at,
        }
    }
}

impl From<storage::AlertRule> for AlertRule {
    fn from(row: storage::AlertRule) -> Self {
        Self {
            id: row.id,
            name: row.name,
            status: row.status,
            alerts_configuration: row.alerts_configuration,
            next_run_at: row.next_run_at,
            created_at: row.created_at,
            last_updated_at: row.last_updated_at,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use serde_json::json;
    use time::macros::datetime;

    use super::*;

    fn configuration() -> AlertConfiguration {
        serde_json::from_value(json!({
            "version": "v1",
            "interval_seconds": 60,
            "query": {
                "datasource": "cloudwatch",
                "version": "v1",
                "region": "us-east-1",
                "namespace": "AWS/RDS",
                "metric_name": "CPUUtilization",
                "dimensions": { "DBClusterIdentifier": "hyperswitchdb-cluster", "Role": "WRITER" },
                "statistic": "Average",
                "period": 60
            },
            "thresholds": {
                "version": "v1",
                "severities": {
                    "critical": {
                        "threshold": 80.0,
                        "comparison_operator": "GreaterThanOrEqualToThreshold",
                        "description": "RDS writer CPU above 80%.",
                        "destination": "infra_oncall"
                    }
                }
            }
        }))
        .unwrap()
    }

    fn rule(id: &str, name: &str) -> Result<AlertRuleNew, String> {
        AlertRuleNew::new(
            id.to_owned(),
            name.to_owned(),
            AlertRuleStatus::Active,
            configuration(),
            datetime!(2026-10-21 09:00:30),
        )
    }

    #[test]
    fn a_valid_rule_is_accepted_and_its_name_trimmed() {
        let rule = rule("rds-writer-cpu", "  RDS writer CPU ").unwrap();

        assert_eq!(rule.id(), "rds-writer-cpu");
        assert_eq!(rule.name, "RDS writer CPU");
    }

    #[test]
    fn ids_and_names_are_checked() {
        for id in ["rds-writer-cpu", "rds_writer_cpu", "0cpu", "a"] {
            assert!(rule(id, "n").is_ok(), "`{id}` should be accepted");
        }

        let too_long = "a".repeat(MAX_ID_LENGTH + 1);
        for id in [
            "",
            "RDS",
            "-cpu",
            "_cpu",
            "rds cpu",
            "rds.cpu",
            too_long.as_str(),
        ] {
            let reason = rule(id, "n").unwrap_err();
            assert!(reason.starts_with("id "), "`{id}`: {reason}");
        }

        for name in ["   ".to_owned(), "n".repeat(MAX_NAME_LENGTH + 1)] {
            let reason = rule("rule", &name).unwrap_err();
            assert!(reason.starts_with("name "), "{reason}");
        }
    }

    #[test]
    fn the_configuration_is_checked_too() {
        let mut invalid = configuration();
        let AlertConfiguration::V1(v1) = &mut invalid;
        v1.interval_seconds = 90;

        let reason = AlertRuleNew::new(
            "rule".to_owned(),
            "n".to_owned(),
            AlertRuleStatus::Active,
            invalid,
            datetime!(2026-10-21 09:00:30),
        )
        .unwrap_err();
        assert!(
            reason.starts_with("alerts_configuration.interval_seconds"),
            "{reason}"
        );
    }

    #[test]
    fn a_rule_is_stored_and_read_back_unchanged() {
        let new = rule("rds-writer-cpu", "RDS writer CPU").unwrap();
        let stored = storage::AlertRuleNew::from(new.clone());

        assert_eq!(stored.id, "rds-writer-cpu");
        assert_eq!(stored.status, AlertRuleStatus::Active);
        assert_eq!(stored.alerts_configuration, new.alerts_configuration);
        assert_eq!(stored.next_run_at, datetime!(2026-10-21 09:00:30));

        let read = AlertRule::from(storage::AlertRule {
            id: stored.id,
            name: stored.name,
            status: stored.status,
            alerts_configuration: stored.alerts_configuration,
            next_run_at: datetime!(2026-10-21 09:00:00),
            created_at: datetime!(2026-10-21 09:00:00),
            last_updated_at: datetime!(2026-10-21 09:00:00),
        });

        assert_eq!(read.name, new.name);
        assert_eq!(read.status, AlertRuleStatus::Active);
        assert_eq!(read.alerts_configuration, new.alerts_configuration);
    }
}
