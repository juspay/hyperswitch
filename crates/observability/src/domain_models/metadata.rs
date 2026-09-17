//! Validated per-alert metadata models and layer conversions.

use api_models::observability::alert_manager::metadata as api;
use diesel_models::observability::alert_manager::metadata as storage;
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

#[derive(Clone, Debug)]
pub struct AlertMetadataPatch {
    pub id: String,
    pub metadata: Option<String>,
    pub snooze: Option<String>,
    pub updated_by: String,
}

#[derive(Clone, Debug)]
pub struct AlertMetadataEntry {
    pub id: String,
    pub metadata: String,
    pub snooze: String,
    pub updated_by: String,
    pub last_updated_at: PrimitiveDateTime,
}

impl AlertMetadataPatch {
    pub fn try_from_request(
        id: String,
        request: api::AlertMetadataPatchRequest,
    ) -> ObservabilityApiResult<Self> {
        let id = id.trim().to_owned();
        if id.is_empty() {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("id must not be empty");
        }
        let updated_by = request.updated_by.trim().to_owned();
        if updated_by.is_empty() {
            return Err(report!(ObservabilityError::InvalidRequest))
                .attach_printable("updated_by must not be blank");
        }

        Ok(Self {
            id,
            metadata: request.metadata,
            snooze: request.snooze,
            updated_by,
        })
    }

    pub fn into_storage(self) -> (storage::AlertMetadataNew, storage::AlertMetadataChangeset) {
        let new = storage::AlertMetadataNew {
            id: self.id,
            metadata: self.metadata.clone().unwrap_or_else(|| "{}".to_owned()),
            snooze: self.snooze.clone().unwrap_or_default(),
            updated_by: self.updated_by.clone(),
        };
        let changeset = storage::AlertMetadataChangeset {
            metadata: self.metadata,
            snooze: self.snooze,
            updated_by: self.updated_by,
        };
        (new, changeset)
    }
}

impl From<storage::AlertMetadataEntry> for AlertMetadataEntry {
    fn from(row: storage::AlertMetadataEntry) -> Self {
        Self {
            id: row.id,
            metadata: row.metadata,
            snooze: row.snooze,
            updated_by: row.updated_by,
            last_updated_at: row.last_updated_at,
        }
    }
}

impl From<AlertMetadataEntry> for api::AlertMetadataEntryResponse {
    fn from(row: AlertMetadataEntry) -> Self {
        Self {
            id: row.id,
            metadata: row.metadata,
            snooze: row.snooze,
            updated_by: row.updated_by,
            last_updated_at: row.last_updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_trimmed_and_blank_id_is_rejected() {
        let request = api::AlertMetadataPatchRequest {
            metadata: Some("{}".into()),
            snooze: None,
            updated_by: "dashboard".into(),
        };
        let patch = AlertMetadataPatch::try_from_request(" alert_1 ".into(), request).unwrap();
        assert_eq!(patch.id, "alert_1");

        let request = api::AlertMetadataPatchRequest {
            metadata: None,
            snooze: None,
            updated_by: "dashboard".into(),
        };
        assert!(AlertMetadataPatch::try_from_request("   ".into(), request).is_err());

        let request = api::AlertMetadataPatchRequest {
            metadata: None,
            snooze: None,
            updated_by: "   ".into(),
        };
        assert!(AlertMetadataPatch::try_from_request("alert_1".into(), request).is_err());
    }

    #[test]
    fn omitted_fields_receive_insert_defaults_without_entering_changeset() {
        let request = api::AlertMetadataPatchRequest {
            metadata: None,
            snooze: Some("{\"until\":\"tomorrow\"}".into()),
            updated_by: "dashboard".into(),
        };
        let patch = AlertMetadataPatch::try_from_request("alert_1".into(), request).unwrap();
        let (new, changeset) = patch.into_storage();
        assert_eq!(new.metadata, "{}");
        assert_eq!(new.snooze, "{\"until\":\"tomorrow\"}");
        assert_eq!(changeset.metadata, None);
    }
}
