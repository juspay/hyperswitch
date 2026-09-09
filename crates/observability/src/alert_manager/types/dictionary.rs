//! The wire contract for the mappers dictionary.
//!
//! An entry crosses this boundary as the bytes the dashboard sent — see the `json` columns in
//! [`super`]'s module docs — and a read that finds nothing is a [`ReadStatus::Absent`] rather than
//! a `404`.

use diesel_models::observability::{alerts_dicts::AlertsDict, raw_json::RawJson};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};

/// One dictionary entry, as the mappers screen reads it.
///
/// Every field is always present, including the ones that are `null` — unlike the notify
/// responses, which skip what does not apply. Here a `null` is a stored fact ("this entry has no
/// product"), and a screen that iterates rows should not have to tell a missing key from a null
/// one.
#[derive(Debug, Serialize)]
pub struct DictionaryEntry {
    /// The dictionary this entry belongs to.
    pub name: String,
    /// The entry within it. Spelled `key_`, as the column and the dashboard both spell it.
    pub key_: String,
    /// Returned as the bytes that were saved. See the module docs.
    pub product: Option<Box<RawValue>>,
    /// Returned as the bytes that were saved. See the module docs.
    pub values_: Option<Box<RawValue>>,
    /// Returned as the bytes that were saved. See the module docs.
    pub metadata: Option<Box<RawValue>>,
    /// When the entry was last saved. Named for its column, which the mappers screen already
    /// displays as the entry's timestamp.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_created: Option<PrimitiveDateTime>,
    /// Who last saved it, as asserted by whoever saved it.
    pub username: Option<String>,
}

/// The body of `POST /alerts/config/dictionary`.
///
/// The whole entry every time. A partial update was rejected: an entry is one value the dashboard
/// edits as a whole, and merging fields would need a concurrency story for a screen that has no
/// way to express one.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictionaryUpsertRequest {
    /// The dictionary to save into.
    pub name: String,
    /// The entry within it.
    pub key_: String,
    /// Stored as the bytes sent. See the module docs.
    #[serde(default)]
    pub product: Option<Box<RawValue>>,
    /// Stored as the bytes sent. See the module docs.
    #[serde(default)]
    pub values_: Option<Box<RawValue>>,
    /// Stored as the bytes sent. See the module docs.
    #[serde(default)]
    pub metadata: Option<Box<RawValue>>,
}

/// What `GET /alerts/config/dictionary` returns.
#[derive(Debug, Serialize)]
pub struct DictionaryListResponse {
    /// Whether the dictionary holds anything. Always present.
    pub status: ReadStatus,
    /// Every live entry, ordered by name and then key. Never `null`; an empty dictionary is an
    /// empty list alongside `absent`.
    pub entries: Vec<DictionaryEntry>,
}

/// What `GET /alerts/config/dictionary/{name}/{key}` returns.
#[derive(Debug, Serialize)]
pub struct DictionaryReadResponse {
    /// Whether the entry exists. Always present.
    pub status: ReadStatus,
    /// The entry, or `null` when there is none.
    pub entry: Option<DictionaryEntry>,
}

/// What `POST /alerts/config/dictionary` returns.
#[derive(Debug, Serialize)]
pub struct DictionarySaveResponse {
    /// Always [`WriteStatus::Saved`]; a save that did not save is an error.
    pub status: WriteStatus,
    /// The stored entry, read back from the row that was written — so a caller can see what it
    /// saved rather than assume its own copy.
    pub entry: DictionaryEntry,
}

/// What `DELETE /alerts/config/dictionary/{name}/{key}` returns.
#[derive(Debug, Serialize)]
pub struct DictionaryDeleteResponse {
    /// [`WriteStatus::Retired`] or [`WriteStatus::Absent`]. Always present, so a caller can tell
    /// "it is gone now" from "it was already gone" — the same request answers both, which is what
    /// makes retrying one safe.
    pub status: WriteStatus,
}

impl From<AlertsDict> for DictionaryEntry {
    fn from(entry: AlertsDict) -> Self {
        Self {
            name: entry.name,
            key_: entry.key_,
            // `into_raw` moves the stored bytes onto the wire. Nothing parses and re-encodes them
            // on the way, which is the whole contract of these three columns.
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

    /// The property the `json` columns exist for. `serde_json::Value` sorts object keys, so a round
    /// trip through one hands the mappers screen a document it did not save.
    #[test]
    fn stored_json_keeps_its_key_order_and_spacing() {
        let sent = r#"{"b": 1, "a": [2, 3]}"#;

        // Deserialized from bytes, as a request arrives, rather than from a `Value` that has
        // already re-encoded them.
        let request: DictionaryUpsertRequest = serde_json::from_str(&format!(
            r#"{{"name": "dashboard", "key_": "slack_users", "values_": {sent}}}"#
        ))
        .unwrap();

        // Deserialized from the wire, stored, read back, serialized to the wire: the same bytes at
        // every hop, because nothing on the path builds a tree.
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

    /// The dashboard's double-encoded convention: some entries are a JSON *string* holding JSON,
    /// and the screen parses them twice. Re-encoding either layer breaks the second parse.
    #[test]
    fn a_json_string_holding_json_survives_as_a_string() {
        let stored = RawJson::from(raw(r#""[]""#));

        assert_eq!(stored.get(), r#""[]""#);
    }

    /// An entry is read by a screen that iterates rows, so a stored `null` must not be
    /// indistinguishable from a field nobody serialized.
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

    /// The same property `status` carries on a notification: a caller cannot read the `200` and
    /// stop looking, because "there was nothing there" is spelled out.
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
