use api_models::admin::ConnectorAuthType;
use common_enums::AttemptStatus;
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use error_stack::ResultExt;
use external_services::grpc_client::unified_connector_service::{
    UnifiedConnectorService, UnifiedConnectorServiceError,
};
use hyperswitch_domain_models::{
    merchant_context::MerchantContext,
    router_data::{ErrorResponse, RouterData},
    router_flow_types::payments::Authorize,
    router_request_types::PaymentsAuthorizeData,
    router_response_types::PaymentsResponseData,
};
use masking::PeekInterface;
use rand::Rng;
use rust_grpc_client::payments::{self as payments_grpc};
use tonic::metadata::{MetadataMap, MetadataValue};

use crate::{
    consts,
    core::{
        errors::RouterResult, payments::helpers::MerchantConnectorAccountType, utils::get_flow_name,
    },
    routes::SessionState,
    types::transformers::ForeignTryFrom,
};

pub mod transformers;

pub async fn should_call_unified_connector_service<F: Clone, T>(
    state: &SessionState,
    merchant_context: &MerchantContext,
    router_data: &RouterData<F, T, PaymentsResponseData>,
) -> RouterResult<Option<UnifiedConnectorService>> {
    let merchant_id = merchant_context
        .get_merchant_account()
        .get_id()
        .get_string_repr();

    let connector_name = router_data.connector.clone();

    let payment_method = router_data.payment_method.to_string();
    let flow_name = get_flow_name::<F>()?;

    let config_key = format!(
        "{}_{}_{}_{}_{}",
        consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
        merchant_id,
        connector_name,
        payment_method,
        flow_name
    );

    let db = state.store.as_ref();

    match db.find_config_by_key(&config_key).await {
        Ok(rollout_config) => match rollout_config.config.parse() {
            Ok(rollout_percent) => {
                let random_value: f64 = rand::thread_rng().gen_range(0.0..=1.0);
                if random_value < rollout_percent {
                    if let Some(unified_connector_service_client) =
                        &state.grpc_client.unified_connector_service
                    {
                        Ok(Some(unified_connector_service_client.clone()))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        },
        Err(_) => Ok(None),
    }
}

pub fn build_unified_connector_service_auth_headers(
    merchant_connector_account: MerchantConnectorAccountType,
) -> CustomResult<MetadataMap, UnifiedConnectorServiceError> {
    let mut metadata = MetadataMap::new();

    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(UnifiedConnectorServiceError::FailedToObtainAuthType)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let connector_name = {
        #[cfg(feature = "v1")]
        {
            merchant_connector_account
                .get_connector_name()
                .ok_or_else(|| UnifiedConnectorServiceError::MissingConnectorName)
                .attach_printable("Missing connector name")?
        }

        #[cfg(feature = "v2")]
        {
            merchant_connector_account
                .get_connector_name()
                .map(|connector| connector.to_string())
                .ok_or_else(|| UnifiedConnectorServiceError::MissingConnectorName)
                .attach_printable("Missing connector name")?
        }
    };

    let parsed_connector_name = connector_name
        .parse::<MetadataValue<_>>()
        .change_context(UnifiedConnectorServiceError::InvalidConnectorName)
        .attach_printable(format!("Failed to parse connector name: {connector_name}"))?;

    metadata.append(consts::UCS_HEADER_CONNECTOR, parsed_connector_name);

    let parse_metadata_value =
        |value: &str,
         context: &str|
         -> CustomResult<MetadataValue<_>, UnifiedConnectorServiceError> {
            value
                .parse::<MetadataValue<_>>()
                .change_context(UnifiedConnectorServiceError::HeaderInjectionFailed(
                    context.to_string(),
                ))
                .attach_printable(format!(
                    "Failed to parse metadata value for {context}: {value}"
                ))
        };

    match &auth_type {
        ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } => {
            metadata.append(
                consts::UCS_HEADER_AUTH_TYPE,
                parse_metadata_value(consts::UCS_AUTH_SIGNATURE_KEY, consts::UCS_HEADER_AUTH_TYPE)?,
            );
            metadata.append(
                consts::UCS_HEADER_API_KEY,
                parse_metadata_value(&api_key.peek(), consts::UCS_HEADER_API_KEY)?,
            );
            metadata.append(
                consts::UCS_HEADER_KEY1,
                parse_metadata_value(&key1.peek(), consts::UCS_HEADER_KEY1)?,
            );
            metadata.append(
                consts::UCS_HEADER_API_SECRET,
                parse_metadata_value(&api_secret.peek(), consts::UCS_HEADER_API_SECRET)?,
            );
        }
        ConnectorAuthType::BodyKey { api_key, key1 } => {
            metadata.append(
                consts::UCS_HEADER_AUTH_TYPE,
                parse_metadata_value(consts::UCS_AUTH_BODY_KEY, consts::UCS_HEADER_AUTH_TYPE)?,
            );
            metadata.append(
                consts::UCS_HEADER_API_KEY,
                parse_metadata_value(&api_key.peek(), consts::UCS_HEADER_API_KEY)?,
            );
            metadata.append(
                consts::UCS_HEADER_KEY1,
                parse_metadata_value(&key1.peek(), consts::UCS_HEADER_KEY1)?,
            );
        }
        ConnectorAuthType::HeaderKey { api_key } => {
            metadata.append(
                consts::UCS_HEADER_AUTH_TYPE,
                parse_metadata_value(consts::UCS_AUTH_HEADER_KEY, consts::UCS_HEADER_AUTH_TYPE)?,
            );
            metadata.append(
                consts::UCS_HEADER_API_KEY,
                parse_metadata_value(&api_key.peek(), consts::UCS_HEADER_API_KEY)?,
            );
        }
        _ => {
            return Err(UnifiedConnectorServiceError::FailedToObtainAuthType)
                .attach_printable("Unsupported ConnectorAuthType for header injection")?;
        }
    }

    Ok(metadata)
}

pub fn handle_unified_connector_service_response(
    response: payments_grpc::PaymentsAuthorizeResponse,
    router_data: &mut RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
) -> CustomResult<(), UnifiedConnectorServiceError> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let router_data_response = match status {
        AttemptStatus::Charged |
        AttemptStatus::Authorized |
        AttemptStatus::AuthenticationPending |
        AttemptStatus::DeviceDataCollectionPending => Ok(PaymentsResponseData::TransactionResponse {
            resource_id: hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(response.connector_response_reference_id().to_owned()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(response.connector_response_reference_id().to_owned()),
            incremental_authorization_allowed: None,
            charges: None,
        }),
        _ => Err(ErrorResponse {
            code: response.error_code().to_owned(),
            message: response.error_message().to_owned(),
            reason: Some(response.error_message().to_owned()),
            status_code: 500,
            attempt_status: Some(status),
            connector_transaction_id: Some(response.connector_response_reference_id().to_owned()),
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    };
    router_data.status = status;
    router_data.response = router_data_response;

    Ok(())
}
