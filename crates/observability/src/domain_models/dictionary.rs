//! Validated alert dictionary domain models and layer conversions.

use api_models::observability::alert_manager::dictionary as api;
use diesel_models::observability::alert_manager::dictionary as storage;
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

#[derive(Clone, Debug)]
pub struct DictionaryEntryNew {
    pub name: String,
    pub key_: String,
    pub product: String,
    pub values_: String,
    pub metadata: String,
    pub updated_by: String,
}

#[derive(Clone, Debug)]
pub struct DictionaryEntry {
    pub name: String,
    pub key_: String,
    pub product: String,
    pub values_: String,
    pub metadata: String,
    pub updated_by: String,
    pub last_updated_at: PrimitiveDateTime,
}

impl DictionaryEntryNew {
    pub fn try_from_request(request: api::DictionaryUpsertRequest) -> ObservabilityApiResult<Self> {
        let name = request.name.trim().to_owned();
        let key_ = request.key_.trim().to_owned();
        if name.is_empty() || key_.is_empty() {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("name and key_ must not be empty");
        }

        Ok(Self {
            name,
            key_,
            product: request.product,
            values_: request.values_,
            metadata: request.metadata,
            updated_by: request.updated_by,
        })
    }
}

impl From<DictionaryEntryNew> for storage::DictionaryEntryNew {
    fn from(row: DictionaryEntryNew) -> Self {
        Self {
            name: row.name,
            key_: row.key_,
            product: row.product,
            values_: row.values_,
            metadata: row.metadata,
            updated_by: row.updated_by,
        }
    }
}

impl From<storage::DictionaryEntry> for DictionaryEntry {
    fn from(row: storage::DictionaryEntry) -> Self {
        Self {
            name: row.name,
            key_: row.key_,
            product: row.product,
            values_: row.values_,
            metadata: row.metadata,
            updated_by: row.updated_by,
            last_updated_at: row.last_updated_at,
        }
    }
}

impl From<DictionaryEntry> for api::DictionaryEntryResponse {
    fn from(row: DictionaryEntry) -> Self {
        Self {
            name: row.name,
            key_: row.key_,
            product: row.product,
            values_: row.values_,
            metadata: row.metadata,
            updated_by: row.updated_by,
            last_updated_at: row.last_updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_blank_keys_are_rejected() {
        let request = api::DictionaryUpsertRequest {
            name: " dashboard ".into(),
            key_: " merchant_id ".into(),
            product: "[]".into(),
            values_: "[]".into(),
            metadata: "{}".into(),
            updated_by: "dashboard".into(),
        };
        let row = DictionaryEntryNew::try_from_request(request).unwrap();
        assert_eq!(row.name, "dashboard");
        assert_eq!(row.key_, "merchant_id");

        let request = api::DictionaryUpsertRequest {
            name: "dashboard".into(),
            key_: "   ".into(),
            product: "[]".into(),
            values_: "[]".into(),
            metadata: "{}".into(),
            updated_by: "dashboard".into(),
        };
        assert!(DictionaryEntryNew::try_from_request(request).is_err());
    }
}
