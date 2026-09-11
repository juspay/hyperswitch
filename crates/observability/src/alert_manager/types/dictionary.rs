//! The wire contract for the mappers dictionary.

use diesel_models::observability::{alerts_dicts::AlertsDict, raw_json::RawJson};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};

/// One dictionary entry, as the mappers screen reads it.
#[derive(Debug, Serialize)]
pub struct DictionaryEntry {
    /// The dictionary this entry belongs to.
    pub name: String,
    /// The entry within it.
    pub key_: String,
    /// Returned as the bytes that were saved.
    pub product: Option<Box<RawValue>>,
    /// Returned as the bytes that were saved.
    pub values_: Option<Box<RawValue>>,
    /// Returned as the bytes that were saved.
    pub metadata: Option<Box<RawValue>>,
    /// When the entry was last saved.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_created: Option<PrimitiveDateTime>,
    /// Who last saved it, as asserted by whoever saved it.
    pub username: Option<String>,
}

/// The body of `POST /alerts/config/dictionary`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictionaryUpsertRequest {
    /// The dictionary to save into.
    pub name: String,
    /// The entry within it.
    pub key_: String,
    /// Stored as the bytes sent.
    #[serde(default)]
    pub product: Option<Box<RawValue>>,
    /// Stored as the bytes sent.
    #[serde(default)]
    pub values_: Option<Box<RawValue>>,
    /// Stored as the bytes sent.
    #[serde(default)]
    pub metadata: Option<Box<RawValue>>,
}

/// What `GET /alerts/config/dictionary` returns.
#[derive(Debug, Serialize)]
pub struct DictionaryListResponse {
    /// Whether the dictionary holds anything.
    pub status: ReadStatus,
    /// Every live entry, ordered by name and then key.
    pub entries: Vec<DictionaryEntry>,
}

/// What `GET /alerts/config/dictionary/{name}/{key}` returns.
#[derive(Debug, Serialize)]
pub struct DictionaryReadResponse {
    /// Whether the entry exists.
    pub status: ReadStatus,
    /// The entry, or `null` when there is none.
    pub entry: Option<DictionaryEntry>,
}

/// What `POST /alerts/config/dictionary` returns.
#[derive(Debug, Serialize)]
pub struct DictionarySaveResponse {
    /// Always [`WriteStatus::Saved`]; a save that did not save is an error.
    pub status: WriteStatus,
    /// The stored entry, read back from the row that was written — so a caller can see what it saved rather than assume its own copy.
    pub entry: DictionaryEntry,
}

/// What `DELETE /alerts/config/dictionary/{name}/{key}` returns.
#[derive(Debug, Serialize)]
pub struct DictionaryDeleteResponse {
    /// [`WriteStatus::Retired`] or [`WriteStatus::Absent`].
    pub status: WriteStatus,
}

impl From<AlertsDict> for DictionaryEntry {
    fn from(entry: AlertsDict) -> Self {
        Self {
            name: entry.name,
            key_: entry.key_,
            // `into_raw` moves the stored bytes onto the wire.
            product: entry.product.map(RawJson::into_raw),
            values_: entry.values_.map(RawJson::into_raw),
            metadata: entry.metadata.map(RawJson::into_raw),
            ts_created: entry.ts_created,
            username: entry.username,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn body_of<T: Serialize>(value: &T) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    fn raw(json: &str) -> Box<RawValue> {
        RawValue::from_string(json.to_owned()).unwrap()
    }

    /// The property the `json` columns exist for.
    #[test]
    fn stored_json_keeps_its_key_order_and_spacing() {
        let sent = r#"{"b": 1, "a": [2, 3]}"#;

        // Deserialized from bytes, as a request arrives, rather than from a `Value` that has already re-encoded them.
        let request: DictionaryUpsertRequest = serde_json::from_str(&format!(
            r#"{{"name": "dashboard", "key_": "slack_users", "values_": {sent}}}"#
        ))
        .unwrap();

        // Deserialized from the wire, stored, read back, serialized to the wire:
        let stored = RawJson::from(request.values_.unwrap());
        assert_eq!(stored.get(), sent);

        let entry = DictionaryEntry {
            name: "dashboard".to_owned(),
            key_: "slack_users".to_owned(),
            product: None,
            values_: Some(stored.into_raw()),
            metadata: None,
            ts_created: None,
            username: None,
        };
        assert_eq!(
            serde_json::to_value(&entry).unwrap()["values_"].to_string(),
            r#"{"b":1,"a":[2,3]}"#
        );
    }

    /// The dashboard's double-encoded convention:
    #[test]
    fn a_json_string_holding_json_survives_as_a_string() {
        let stored = RawJson::from(raw(r#""[]""#));

        assert_eq!(stored.get(), r#""[]""#);
    }

    /// An entry is read by a screen that iterates rows, so a stored `null` must not be indistinguishable from a field nobody serialized.
    #[test]
    fn an_entry_carries_every_field_even_when_the_columns_are_null() {
        let body = body_of(&DictionaryEntry {
            name: "dashboard".to_owned(),
            key_: "slack_users".to_owned(),
            product: None,
            values_: None,
            metadata: None,
            ts_created: None,
            username: None,
        });

        for field in ["product", "values_", "metadata", "ts_created", "username"] {
            assert!(
                body.get(field).is_some_and(serde_json::Value::is_null),
                "{field} was omitted"
            );
        }
    }

    /// The same property `status` carries on a notification:
    #[test]
    fn an_empty_dictionary_says_so_rather_than_returning_an_empty_body() {
        let body = body_of(&DictionaryListResponse {
            status: ReadStatus::Absent,
            entries: Vec::new(),
        });

        assert_eq!(body["status"], "absent");
        assert_eq!(body["entries"], serde_json::json!([]));
    }

    #[test]
    fn a_retired_entry_and_one_that_was_never_there_report_different_statuses() {
        assert_eq!(
            body_of(&DictionaryDeleteResponse {
                status: WriteStatus::Retired
            })["status"],
            "retired"
        );
        assert_eq!(
            body_of(&DictionaryDeleteResponse {
                status: WriteStatus::Absent
            })["status"],
            "absent"
        );
    }
}
