use api_models::{admin, webhooks as webhook_api};
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
    router_response_types::PaymentsResponseData,
};
use masking::{ExposeInterface, PeekInterface, Secret};
use time;
use unified_connector_service_client::payments::{
    self as payments_grpc, payment_method::PaymentMethod, CardDetails, CardPaymentMethodType,
    PaymentServiceAuthorizeResponse, PaymentServiceTransformRequest,
    PaymentServiceTransformResponse,
};

use crate::{
    consts,
    core::{
        errors::{ApiErrorResponse, RouterResult},
        payments::helpers::{
            is_ucs_enabled, should_execute_based_on_rollout, MerchantConnectorAccountType,
        },
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
    if state.grpc_client.unified_connector_service_client.is_none() {
        return Ok(false);
    }

    let ucs_config_key = consts::UCS_ENABLED;

    if !is_ucs_enabled(state, ucs_config_key).await {
        return Ok(false);
    }

    let merchant_id = merchant_context
        .get_merchant_account()
        .get_id()
        .get_string_repr();

    let connector_name = router_data.connector.clone();
    let payment_method = router_data.payment_method.to_string();
    let flow_name = get_flow_name::<F>()?;

    let is_ucs_only_connector = state
        .conf
        .grpc_client
        .unified_connector_service
        .as_ref()
        .is_some_and(|config| config.ucs_only_connectors.contains(&connector_name));

    if is_ucs_only_connector {
        return Ok(true);
    }
    let config_key = format!(
        "{}_{}_{}_{}_{}",
        consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
        merchant_id,
        connector_name,
        payment_method,
        flow_name
    );

    let should_execute = should_execute_based_on_rollout(state, &config_key).await?;
    Ok(should_execute)
}

pub async fn should_call_unified_connector_service_for_webhooks(
    state: &SessionState,
    merchant_context: &MerchantContext,
    connector_name: &str,
) -> RouterResult<bool> {
    if state.grpc_client.unified_connector_service_client.is_none() {
        return Ok(false);
    }

    let ucs_config_key = consts::UCS_ENABLED;

    if !is_ucs_enabled(state, ucs_config_key).await {
        return Ok(false);
    }

    let merchant_id = merchant_context
        .get_merchant_account()
        .get_id()
        .get_string_repr();

    let config_key = format!(
        "{}_{}_{}_Webhooks",
        consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
        merchant_id,
        connector_name
    );

    let should_execute = should_execute_based_on_rollout(state, &config_key).await?;

    Ok(should_execute)
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
                hyperswitch_domain_models::payment_method_data::UpiData::UpiIntent(_) => {
                    let upi_details = payments_grpc::UpiIntent {};
                    PaymentMethod::UpiIntent(upi_details)
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

    // <<<<<<< HEAD
    //     let connector_response_reference_id =
    //         response.response_ref_id.as_ref().and_then(|identifier| {
    //             identifier
    //                 .id_type
    //                 .clone()
    //                 .and_then(|id_type| match id_type {
    //                     payments_grpc::identifier::IdType::Id(id) => Some(id),
    //                     payments_grpc::identifier::IdType::EncodedData(encoded_data) => {
    //                         Some(encoded_data)
    //                     }
    //                     payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
    //                 })
    //         });

    //     let transaction_id = response.transaction_id.as_ref().and_then(|id| {
    //         id.id_type.clone().and_then(|id_type| match id_type {
    //             payments_grpc::identifier::IdType::Id(id) => Some(id),
    //             payments_grpc::identifier::IdType::EncodedData(encoded_data) => Some(encoded_data),
    //             payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
    //         })
    //     });

    //     let (connector_metadata, redirection_data) = match response.redirection_data.clone() {
    //         Some(redirection_data) => match redirection_data.form_type {
    //             Some(ref form_type) => match form_type {
    //                 payments_grpc::redirect_form::FormType::Uri(uri) => {
    //                     let image_data = QrImage::new_from_data(uri.uri.clone())
    //                         .change_context(UnifiedConnectorServiceError::ParsingFailed)?;
    //                     let image_data_url = Url::parse(image_data.data.clone().as_str())
    //                         .change_context(UnifiedConnectorServiceError::ParsingFailed)?;
    //                     let qr_code_info = QrCodeInformation::QrDataUrl {
    //                         image_data_url,
    //                         display_to_timestamp: None,
    //                     };
    //                     (
    //                         Some(qr_code_info.encode_to_value())
    //                             .transpose()
    //                             .change_context(UnifiedConnectorServiceError::ParsingFailed)?,
    //                         None,
    //                     )
    //                 }
    //                 _ => (
    //                     None,
    //                     Some(RedirectForm::foreign_try_from(redirection_data)).transpose()?,
    //                 ),
    //             },
    //             None => (None, None),
    //         },
    //         None => (None, None),
    //     };

    //     let router_data_response = match status {
    //         AttemptStatus::Charged |
    //                 AttemptStatus::Authorized |
    //                 AttemptStatus::AuthenticationPending |
    //                 AttemptStatus::DeviceDataCollectionPending |
    //                 AttemptStatus::Started |
    //                 AttemptStatus::AuthenticationSuccessful |
    //                 AttemptStatus::Authorizing |
    //                 AttemptStatus::ConfirmationAwaited |
    //                 AttemptStatus::Pending => Ok(PaymentsResponseData::TransactionResponse {
    //                     resource_id: match transaction_id.as_ref() {
    //                         Some(transaction_id) => hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(transaction_id.clone()),
    //                         None => hyperswitch_domain_models::router_request_types::ResponseId::NoResponseId,
    //                     },
    //                     redirection_data: Box::new(
    //                             redirection_data
    //                     ),
    //                     mandate_reference: Box::new(None),
    //                     connector_metadata,
    //                     network_txn_id: response.network_txn_id.clone(),
    //                     connector_response_reference_id,
    //                     incremental_authorization_allowed: response.incremental_authorization_allowed,
    //                     charges: None,
    //                 }),
    //         AttemptStatus::AuthenticationFailed
    //                 | AttemptStatus::AuthorizationFailed
    //                 | AttemptStatus::Unresolved
    //                 | AttemptStatus::Failure => Err(ErrorResponse {
    //                     code: response.error_code().to_owned(),
    //                     message: response.error_message().to_owned(),
    //                     reason: Some(response.error_message().to_owned()),
    //                     status_code: 500,
    //                     attempt_status: Some(status),
    //                     connector_transaction_id: connector_response_reference_id,
    //                     network_decline_code: None,
    //                     network_advice_code: None,
    //                     network_error_message: None,
    //                 }),
    //         AttemptStatus::RouterDeclined |
    //                     AttemptStatus::CodInitiated |
    //                     AttemptStatus::Voided |
    //                     AttemptStatus::VoidInitiated |
    //                     AttemptStatus::CaptureInitiated |
    //                     AttemptStatus::VoidFailed |
    //                     AttemptStatus::AutoRefunded |
    //                     AttemptStatus::PartialCharged |
    //                     AttemptStatus::PartialChargedAndChargeable |
    //                     AttemptStatus::PaymentMethodAwaited |
    //                     AttemptStatus::CaptureFailed |
    //                     AttemptStatus::IntegrityFailure => return Err(UnifiedConnectorServiceError::NotImplemented(format!(
    //                         "AttemptStatus {status:?} is not implemented for Unified Connector Service"
    //                     )).into()),
    //                 };
    // =======
    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((status, router_data_response))
}

pub fn handle_unified_connector_service_response_for_payment_get(
    response: payments_grpc::PaymentServiceGetResponse,
) -> CustomResult<
    (AttemptStatus, Result<PaymentsResponseData, ErrorResponse>),
    UnifiedConnectorServiceError,
> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((status, router_data_response))
}

pub fn handle_unified_connector_service_response_for_payment_register(
    response: payments_grpc::PaymentServiceRegisterResponse,
) -> CustomResult<
    (AttemptStatus, Result<PaymentsResponseData, ErrorResponse>),
    UnifiedConnectorServiceError,
> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((status, router_data_response))
}

