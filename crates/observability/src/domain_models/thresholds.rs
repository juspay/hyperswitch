//! Validated threshold override domain models and layer conversions.

use api_models::observability::thresholds as api;
use diesel_models::observability::thresholds as storage;
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

#[derive(Clone, Debug)]
pub struct ThresholdOverrideNew {
    pub name: String,
    pub product: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub min_volume: Option<f64>,
    pub min_impacted_volume: Option<f64>,
    pub tolerance: Option<f64>,
    pub diff_threshold: Option<f64>,
    pub updated_by: String,
    pub is_deleted: bool,
}

#[derive(Clone, Debug)]
pub struct ThresholdOverride {
    pub name: String,
    pub product: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub min_volume: Option<f64>,
    pub min_impacted_volume: Option<f64>,
    pub tolerance: Option<f64>,
    pub diff_threshold: Option<f64>,
    pub updated_by: String,
    pub last_updated_at: PrimitiveDateTime,
    pub is_deleted: bool,
}

#[derive(Debug)]
pub enum ThresholdUpsertOutcome {
    Stored(ThresholdOverride),
    ActiveRuleLimitReached,
}

fn validate_key_and_actor(
    name: &str,
    product: &str,
    merchant_id: &str,
    updated_by: &str,
) -> ObservabilityApiResult<()> {
    for (field, value) in [
        ("name", name),
        ("product", product),
        ("merchant_id", merchant_id),
        ("updated_by", updated_by),
    ] {
        if value.trim().is_empty() {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable(format!("{field} must not be empty"));
        }
    }
    Ok(())
}

fn validate_numbers(values: [Option<f64>; 4]) -> ObservabilityApiResult<()> {
    if values.into_iter().flatten().any(|value| !value.is_finite()) {
        return Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable("threshold values must be finite numbers or null");
    }
    Ok(())
}

impl TryFrom<api::ThresholdUpsertRequest> for ThresholdOverrideNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::ThresholdUpsertRequest) -> Result<Self, Self::Error> {
        let merchant_id = request.merchant_id.trim().to_owned();
        let profile_id = request.profile_id.trim().to_owned();
        validate_key_and_actor(
            &request.name,
            &request.product,
            &merchant_id,
            &request.updated_by,
        )?;
        validate_numbers([
            request.min_volume,
            request.min_impacted_volume,
            request.tolerance,
            request.diff_threshold,
        ])?;

        Ok(Self {
            name: request.name,
            product: request.product,
            merchant_id,
            profile_id,
            min_volume: request.min_volume,
            min_impacted_volume: request.min_impacted_volume,
            tolerance: request.tolerance,
            diff_threshold: request.diff_threshold,
            updated_by: request.updated_by,
            is_deleted: false,
        })
    }
}

impl TryFrom<api::ThresholdDeleteRequest> for ThresholdOverrideNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::ThresholdDeleteRequest) -> Result<Self, Self::Error> {
        let merchant_id = request.merchant_id.trim().to_owned();
        let profile_id = request.profile_id.trim().to_owned();
        validate_key_and_actor(
            &request.name,
            &request.product,
            &merchant_id,
            &request.updated_by,
        )?;

        Ok(Self {
            name: request.name,
            product: request.product,
            merchant_id,
            profile_id,
            min_volume: None,
            min_impacted_volume: None,
            tolerance: None,
            diff_threshold: None,
            updated_by: request.updated_by,
            is_deleted: true,
        })
    }
}

impl From<ThresholdOverrideNew> for storage::ThresholdOverrideNew {
    fn from(row: ThresholdOverrideNew) -> Self {
        Self {
            name: row.name,
            product: row.product,
            merchant_id: row.merchant_id,
            profile_id: row.profile_id,
            min_volume: row.min_volume,
            min_impacted_volume: row.min_impacted_volume,
            tolerance: row.tolerance,
            diff_threshold: row.diff_threshold,
            updated_by: row.updated_by,
            is_deleted: row.is_deleted,
        }
    }
}

impl From<storage::ThresholdOverride> for ThresholdOverride {
    fn from(row: storage::ThresholdOverride) -> Self {
        Self {
            name: row.name,
            product: row.product,
            merchant_id: row.merchant_id,
            profile_id: row.profile_id,
            min_volume: row.min_volume,
            min_impacted_volume: row.min_impacted_volume,
            tolerance: row.tolerance,
            diff_threshold: row.diff_threshold,
            updated_by: row.updated_by,
            last_updated_at: row.last_updated_at,
            is_deleted: row.is_deleted,
        }
    }
}

impl From<ThresholdOverride> for api::ThresholdResponse {
    fn from(row: ThresholdOverride) -> Self {
        Self {
            name: row.name,
            product: row.product,
            merchant_id: row.merchant_id,
            profile_id: row.profile_id,
            min_volume: row.min_volume,
            min_impacted_volume: row.min_impacted_volume,
            tolerance: row.tolerance,
            diff_threshold: row.diff_threshold,
            updated_by: row.updated_by,
            last_updated_at: row.last_updated_at,
            is_deleted: row.is_deleted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_trims_scope_ids_and_preserves_nullable_values() {
        let row = ThresholdOverrideNew::try_from(api::ThresholdUpsertRequest {
            name: "Success-rate drop".into(),
            product: "payments".into(),
            merchant_id: " merchant_123 ".into(),
            profile_id: " profile_123 ".into(),
            min_volume: None,
            min_impacted_volume: Some(-1.0),
            tolerance: Some(0.0),
            diff_threshold: None,
            updated_by: "dashboard".into(),
        })
        .unwrap();

        assert_eq!(row.merchant_id, "merchant_123");
        assert_eq!(row.profile_id, "profile_123");
        assert_eq!(row.min_volume, None);
        assert_eq!(row.min_impacted_volume, Some(-1.0));
    }

    #[test]
    fn empty_keys_and_actor_are_rejected() {
        for (merchant_id, updated_by) in [("  ", "dashboard"), ("merchant", "  ")] {
            let request = api::ThresholdDeleteRequest {
                name: "name".into(),
                product: "product".into(),
                merchant_id: merchant_id.into(),
                profile_id: String::new(),
                updated_by: updated_by.into(),
            };
            assert!(ThresholdOverrideNew::try_from(request).is_err());
        }
    }

    #[test]
    fn non_finite_numbers_are_rejected() {
        let request = api::ThresholdUpsertRequest {
            name: "name".into(),
            product: "product".into(),
            merchant_id: "merchant".into(),
            profile_id: String::new(),
            min_volume: Some(f64::INFINITY),
            min_impacted_volume: None,
            tolerance: None,
            diff_threshold: None,
            updated_by: "dashboard".into(),
        };
        assert!(ThresholdOverrideNew::try_from(request).is_err());
    }
}
