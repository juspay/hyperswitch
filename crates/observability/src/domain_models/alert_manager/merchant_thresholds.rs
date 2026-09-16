//! Per-merchant threshold overrides: what an alert's thresholds should be for one merchant (and,
//! optionally, one profile) rather than the alert's own definition. A job coalesces a row's value
//! with the definition's, favouring the row where it is not `NULL`.

use api_models::observability::alert_manager::merchant_thresholds as api;
use common_utils::generate_time_ordered_id;
use diesel_models::observability::alert_manager::merchant_thresholds as storage;
use error_stack::{report, ResultExt};
use serde_json::Value;
use time::PrimitiveDateTime;

use crate::domain_models::{optional_text, SHORT_TEXT_MAX_CHARS};
use crate::errors::{ObservabilityApiResult, ObservabilityError};

/// An override that has not been stored yet.
#[derive(Clone, Debug)]
pub struct MerchantThresholdsNew {
    pub id: String,
    pub name: String,
    pub product: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub thresholds_min_volume: Option<f64>,
    pub thresholds_min_impacted_volume: Option<f64>,
    pub thresholds_tolerance: Option<f64>,
    pub thresholds_diff_threshold: Option<f64>,
    pub thresholds_merchant_impact: Option<f64>,
    pub thresholds_alert_period: Option<f64>,
    pub thresholds_min_observations: Option<f64>,
    pub thresholds_min_history_volume: Option<f64>,
    pub thresholds_filter_percentile: Option<f64>,
    pub thresholds_current_min_volume: Option<f64>,
    pub metadata: Option<Value>,
    pub author: Option<String>,
    pub is_enabled: Option<bool>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// A stored override.
#[derive(Clone, Debug)]
pub struct MerchantThresholds {
    pub id: String,
    pub name: Option<String>,
    pub product: Option<String>,
    pub merchant_id: Option<String>,
    pub profile_id: String,
    pub thresholds_min_volume: Option<f64>,
    pub thresholds_min_impacted_volume: Option<f64>,
    pub thresholds_tolerance: Option<f64>,
    pub thresholds_diff_threshold: Option<f64>,
    pub thresholds_merchant_impact: Option<f64>,
    pub thresholds_alert_period: Option<f64>,
    pub thresholds_min_observations: Option<f64>,
    pub thresholds_min_history_volume: Option<f64>,
    pub thresholds_filter_percentile: Option<f64>,
    pub thresholds_current_min_volume: Option<f64>,
    pub metadata: Option<Value>,
    pub author: Option<String>,
    pub is_enabled: Option<bool>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// The filters `list_merchant_thresholds` accepts.
#[derive(Clone, Debug, Default)]
pub struct MerchantThresholdsFilter {
    pub ids: Option<Vec<String>>,
    pub names: Option<Vec<String>>,
    pub products: Option<Vec<String>>,
    pub merchant_ids: Option<Vec<String>>,
    pub profile_ids: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
    pub is_enabled: Option<bool>,
    pub metadata: Vec<(String, Vec<String>)>,
    pub updated_from: Option<PrimitiveDateTime>,
    pub updated_to: Option<PrimitiveDateTime>,
}

/// The keys an update or a bulk delete selects rows by. Only the keys sent filter; at least one
/// must be sent.
#[derive(Clone, Debug, Default)]
pub struct MerchantThresholdsKeyFilter {
    pub name: Option<String>,
    pub product: Option<String>,
    pub merchant_id: Option<String>,
    pub profile_id: Option<String>,
}

/// What a write does to the `metadata` column.
#[derive(Clone, Debug, Default)]
pub enum MerchantThresholdsMetadataChange {
    /// Leave the column as it is.
    #[default]
    Keep,
    /// Set it to `NULL`.
    Clear,
    /// Overwrite it with the given value.
    Replace(Value),
    /// `metadata || value`, so a `NULL` column stays `NULL`.
    Merge(Value),
}

/// A validated change to apply to whatever rows a [`MerchantThresholdsKeyFilter`] selects.
#[derive(Clone, Debug, Default)]
pub struct MerchantThresholdsUpdate {
    pub thresholds_min_volume: Option<Option<f64>>,
    pub thresholds_min_impacted_volume: Option<Option<f64>>,
    pub thresholds_tolerance: Option<Option<f64>>,
    pub thresholds_diff_threshold: Option<Option<f64>>,
    pub thresholds_merchant_impact: Option<Option<f64>>,
    pub thresholds_alert_period: Option<Option<f64>>,
    pub thresholds_min_observations: Option<Option<f64>>,
    pub thresholds_min_history_volume: Option<Option<f64>>,
    pub thresholds_filter_percentile: Option<Option<f64>>,
    pub thresholds_current_min_volume: Option<Option<f64>>,
    pub metadata: MerchantThresholdsMetadataChange,
    pub author: Option<Option<String>>,
    pub is_enabled: Option<Option<bool>>,
    pub last_updated_at: Option<Option<PrimitiveDateTime>>,
}

/// A validated `update_merchant_thresholds` request: the keys to select rows, and the change to
/// apply to them.
#[derive(Clone, Debug)]
pub struct MerchantThresholdsBulkUpdate {
    pub filter: MerchantThresholdsKeyFilter,
    pub update: MerchantThresholdsUpdate,
}

impl MerchantThresholdsNew {
    /// Reject what the table would refuse, so a bad value is a `400` rather than a database error.
    /// `name`, `product`, `merchant_id` and `profile_id` may be empty, as r-apps allows, but not
    /// longer than the column.
    fn validate(&self) -> ObservabilityApiResult<()> {
        optional_text("name", Some(&self.name), SHORT_TEXT_MAX_CHARS)?;
        optional_text("product", Some(&self.product), SHORT_TEXT_MAX_CHARS)?;
        optional_text("merchant_id", Some(&self.merchant_id), SHORT_TEXT_MAX_CHARS)?;
        optional_text("profile_id", Some(&self.profile_id), SHORT_TEXT_MAX_CHARS)?;
        optional_text("author", self.author.as_deref(), SHORT_TEXT_MAX_CHARS)?;

        Ok(())
    }