pub fn handle_unified_connector_service_response_for_payment_repeat(
    response: payments_grpc::PaymentServiceRepeatEverythingResponse,
) -> CustomResult<
    (AttemptStatus, Result<PaymentsResponseData, ErrorResponse>),
    UnifiedConnectorServiceError,
> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((status, router_data_response))
}

pub fn build_unified_connector_service_webhook_transform_request(
    webhook_body: &[u8],
    headers: &actix_web::http::header::HeaderMap,
    query_params: Option<&str>,
    webhook_secrets: Option<payments_grpc::WebhookSecrets>,
    merchant_id: &str,
    connector_id: &str,
) -> CustomResult<PaymentServiceTransformRequest, UnifiedConnectorServiceError> {
    let body_string = String::from_utf8(webhook_body.to_vec()).change_context(
        UnifiedConnectorServiceError::InvalidDataFormat {
            field_name: "webhook_body",
        },
    )?;

    let headers_map = headers
        .iter()
        .map(|(key, value)| {
            let value_string = value.to_str().unwrap_or_default().to_string();
            (key.as_str().to_string(), value_string)
        })
        .collect();

    Ok(PaymentServiceTransformRequest {
        request_ref_id: Some(payments_grpc::Identifier {
            id_type: Some(payments_grpc::identifier::IdType::Id(format!(
                "{}_{}_{}",
                merchant_id,
                connector_id,
                time::OffsetDateTime::now_utc().unix_timestamp()
            ))),
        }),
        request_details: Some(payments_grpc::RequestDetails {
            method: 1, // POST method
            uri: Some(
                headers
                    .get("x-forwarded-path")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("/webhook")
                    .to_string(),
            ),
            body: body_string.into_bytes(),
            headers: headers_map,
            query_params: Some(query_params.unwrap_or_default().to_string()),
        }),
        webhook_secrets,
    })
}

