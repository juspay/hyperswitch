use api_models::routing::{
    MerchantRoutingAlgorithm, RoutableConnectorChoice, RoutingAlgorithm as Algorithm,
    RoutingAlgorithmKind, RoutingDictionaryRecord,
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::open_router::{OpenRouterDecideGatewayRequest, PaymentInfo, RankingAlgorithm};
use common_utils::ext_traits::ValueExt;
use euclid::frontend::ast as dsl_ast;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use hyperswitch_domain_models::{
     payments::payment_attempt::PaymentAttempt,
};
use api_models::{
    routing as routing_types,
};
use storage_impl::routing_algorithm::{common_enums, storage_models as storage_enums};

pub trait ForeignFrom<F>: Sized {
    fn foreign_from(from: F) -> Self;
}

pub trait ForeignInto<T>: Sized {
    fn foreign_into(self) -> T;
}

impl<F, T> ForeignInto<T> for F
where
    T: ForeignFrom<F>,
{
    fn foreign_into(self) -> T {
        T::foreign_from(self)
    }
}

pub trait ForeignTryFrom<F>: Sized {
    type Error;
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

pub trait ForeignTryInto<T>: Sized {
    type Error;
    fn foreign_try_into(self) -> Result<T, Self::Error>;
}

impl<F, T> ForeignTryInto<T> for F
where
    T: ForeignTryFrom<F>,
{
    type Error = <T as ForeignTryFrom<F>>::Error;

    fn foreign_try_into(self) -> Result<T, Self::Error> {
        T::foreign_try_from(self)
    }
}

impl ForeignFrom<RoutableConnectorChoice> for dsl_ast::ConnectorChoice {
    fn foreign_from(from: RoutableConnectorChoice) -> Self {
        Self {
            connector: from.connector,
        }
    }
}
impl ForeignFrom<common_enums::CaptureMethod> for Option<common_enums::enums::CaptureMethod> {
    fn foreign_from(value: common_enums::CaptureMethod) -> Self {
        match value {
            common_enums::CaptureMethod::Automatic => Some(common_enums::enums::CaptureMethod::Automatic),
            common_enums::CaptureMethod::SequentialAutomatic => {
                Some(common_enums::enums::CaptureMethod::SequentialAutomatic)
            }
            common_enums::CaptureMethod::Manual => Some(common_enums::enums::CaptureMethod::Manual),
            _ => None,
        }
    }
}
impl ForeignFrom<storage_enums::RoutingProfileMetadata> for RoutingDictionaryRecord {
    fn foreign_from(value: storage_enums::RoutingProfileMetadata) -> Self {
        Self {
            id: value.algorithm_id,

            profile_id: value.profile_id,
            name: value.name,
            kind: value.kind.foreign_into(),
            description: value.description.unwrap_or_default(),
            created_at: value.created_at.assume_utc().unix_timestamp(),
            modified_at: value.modified_at.assume_utc().unix_timestamp(),
            algorithm_for: Some(value.algorithm_for),
            decision_engine_routing_id: None,
        }
    }
}

impl ForeignFrom<storage_enums::RoutingAlgorithm> for RoutingDictionaryRecord {
    fn foreign_from(value: storage_enums::RoutingAlgorithm) -> Self {
        Self {
            id: value.algorithm_id,

            profile_id: value.profile_id,
            name: value.name,
            kind: value.kind.foreign_into(),
            description: value.description.unwrap_or_default(),
            created_at: value.created_at.assume_utc().unix_timestamp(),
            modified_at: value.modified_at.assume_utc().unix_timestamp(),
            algorithm_for: Some(value.algorithm_for),
            decision_engine_routing_id: value.decision_engine_routing_id,
        }
    }
}

use common_utils::errors::ParsingError as CommonParsingError; // Import for clarity

impl ForeignTryFrom<storage_enums::RoutingAlgorithm> for MerchantRoutingAlgorithm {
    type Error = error_stack::Report<CommonParsingError>; // Use ParsingError from common_utils

    fn foreign_try_from(value: storage_enums::RoutingAlgorithm) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.algorithm_id,
            name: value.name,

            profile_id: value.profile_id,
            description: value.description.unwrap_or_default(),
            algorithm: value
                .algorithm_data
                .parse_value::<Algorithm>("RoutingAlgorithm")?,
            created_at: value.created_at.assume_utc().unix_timestamp(),
            modified_at: value.modified_at.assume_utc().unix_timestamp(),
            algorithm_for: value.algorithm_for,
        })
    }
}