    /// What an upsert sets on a conflict: every threshold, `metadata` and `last_updated_at` that
    /// was actually sent. `author` and `is_enabled` are never here — they are part of the conflict
    /// key, not something a conflicting row can change. `None` when nothing was sent, so the
    /// caller can turn the write into `DO NOTHING`.
    pub fn conflict_update(&self) -> Option<MerchantThresholdsUpdate> {
        let mut update = MerchantThresholdsUpdate::default();
        let mut changed = false;

        macro_rules! copy_threshold {
            ($field:ident) => {
                if let Some(value) = self.$field {
                    update.$field = Some(Some(value));
                    changed = true;
                }
            };
        }

        copy_threshold!(thresholds_min_volume);
        copy_threshold!(thresholds_min_impacted_volume);
        copy_threshold!(thresholds_tolerance);
        copy_threshold!(thresholds_diff_threshold);
        copy_threshold!(thresholds_merchant_impact);
        copy_threshold!(thresholds_alert_period);
        copy_threshold!(thresholds_min_observations);
        copy_threshold!(thresholds_min_history_volume);
        copy_threshold!(thresholds_filter_percentile);
        copy_threshold!(thresholds_current_min_volume);

        if let Some(metadata) = self.metadata.clone() {
            update.metadata = MerchantThresholdsMetadataChange::Replace(metadata);
            changed = true;
        }

        if let Some(last_updated_at) = self.last_updated_at {
            update.last_updated_at = Some(Some(last_updated_at));
            changed = true;
        }

        changed.then_some(update)
    }
}

impl MerchantThresholdsUpdate {
    /// Reject a write that would touch nothing, and a `metadata` merge that is not an object —
    /// `metadata || value` needs an object on both sides.
    fn validate(&self) -> ObservabilityApiResult<()> {
        let nothing_to_change = self.thresholds_min_volume.is_none()
            && self.thresholds_min_impacted_volume.is_none()
            && self.thresholds_tolerance.is_none()
            && self.thresholds_diff_threshold.is_none()
            && self.thresholds_merchant_impact.is_none()
            && self.thresholds_alert_period.is_none()
            && self.thresholds_min_observations.is_none()
            && self.thresholds_min_history_volume.is_none()
            && self.thresholds_filter_percentile.is_none()
            && self.thresholds_current_min_volume.is_none()
            && self.author.is_none()
            && self.is_enabled.is_none()
            && self.last_updated_at.is_none()
            && matches!(self.metadata, MerchantThresholdsMetadataChange::Keep);

        if nothing_to_change {
            Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("nothing to update")?;
        }

        if let MerchantThresholdsMetadataChange::Merge(value) = &self.metadata {
            if !value.is_object() {
                Err(report!(ObservabilityError::InvalidRequest))
                    .attach_printable("metadata must be an object")?;
            }
        }

        Ok(())
    }

