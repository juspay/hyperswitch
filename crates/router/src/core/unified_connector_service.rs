use api_models::admin::ConnectorAuthType;
use common_enums::AttemptStatus;
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_context::MerchantContext,
    router_data::{ErrorResponse, RouterData},
    router_response_types::{MandateReference, PaymentsResponseData, RedirectForm},
};
use masking::PeekInterface;
use router_env::logger;
use rust_grpc_client::payments::{
    self as payments_grpc, payment_service_client::PaymentServiceClient, PaymentsAuthorizeResponse,
};
use tonic::metadata::{MetadataMap, MetadataValue};

use crate::{
    configs::settings::UnifiedConnectorService,
    consts,
    core::{
        errors::RouterResult,
        payments::helpers::{should_execute_based_on_rollout, MerchantConnectorAccountType},
        unified_connector_service::errors::UnifiedConnectorServiceError,
        utils::get_flow_name,
    },
    routes::SessionState,
    types::transformers::ForeignTryFrom,
};

pub mod errors;
pub mod transformers;

/// Result type for Dynamic Routing
pub type UnifiedConnectorServiceResult<T> = CustomResult<T, UnifiedConnectorServiceError>;
/// Contains the  Unified Connector Service client
#[derive(Debug, Clone)]
pub struct UnifiedConnectorServiceClient {
    /// The Unified Connector Service Client
    pub client: PaymentServiceClient<tonic::transport::Channel>,
}

impl UnifiedConnectorServiceClient {
    /// Builds the connection to the gRPC service
    pub async fn build_connections(config: UnifiedConnectorService) -> Option<Self> {
        match PaymentServiceClient::connect(config.base_url.clone().get_string_repr().to_owned())
            .await
        {
            Ok(unified_connector_service_client) => Some(Self {
                client: unified_connector_service_client,
            }),
            Err(err) => {
                logger::error!(error = ?err, "Failed to connect to Unified Connector Service");
                None
            }
        }
    }

    /// Performs Payment Authorize
    pub async fn payment_authorize(
        &self,
        request: tonic::Request<payments_grpc::PaymentsAuthorizeRequest>,
    ) -> UnifiedConnectorServiceResult<tonic::Response<PaymentsAuthorizeResponse>> {
        self.client
            .clone()
            .payment_authorize(request)
            .await
            .change_context(UnifiedConnectorServiceError::ConnectionError(
                "Failed to authorize payment through Unified Connector Service".to_owned(),
            ))
            .map_err(|err| {
                logger::error!(error=?err);
                err
            })
    }
}

pub async fn should_call_unified_connector_service<F: Clone, T>(
    state: &SessionState,
    merchant_context: &MerchantContext,
    router_data: &RouterData<F, T, PaymentsResponseData>,
) -> RouterResult<bool> {
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

    let should_execute = should_execute_based_on_rollout(state, &config_key).await?;
    Ok(should_execute && state.unified_connector_service_client.is_some())
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
                .ok_or(UnifiedConnectorServiceError::MissingConnectorName)
                .attach_printable("Missing connector name")?
        }

        #[cfg(feature = "v2")]
        {
            merchant_connector_account
                .get_connector_name()
                .map(|connector| connector.to_string())
                .ok_or(UnifiedConnectorServiceError::MissingConnectorName)
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
                parse_metadata_value(api_key.peek(), consts::UCS_HEADER_API_KEY)?,
            );
            metadata.append(
                consts::UCS_HEADER_KEY1,
                parse_metadata_value(key1.peek(), consts::UCS_HEADER_KEY1)?,
            );
            metadata.append(
                consts::UCS_HEADER_API_SECRET,
                parse_metadata_value(api_secret.peek(), consts::UCS_HEADER_API_SECRET)?,
            );
        }
        ConnectorAuthType::BodyKey { api_key, key1 } => {
            metadata.append(
                consts::UCS_HEADER_AUTH_TYPE,
                parse_metadata_value(consts::UCS_AUTH_BODY_KEY, consts::UCS_HEADER_AUTH_TYPE)?,
            );
            metadata.append(
                consts::UCS_HEADER_API_KEY,
                parse_metadata_value(api_key.peek(), consts::UCS_HEADER_API_KEY)?,
            );
            metadata.append(
                consts::UCS_HEADER_KEY1,
                parse_metadata_value(key1.peek(), consts::UCS_HEADER_KEY1)?,
            );
        }
        ConnectorAuthType::HeaderKey { api_key } => {
            metadata.append(
                consts::UCS_HEADER_AUTH_TYPE,
                parse_metadata_value(consts::UCS_AUTH_HEADER_KEY, consts::UCS_HEADER_AUTH_TYPE)?,
            );
            metadata.append(
                consts::UCS_HEADER_API_KEY,
                parse_metadata_value(api_key.peek(), consts::UCS_HEADER_API_KEY)?,
            );
        }
        _ => {
            return Err(UnifiedConnectorServiceError::FailedToObtainAuthType)
                .attach_printable("Unsupported ConnectorAuthType for header injection")?;
        }
    }

    Ok(metadata)
}

pub fn handle_unified_connector_service_response_for_payment_authorize(
    response: PaymentsAuthorizeResponse,
) -> CustomResult<
    (AttemptStatus, Result<PaymentsResponseData, ErrorResponse>),
    UnifiedConnectorServiceError,
> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let router_data_response = match status {
        AttemptStatus::Charged |
        AttemptStatus::Authorized |
        AttemptStatus::AuthenticationPending |
        AttemptStatus::DeviceDataCollectionPending => Ok(PaymentsResponseData::TransactionResponse {
            resource_id: hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(response.connector_response_reference_id().to_owned()),
            redirection_data: Box::new(
                response
                    .redirection_data
                    .clone()
                    .map(RedirectForm::foreign_try_from)
                    .transpose()?
            ),
            mandate_reference: Box::new(
                response
                    .mandate_reference
                    .clone()
                    .map(MandateReference::foreign_try_from)
                    .transpose()?
            ),
            connector_metadata: None,
            network_txn_id: response.network_txn_id.clone(),
            connector_response_reference_id: Some(response.connector_response_reference_id().to_owned()),
            incremental_authorization_allowed: response.incremental_authorization_allowed,
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

    Ok((status, router_data_response))
}
