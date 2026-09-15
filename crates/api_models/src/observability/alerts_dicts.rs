//! The mappers dictionary, as the observability service's `/alerts/alerts_manager/dicts` routes
//! accept and return it.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::PrimitiveDateTime;

/// The body of `POST /alerts/alerts_manager/dicts`.
///
/// `values_` is not `Option`, so a missing key fails to parse while `null` parses and is stored as
/// an empty array.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertsDictsCreateRequest {
    pub name: String,
    pub key_: String,
    pub product: Option<Value>,
    pub values_: Value,
    pub username: Option<String>,
    pub metadata: Option<Value>,
}

/// The query of `GET /alerts/alerts_manager/dicts`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertsDictsListRequest {
    pub name: Option<String>,
    pub key_: Option<String>,
    pub is_enabled: Option<bool>,
}

/// Built from the `{id}` path of `GET /alerts/alerts_manager/dicts/{id}`.
#[derive(Clone, Debug)]
pub struct AlertsDictsRetrieveRequest {
    pub id: String,
}

/// Built from the `{id}` path of `DELETE /alerts/alerts_manager/dicts/{id}`.
#[derive(Clone, Debug)]
pub struct AlertsDictsDeleteRequest {
    pub id: String,
}

/// One stored dictionary entry.
#[derive(Clone, Debug, Serialize)]
pub struct AlertsDictsResponse {
    pub id: String,
    pub name: String,
    pub key_: String,
    pub product: Option<Value>,
    pub values_: Option<Value>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_created: Option<PrimitiveDateTime>,
    pub is_enabled: Option<bool>,
    pub username: Option<String>,
    pub metadata: Option<Value>,
}

/// The body of `GET /alerts/alerts_manager/dicts`.
#[derive(Clone, Debug, Serialize)]
pub struct AlertsDictsListResponse {
    pub count: usize,
    pub data: Vec<AlertsDictsResponse>,
}

/// The body of `DELETE /alerts/alerts_manager/dicts/{id}`.
#[derive(Clone, Debug, Serialize)]
pub struct AlertsDictsDeleteResponse {
    pub id: String,
    pub deleted: bool,
}