    /// Split into what an `AsChangeset` can express directly and the `metadata || value` merge,
    /// which cannot: an `AsChangeset` column can only be skipped or set, never assigned from an
    /// expression over its own current value.
    pub fn into_storage(self) -> (storage::MerchantThresholdsUpdate, Option<Value>) {
        let (metadata, merge) = match self.metadata {
            MerchantThresholdsMetadataChange::Keep => (None, None),
            MerchantThresholdsMetadataChange::Clear => (Some(None), None),
            MerchantThresholdsMetadataChange::Replace(value) => (Some(Some(value)), None),
            MerchantThresholdsMetadataChange::Merge(value) => (None, Some(value)),
        };

        (
            storage::MerchantThresholdsUpdate {
                thresholds_min_volume: self.thresholds_min_volume,
                thresholds_min_impacted_volume: self.thresholds_min_impacted_volume,
                thresholds_tolerance: self.thresholds_tolerance,
                thresholds_diff_threshold: self.thresholds_diff_threshold,
                thresholds_merchant_impact: self.thresholds_merchant_impact,
                thresholds_alert_period: self.thresholds_alert_period,
                thresholds_min_observations: self.thresholds_min_observations,
                thresholds_min_history_volume: self.thresholds_min_history_volume,
                thresholds_filter_percentile: self.thresholds_filter_percentile,
                thresholds_current_min_volume: self.thresholds_current_min_volume,
                metadata,
                author: self.author,
                is_enabled: self.is_enabled,
                last_updated_at: self.last_updated_at,
            },
            merge,
        )
    }
}

impl TryFrom<api::MerchantThresholdsUpsertRequest> for MerchantThresholdsNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::MerchantThresholdsUpsertRequest) -> Result<Self, Self::Error> {
        let new = Self {
            id: generate_time_ordered_id("merchant_threshold"),
            name: request.name,
            product: request.product,
            merchant_id: request.merchant_id,
            profile_id: request.profile_id,
            thresholds_min_volume: request.thresholds_min_volume,
            thresholds_min_impacted_volume: request.thresholds_min_impacted_volume,
            thresholds_tolerance: request.thresholds_tolerance,
            thresholds_diff_threshold: request.thresholds_diff_threshold,
            thresholds_merchant_impact: request.thresholds_merchant_impact,
            thresholds_alert_period: request.thresholds_alert_period,
            thresholds_min_observations: request.thresholds_min_observations,
            thresholds_min_history_volume: request.thresholds_min_history_volume,
            thresholds_filter_percentile: request.thresholds_filter_percentile,
            thresholds_current_min_volume: request.thresholds_current_min_volume,
            metadata: request.metadata,
            author: request.author,
            is_enabled: request.is_enabled,
            last_updated_at: request.last_updated_at,
        };

        new.validate()?;

        Ok(new)
    }
}

impl TryFrom<api::MerchantThresholdsUpdateRequest> for MerchantThresholdsBulkUpdate {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::MerchantThresholdsUpdateRequest) -> Result<Self, Self::Error> {
        let filter = MerchantThresholdsKeyFilter {
            name: request.name,
            product: request.product,
            merchant_id: request.merchant_id,
            profile_id: request.profile_id,
        };

