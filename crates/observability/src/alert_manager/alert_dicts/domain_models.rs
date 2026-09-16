//! The mappers dictionary: small named lists looked up by `(name, key_)`. Every save disables the
//! live row and inserts a new one, so a value here is always a version rather than an edit.

use api_models::observability::alert_manager::alert_dicts as api;
use common_utils::{date_time, generate_time_ordered_id};
use diesel_models::observability::alert_manager::alert_dicts as storage;
use error_stack::ResultExt;
use serde_json::Value;
use time::PrimitiveDateTime;

use crate::{
    domain_models::utils::{optional_text, required_text},
    errors::{ObservabilityApiResult, ObservabilityError},
};

const NAME_MAX_CHARS: usize = 64;
const KEY_MAX_CHARS: usize = 255;
const USERNAME_MAX_CHARS: usize = 64;

/// A dictionary version that has not been stored yet.
#[derive(Clone, Debug)]
pub struct AlertsDictsNew {
    pub id: String,
    pub name: String,
    pub key: String,
    pub product: Value,
    pub values_: Value,
    pub ts_created: PrimitiveDateTime,
    pub is_enabled: bool,
    pub username: Option<String>,
    pub metadata: Value,
}

/// A stored dictionary version.
#[derive(Clone, Debug)]
pub struct AlertsDicts {
    pub id: String,
    pub name: String,
    pub key: String,
    pub product: Option<Value>,
    pub values_: Option<Value>,
    pub ts_created: Option<PrimitiveDateTime>,
    pub is_enabled: Option<bool>,
    pub username: Option<String>,
    pub metadata: Option<Value>,
}

/// The filters `list_alert_dicts` accepts.
#[derive(Clone, Debug)]
pub struct AlertsDictsFilter {
    pub name: Option<String>,
    pub key: Option<String>,
    pub is_enabled: bool,
}

impl AlertsDictsNew {
    /// Reject what the table would refuse, so a bad value is a `400` rather than a database error.
    fn validate(&self) -> ObservabilityApiResult<()> {
        required_text("name", &self.name, NAME_MAX_CHARS)?;
        required_text("key", &self.key, KEY_MAX_CHARS)?;
        optional_text("username", self.username.as_deref(), USERNAME_MAX_CHARS)?;

        Ok(())
    }
}

/// `None` or `null` becomes an empty array; anything else is kept as sent.
fn empty_array_if_null(value: Option<Value>) -> Value {
    match value {
        None | Some(Value::Null) => Value::Array(Vec::new()),
        Some(value) => value,
    }
}

impl TryFrom<api::AlertsDictsCreateRequest> for AlertsDictsNew {
    type Error = error_stack::Report<ObservabilityError>;

    /// The server stamps `ts_created` rather than trusting a caller for it, truncated to the
    /// second as r-apps does.
    fn try_from(request: api::AlertsDictsCreateRequest) -> Result<Self, Self::Error> {
        let now = date_time::now();

        let new = Self {
            id: generate_time_ordered_id("dict"),
            name: request.name,
            key: request.key,
            product: empty_array_if_null(request.product),
            values_: empty_array_if_null(Some(request.values_)),
            ts_created: now.replace_millisecond(0).unwrap_or(now),
            is_enabled: true,
            username: request.username,
            metadata: empty_array_if_null(request.metadata),
        };

        new.validate()?;

        Ok(new)
    }
}

impl From<AlertsDictsNew> for storage::AlertsDictsNew {
    fn from(new: AlertsDictsNew) -> Self {
        Self {
            id: new.id,
            name: new.name,
            key_: new.key,
            product: new.product,
            values_: new.values_,
            ts_created: new.ts_created,
            is_enabled: new.is_enabled,
            username: new.username,
            metadata: new.metadata,
        }
    }
}

impl From<storage::AlertsDicts> for AlertsDicts {
    fn from(row: storage::AlertsDicts) -> Self {
        Self {
            id: row.id,
            name: row.name,
            key: row.key_,
            product: row.product,
            values_: row.values_,
            ts_created: row.ts_created,
            is_enabled: row.is_enabled,
            username: row.username,
            metadata: row.metadata,
        }
    }
}

impl From<AlertsDicts> for api::AlertsDictsResponse {
    fn from(entry: AlertsDicts) -> Self {
        Self {
            id: entry.id,
            name: entry.name,
            key: entry.key,
            product: entry.product,
            values_: entry.values_,
            ts_created: entry.ts_created,
            is_enabled: entry.is_enabled,
            username: entry.username,
            metadata: entry.metadata,
        }
    }
}

impl From<api::AlertsDictsListRequest> for AlertsDictsFilter {
    fn from(request: api::AlertsDictsListRequest) -> Self {
        Self {
            name: request.name,
            key: request.key,
            is_enabled: request.is_enabled.unwrap_or(true),
        }
    }
}

