use api_models::admin;
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
use router_env::logger;
use unified_connector_service_client::payments::{
    self as payments_grpc, payment_method::PaymentMethod, CardDetails, CardPaymentMethodType,
    PaymentServiceAuthorizeResponse,
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
    utils,
};

pub mod transformers;

// Re-export webhook transformer types for easier access
pub use transformers::WebhookTransformData;

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
        logger::debug!(
            connector = connector_name.to_string(),
            "Unified Connector Service client is not available for webhooks"
        );
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
    (
        AttemptStatus,
        Result<PaymentsResponseData, ErrorResponse>,
        u16,
    ),
    UnifiedConnectorServiceError,
> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((status, router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_get(
    response: payments_grpc::PaymentServiceGetResponse,
) -> CustomResult<
    (
        AttemptStatus,
        Result<PaymentsResponseData, ErrorResponse>,
        u16,
    ),
    UnifiedConnectorServiceError,
> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((status, router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_register(
    response: payments_grpc::PaymentServiceRegisterResponse,
) -> CustomResult<
    (
        AttemptStatus,
        Result<PaymentsResponseData, ErrorResponse>,
        u16,
    ),
    UnifiedConnectorServiceError,
> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((status, router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_repeat(
    response: payments_grpc::PaymentServiceRepeatEverythingResponse,
) -> CustomResult<
    (
        AttemptStatus,
        Result<PaymentsResponseData, ErrorResponse>,
        u16,
    ),
    UnifiedConnectorServiceError,
> {
    let status = AttemptStatus::foreign_try_from(response.status())?;

    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((status, router_data_response, status_code))
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
    let webhook_details = match merchant_connector_account {
        MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(mca) => {
            mca.connector_webhook_details.as_ref()
        }
        MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => None,
    };

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

/// High-level abstraction for calling UCS webhook transformation
/// This provides a clean interface similar to payment flow UCS calls
pub async fn call_unified_connector_service_for_webhook(
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
    let ucs_client = state
        .grpc_client
        .unified_connector_service_client
        .as_ref()
        .ok_or_else(|| {
            error_stack::report!(ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("UCS client is not available for webhook processing")
        })?;

    // Build webhook secrets from merchant connector account
    let webhook_secrets = merchant_connector_account.and_then(|mca| {
        #[cfg(feature = "v1")]
        let mca_type = MerchantConnectorAccountType::DbVal(Box::new(mca.clone()));
        #[cfg(feature = "v2")]
        let mca_type =
            MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(mca.clone()));

        build_webhook_secrets_from_merchant_connector_account(&mca_type)
            .map_err(|e| {
                logger::warn!(
                    build_error=?e,
                    connector_name=connector_name,
                    "Failed to build webhook secrets from merchant connector account in call_unified_connector_service_for_webhook"
                );
                e
            })
            .ok()
            .flatten()
    });

    // Build UCS transform request using new webhook transformers
    let transform_request = transformers::build_webhook_transform_request(
        body,
        request_details,
        webhook_secrets,
        merchant_context
            .get_merchant_account()
            .get_id()
            .get_string_repr(),
        connector_name,
    )?;

    // Build connector auth metadata
    let connector_auth_metadata = merchant_connector_account
        .map(|mca| {
            #[cfg(feature = "v1")]
            let mca_type = MerchantConnectorAccountType::DbVal(Box::new(mca.clone()));
            #[cfg(feature = "v2")]
            let mca_type = MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
                mca.clone(),
            ));

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
        request_id: Some(utils::generate_id(consts::ID_LENGTH, "webhook_req")),
    };

    // Make UCS call - client availability already verified
    match ucs_client
        .transform_incoming_webhook(transform_request, connector_auth_metadata, grpc_headers)
        .await
    {
        Ok(response) => {
            let transform_response = response.into_inner();
            let transform_data = transformers::transform_ucs_webhook_response(transform_response)?;

            // UCS handles everything internally - event type, source verification, decoding
            Ok((
                transform_data.event_type,
                transform_data.source_verified,
                transform_data,
            ))
        }
        Err(err) => {
            // When UCS is configured, we don't fall back to direct connector processing
            Err(ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable(format!("UCS webhook processing failed: {err}"))
        }
    }
}

/// Extract webhook content from UCS response for further processing
/// This provides a helper function to extract specific data from UCS responses
pub fn extract_webhook_content_from_ucs_response(
    transform_data: &WebhookTransformData,
) -> Option<&unified_connector_service_client::payments::WebhookResponseContent> {
    transform_data.webhook_content.as_ref()
}
