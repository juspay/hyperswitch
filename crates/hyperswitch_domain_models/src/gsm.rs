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
    pub unified_code: Option<String>,
    pub unified_message: Option<String>,
    pub error_category: Option<common_enums::ErrorCategory>,
    pub feature_data: FeatureData,
    pub feature: common_enums::GsmFeature,
}

impl FeatureData {
    pub fn get_retry_feature_data(&self) -> Option<RetryFeatureData> {
        match self {
            Self::Retry(data) => Some(data.clone()),
        }
    }

    pub fn get_decision(&self) -> common_enums::GsmDecision {
        match self {
            Self::Retry(data) => data.decision,
        }
    }
}

impl RetryFeatureData {
    pub fn is_step_up_possible(&self) -> bool {
        self.step_up_possible
    }

    pub fn is_clear_pan_possible(&self) -> bool {
        self.clear_pan_possible
    }

    pub fn is_alternate_network_possible(&self) -> bool {
        self.alternate_network_possible
    }

    pub fn get_decision(&self) -> common_enums::GsmDecision {
        self.decision
    }
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
            status: value.status.clone(),
            router_error: value.router_error,
            decision: value.feature_data.get_decision().to_string(),
            step_up_possible: value
                .feature_data
                .get_retry_feature_data()
                .map(|retry_feature_data| retry_feature_data.is_step_up_possible())
                .unwrap_or(false),
            unified_code: value.unified_code,
            unified_message: value.unified_message,
            error_category: value.error_category,
            clear_pan_possible: value
                .feature_data
                .get_retry_feature_data()
                .map(|retry_feature_data| retry_feature_data.is_clear_pan_possible())
                .unwrap_or(false),
            feature_data: Some(Secret::new(
                serde_json::to_value(value.feature_data).change_context(
                    ValidationError::InvalidValue {
                        message: "Failed to serialize gsm feature data".to_string(),
                    },
                )?,
            )),
            feature: Some(value.feature.to_string()),
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
        let decision =
            StringExt::<common_enums::GsmDecision>::parse_enum(item.decision, "GsmDecision")
                .map_err(|_| ValidationError::InvalidValue {
                    message: "Failed to parse GsmDecision".to_string(),
                })?;
        let db_feature_data: Option<FeatureData> = item
            .feature_data
            .map(|data| {
                serde_json::from_value(data.expose()).map_err(|_| ValidationError::InvalidValue {
                    message: "Failed to deserialize gsm feature data".to_string(),
                })
            })
            .transpose()?;

        // The only case where `FeatureData` can be null is for legacy records
        // (i.e., records created before `FeatureData` and related features were introduced).
        // At that time, the only supported feature was `Retry`, so it's safe to default to it.
        let feature_data = match db_feature_data {
            Some(FeatureData::Retry(data)) => FeatureData::Retry(data),
            None => FeatureData::Retry(RetryFeatureData {
                step_up_possible: item.step_up_possible,
                clear_pan_possible: item.clear_pan_possible,
                alternate_network_possible: false,
                decision,
            }),
        };

        let feature = item
            .feature
            .map(|gsm_feature| {
                StringExt::<common_enums::GsmFeature>::parse_enum(gsm_feature, "GsmFeature")
                    .map_err(|_| ValidationError::InvalidValue {
                        message: "Failed to parse GsmFeature".to_string(),
                    })
            })
            .transpose()?
            .unwrap_or(common_enums::GsmFeature::Retry);
        Ok(Self {
            connector: item.connector,
            flow: item.flow,
            sub_flow: item.sub_flow,
            code: item.code,
            message: item.message,
            status: item.status,
            router_error: item.router_error,
            unified_code: item.unified_code,
            unified_message: item.unified_message,
            error_category: item.error_category,
            feature_data,
            feature,
        })
    }
}