pub struct WebhookTransformData {
    pub event_type: webhook_api::IncomingWebhookEvent,
    pub source_verified: bool,
    pub webhook_content: Option<payments_grpc::WebhookResponseContent>,
    pub response_ref_id: Option<String>,
}

pub fn handle_unified_connector_service_webhook_transform_response(
    response: PaymentServiceTransformResponse,
) -> CustomResult<WebhookTransformData, UnifiedConnectorServiceError> {
    let event_type = match response.event_type {
        0 => webhook_api::IncomingWebhookEvent::PaymentIntentSuccess,
        1 => webhook_api::IncomingWebhookEvent::PaymentIntentFailure,
        2 => webhook_api::IncomingWebhookEvent::PaymentIntentProcessing,
        3 => webhook_api::IncomingWebhookEvent::PaymentIntentCancelled,
        4 => webhook_api::IncomingWebhookEvent::RefundSuccess,
        5 => webhook_api::IncomingWebhookEvent::RefundFailure,
        6 => webhook_api::IncomingWebhookEvent::MandateRevoked,
        _ => webhook_api::IncomingWebhookEvent::EventNotSupported,
    };

    Ok(WebhookTransformData {
        event_type,
        source_verified: response.source_verified,
        webhook_content: response.content,
        response_ref_id: response.response_ref_id.and_then(|identifier| {
            identifier.id_type.and_then(|id_type| match id_type {
                payments_grpc::identifier::IdType::Id(id) => Some(id),
                payments_grpc::identifier::IdType::EncodedData(encoded_data) => Some(encoded_data),
                payments_grpc::identifier::IdType::NoResponseIdMarker(_) => None,
            })
        }),
    })
}

pub fn build_webhook_secrets_from_merchant_connector_account(
    #[cfg(feature = "v1")] merchant_connector_account: &MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: &MerchantConnectorAccountTypeDetails,
) -> CustomResult<Option<payments_grpc::WebhookSecrets>, UnifiedConnectorServiceError> {
    // Extract webhook credentials from merchant connector account
    // This depends on how webhook secrets are stored in the merchant connector account

    #[cfg(feature = "v1")]
    let webhook_details = merchant_connector_account
        .get_webhook_details()
        .map_err(|_| UnifiedConnectorServiceError::FailedToObtainAuthType)?;

    #[cfg(feature = "v2")]
    let webhook_details = merchant_connector_account
        .get_webhook_details()
        .ok_or(UnifiedConnectorServiceError::FailedToObtainAuthType)?;

    match webhook_details {
        Some(details) => {
            // Parse the webhook details JSON to extract secrets
            let webhook_details: admin::MerchantConnectorWebhookDetails = details
                .clone()
                .parse_value("MerchantConnectorWebhookDetails")
                .change_context(UnifiedConnectorServiceError::FailedToObtainAuthType)
                .attach_printable("Failed to parse MerchantConnectorWebhookDetails")?;

            // Build gRPC WebhookSecrets from parsed details
            Ok(Some(payments_grpc::WebhookSecrets {
                secret: webhook_details.merchant_secret.expose().to_string(),
                additional_secret: webhook_details
                    .additional_secret
                    .map(|secret| secret.expose().to_string()),
            }))
        }
        None => Ok(None),
    }
}

