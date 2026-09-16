//! The per (alert name, product) switch for merchant-facing delivery.

use api_models::observability::alert_manager::merchants_alert_external_config as api;
use diesel_models::observability::alert_manager::merchants_alert_external_config as storage;
use serde_json::Value;
use time::PrimitiveDateTime;

use crate::domain_models::{optional_text, required_text, SHORT_TEXT_MAX_CHARS};
use crate::errors::{ObservabilityApiResult, ObservabilityError};

/// A merchant alert delivery switch that has not been stored yet.
#[derive(Clone, Debug)]
pub struct MerchantsAlertExternalConfigNew {
    pub name: String,
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<Value>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// A stored merchant alert delivery switch.
#[derive(Clone, Debug)]
pub struct MerchantsAlertExternalConfig {
    pub name: String,
    pub product: String,
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<Value>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// What an update may change.
#[derive(Clone, Debug)]
pub struct MerchantsAlertExternalConfigUpdate {
    pub category: Option<String>,
    pub is_enabled: Option<bool>,
    pub metadata: Option<Value>,
    pub last_updated_at: PrimitiveDateTime,
}

/// The filters `list_merchant_alert_external_configs` accepts.
#[derive(Clone, Debug, Default)]
pub struct MerchantsAlertExternalConfigListFilter {
    pub names: Option<Vec<String>>,
    pub products: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub is_enabled: Option<bool>,
    pub metadata: Vec<(String, Vec<String>)>,
    pub last_updated_at_start: Option<PrimitiveDateTime>,
    pub last_updated_at_end: Option<PrimitiveDateTime>,
}

impl MerchantsAlertExternalConfigNew {
    /// Reject what the table would refuse, so a bad value is a `400` rather than a database
    /// error.
    fn validate(&self) -> ObservabilityApiResult<()> {
        required_text("name", &self.name, SHORT_TEXT_MAX_CHARS)?;
        required_text("product", &self.product, SHORT_TEXT_MAX_CHARS)?;
        optional_text("category", self.category.as_deref(), SHORT_TEXT_MAX_CHARS)?;

        Ok(())
    }
}

impl MerchantsAlertExternalConfigUpdate {
    /// Reject what the table would refuse, so a bad value is a `400` rather than a database
    /// error.
    fn validate(&self) -> ObservabilityApiResult<()> {
        optional_text("category", self.category.as_deref(), SHORT_TEXT_MAX_CHARS)?;

        Ok(())
    }
}

/// r-apps' `now`, truncated to the second, used when an update does not send its own
/// `last_updated_at`.
fn now_to_the_second() -> PrimitiveDateTime {
    let now = common_utils::date_time::now();
    now.replace_nanosecond(0).unwrap_or(now)
}

impl TryFrom<api::MerchantsAlertExternalConfigCreateRequest> for MerchantsAlertExternalConfigNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(
        request: api::MerchantsAlertExternalConfigCreateRequest,
    ) -> Result<Self, Self::Error> {
        let new = Self {
            name: request.name,
            product: request.product,
            category: request.category,
            is_enabled: request.is_enabled,
            metadata: request.metadata,
            last_updated_at: request.last_updated_at,
        };

        new.validate()?;

        Ok(new)
    }
}

impl TryFrom<api::MerchantsAlertExternalConfigUpdateRequest>
    for MerchantsAlertExternalConfigUpdate
{
    type Error = error_stack::Report<ObservabilityError>;

    /// A `last_updated_at` the caller sent overrides `now`, as r-apps sets `now` and then lets
    /// the body override it.
    fn try_from(
        request: api::MerchantsAlertExternalConfigUpdateRequest,
    ) -> Result<Self, Self::Error> {
        let update = Self {
            category: request.category,
            is_enabled: request.is_enabled,
            metadata: request.metadata,
            last_updated_at: request.last_updated_at.unwrap_or_else(now_to_the_second),
        };

        update.validate()?;

        Ok(update)
    }
}

impl From<api::MerchantsAlertExternalConfigListRequest> for MerchantsAlertExternalConfigListFilter {
    fn from(request: api::MerchantsAlertExternalConfigListRequest) -> Self {
        let non_empty = |values: Option<api::MerchantsAlertExternalConfigFilterValues>| {
            values
                .map(api::MerchantsAlertExternalConfigFilterValues::into_vec)
                .filter(|values| !values.is_empty())
        };

        let metadata = request
            .metadata
            .into_iter()
            .flatten()
            .filter_map(|(key, values)| {
                let values = values.into_vec();
                (!values.is_empty()).then_some((key, values))
            })
            .collect();

        let (last_updated_at_start, last_updated_at_end) = request
            .last_updated_at
            .map(|range| (range.start, range.end))
            .unwrap_or_default();

        Self {
            names: non_empty(request.name),
            products: non_empty(request.product),
            categories: non_empty(request.category),
            is_enabled: request.is_enabled,
            metadata,
            last_updated_at_start,
            last_updated_at_end,
        }
    }
}

impl From<MerchantsAlertExternalConfigNew> for storage::MerchantsAlertExternalConfigNew {
    fn from(new: MerchantsAlertExternalConfigNew) -> Self {
        Self {
            name: new.name,
            product: new.product,
            category: new.category,
            is_enabled: new.is_enabled,
            metadata: new.metadata,
            last_updated_at: new.last_updated_at,
        }
    }
}

impl From<MerchantsAlertExternalConfigUpdate> for storage::MerchantsAlertExternalConfigUpdate {
    fn from(update: MerchantsAlertExternalConfigUpdate) -> Self {
        Self {
            category: update.category,
            is_enabled: update.is_enabled,
            metadata: update.metadata,
            last_updated_at: update.last_updated_at,
        }
    }
}

impl From<MerchantsAlertExternalConfigListFilter>
    for storage::MerchantsAlertExternalConfigListFilter
{
    fn from(filter: MerchantsAlertExternalConfigListFilter) -> Self {
        Self {
            names: filter.names,
            products: filter.products,
            categories: filter.categories,
            is_enabled: filter.is_enabled,
            metadata: filter.metadata,
            last_updated_at_start: filter.last_updated_at_start,
            last_updated_at_end: filter.last_updated_at_end,
        }
    }
}

impl From<storage::MerchantsAlertExternalConfig> for MerchantsAlertExternalConfig {
    fn from(row: storage::MerchantsAlertExternalConfig) -> Self {
        Self {
            name: row.name,
            product: row.product,
            category: row.category,
            is_enabled: row.is_enabled,
            metadata: row.metadata,
            last_updated_at: row.last_updated_at,
        }
    }
}

impl From<MerchantsAlertExternalConfig> for api::MerchantsAlertExternalConfigResponse {
    fn from(config: MerchantsAlertExternalConfig) -> Self {
        Self {
            name: config.name,
            product: config.product,
            category: config.category,
            is_enabled: config.is_enabled,
            metadata: config.metadata,
            last_updated_at: config.last_updated_at,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use serde_json::json;
    use time::macros::datetime;

    use super::*;

    fn create_request() -> api::MerchantsAlertExternalConfigCreateRequest {
        api::MerchantsAlertExternalConfigCreateRequest {
            name: "Zero Volume".to_owned(),
            product: "payments".to_owned(),
            category: None,
            is_enabled: None,
            metadata: None,
            last_updated_at: None,
        }
    }

    fn update_request() -> api::MerchantsAlertExternalConfigUpdateRequest {
        api::MerchantsAlertExternalConfigUpdateRequest {
            name: "Zero Volume".to_owned(),
            product: "payments".to_owned(),
            category: None,
            is_enabled: None,
            metadata: None,
            last_updated_at: None,
        }
    }

    #[test]
    fn a_create_leaves_absent_fields_to_the_column_defaults() {
        let new = MerchantsAlertExternalConfigNew::try_from(create_request()).unwrap();

        assert!(new.category.is_none());
        assert!(new.is_enabled.is_none());
        assert!(new.metadata.is_none());
        assert!(new.last_updated_at.is_none());
    }

    #[test]
    fn a_create_refuses_a_blank_name() {
        let error = MerchantsAlertExternalConfigNew::try_from(
            api::MerchantsAlertExternalConfigCreateRequest {
                name: "  ".to_owned(),
                ..create_request()
            },
        )
        .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
    }

    #[test]
    fn a_create_refuses_a_blank_product() {
        let error = MerchantsAlertExternalConfigNew::try_from(
            api::MerchantsAlertExternalConfigCreateRequest {
                product: "  ".to_owned(),
                ..create_request()
            },
        )
        .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
    }

    #[test]
    fn a_create_refuses_a_65_character_name_product_or_category() {
        let too_long = "a".repeat(SHORT_TEXT_MAX_CHARS + 1);

        assert!(MerchantsAlertExternalConfigNew::try_from(
            api::MerchantsAlertExternalConfigCreateRequest {
                name: too_long.clone(),
                ..create_request()
            },
        )
        .is_err());
        assert!(MerchantsAlertExternalConfigNew::try_from(
            api::MerchantsAlertExternalConfigCreateRequest {
                product: too_long.clone(),
                ..create_request()
            },
        )
        .is_err());
        assert!(MerchantsAlertExternalConfigNew::try_from(
            api::MerchantsAlertExternalConfigCreateRequest {
                category: Some(too_long),
                ..create_request()
            },
        )
        .is_err());
    }

    #[test]
    fn an_update_stamps_now_to_the_second() {
        let before = common_utils::date_time::now();
        let update = MerchantsAlertExternalConfigUpdate::try_from(update_request()).unwrap();
        let after = common_utils::date_time::now();

        assert_eq!(update.last_updated_at.nanosecond(), 0);
        assert!(before.replace_nanosecond(0).unwrap() <= update.last_updated_at);
        assert!(update.last_updated_at <= after);
    }

    #[test]
    fn an_update_keeps_a_last_updated_at_the_caller_sent() {
        let sent = datetime!(2026-01-01 00:00:00);
        let update = MerchantsAlertExternalConfigUpdate::try_from(
            api::MerchantsAlertExternalConfigUpdateRequest {
                last_updated_at: Some(sent),
                ..update_request()
            },
        )
        .unwrap();

        assert_eq!(update.last_updated_at, sent);
    }

    #[test]
    fn an_update_refuses_a_65_character_category() {
        let error = MerchantsAlertExternalConfigUpdate::try_from(
            api::MerchantsAlertExternalConfigUpdateRequest {
                category: Some("a".repeat(SHORT_TEXT_MAX_CHARS + 1)),
                ..update_request()
            },
        )
        .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
    }

    #[test]
    fn a_list_filter_drops_empty_arrays_and_empty_metadata_values() {
        let filter = MerchantsAlertExternalConfigListFilter::from(
            api::MerchantsAlertExternalConfigListRequest {
                name: Some(api::MerchantsAlertExternalConfigFilterValues::Many(vec![])),
                product: None,
                category: None,
                is_enabled: None,
                metadata: Some(
                    [(
                        "tier".to_owned(),
                        api::MerchantsAlertExternalConfigFilterValues::Many(vec![]),
                    )]
                    .into_iter()
                    .collect(),
                ),
                last_updated_at: None,
            },
        );

        assert!(filter.names.is_none());
        assert!(filter.metadata.is_empty());
    }

    #[test]
    fn a_list_filter_takes_a_string_or_an_array() {
        let one = api::MerchantsAlertExternalConfigFilterValues::One("a".to_owned());
        let many = api::MerchantsAlertExternalConfigFilterValues::Many(vec![
            "a".to_owned(),
            "b".to_owned(),
        ]);

        assert_eq!(one.into_vec(), vec!["a".to_owned()]);
        assert_eq!(many.into_vec(), vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn an_update_body_cannot_carry_the_key() {
        let result: Result<api::MerchantsAlertExternalConfigUpdateRequest, _> =
            serde_json::from_value(json!({ "name": "x" }));

        assert!(result.is_err());
    }

    #[test]
    fn a_list_body_refuses_format() {
        let result: Result<api::MerchantsAlertExternalConfigListRequest, _> =
            serde_json::from_value(json!({ "format": "csv" }));

        assert!(result.is_err());
    }

    #[test]
    fn a_stored_row_reaches_the_response_unchanged() {
        let row = storage::MerchantsAlertExternalConfig {
            name: "Zero Volume".to_owned(),
            product: "payments".to_owned(),
            category: Some("Volume".to_owned()),
            is_enabled: Some(true),
            metadata: Some(json!({"slackConfig": {"statType": "Volume"}})),
            last_updated_at: Some(datetime!(2026-09-15 09:42:10)),
        };

        let response = api::MerchantsAlertExternalConfigResponse::from(
            MerchantsAlertExternalConfig::from(row),
        );
        let serialized = serde_json::to_value(&response).unwrap();

        assert_eq!(serialized["name"], "Zero Volume");
        assert_eq!(serialized["category"], "Volume");
        assert!(serialized["last_updated_at"]
            .as_str()
            .is_some_and(|value| value.contains('T')));
    }
}
