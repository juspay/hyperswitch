use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotificationReadsRetrieveRequest {
    pub user_name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotificationReadsUpsertRequest {
    pub user_name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct NotificationReadsResponse {
    pub user_name: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_read_at: PrimitiveDateTime,
}