        if filter.name.is_none()
            && filter.product.is_none()
            && filter.merchant_id.is_none()
            && filter.profile_id.is_none()
        {
            Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("name, product, merchant_id or profile_id is required")?;
        }

        let metadata = match request.metadata {
            None => MerchantThresholdsMetadataChange::Keep,
            Some(None) => MerchantThresholdsMetadataChange::Clear,
            Some(Some(value)) => MerchantThresholdsMetadataChange::Merge(value),
        };

        let update = MerchantThresholdsUpdate {
            thresholds_min_volume: request.thresholds_min_volume,
            thresholds_min_impacted_volume: request.thresholds_min_impacted_volume,
            thresholds_tolerance: request.thresholds_tolerance,
            thresholds_diff_threshold: request.thresholds_diff_threshold,
            thresholds_merchant_impact: request.thresholds_merchant_impact,
            thresholds_alert_period: request.thresholds_alert_period,
            thresholds_min_observations: request.thresholds_min_observations,
            thresholds_min_history_volume: request.thresholds_min_history_volume,
            thresholds_filter_percentile: request.thresholds_filter_percentile,
            thresholds_current_min_volume: request.thresholds_current_min_volume,
            metadata,
            author: request.author,
            is_enabled: request.is_enabled,
            last_updated_at: request.last_updated_at,
        };

        update.validate()?;

        Ok(Self { filter, update })
    }
}

/// `One(v)` is the one filter value; an empty `Many` means no filter on this field.
fn text_values(filter: api::MerchantThresholdsTextFilter) -> Option<Vec<String>> {
    match filter {
        api::MerchantThresholdsTextFilter::One(value) => Some(vec![value]),
        api::MerchantThresholdsTextFilter::Many(values) if values.is_empty() => None,
        api::MerchantThresholdsTextFilter::Many(values) => Some(values),
    }
}

impl TryFrom<api::MerchantThresholdsListRequest> for MerchantThresholdsFilter {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::MerchantThresholdsListRequest) -> Result<Self, Self::Error> {
        let ids = request
            .id
            .and_then(text_values)
            .map(|ids| ids.iter().map(|id| parse_id(id)).collect::<Result<_, _>>())
            .transpose()?;

        let metadata = request
            .metadata
            .into_iter()
            .flatten()
            .filter_map(|(key, filter)| text_values(filter).map(|values| (key, values)))
            .collect();

        let (updated_from, updated_to) = match request.last_updated_at {
            Some(api::MerchantThresholdsTimeFilter::Range { start, end }) => (start, end),
            Some(api::MerchantThresholdsTimeFilter::From(at)) => (Some(at), None),
            None => (None, None),
        };

        Ok(Self {
            ids,
            names: request.name.and_then(text_values),
            products: request.product.and_then(text_values),
            merchant_ids: request.merchant_id.and_then(text_values),
            profile_ids: request.profile_id.and_then(text_values),
            authors: request.author.and_then(text_values),
            is_enabled: request.is_enabled,
            metadata,
            updated_from,
            updated_to,
        })
    }
}

impl From<api::MerchantThresholdsDeleteByFilterRequest> for MerchantThresholdsKeyFilter {
    fn from(request: api::MerchantThresholdsDeleteByFilterRequest) -> Self {
        Self {
            name: Some(request.name),
            product: Some(request.product),
            merchant_id: request.merchant_id,
            profile_id: request.profile_id,
        }
    }
}

impl From<MerchantThresholdsNew> for storage::MerchantThresholdsNew {
    fn from(new: MerchantThresholdsNew) -> Self {
        Self {
            id: new.id,
            name: new.name,
            product: new.product,
            merchant_id: new.merchant_id,
            profile_id: new.profile_id,
            thresholds_min_volume: new.thresholds_min_volume,
            thresholds_min_impacted_volume: new.thresholds_min_impacted_volume,
            thresholds_tolerance: new.thresholds_tolerance,
            thresholds_diff_threshold: new.thresholds_diff_threshold,
            thresholds_merchant_impact: new.thresholds_merchant_impact,
            thresholds_alert_period: new.thresholds_alert_period,
            thresholds_min_observations: new.thresholds_min_observations,
            thresholds_min_history_volume: new.thresholds_min_history_volume,
            thresholds_filter_percentile: new.thresholds_filter_percentile,
            thresholds_current_min_volume: new.thresholds_current_min_volume,
            metadata: new.metadata,
            author: new.author,
            is_enabled: new.is_enabled,
            last_updated_at: new.last_updated_at,
        }
    }
}

