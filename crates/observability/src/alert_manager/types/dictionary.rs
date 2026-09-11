use diesel_models::observability::{alerts_dicts::AlertsDict, raw_json::RawJson};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};

#[derive(Debug, Serialize)]
pub struct DictionaryEntry {
    pub name: String,
    pub key_: String,
    pub product: Option<Box<RawValue>>,
    pub values_: Option<Box<RawValue>>,
    pub metadata: Option<Box<RawValue>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_created: Option<PrimitiveDateTime>,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictionaryUpsertRequest {
    pub name: String,
    pub key_: String,
    #[serde(default)]
    pub product: Option<Box<RawValue>>,
    #[serde(default)]
    pub values_: Option<Box<RawValue>>,
    #[serde(default)]
    pub metadata: Option<Box<RawValue>>,
}

#[derive(Debug, Serialize)]
pub struct DictionaryListResponse {
    pub status: ReadStatus,
    pub entries: Vec<DictionaryEntry>,
}

#[derive(Debug, Serialize)]
pub struct DictionaryReadResponse {
    pub status: ReadStatus,
    pub entry: Option<DictionaryEntry>,
}

#[derive(Debug, Serialize)]
pub struct DictionarySaveResponse {
    pub status: WriteStatus,
    pub entry: DictionaryEntry,
}

#[derive(Debug, Serialize)]
pub struct DictionaryDeleteResponse {
    pub status: WriteStatus,
}

impl From<AlertsDict> for DictionaryEntry {
    fn from(entry: AlertsDict) -> Self {
        Self {
            name: entry.name,
            key_: entry.key_,
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

    #[test]
    fn stored_json_keeps_its_key_order_and_spacing() {
        let sent = r#"{"b": 1, "a": [2, 3]}"#;

        let request: DictionaryUpsertRequest = serde_json::from_str(&format!(
            r#"{{"name": "dashboard", "key_": "slack_users", "values_": {sent}}}"#
        ))
        .unwrap();

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

    #[test]
    fn a_json_string_holding_json_survives_as_a_string() {
        let stored = RawJson::from(raw(r#""[]""#));

        assert_eq!(stored.get(), r#""[]""#);
    }

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
