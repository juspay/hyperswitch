use common_enums::{AttemptStatus, PaymentMethodType};
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use error_stack::ResultExt;
use external_services::grpc_client::unified_connector_service::{
    ConnectorAuthMetadata, UnifiedConnectorServiceError,
};
use hyperswitch_connectors::utils::CardData;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails;
use hyperswitch_domain_models::{
    merchant_context::MerchantContext,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_response_types::{PaymentsResponseData, RedirectForm},
};
use masking::{ExposeInterface, PeekInterface, Secret};
use unified_connector_service_client::payments::{
    self as payments_grpc, payment_method::PaymentMethod, CardDetails, CardPaymentMethodType,
    PaymentServiceAuthorizeResponse,
};

use crate::{
    consts,
    core::{
        errors::RouterResult,
        payments::helpers::{should_execute_based_on_rollout, MerchantConnectorAccountType},
        utils::get_flow_name,
    },
    routes::SessionState,
    types::transformers::ForeignTryFrom,
};

mod transformers;

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
    Ok(should_execute && state.grpc_client.unified_connector_service_client.is_some())
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

            let card_network = card
                .card_network
                .clone()
                .map(payments_grpc::CardNetwork::foreign_try_from)
                .transpose()?;

            let card_details = CardDetails {
                card_number: card.card_number.get_card_no(),
                card_exp_month,
                card_exp_year: card.get_expiry_year_4_digit().peek().to_string(),
                card_cvc: card.card_cvc.peek().to_string(),
                card_holder_name: card.card_holder_name.map(|name| name.expose()),
                card_issuer: card.card_issuer.clone(),
                card_network: card_network.map(|card_network| card_network.into()),
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
                        "Unimplemented payment method subtype: {payment_method_type:?}"
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
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Upi(upi_data) => {
            let upi_type = match upi_data {
                hyperswitch_domain_models::payment_method_data::UpiData::UpiCollect(
                    upi_collect_data,
                ) => {
                    let vpa_id = upi_collect_data.vpa_id.map(|vpa| vpa.expose());
                    let upi_details = payments_grpc::UpiCollect { vpa_id };
                    PaymentMethod::UpiCollect(upi_details)
                }
                _ => {
                    return Err(UnifiedConnectorServiceError::NotImplemented(format!(
                        "Unimplemented payment method subtype: {payment_method_type:?}"
                    ))
                    .into());
                }
            };

            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(upi_type),
            })
        }
        _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
            "Unimplemented payment method: {payment_method_data:?}"
        ))
        .into()),
    }
}

