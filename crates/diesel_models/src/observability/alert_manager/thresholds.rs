//! PostgreSQL rows for success-rate threshold overrides.

use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::success_rate_threshold_overrides;

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = success_rate_threshold_overrides)]
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

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(
    table_name = success_rate_threshold_overrides,
    primary_key(name, product, merchant_id, profile_id),
    check_for_backend(diesel::pg::Pg)
)]
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