impl From<storage::MerchantThresholds> for MerchantThresholds {
    fn from(row: storage::MerchantThresholds) -> Self {
        Self {
            id: row.id,
            name: row.name,
            product: row.product,
            merchant_id: row.merchant_id,
            profile_id: row.profile_id,
            thresholds_min_volume: row.thresholds_min_volume,
            thresholds_min_impacted_volume: row.thresholds_min_impacted_volume,
            thresholds_tolerance: row.thresholds_tolerance,
            thresholds_diff_threshold: row.thresholds_diff_threshold,
            thresholds_merchant_impact: row.thresholds_merchant_impact,
            thresholds_alert_period: row.thresholds_alert_period,
            thresholds_min_observations: row.thresholds_min_observations,
            thresholds_min_history_volume: row.thresholds_min_history_volume,
            thresholds_filter_percentile: row.thresholds_filter_percentile,
            thresholds_current_min_volume: row.thresholds_current_min_volume,
            metadata: row.metadata,
            author: row.author,
            is_enabled: row.is_enabled,
            last_updated_at: row.last_updated_at,
        }
    }
}

impl From<MerchantThresholdsFilter> for storage::MerchantThresholdsFilter {
    fn from(filter: MerchantThresholdsFilter) -> Self {
        Self {
            ids: filter.ids,
            names: filter.names,
            products: filter.products,
            merchant_ids: filter.merchant_ids,
            profile_ids: filter.profile_ids,
            authors: filter.authors,
            is_enabled: filter.is_enabled,
            metadata: filter.metadata,
            updated_from: filter.updated_from,
            updated_to: filter.updated_to,
        }
    }
}

impl From<MerchantThresholdsKeyFilter> for storage::MerchantThresholdsKeyFilter {
    fn from(filter: MerchantThresholdsKeyFilter) -> Self {
        Self {
            name: filter.name,
            product: filter.product,
            merchant_id: filter.merchant_id,
            profile_id: filter.profile_id,
        }
    }
}

impl From<MerchantThresholds> for api::MerchantThresholdsResponse {
    fn from(row: MerchantThresholds) -> Self {
        Self {
            id: row.id.to_string(),
            name: row.name,
            product: row.product,
            merchant_id: row.merchant_id,
            profile_id: row.profile_id,
            thresholds_min_volume: row.thresholds_min_volume,
            thresholds_min_impacted_volume: row.thresholds_min_impacted_volume,
            thresholds_tolerance: row.thresholds_tolerance,
            thresholds_diff_threshold: row.thresholds_diff_threshold,
            thresholds_merchant_impact: row.thresholds_merchant_impact,
            thresholds_alert_period: row.thresholds_alert_period,
            thresholds_min_observations: row.thresholds_min_observations,
            thresholds_min_history_volume: row.thresholds_min_history_volume,
            thresholds_filter_percentile: row.thresholds_filter_percentile,
            thresholds_current_min_volume: row.thresholds_current_min_volume,
            metadata: row.metadata,
            author: row.author,
            is_enabled: row.is_enabled,
            last_updated_at: row.last_updated_at,
        }
    }
}

/// The body of every route that returns more than one row. A free function, since the orphan rule
/// blocks `impl From<Vec<MerchantThresholds>> for api::MerchantThresholdsListResponse` here.
pub fn list_response(rows: Vec<MerchantThresholds>) -> api::MerchantThresholdsListResponse {
    let data: Vec<api::MerchantThresholdsResponse> = rows
        .into_iter()
        .map(api::MerchantThresholdsResponse::from)
        .collect();

    api::MerchantThresholdsListResponse {
        count: data.len(),
        data,
    }
}

