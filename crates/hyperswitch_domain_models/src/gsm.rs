use common_utils::{errors::ValidationError, ext_traits::StringExt};
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
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
    pub decision: common_enums::GsmDecision,
    pub step_up_possible: bool,
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub error_category: Option<common_enums::ErrorCategory>,
    pub clear_pan_possible: bool,
    pub feature_data: Option<FeatureData>,
    pub feature: Option<common_enums::GsmFeature>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureData {
    Retry(RetryFeatureData),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RetryFeatureData {
    pub step_up_possible: bool,
    pub clear_pan_possible: bool,
    pub alternate_network_possible: bool,
    pub decision: common_enums::GsmDecision,
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
    pub feature_data: Option<FeatureData>,
    pub feature: Option<common_enums::GsmFeature>,
}

impl TryFrom<GatewayStatusMap> for diesel_models::gsm::GatewayStatusMappingNew {
    type Error = error_stack::Report<ValidationError>;

    fn try_from(value: GatewayStatusMap) -> Result<Self, Self::Error> {
        Ok(Self {
            connector: value.connector.to_string(),
            flow: value.flow,
            sub_flow: value.sub_flow,
            code: value.code,
            message: value.message,
            status: value.status,
            router_error: value.router_error,
            decision: value.decision.to_string(),
            step_up_possible: value.step_up_possible,
            unified_code: value.unified_code,
            unified_message: value.unified_message,
            error_category: value.error_category,
            clear_pan_possible: value.clear_pan_possible,
            feature_data: value
                .feature_data
                .map(|data| {
                    serde_json::to_value(data).change_context(ValidationError::InvalidValue {
                        message: "Failed to serialize gsm feature data".to_string(),
                    })
                })
                .transpose()?
                .map(Secret::new),
            feature: value.feature.map(|gsm_feature| gsm_feature.to_string()),
        })
    }
}

impl TryFrom<GatewayStatusMappingUpdate> for diesel_models::gsm::GatewayStatusMappingUpdate {
    type Error = error_stack::Report<ValidationError>;

    fn try_from(value: GatewayStatusMappingUpdate) -> Result<Self, Self::Error> {
        Ok(Self {
            status: value.status,
            router_error: value.router_error,
            decision: value.decision.map(|gsm_decision| gsm_decision.to_string()),
            step_up_possible: value.step_up_possible,
            unified_code: value.unified_code,
            unified_message: value.unified_message,
            error_category: value.error_category,
            clear_pan_possible: value.clear_pan_possible,
            feature_data: value
                .feature_data
                .map(|data| {
                    serde_json::to_value(data).change_context(ValidationError::InvalidValue {
                        message: "Failed to serialize gsm feature data".to_string(),
                    })
                })
                .transpose()?
                .map(Secret::new),
            feature: value.feature.map(|gsm_feature| gsm_feature.to_string()),
        })
    }
}

impl TryFrom<diesel_models::gsm::GatewayStatusMap> for GatewayStatusMap {
    type Error = ValidationError;

    fn try_from(item: diesel_models::gsm::GatewayStatusMap) -> Result<Self, Self::Error> {
        Ok(Self {
            connector: item.connector,
            flow: item.flow,
            sub_flow: item.sub_flow,
            code: item.code,
            message: item.message,
            status: item.status,
            router_error: item.router_error,
            decision: StringExt::<common_enums::GsmDecision>::parse_enum(
                item.decision,
                "GsmDecision",
            )
            .map_err(|_| ValidationError::InvalidValue {
                message: "Failed to parse GsmDecision".to_string(),
            })?,
            step_up_possible: item.step_up_possible,
            unified_code: item.unified_code,
            unified_message: item.unified_message,
            error_category: item.error_category,
            clear_pan_possible: item.clear_pan_possible,
            feature_data: item
                .feature_data
                .map(|data| {
                    serde_json::from_value(data.expose()).map_err(|_| {
                        ValidationError::InvalidValue {
                            message: "Failed to deserialize gsm feature data".to_string(),
                        }
                    })
                })
                .transpose()?,
            feature: item
                .feature
                .map(|gsm_feature| {
                    StringExt::<common_enums::GsmFeature>::parse_enum(gsm_feature, "GsmFeature")
                        .map_err(|_| ValidationError::InvalidValue {
                            message: "Failed to parse GsmFeature".to_string(),
                        })
                })
                .transpose()?,
        })
    }
}