/// Validate a path `id` before passing it to the database.
pub fn parse_alert_dict_id(id: &str) -> ObservabilityApiResult<String> {
    if id.trim().is_empty() {
        Err(error_stack::report!(ObservabilityError::InvalidRequest))
            .attach_printable("id must not be empty")?
    }
    Ok(id.to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use serde_json::json;

    use super::*;

    fn request() -> api::AlertsDictsCreateRequest {
        api::AlertsDictsCreateRequest {
            name: "dashboard".to_owned(),
            key: "slack_users".to_owned(),
            product: None,
            values_: json!(["alice", "bob"]),
            username: None,
            metadata: None,
        }
    }

    #[test]
    fn absent_or_null_json_fields_become_an_empty_array() {
        let new = AlertsDictsNew::try_from(api::AlertsDictsCreateRequest {
            values_: Value::Null,
            ..request()
        })
        .unwrap();

        assert_eq!(new.product, json!([]));
        assert_eq!(new.metadata, json!([]));
        assert_eq!(new.values_, json!([]));
    }

    #[test]
    fn json_is_kept_as_sent() {
        let new = AlertsDictsNew::try_from(api::AlertsDictsCreateRequest {
            product: Some(json!("a JSON-holding string")),
            values_: json!(""),
            metadata: Some(json!({"updated_by": "reliability_team"})),
            ..request()
        })
        .unwrap();

        assert_eq!(new.product, json!("a JSON-holding string"));
        assert_eq!(new.values_, json!(""));
        assert_eq!(new.metadata, json!({"updated_by": "reliability_team"}));
    }

    #[test]
    fn a_new_version_is_enabled_and_stamped_to_the_second() {
        let new = AlertsDictsNew::try_from(request()).unwrap();

        assert!(new.is_enabled);
        assert_eq!(new.ts_created.nanosecond(), 0);
    }

    #[test]
    fn an_absent_username_is_left_to_the_column_default() {
        let new = AlertsDictsNew::try_from(request()).unwrap();

        assert!(new.username.is_none());
    }

    #[test]
    fn blank_name_or_key_is_refused() {
        assert!(AlertsDictsNew::try_from(api::AlertsDictsCreateRequest {
            name: "  ".to_owned(),
            ..request()
        })
        .is_err());
        assert!(AlertsDictsNew::try_from(api::AlertsDictsCreateRequest {
            key: "  ".to_owned(),
            ..request()
        })
        .is_err());
    }

    #[test]
    fn over_long_fields_are_refused() {
        assert!(AlertsDictsNew::try_from(api::AlertsDictsCreateRequest {
            name: "a".repeat(65),
            ..request()
        })
        .is_err());
        assert!(AlertsDictsNew::try_from(api::AlertsDictsCreateRequest {
            key: "a".repeat(256),
            ..request()
        })
        .is_err());
        assert!(AlertsDictsNew::try_from(api::AlertsDictsCreateRequest {
            username: Some("a".repeat(65)),
            ..request()
        })
        .is_err());
        assert!(AlertsDictsNew::try_from(api::AlertsDictsCreateRequest {
            key: "é".repeat(255),
            ..request()
        })
        .is_ok());
    }

    #[test]
    fn the_list_filter_defaults_to_enabled_rows() {
        let filter = AlertsDictsFilter::from(api::AlertsDictsListRequest {
            name: Some("dashboard".to_owned()),
            key: None,
            is_enabled: None,
        });

        assert!(filter.is_enabled);

        let filter = AlertsDictsFilter::from(api::AlertsDictsListRequest {
            name: None,
            key: None,
            is_enabled: Some(false),
        });

        assert!(!filter.is_enabled);
    }

    #[test]
    fn a_non_empty_string_is_an_id() {
        assert!(parse_alert_dict_id("dict_123").is_ok());
        assert!(parse_alert_dict_id("").is_err());
    }

    #[test]
    fn the_response_uses_column_names_and_iso8601() {
        let entry = AlertsDicts {
            id: "dict_123".to_owned(),
            name: "dashboard".to_owned(),
            key: "slack_users".to_owned(),
            product: Some(json!([])),
            values_: Some(json!(["alice"])),
            ts_created: Some(date_time::now()),
            is_enabled: Some(true),
            username: Some("reliability_team".to_owned()),
            metadata: Some(json!({})),
        };

        let response = api::AlertsDictsResponse::from(entry);
        let serialized = serde_json::to_value(&response).unwrap();

        assert_eq!(serialized["id"], "dict_123");
        assert_eq!(serialized["key"], "slack_users");
        assert!(serialized["ts_created"]
            .as_str()
            .is_some_and(|value| value.contains('T')));
    }
}
