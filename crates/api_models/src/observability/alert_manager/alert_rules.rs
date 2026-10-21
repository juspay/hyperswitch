use common_enums::AlertRuleStatus;
use common_types::alert_rules::AlertRuleConfigV1;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleCreateRequest {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfigV1,
    pub status: Option<AlertRuleStatus>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleUpdateRequest {
    pub name: Option<String>,
    pub config: Option<AlertRuleConfigV1>,
    pub status: Option<AlertRuleStatus>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AlertRuleResponse {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfigV1,
    pub status: AlertRuleStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub next_run_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_evaluated_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct AlertRuleListResponse {
    pub rules: Vec<AlertRuleResponse>,
}
