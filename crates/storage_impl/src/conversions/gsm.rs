//! Conversion implementations for GSM (Gateway Status Map)

use common_utils::{errors::ValidationError, ext_traits::StringExt};
use hyperswitch_domain_models::gsm::{GatewayStatusMap, GatewayStatusMappingUpdate};

use crate::transformers::ForeignTryFrom;

impl ForeignTryFrom<GatewayStatusMap> for diesel_models::gsm::GatewayStatusMappingNew {
    type Error = error_stack::Report<ValidationError>;

    fn foreign_try_from(from: GatewayStatusMap) -> error_stack::Result<Self, Self::Error> {
        Ok(Self {
            connector: from.connector.to_string(),
            flow: from.flow,
            sub_flow: from.sub_flow,
            code: from.code,
            message: from.message,
            status: from.status.clone(),
            router_error: from.router_error,
            decision: from.feature_data.get_decision().to_string(),
            step_up_possible: from
                .feature_data
                .get_retry_feature_data()
                .map(|retry_feature_data| retry_feature_data.is_step_up_possible())
                .unwrap_or(false),
            unified_code: from.unified_code,
            unified_message: from.unified_message,
            error_category: from.error_category,
            clear_pan_possible: from
                .feature_data
                .get_retry_feature_data()
                .map(|retry_feature_data| retry_feature_data.is_clear_pan_possible())
                .unwrap_or(false),
            feature_data: Some(from.feature_data),
            feature: Some(from.feature),
            standardised_code: from.standardised_code,
            description: from.description,
            user_guidance_message: from.user_guidance_message,
        })
    }
}

impl ForeignTryFrom<GatewayStatusMappingUpdate> for diesel_models::gsm::GatewayStatusMappingUpdate {
    type Error = error_stack::Report<ValidationError>;

    fn foreign_try_from(
        from: GatewayStatusMappingUpdate,
    ) -> error_stack::Result<Self, Self::Error> {
        Ok(Self {
            status: from.status,
            router_error: from.router_error,
            decision: from.decision.map(|gsm_decision| gsm_decision.to_string()),
            step_up_possible: from.step_up_possible,
            unified_code: from.unified_code,
            unified_message: from.unified_message,
            error_category: from.error_category,
            clear_pan_possible: from.clear_pan_possible,
            feature_data: from.feature_data,
            feature: from.feature,
            standardised_code: from.standardised_code,
            description: from.description,
            user_guidance_message: from.user_guidance_message,
        })
    }
}

impl ForeignTryFrom<diesel_models::gsm::GatewayStatusMap> for GatewayStatusMap {
    type Error = ValidationError;

    fn foreign_try_from(
        item: diesel_models::gsm::GatewayStatusMap,
    ) -> error_stack::Result<Self, Self::Error> {
        let decision =
            common_utils::ext_traits::StringExt::<common_enums::GsmDecision>::parse_enum(
                item.decision,
                "GsmDecision",
            )
            .map_err(|_| ValidationError::InvalidValue {
                message: "Failed to parse GsmDecision".to_string(),
            })?;
        let db_feature_data = item.feature_data;

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
            standardised_code: item.standardised_code,
            description: item.description,
            user_guidance_message: item.user_guidance_message,
        })
    }
}

impl ForeignTryFrom<diesel_models::gsm::GatewayStatusMappingUpdate> for GatewayStatusMappingUpdate {
    type Error = ValidationError;

    fn foreign_try_from(
        from: diesel_models::gsm::GatewayStatusMappingUpdate,
    ) -> error_stack::Result<Self, Self::Error> {
        let decision = from.decision.and_then(|d| {
            common_utils::ext_traits::StringExt::<common_enums::GsmDecision>::parse_enum(
                d,
                "GsmDecision",
            )
            .ok()
        });

        Ok(Self {
            status: from.status,
            router_error: from.router_error,
            decision,
            step_up_possible: from.step_up_possible,
            unified_code: from.unified_code,
            unified_message: from.unified_message,
            error_category: from.error_category,
            clear_pan_possible: from.clear_pan_possible,
            feature_data: from.feature_data,
            feature: from.feature,
            standardised_code: from.standardised_code,
            description: from.description,
            user_guidance_message: from.user_guidance_message,
        })
    }
}
