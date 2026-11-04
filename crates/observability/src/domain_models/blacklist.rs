//! Validated alert blacklist domain models and layer conversions.

use api_models::observability::alert_manager::blacklist as api;
use diesel_models::observability::alert_manager::blacklist as storage;
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

#[derive(Clone, Debug)]
pub struct BlacklistEntryNew {
    pub rule_id: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub scope: serde_json::Value,
    pub reason: String,
    pub created_by: String,
    pub is_deleted: bool,
}

#[derive(Clone, Debug)]
pub struct BlacklistEntry {
    pub rule_id: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub scope: serde_json::Value,
    pub reason: String,
    pub created_by: String,
    pub last_updated_at: PrimitiveDateTime,
    pub is_deleted: bool,
}

#[derive(Debug)]
pub enum BlacklistUpsertOutcome {
    Stored(Box<BlacklistEntry>),
    ActiveRuleLimitReached,
}

fn validate_request(
    rule_id: &str,
    merchant_id: &str,
    created_by: &str,
) -> ObservabilityApiResult<()> {
    if rule_id.is_empty() {
        return Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable("rule_id must not be blank");
    }
    if merchant_id.is_empty() {
        return Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable("merchant_id must not be empty");
    }
    if created_by.is_empty() {
        return Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable("created_by must not be blank");
    }
    Ok(())
}

fn validate_scope(scope: &serde_json::Value) -> ObservabilityApiResult<()> {
    if !scope.is_object() {
        return Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable("scope must be a JSON object of alert dimensions");
    }
    Ok(())
}

impl TryFrom<api::BlacklistUpsertRequest> for BlacklistEntryNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::BlacklistUpsertRequest) -> Result<Self, Self::Error> {
        let rule_id = request.rule_id.trim().to_owned();
        let merchant_id = request.merchant_id.trim().to_owned();
        let profile_id = request.profile_id.trim().to_owned();
        let created_by = request.created_by.trim().to_owned();
        validate_request(&rule_id, &merchant_id, &created_by)?;
        validate_scope(&request.scope)?;
        Ok(Self {
            rule_id,
            merchant_id,
            profile_id,
            scope: request.scope,
            reason: request.reason,
            created_by,
            is_deleted: false,
        })
    }
}

impl TryFrom<api::BlacklistDeleteRequest> for BlacklistEntryNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::BlacklistDeleteRequest) -> Result<Self, Self::Error> {
        let rule_id = request.rule_id.trim().to_owned();
        let merchant_id = request.merchant_id.trim().to_owned();
        let profile_id = request.profile_id.trim().to_owned();
        let created_by = request.created_by.trim().to_owned();
        validate_request(&rule_id, &merchant_id, &created_by)?;
        validate_scope(&request.scope)?;
        Ok(Self {
            rule_id,
            merchant_id,
            profile_id,
            scope: request.scope,
            reason: String::new(),
            created_by,
            is_deleted: true,
        })
    }
}

impl From<BlacklistEntryNew> for storage::BlacklistEntryNew {
    fn from(row: BlacklistEntryNew) -> Self {
        Self {
            rule_id: row.rule_id,
            merchant_id: row.merchant_id,
            profile_id: row.profile_id,
            scope: row.scope,
            reason: row.reason,
            created_by: row.created_by,
            is_deleted: row.is_deleted,
        }
    }
}

impl From<storage::BlacklistEntry> for BlacklistEntry {
    fn from(row: storage::BlacklistEntry) -> Self {
        Self {
            rule_id: row.rule_id,
            merchant_id: row.merchant_id,
            profile_id: row.profile_id,
            scope: row.scope,
            reason: row.reason,
            created_by: row.created_by,
            last_updated_at: row.last_updated_at,
            is_deleted: row.is_deleted,
        }
    }
}

impl From<BlacklistEntry> for api::BlacklistEntry {
    fn from(row: BlacklistEntry) -> Self {
        Self {
            rule_id: row.rule_id,
            merchant_id: row.merchant_id,
            profile_id: row.profile_id,
            scope: row.scope,
            reason: row.reason,
            created_by: row.created_by,
            last_updated_at: row.last_updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_scope_trimming_match_the_contract() {
        let row = BlacklistEntryNew::try_from(api::BlacklistUpsertRequest {
            rule_id: "all".into(),
            merchant_id: " merchant_1 ".into(),
            profile_id: " profile_1 ".into(),
            scope: serde_json::json!({}),
            reason: String::new(),
            created_by: "dashboard".into(),
        })
        .unwrap();
        assert_eq!(row.merchant_id, "merchant_1");
        assert_eq!(row.profile_id, "profile_1");
    }

    #[test]
    fn blank_keys_and_actor_are_rejected_but_empty_profile_is_valid() {
        let request =
            |rule_id: &str, merchant_id: &str, created_by: &str| api::BlacklistDeleteRequest {
                rule_id: rule_id.into(),
                merchant_id: merchant_id.into(),
                profile_id: String::new(),
                scope: serde_json::json!({}),
                created_by: created_by.into(),
            };
        assert!(BlacklistEntryNew::try_from(request("all", "  ", "dashboard")).is_err());
        assert!(BlacklistEntryNew::try_from(request("  ", "merchant", "dashboard")).is_err());
        assert!(BlacklistEntryNew::try_from(request("all", "merchant", "  ")).is_err());
        assert!(BlacklistEntryNew::try_from(request("all", "merchant", "dashboard")).is_ok());
    }

    #[test]
    fn preserves_all_selected_dimensions_in_scope() {
        let row = BlacklistEntryNew::try_from(api::BlacklistUpsertRequest {
            rule_id: "all".into(),
            merchant_id: "merchant_1".into(),
            profile_id: "profile_1".into(),
            scope: serde_json::json!({
                "organization_id": "org_1",
                "connector": "adyen",
                "payment_method_type": ["card", "wallet"]
            }),
            reason: String::new(),
            created_by: "dashboard".into(),
        })
        .unwrap();
        assert_eq!(row.scope["organization_id"], "org_1");
        assert_eq!(row.scope["payment_method_type"][0], "card");
    }
}
