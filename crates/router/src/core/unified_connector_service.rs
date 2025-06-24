use api_models::admin::ConnectorAuthType;
use common_enums::{AttemptStatus, PaymentMethodType};
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use error_stack::ResultExt;
use hyperswitch_connectors::utils::CardData;
use hyperswitch_domain_models::{
    merchant_context::MerchantContext,
    router_data::{ErrorResponse, RouterData},
    router_response_types::{PaymentsResponseData, RedirectForm},
};
use masking::{ExposeInterface, PeekInterface};
use router_env::logger;
use tonic::metadata::{MetadataMap, MetadataValue};
use unified_connector_service_client::payments::{
    self as payments_grpc, payment_method::PaymentMethod,
    payment_service_client::PaymentServiceClient, CardDetails, CardPaymentMethodType,
    PaymentServiceAuthorizeResponse,
};

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

mod errors;
mod transformers;

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
        request: tonic::Request<payments_grpc::PaymentServiceAuthorizeRequest>,
    ) -> UnifiedConnectorServiceResult<tonic::Response<PaymentServiceAuthorizeResponse>> {
        self.client
            .clone()
            .authorize(request)
            .await
            .change_context(UnifiedConnectorServiceError::ConnectionError(
                "Failed to authorize payment through Unified Connector Service".to_owned(),
            ))
            .inspect_err(|error| logger::error!(?error))
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

pub fn build_unified_connector_service_payment_method(
    payment_method_data: hyperswitch_domain_models::payment_method_data::PaymentMethodData,
    payment_method_type: PaymentMethodType,
) -> CustomResult<payments_grpc::PaymentMethod, UnifiedConnectorServiceError> {
    match payment_method_data {
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card) => {
            let card_exp_month = card
                .get_card_expiry_month_2_digit()
                .attach_printable("Failed to extract 2-digit expiry month from card")
                .change_context(UnifiedConnectorServiceError::InvalidDataFormat {
                    field_name: "card_exp_month",
                })?
                .peek()
                .to_string();

            let card_details = CardDetails {
                card_number: card.card_number.get_card_no(),
                card_exp_month,
                card_exp_year: card.get_expiry_year_4_digit().peek().to_string(),
                card_cvc: card.card_cvc.peek().to_string(),
                card_holder_name: card.card_holder_name.map(|name| name.expose()),
                card_issuer: card.card_issuer.clone(),
                card_network: None,
                card_type: card.card_type.clone(),
                bank_code: card.bank_code.clone(),
                nick_name: card.nick_name.map(|n| n.expose()),
                card_issuing_country_alpha2: card.card_issuing_country.clone(),
            };

            let grpc_card_type = match payment_method_type {
                PaymentMethodType::Credit => {
                    payments_grpc::card_payment_method_type::CardType::Credit(card_details)
                }
                PaymentMethodType::Debit => {
                    payments_grpc::card_payment_method_type::CardType::Debit(card_details)
                }
                _ => {
                    return Err(UnifiedConnectorServiceError::NotImplemented(format!(
                        "Unimplemented card payment method type: {:?}",
                        payment_method_type
                    ))
                    .into());
                }
            };

            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::Card(CardPaymentMethodType {
                    card_type: Some(grpc_card_type),
                })),
            })
        }

        _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
            "Unimplemented payment method: {:?}",
            payment_method_data
        ))
        .into()),
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
    response: PaymentServiceAuthorizeResponse,
) -> CustomResult<
    (AttemptStatus, Result<PaymentsResponseData, ErrorResponse>),
    UnifiedConnectorServiceError,
> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let connector_response_reference_id =
        response.response_ref_id.as_ref().and_then(|identifier| {
            identifier
                .id_type
                .clone()
                .and_then(|id_type| match id_type {
                    payments_grpc::identifier::IdType::Id(id) => Some(id),
                    payments_grpc::identifier::IdType::EncodedData(encoded_data) => {
                        Some(encoded_data)
                    }
                    payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
                })
        });

    let router_data_response = match status {
        AttemptStatus::Charged |
        AttemptStatus::Authorized |
        AttemptStatus::AuthenticationPending |
        AttemptStatus::DeviceDataCollectionPending => Ok(PaymentsResponseData::TransactionResponse {
            resource_id: match connector_response_reference_id.as_ref() {
                Some(connector_response_reference_id) => hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(connector_response_reference_id.clone()),
                None => hyperswitch_domain_models::router_request_types::ResponseId::NoResponseId,
            },
            redirection_data: Box::new(
                response
                    .redirection_data
                    .clone()
                    .map(RedirectForm::foreign_try_from)
                    .transpose()?
            ),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: response.network_txn_id.clone(),
            connector_response_reference_id,
            incremental_authorization_allowed: response.incremental_authorization_allowed,
            charges: None,
        }),
        _ => Err(ErrorResponse {
            code: response.error_code().to_owned(),
            message: response.error_message().to_owned(),
            reason: Some(response.error_message().to_owned()),
            status_code: 500,
            attempt_status: Some(status),
            connector_transaction_id: connector_response_reference_id,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    };

    Ok((status, router_data_response))
}
