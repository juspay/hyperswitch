//! Validated alert rule-toggle domain models and layer conversions.

use api_models::observability::alert_manager::rule_toggles as api;
use diesel_models::observability::alert_manager::rule_toggles as storage;
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

#[derive(Clone, Debug)]
pub struct RuleToggleNew {
    pub rule_id: String,
    pub is_enabled: bool,
    pub updated_by: String,
}

#[derive(Clone, Debug)]
pub struct RuleToggle {
    pub rule_id: String,
    pub is_enabled: bool,
    pub updated_by: String,
    pub last_updated_at: PrimitiveDateTime,
}

impl RuleToggleNew {
    pub fn try_from_request(
        rule_id: String,
        request: api::RuleToggleSetRequest,
    ) -> ObservabilityApiResult<Self> {
        let rule_id = rule_id.trim().to_owned();
        if rule_id.is_empty() {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("rule_id must not be empty");
        }

        Ok(Self {
            rule_id,
            is_enabled: request.is_enabled,
            updated_by: request.updated_by,
        })
    }
}

impl From<RuleToggleNew> for storage::RuleToggleNew {
    fn from(row: RuleToggleNew) -> Self {
        Self {
            rule_id: row.rule_id,
            is_enabled: row.is_enabled,
            updated_by: row.updated_by,
        }
    }
}

impl From<storage::RuleToggle> for RuleToggle {
    fn from(row: storage::RuleToggle) -> Self {
        Self {
            rule_id: row.rule_id,
            is_enabled: row.is_enabled,
            updated_by: row.updated_by,
            last_updated_at: row.last_updated_at,
        }
    }
}

impl From<RuleToggle> for api::RuleToggleResponse {
    fn from(row: RuleToggle) -> Self {
        Self {
            rule_id: row.rule_id,
            is_enabled: row.is_enabled,
            updated_by: row.updated_by,
            last_updated_at: row.last_updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_id_is_trimmed_and_blank_is_rejected() {
        let request = api::RuleToggleSetRequest {
            is_enabled: false,
            updated_by: "dashboard".into(),
        };
        let row = RuleToggleNew::try_from_request(" composite_id ".into(), request).unwrap();
        assert_eq!(row.rule_id, "composite_id");

        let request = api::RuleToggleSetRequest {
            is_enabled: true,
            updated_by: "dashboard".into(),
        };
        assert!(RuleToggleNew::try_from_request("   ".into(), request).is_err());
    }
}