impl ForeignFrom<common_enums::RoutingAlgorithmKind> for RoutingAlgorithmKind {
    fn foreign_from(value: common_enums::RoutingAlgorithmKind) -> Self {
        match value {
            common_enums::RoutingAlgorithmKind::Single => Self::Single,
            common_enums::RoutingAlgorithmKind::Priority => Self::Priority,
            common_enums::RoutingAlgorithmKind::VolumeSplit => Self::VolumeSplit,
            common_enums::RoutingAlgorithmKind::Advanced => Self::Advanced,
            common_enums::RoutingAlgorithmKind::Dynamic => Self::Dynamic,
        }
    }
}

impl ForeignFrom<RoutingAlgorithmKind> for common_enums::RoutingAlgorithmKind {
    fn foreign_from(value: RoutingAlgorithmKind) -> Self {
        match value {
            RoutingAlgorithmKind::Single => Self::Single,
            RoutingAlgorithmKind::Priority => Self::Priority,
            RoutingAlgorithmKind::VolumeSplit => Self::VolumeSplit,
            RoutingAlgorithmKind::Advanced => Self::Advanced,
            RoutingAlgorithmKind::Dynamic => Self::Dynamic,
        }
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub trait OpenRouterDecideGatewayRequestExt {
    fn construct_sr_request(
        attempt: &PaymentAttempt,
        eligible_gateway_list: Vec<RoutableConnectorChoice>,
        ranking_algorithm: Option<RankingAlgorithm>,
        is_elimination_enabled: bool,
    ) -> Self
    where
        Self: Sized;
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
impl OpenRouterDecideGatewayRequestExt for OpenRouterDecideGatewayRequest {
    fn construct_sr_request(
        attempt: &PaymentAttempt,
        eligible_gateway_list: Vec<RoutableConnectorChoice>,
        ranking_algorithm: Option<RankingAlgorithm>,
        is_elimination_enabled: bool,
    ) -> Self {
        Self {
            payment_info: PaymentInfo {
                payment_id: attempt.payment_id.clone(),
                amount: attempt.net_amount.get_order_amount(),
                currency: attempt.currency.unwrap_or(common_enums::enums::Currency::USD),
                payment_type: "ORDER_PAYMENT".to_string(),
                // payment_method_type: attempt.payment_method_type.clone().unwrap(),
                payment_method_type: "UPI".into(), // TODO: once open-router makes this field string, we can send from attempt
                payment_method: attempt.payment_method.unwrap_or_default(),
            },
            merchant_id: attempt.profile_id.clone(),
            eligible_gateway_list: Some(
                eligible_gateway_list
                    .into_iter()
                    .map(|connector| connector.to_string())
                    .collect(),
            ),
            ranking_algorithm,
            elimination_enabled: Some(is_elimination_enabled),
        }
    }
}

// #[cfg(feature = "v1")]
// impl ForeignTryFrom<merchant_connector_account::MerchantConnectorAccount>
//     for api_models::admin::MerchantConnectorResponse
// {
//     type Error = error_stack::Report<errors::ApiErrorResponse>;
//     fn foreign_try_from(
//         item: merchant_connector_account::MerchantConnectorAccount,
//     ) -> Result<Self, Self::Error> {
//         let payment_methods_enabled = match item.payment_methods_enabled.clone() {
//             Some(secret_val) => {
//                 let val = secret_val
//                     .into_iter()
//                     .map(|secret| secret.expose())
//                     .collect();
//                 serde_json::Value::Array(val)
//                     .parse_value("PaymentMethods")
//                     .change_context(errors::ApiErrorResponse::InternalServerError)?
//             }
//             None => None,
//         };
//         let frm_configs = match item.frm_configs {
//             Some(ref frm_value) => {
//                 let configs_for_frm : Vec<api_models::admin::FrmConfigs> = frm_value
//                     .iter()
//                     .map(|config| { config
//                         .peek()
//                         .clone()
//                         .parse_value("FrmConfigs")
//                         .change_context(errors::ApiErrorResponse::InvalidDataFormat {
//                             field_name: "frm_configs".to_string(),
//                             expected_format: r#"[{ "gateway": "stripe", "payment_methods": [{ "payment_method": "card","payment_method_types": [{"payment_method_type": "credit","card_networks": ["Visa"],"flow": "pre","action": "cancel_txn"}]}]}]"#.to_string(),
//                         })
//                     })
//                     .collect::<Result<Vec<_>, _>>()?;
//                 Some(configs_for_frm)
//             }
//             None => None,
//         };
//         // parse the connector_account_details into ConnectorAuthType
//         let connector_account_details: hyperswitch_domain_models::router_data::ConnectorAuthType =
//             item.connector_account_details
//                 .clone()
//                 .into_inner()
//                 .parse_value("ConnectorAuthType")
//                 .change_context(errors::ApiErrorResponse::InternalServerError)
//                 .attach_printable("Failed while parsing value for ConnectorAuthType")?;
//         // get the masked keys from the ConnectorAuthType and encode it to secret value
//         let masked_connector_account_details = Secret::new(
//             connector_account_details
//                 .get_masked_keys()
//                 .encode_to_value()
//                 .change_context(errors::ApiErrorResponse::InternalServerError)
//                 .attach_printable("Failed to encode ConnectorAuthType")?,
//         );
//         #[cfg(feature = "v2")]
//         let response = Self {
//             id: item.get_id(),
//             connector_type: item.connector_type,
//             connector_name: item.connector_name,
//             connector_label: item.connector_label,
//             connector_account_details: masked_connector_account_details,
//             disabled: item.disabled,
//             payment_methods_enabled,
//             metadata: item.metadata,
//             frm_configs,
//             connector_webhook_details: item
//                 .connector_webhook_details
//                 .map(|webhook_details| {
//                     serde_json::Value::parse_value(
//                         webhook_details.expose(),
//                         "MerchantConnectorWebhookDetails",
//                     )
//                     .attach_printable("Unable to deserialize connector_webhook_details")
//                     .change_context(errors::ApiErrorResponse::InternalServerError)
//                 })
//                 .transpose()?,
//             profile_id: item.profile_id,
//             applepay_verified_domains: item.applepay_verified_domains,
//             pm_auth_config: item.pm_auth_config,
//             status: item.status,
//             additional_merchant_data: item
//                 .additional_merchant_data
//                 .map(|data| {
//                     let data = data.into_inner();
//                     serde_json::Value::parse_value::<router_types::AdditionalMerchantData>(
//                         data.expose(),
//                         "AdditionalMerchantData",
//                     )
//                     .attach_printable("Unable to deserialize additional_merchant_data")
//                     .change_context(errors::ApiErrorResponse::InternalServerError)
//                 })
//                 .transpose()?
//                 .map(api_models::admin::AdditionalMerchantData::foreign_from),
//             connector_wallets_details: item
//                 .connector_wallets_details
//                 .map(|data| {
//                     data.into_inner()
//                         .expose()
//                         .parse_value::<api_models::admin::ConnectorWalletDetails>(
//                             "ConnectorWalletDetails",
//                         )
//                         .attach_printable("Unable to deserialize connector_wallets_details")
//                         .change_context(errors::ApiErrorResponse::InternalServerError)
//                 })
//                 .transpose()?,
//         };
//         #[cfg(feature = "v1")]
//         let response = Self {
//             connector_type: item.connector_type,
//             connector_name: item.connector_name,
//             connector_label: item.connector_label,
//             merchant_connector_id: item.merchant_connector_id,
//             connector_account_details: masked_connector_account_details,
//             test_mode: item.test_mode,
//             disabled: item.disabled,
//             payment_methods_enabled,
//             metadata: item.metadata,
//             business_country: item.business_country,
//             business_label: item.business_label,
//             business_sub_label: item.business_sub_label,
//             frm_configs,
//             connector_webhook_details: item
//                 .connector_webhook_details
//                 .map(|webhook_details| {
//                     serde_json::Value::parse_value(
//                         webhook_details.expose(),
//                         "MerchantConnectorWebhookDetails",
//                     )
//                     .attach_printable("Unable to deserialize connector_webhook_details")
//                     .change_context(errors::ApiErrorResponse::InternalServerError)
//                 })
//                 .transpose()?,
//             profile_id: item.profile_id,
//             applepay_verified_domains: item.applepay_verified_domains,
//             pm_auth_config: item.pm_auth_config,
//             status: item.status,
//             additional_merchant_data: item
//                 .additional_merchant_data
//                 .map(|data| {
//                     let data = data.into_inner();
//                     serde_json::Value::parse_value::<router_types::AdditionalMerchantData>(
//                         data.expose(),
//                         "AdditionalMerchantData",
//                     )
//                     .attach_printable("Unable to deserialize additional_merchant_data")
//                     .change_context(errors::ApiErrorResponse::InternalServerError)
//                 })
//                 .transpose()?
//                 .map(api_models::admin::AdditionalMerchantData::foreign_from),
//             connector_wallets_details: item
//                 .connector_wallets_details
//                 .map(|data| {
//                     data.into_inner()
//                         .expose()
//                         .parse_value::<api_models::admin::ConnectorWalletDetails>(
//                             "ConnectorWalletDetails",
//                         )
//                         .attach_printable("Unable to deserialize connector_wallets_details")
//                         .change_context(errors::ApiErrorResponse::InternalServerError)
//                 })
//                 .transpose()?,
//         };
//         Ok(response)
//     }
// }

#[cfg(feature = "v2")]
impl ForeignTryFrom<domain::MerchantConnectorAccount>
    for api_models::admin::MerchantConnectorResponse
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn foreign_try_from(item: domain::MerchantConnectorAccount) -> Result<Self, Self::Error> {
        let frm_configs = match item.frm_configs {
            Some(ref frm_value) => {
                let configs_for_frm : Vec<api_models::admin::FrmConfigs> = frm_value
                    .iter()
                    .map(|config| { config
                        .peek()
                        .clone()
                        .parse_value("FrmConfigs")
                        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                            field_name: "frm_configs".to_string(),
                            expected_format: r#"[{ "gateway": "stripe", "payment_methods": [{ "payment_method": "card","payment_method_types": [{"payment_method_type": "credit","card_networks": ["Visa"],"flow": "pre","action": "cancel_txn"}]}]}]"#.to_string(),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Some(configs_for_frm)
            }
            None => None,
        };

        // parse the connector_account_details into ConnectorAuthType
        let connector_account_details: hyperswitch_domain_models::router_data::ConnectorAuthType =
            item.connector_account_details
                .clone()
                .into_inner()
                .parse_value("ConnectorAuthType")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while parsing value for ConnectorAuthType")?;
        // get the masked keys from the ConnectorAuthType and encode it to secret value
        let masked_connector_account_details = Secret::new(
            connector_account_details
                .get_masked_keys()
                .encode_to_value()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to encode ConnectorAuthType")?,
        );

        let feature_metadata = item.feature_metadata.as_ref().map(|metadata| {
            api_models::admin::MerchantConnectorAccountFeatureMetadata::foreign_from(metadata)
        });

        let response = Self {
            id: item.get_id(),
            connector_type: item.connector_type,
            connector_name: item.connector_name,
            connector_label: item.connector_label,
            connector_account_details: masked_connector_account_details,
            disabled: item.disabled,
            payment_methods_enabled: item.payment_methods_enabled,
            metadata: item.metadata,
            frm_configs,
            connector_webhook_details: item
                .connector_webhook_details
                .map(|webhook_details| {
                    serde_json::Value::parse_value(
                        webhook_details.expose(),
                        "MerchantConnectorWebhookDetails",
                    )
                    .attach_printable("Unable to deserialize connector_webhook_details")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                })
                .transpose()?,
            profile_id: item.profile_id,
            applepay_verified_domains: item.applepay_verified_domains,
            pm_auth_config: item.pm_auth_config,
            status: item.status,
            additional_merchant_data: item
                .additional_merchant_data
                .map(|data| {
                    let data = data.into_inner();
                    serde_json::Value::parse_value::<router_types::AdditionalMerchantData>(
                        data.expose(),
                        "AdditionalMerchantData",
                    )
                    .attach_printable("Unable to deserialize additional_merchant_data")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                })
                .transpose()?
                .map(api_models::admin::AdditionalMerchantData::foreign_from),
            connector_wallets_details: item
                .connector_wallets_details
                .map(|data| {
                    data.into_inner()
                        .expose()
                        .parse_value::<api_models::admin::ConnectorWalletDetails>(
                            "ConnectorWalletDetails",
                        )
                        .attach_printable("Unable to deserialize connector_wallets_details")
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                })
                .transpose()?,
            feature_metadata,
        };
        Ok(response)
    }
}

impl ForeignFrom<routing_types::ConnectorSelection> for routing_types::RoutingAlgorithm {
    fn foreign_from(value: routing_types::ConnectorSelection) -> Self {
        match value {
            routing_types::ConnectorSelection::Priority(connectors) => Self::Priority(connectors),

            routing_types::ConnectorSelection::VolumeSplit(splits) => Self::VolumeSplit(splits),
        }
    }
}