/// High-level abstraction for transforming webhooks via UCS
/// This function encapsulates all UCS communication and request building logic
pub async fn transform_webhook_via_ucs(
    state: &SessionState,
    merchant_context: &MerchantContext,
    connector_name: &str,
    body: &actix_web::web::Bytes,
    request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
    merchant_connector_account: Option<
        &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    >,
) -> RouterResult<(
    api_models::webhooks::IncomingWebhookEvent,
    bool,
    WebhookTransformData,
)> {
    // Build webhook secrets from merchant connector account
    let webhook_secrets = merchant_connector_account.and_then(|mca| {
        #[cfg(feature = "v1")]
        let mca_type = MerchantConnectorAccountType::DbVal(Box::new(mca.clone()));
        #[cfg(feature = "v2")]
        let mca_type = mca.get_connector_account_details().clone();

        build_webhook_secrets_from_merchant_connector_account(&mca_type)
            .ok()
            .flatten()
    });

    // Build UCS transform request
    let transform_request = build_unified_connector_service_webhook_transform_request(
        body, // Pass raw body to UCS (matches proto RequestDetails.body: bytes)
        request_details.headers,
        Some(&request_details.query_params),
        webhook_secrets,
        merchant_context
            .get_merchant_account()
            .get_id()
            .get_string_repr(),
        connector_name,
    )
    .change_context(ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build UCS webhook transform request")?;

    // Build connector auth metadata
    let connector_auth_metadata = merchant_connector_account
        .map(|mca| {
            #[cfg(feature = "v1")]
            let mca_type = MerchantConnectorAccountType::DbVal(Box::new(mca.clone()));
            #[cfg(feature = "v2")]
            let mca_type = mca.get_connector_account_details().clone();

            build_unified_connector_service_auth_metadata(mca_type, merchant_context)
        })
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to build UCS auth metadata")?
        .ok_or_else(|| {
            error_stack::report!(ApiErrorResponse::InternalServerError).attach_printable(
                "Missing merchant connector account for UCS webhook transformation",
            )
        })?;

    // Build gRPC headers
    let grpc_headers = external_services::grpc_client::GrpcHeaders {
        tenant_id: state.tenant.tenant_id.get_string_repr().to_string(),
        request_id: Some(crate::utils::generate_id(consts::ID_LENGTH, "webhook_req")),
    };

    // Make UCS call
    if let Some(ucs_client) = &state.grpc_client.unified_connector_service_client {
        match ucs_client
            .transform_incoming_webhook(transform_request, connector_auth_metadata, grpc_headers)
            .await
        {
            Ok(response) => {
                let transform_response = response.into_inner();
                let transform_data =
                    handle_unified_connector_service_webhook_transform_response(transform_response)
                        .change_context(ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to handle UCS webhook transform response")?;

                // UCS handles everything internally - event type, source verification, decoding
                Ok((
                    transform_data.event_type,
                    transform_data.source_verified,
                    transform_data,
                ))
            }
            Err(err) => {
                // When UCS is configured, we don't fall back to direct connector processing
                // since the goal is to remove direct connector code in the future
                router_env::logger::error!(
                    error = ?err,
                    "UCS webhook transformation failed for UCS-enabled merchant/connector"
                );
                Err(ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable(format!("UCS webhook processing failed: {err}"))
            }
        }
    } else {
        // UCS client not available but UCS is configured
        // We don't fall back to direct connector processing when UCS is configured
        router_env::logger::error!(
            "UCS client not available but UCS is configured for this merchant/connector"
        );
        Err(ApiErrorResponse::WebhookProcessingFailure).attach_printable(
            "UCS webhook processing is configured but UCS client is not available",
        )
    }
}
