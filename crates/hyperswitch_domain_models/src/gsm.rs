use common_utils::{errors::ValidationError, ext_traits::StringExt};
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
            feature_data: Some(value.feature_data),
            feature: Some(value.feature),
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
            feature_data: value.feature_data,
            feature: value.feature,
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
        let db_feature_data = item.feature_data;

        // The only case where `FeatureData` can be null is for legacy records
        // (i.e., records created before `FeatureData` and related features were introduced).
        // At that time, the only supported feature was `Retry`, so it's safe to default to it.
        let feature_data = match db_feature_data {
            Some(common_types::domain::GsmFeatureData::Retry(data)) => {
                common_types::domain::GsmFeatureData::Retry(data)
            }
            None => common_types::domain::GsmFeatureData::Retry(
                common_types::domain::RetryFeatureData {
                    step_up_possible: item.step_up_possible,
                    clear_pan_possible: item.clear_pan_possible,
                    alternate_network_possible: false,
                    decision,
                },
            ),
        };

        let feature = item.feature.unwrap_or(common_enums::GsmFeature::Retry);
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