pub fn build_unified_connector_service_auth_metadata(
    #[cfg(feature = "v1")] merchant_connector_account: MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: MerchantConnectorAccountTypeDetails,
    merchant_context: &MerchantContext,
) -> CustomResult<ConnectorAuthMetadata, UnifiedConnectorServiceError> {
    #[cfg(feature = "v1")]
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(UnifiedConnectorServiceError::FailedToObtainAuthType)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    #[cfg(feature = "v2")]
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .change_context(UnifiedConnectorServiceError::FailedToObtainAuthType)
        .attach_printable("Failed to obtain ConnectorAuthType")?;

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

    let merchant_id = merchant_context
        .get_merchant_account()
        .get_id()
        .get_string_repr();

    match &auth_type {
        ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_SIGNATURE_KEY.to_string(),
            api_key: Some(api_key.clone()),
            key1: Some(key1.clone()),
            api_secret: Some(api_secret.clone()),
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        ConnectorAuthType::BodyKey { api_key, key1 } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_BODY_KEY.to_string(),
            api_key: Some(api_key.clone()),
            key1: Some(key1.clone()),
            api_secret: None,
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        ConnectorAuthType::HeaderKey { api_key } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_HEADER_KEY.to_string(),
            api_key: Some(api_key.clone()),
            key1: None,
            api_secret: None,
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        _ => Err(UnifiedConnectorServiceError::FailedToObtainAuthType)
            .attach_printable("Unsupported ConnectorAuthType for header injection"),
    }
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

    let transaction_id = response.transaction_id.as_ref().and_then(|id| {
        id.id_type.clone().and_then(|id_type| match id_type {
            payments_grpc::identifier::IdType::Id(id) => Some(id),
            payments_grpc::identifier::IdType::EncodedData(encoded_data) => Some(encoded_data),
            payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
        })
    });

    let router_data_response = match status {
        AttemptStatus::Charged |
                AttemptStatus::Authorized |
                AttemptStatus::AuthenticationPending |
                AttemptStatus::DeviceDataCollectionPending |
                AttemptStatus::Started |
                AttemptStatus::AuthenticationSuccessful |
                AttemptStatus::Authorizing |
                AttemptStatus::ConfirmationAwaited |
                AttemptStatus::Pending => Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: match transaction_id.as_ref() {
                        Some(transaction_id) => hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(transaction_id.clone()),
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
        AttemptStatus::AuthenticationFailed
                | AttemptStatus::AuthorizationFailed
                | AttemptStatus::Unresolved
                | AttemptStatus::Failure => Err(ErrorResponse {
                    code: response.error_code().to_owned(),
                    message: response.error_message().to_owned(),
                    reason: Some(response.error_message().to_owned()),
                    status_code: 500,
                    attempt_status: Some(status),
                    connector_transaction_id: connector_response_reference_id,
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                }),
        AttemptStatus::RouterDeclined |
                    AttemptStatus::CodInitiated |
                    AttemptStatus::Voided |
                    AttemptStatus::VoidInitiated |
                    AttemptStatus::CaptureInitiated |
                    AttemptStatus::VoidFailed |
                    AttemptStatus::AutoRefunded |
                    AttemptStatus::PartialCharged |
                    AttemptStatus::PartialChargedAndChargeable |
                    AttemptStatus::PaymentMethodAwaited |
                    AttemptStatus::CaptureFailed |
                    AttemptStatus::IntegrityFailure => return Err(UnifiedConnectorServiceError::NotImplemented(format!(
                        "AttemptStatus {status:?} is not implemented for Unified Connector Service"
                    )).into()),
                };

    Ok((status, router_data_response))
}

pub fn handle_unified_connector_service_response_for_payment_get(
    response: payments_grpc::PaymentServiceGetResponse,
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
                AttemptStatus::DeviceDataCollectionPending |
                AttemptStatus::Started |
                AttemptStatus::AuthenticationSuccessful |
                AttemptStatus::Authorizing |
                AttemptStatus::ConfirmationAwaited |
                AttemptStatus::Pending => Ok(
            PaymentsResponseData::TransactionResponse {
                resource_id: match connector_response_reference_id.as_ref() {
                Some(connector_response_reference_id) => hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(connector_response_reference_id.clone()),
                None => hyperswitch_domain_models::router_request_types::ResponseId::NoResponseId,
            },
                redirection_data: Box::new(
                    None
                ),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: response.network_txn_id.clone(),
                connector_response_reference_id,
                incremental_authorization_allowed: None,
                charges: None,
            }
        ),
        AttemptStatus::AuthenticationFailed
                | AttemptStatus::AuthorizationFailed
                | AttemptStatus::Failure => Err(ErrorResponse {
            code: response.error_code().to_owned(),
            message: response.error_message().to_owned(),
            reason: Some(response.error_message().to_owned()),
            status_code: 500,
            attempt_status: Some(status),
            connector_transaction_id: connector_response_reference_id,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        }),
        AttemptStatus::RouterDeclined |
                    AttemptStatus::CodInitiated |
                    AttemptStatus::Voided |
                    AttemptStatus::VoidInitiated |
                    AttemptStatus::CaptureInitiated |
                    AttemptStatus::VoidFailed |
                    AttemptStatus::AutoRefunded |
                    AttemptStatus::PartialCharged |
                    AttemptStatus::PartialChargedAndChargeable |
                    AttemptStatus::Unresolved |
                    AttemptStatus::PaymentMethodAwaited |
                    AttemptStatus::CaptureFailed |
                    AttemptStatus::IntegrityFailure => return Err(UnifiedConnectorServiceError::NotImplemented(format!(
                        "AttemptStatus {status:?} is not implemented for Unified Connector Service"
                    )).into()),
    };

    Ok((status, router_data_response))
}
