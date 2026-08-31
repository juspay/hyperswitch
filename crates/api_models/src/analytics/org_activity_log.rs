use common_utils::pii;
use hyperswitch_masking::Secret;

use super::TimeRange;

pub const ORG_ACTIVITY_LOG_DEFAULT_LIMIT: u64 = 20;
pub const ORG_ACTIVITY_LOG_MAX_LIMIT: u64 = 100;

fn default_activity_log_limit() -> u64 {
    ORG_ACTIVITY_LOG_DEFAULT_LIMIT
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgActivityLogRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "default_activity_log_limit")]
    pub limit: u64,
    #[serde(default)]
    pub user_ids: Option<Vec<String>>,
    #[serde(default)]
    pub api_flows: Option<Vec<String>>,
    #[serde(default)]
    pub merchant_ids: Option<Vec<common_utils::id_type::MerchantId>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgActivityLogResponse {
    pub total_count: u64,
    pub activity_logs: Vec<OrgActivityLogEntry>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgActivityLogEntry {
    pub user_id: Option<String>,
    pub user_email: Option<pii::Email>,
    pub user_name: Option<Secret<String>>,
    pub api_flow: String,
    pub flow_type: String,
    pub url_path: Option<String>,
    pub http_method: Option<String>,
    pub status_code: u16,
    pub merchant_id: Option<common_utils::id_type::MerchantId>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgActivityLogFilterValues {
    pub api_flows: Vec<String>,
}