/// Parse a path `id` or an `id` filter value into what the table's primary key actually is, so an
/// Reject a blank ID before it reaches the database.
pub fn parse_id(id: &str) -> ObservabilityApiResult<String> {
    if id.trim().is_empty() {
        Err(report!(ObservabilityError::InvalidRequest)).attach_printable("id must not be empty")?
    }

    Ok(id.to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn upsert_request() -> api::MerchantThresholdsUpsertRequest {
        api::MerchantThresholdsUpsertRequest {
            name: "Volume Drop".to_owned(),
            product: "payments".to_owned(),
            merchant_id: "acme_store".to_owned(),
            profile_id: String::new(),
            thresholds_min_volume: None,
            thresholds_min_impacted_volume: None,
            thresholds_tolerance: None,
            thresholds_diff_threshold: None,
            thresholds_merchant_impact: None,
            thresholds_alert_period: None,
            thresholds_min_observations: None,
            thresholds_min_history_volume: None,
            thresholds_filter_percentile: None,
            thresholds_current_min_volume: None,
            metadata: None,
            author: None,
            is_enabled: None,
            last_updated_at: None,
        }
    }

    #[test]
    fn an_upsert_leaves_absent_fields_to_the_database() {
        let new = MerchantThresholdsNew::try_from(upsert_request()).unwrap();

        assert!(new.author.is_none());
        assert!(new.is_enabled.is_none());
        assert!(new.thresholds_min_volume.is_none());
    }

    #[test]
    fn an_upsert_with_only_keys_does_nothing_on_conflict() {
        let new = MerchantThresholdsNew::try_from(upsert_request()).unwrap();

        assert!(new.conflict_update().is_none());
    }

    #[test]
    fn an_upsert_conflict_sets_only_sent_non_key_fields() {
        let new = MerchantThresholdsNew::try_from(api::MerchantThresholdsUpsertRequest {
            thresholds_min_volume: Some(50.0),
            metadata: Some(serde_json::json!({"k": "v"})),
            ..upsert_request()
        })
        .unwrap();

        let update = new.conflict_update().unwrap();

        assert_eq!(update.thresholds_min_volume, Some(Some(50.0)));
        assert!(matches!(
            update.metadata,
            MerchantThresholdsMetadataChange::Replace(_)
        ));
        assert!(update.author.is_none());
        assert!(update.is_enabled.is_none());
    }

    #[test]
    fn a_threshold_is_stored_as_the_value_sent() {
        let new = MerchantThresholdsNew::try_from(api::MerchantThresholdsUpsertRequest {
            thresholds_tolerance: Some(0.1),
            thresholds_min_volume: Some(100.0),
            ..upsert_request()
        })
        .unwrap();

        assert_eq!(new.thresholds_tolerance, Some(0.1));
        assert_eq!(new.thresholds_min_volume, Some(100.0));
    }

    fn update_request() -> api::MerchantThresholdsUpdateRequest {
        api::MerchantThresholdsUpdateRequest {
            merchant_id: Some("acme_store".to_owned()),
            ..Default::default()
        }
    }

    #[test]
    fn an_update_needs_a_key() {
        let error = MerchantThresholdsBulkUpdate::try_from(api::MerchantThresholdsUpdateRequest {
            merchant_id: None,
            thresholds_min_volume: Some(Some(1.0)),
            ..Default::default()
        })
        .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
    }

    #[test]
    fn an_update_needs_a_change() {
        let error = MerchantThresholdsBulkUpdate::try_from(update_request()).unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
    }

    #[test]
    fn an_update_null_clears_and_absent_keeps() {
        let bulk_update =
            MerchantThresholdsBulkUpdate::try_from(api::MerchantThresholdsUpdateRequest {
                thresholds_min_volume: Some(None),
                ..update_request()
            })
            .unwrap();

        let (changeset, merge) = bulk_update.update.into_storage();

        assert_eq!(changeset.thresholds_min_volume, Some(None));
        assert!(changeset.thresholds_tolerance.is_none());
        assert!(merge.is_none());
    }

    #[test]
    fn an_update_metadata_object_merges_null_clears_other_refused() {
        let merged = MerchantThresholdsBulkUpdate::try_from(api::MerchantThresholdsUpdateRequest {
            metadata: Some(Some(serde_json::json!({"a": 1}))),
            ..update_request()
        })
        .unwrap();
        let (_, merge) = merged.update.into_storage();
        assert_eq!(merge, Some(serde_json::json!({"a": 1})));

        let cleared =
            MerchantThresholdsBulkUpdate::try_from(api::MerchantThresholdsUpdateRequest {
                metadata: Some(None),
                ..update_request()
            })
            .unwrap();
        let (changeset, merge) = cleared.update.into_storage();
        assert_eq!(changeset.metadata, Some(None));
        assert!(merge.is_none());

        let error = MerchantThresholdsBulkUpdate::try_from(api::MerchantThresholdsUpdateRequest {
            metadata: Some(Some(serde_json::json!(["not", "an", "object"]))),
            ..update_request()
        })
        .unwrap_err();
        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
    }

    #[test]
    fn a_stored_row_reaches_the_response_unchanged() {
        let row = MerchantThresholds {
            id: "01JTESTID".to_owned(),
            name: Some("Volume Drop".to_owned()),
            product: Some("payments".to_owned()),
            merchant_id: Some("acme_store".to_owned()),
            profile_id: String::new(),
            thresholds_min_volume: Some(50.0),
            thresholds_min_impacted_volume: None,
            thresholds_tolerance: None,
            thresholds_diff_threshold: None,
            thresholds_merchant_impact: None,
            thresholds_alert_period: None,
            thresholds_min_observations: None,
            thresholds_min_history_volume: None,
            thresholds_filter_percentile: None,
            thresholds_current_min_volume: None,
            metadata: None,
            author: Some("reliability_team".to_owned()),
            is_enabled: Some(false),
            last_updated_at: Some(datetime!(2026-09-15 09:42:10.512)),
        };

        let response = api::MerchantThresholdsResponse::from(row);
        let serialized = serde_json::to_value(&response).unwrap();

        assert_eq!(serialized["id"], "01JTESTID");
        assert_eq!(serialized["thresholds_min_volume"], 50.0);
        assert!(serialized["last_updated_at"]
            .as_str()
            .is_some_and(|value| value.contains('T')));
    }

    #[test]
    fn a_text_filter_takes_one_value_or_a_list() {
        assert_eq!(
            text_values(api::MerchantThresholdsTextFilter::One("a".to_owned())),
            Some(vec!["a".to_owned()])
        );
        assert_eq!(
            text_values(api::MerchantThresholdsTextFilter::Many(vec![
                "a".to_owned(),
                "b".to_owned()
            ])),
            Some(vec!["a".to_owned(), "b".to_owned()])
        );
        assert_eq!(
            text_values(api::MerchantThresholdsTextFilter::Many(Vec::new())),
            None
        );
    }

    #[test]
    fn a_time_filter_takes_a_range_or_a_bare_timestamp() {
        let filter: api::MerchantThresholdsListRequest =
            serde_json::from_value(serde_json::json!({
                "last_updated_at": "2026-09-15T00:00:00.000Z"
            }))
            .unwrap();
        let parsed = MerchantThresholdsFilter::try_from(filter).unwrap();
        assert!(parsed.updated_from.is_some());
        assert!(parsed.updated_to.is_none());

        let filter: api::MerchantThresholdsListRequest =
            serde_json::from_value(serde_json::json!({
                "last_updated_at": {
                    "start": "2026-09-15T00:00:00.000Z",
                    "end": "2026-09-16T00:00:00.000Z"
                }
            }))
            .unwrap();
        let parsed = MerchantThresholdsFilter::try_from(filter).unwrap();
        assert!(parsed.updated_from.is_some());
        assert!(parsed.updated_to.is_some());
    }

    #[test]
    fn a_blank_id_is_refused() {
        assert!(parse_id("").is_err());
        assert!(parse_id("01JTESTID").is_ok());

        let filter: api::MerchantThresholdsListRequest =
            serde_json::from_value(serde_json::json!({
                "id": "01JTESTID"
            }))
            .unwrap();
        assert!(MerchantThresholdsFilter::try_from(filter).is_ok());
    }
}
