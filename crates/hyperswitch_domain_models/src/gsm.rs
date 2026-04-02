use serde::{self, Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GatewayStatusMap {
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub code: String,
    pub message: String,
    pub status: String,
    pub router_error: Option<String>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub error_category: Option<common_enums::ErrorCategory>,
    pub feature_data: common_types::domain::GsmFeatureData,
    pub feature: common_enums::GsmFeature,
    pub standardised_code: Option<common_enums::StandardisedCode>,
    pub description: Option<String>,
    pub user_guidance_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GatewayStatusMappingUpdate {
    pub status: Option<String>,
    pub router_error: Option<Option<String>>,
    pub decision: Option<common_enums::GsmDecision>,
    pub step_up_possible: Option<bool>,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub error_category: Option<common_enums::ErrorCategory>,
    pub clear_pan_possible: Option<bool>,
    pub feature_data: Option<common_types::domain::GsmFeatureData>,
    pub feature: Option<common_enums::GsmFeature>,
    pub standardised_code: Option<common_enums::StandardisedCode>,
    pub description: Option<String>,
    pub user_guidance_message: Option<String>,
}
