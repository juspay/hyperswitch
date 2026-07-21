use common_enums::{ExecutionMode, ExecutionPath};
use common_utils::errors::CustomResult;
use error_stack::{Report, ResultExt};
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{
    router_request_types::merchant_connector_webhook_management::{
        ConnectorWebhookRegisterRequest, ScopeIdentifier,
    },
    router_response_types::merchant_connector_webhook_management::ConnectorWebhookRegisterResponse,
};
use hyperswitch_interfaces::errors::ConnectorError;
use hyperswitch_masking::{ExposeInterface, Secret};
use router_env::{logger, tracing::Instrument};
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    core::{
        payments::{
            gateway::convert_ucs_error_to_connector_error, helpers::MerchantConnectorAccountType,
        },
        unified_connector_service::{
            self, build_unified_connector_service_auth_metadata,
            transformers::convert_connector_service_status_code,
        },
    },
    routes::SessionState,
    services,
    types::{
        self,
        api::{self, ConnectorWebhookRegister},
        domain,
        transformers::ForeignFrom,
    },
};

type WebhookRegisterRouterData = types::RouterData<
    ConnectorWebhookRegister,
    ConnectorWebhookRegisterRequest,
    ConnectorWebhookRegisterResponse,
>;

pub async fn execute_connector_webhook_register(
    state: &SessionState,
    platform: &domain::Platform,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    connector_data: &api::ConnectorData,
    router_data: &WebhookRegisterRouterData,
    execution_path: ExecutionPath,
) -> CustomResult<WebhookRegisterRouterData, ConnectorError> {
    let connector_integration = connector_data.connector.get_connector_integration();

    match execution_path {
        ExecutionPath::Direct => execute_direct(state, connector_integration, router_data).await,
        ExecutionPath::UnifiedConnectorService => {
            execute_ucs(
                state,
                platform,
                merchant_connector_account,
                router_data.clone(),
                ExecutionMode::Primary,
            )
            .await
        }
        ExecutionPath::ShadowUnifiedConnectorService => {
            let direct_result =
                execute_direct(state, connector_integration.clone_box(), router_data).await;

            let direct_for_compare = direct_result
                .as_ref()
                .map(Clone::clone)
                .map_err(|error| format!("{error:?}"));
            let state = state.clone();
            let platform = platform.clone();
            let merchant_connector_account = merchant_connector_account.clone();
            let router_data = router_data.clone();
            let connector_name = router_data.connector.clone();

            tokio::spawn(
                async move {
                    let ucs_result = execute_ucs(
                        &state,
                        &platform,
                        &merchant_connector_account,
                        router_data,
                        ExecutionMode::Shadow,
                    )
                    .await;

                    if let Err(error) = &ucs_result {
                        logger::error!(?error, "UCS webhook registration shadow call failed");
                    }

                    let ucs_for_compare = ucs_result.map_err(|error| format!("{error:?}"));
                    hyperswitch_interfaces::helpers::serialize_comparison_results_and_send(
                        &state,
                        connector_name,
                        direct_for_compare,
                        ucs_for_compare,
                    )
                    .await;
                }
                .instrument(router_env::tracing::Span::current()),
            );

            direct_result
        }
    }
}

async fn execute_direct(
    state: &SessionState,
    connector_integration: services::BoxedConnectorWebhookConfigurationInterface<
        ConnectorWebhookRegister,
        ConnectorWebhookRegisterRequest,
        ConnectorWebhookRegisterResponse,
    >,
    router_data: &WebhookRegisterRouterData,
) -> CustomResult<WebhookRegisterRouterData, ConnectorError> {
    services::execute_connector_processing_step(
        state,
        connector_integration,
        router_data,
        common_enums::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
}

async fn execute_ucs(
    state: &SessionState,
    platform: &domain::Platform,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    router_data: WebhookRegisterRouterData,
    execution_mode: ExecutionMode,
) -> CustomResult<WebhookRegisterRouterData, ConnectorError> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to fetch Unified Connector Service client")?;

    let request = build_ucs_request(&router_data)
        .change_context(ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to construct connector webhook register request")?;
    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        MerchantConnectorAccountType::DbVal(Box::new(merchant_connector_account.clone())),
        platform.get_processor().get_account().get_id(),
        router_data.connector.clone(),
    )
    .change_context(ConnectorError::RequestEncodingFailed)
    .attach_printable("Failed to construct UCS webhook registration metadata")?;
    let grpc_headers = state
        .get_grpc_headers_ucs(execution_mode)
        .lineage_ids(LineageIds::new(
            merchant_connector_account.merchant_id.clone(),
            merchant_connector_account.profile_id.clone(),
        ))
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None);

    unified_connector_service::ucs_logging_wrapper_granular(
        router_data,
        state,
        request,
        grpc_headers,
        execution_mode,
        |mut router_data, request, grpc_headers| async move {
            let response = client
                .connector_webhook_register(request, connector_auth_metadata, grpc_headers)
                .await?
                .into_inner();

            let status_code = convert_connector_service_status_code(response.status_code)?;
            router_data.connector_http_status_code = Some(status_code);
            router_data.response = convert_ucs_response(&router_data.request.scope, &response)?;

            Ok((router_data, (), response))
        },
    )
    .await
    .map(|(router_data, ())| router_data)
    .map_err(convert_ucs_error_to_connector_error)
}

fn build_ucs_request(
    router_data: &WebhookRegisterRouterData,
) -> Result<payments_grpc::ConnectorWebhookRegisterRequest, Report<ConnectorError>> {
    Ok(payments_grpc::ConnectorWebhookRegisterRequest {
        scope: Some(build_ucs_scope(&router_data.request.scope)),
        webhook_url: Some(Secret::new(
            router_data.request.webhook_url.clone().expose().to_string(),
        )),
        connector_webhook_registration_url: router_data.request.base_url.to_string(),
        state: router_data
            .access_token
            .as_ref()
            .map(payments_grpc::ConnectorState::foreign_from),
    })
}

fn build_ucs_scope(scope: &ScopeIdentifier) -> payments_grpc::ConnectorWebhookRegistrationScope {
    use payments_grpc::connector_webhook_registration_scope::Identifier;

    let identifier = match scope {
        ScopeIdentifier::NotSpecific => Identifier::NotSpecific(()),
        ScopeIdentifier::PaymentMethodType(payment_method_type) => Identifier::PaymentMethodType(
            payments_grpc::PaymentMethodType::foreign_from(*payment_method_type) as i32,
        ),
        ScopeIdentifier::EventType(event_type) => {
            Identifier::EventType(build_ucs_event_type(*event_type) as i32)
        }
        ScopeIdentifier::EventTypes(event_types) => {
            Identifier::EventTypes(payments_grpc::ConnectorWebhookRegistrationEventTypes {
                event_types: event_types
                    .iter()
                    .map(|event_type| build_ucs_event_type(*event_type) as i32)
                    .collect(),
            })
        }
    };

    payments_grpc::ConnectorWebhookRegistrationScope {
        identifier: Some(identifier),
    }
}

fn build_ucs_event_type(
    event_type: common_enums::EventType,
) -> payments_grpc::ConnectorWebhookRegistrationEventType {
    use common_enums::EventType as Event;
    use payments_grpc::ConnectorWebhookRegistrationEventType as GrpcEvent;

    match event_type {
        Event::PaymentSucceeded => GrpcEvent::PaymentSucceeded,
        Event::PaymentFailed => GrpcEvent::PaymentFailed,
        Event::PaymentProcessing => GrpcEvent::PaymentProcessing,
        Event::PaymentCancelled => GrpcEvent::PaymentCancelled,
        Event::PaymentCancelledPostCapture => GrpcEvent::PaymentCancelledPostCapture,
        Event::PaymentAuthorized => GrpcEvent::PaymentAuthorized,
        Event::PaymentPartiallyAuthorized => GrpcEvent::PaymentPartiallyAuthorized,
        Event::PaymentCaptured => GrpcEvent::PaymentCaptured,
        Event::PaymentExpired => GrpcEvent::PaymentExpired,
        Event::ActionRequired => GrpcEvent::ActionRequired,
        Event::RefundProcessing => GrpcEvent::RefundProcessing,
        Event::RefundSucceeded => GrpcEvent::RefundSucceeded,
        Event::RefundFailed => GrpcEvent::RefundFailed,
        Event::DisputeOpened => GrpcEvent::DisputeOpened,
        Event::DisputeExpired => GrpcEvent::DisputeExpired,
        Event::DisputeAccepted => GrpcEvent::DisputeAccepted,
        Event::DisputeCancelled => GrpcEvent::DisputeCancelled,
        Event::DisputeChallenged => GrpcEvent::DisputeChallenged,
        Event::DisputeWon => GrpcEvent::DisputeWon,
        Event::DisputeLost => GrpcEvent::DisputeLost,
        Event::MandateActive => GrpcEvent::MandateActive,
        Event::MandateRevoked => GrpcEvent::MandateRevoked,
        #[cfg(feature = "payouts")]
        Event::PayoutSuccess => GrpcEvent::PayoutSuccess,
        #[cfg(feature = "payouts")]
        Event::PayoutFailed => GrpcEvent::PayoutFailed,
        #[cfg(feature = "payouts")]
        Event::PayoutInitiated => GrpcEvent::PayoutInitiated,
        #[cfg(feature = "payouts")]
        Event::PayoutProcessing => GrpcEvent::PayoutProcessing,
        #[cfg(feature = "payouts")]
        Event::PayoutCancelled => GrpcEvent::PayoutCancelled,
        #[cfg(feature = "payouts")]
        Event::PayoutExpired => GrpcEvent::PayoutExpired,
        #[cfg(feature = "payouts")]
        Event::PayoutReversed => GrpcEvent::PayoutReversed,
        Event::InvoicePaid => GrpcEvent::InvoicePaid,
        Event::SurchargePaymentSucceeded => GrpcEvent::SurchargePaymentSucceeded,
        Event::SurchargeRefundSucceeded => GrpcEvent::SurchargeRefundSucceeded,
    }
}

fn convert_ucs_response(
    scope: &ScopeIdentifier,
    response: &payments_grpc::ConnectorWebhookRegisterResponse,
) -> CustomResult<
    Result<ConnectorWebhookRegisterResponse, types::ErrorResponse>,
    external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceError,
> {
    let status_code = convert_connector_service_status_code(response.status_code)?;

    if let Some(error) = response.error.as_ref() {
        let connector_details = error.connector_details.as_ref();
        return Ok(Err(types::ErrorResponse {
            code: connector_details
                .and_then(|details| details.code.clone())
                .unwrap_or_else(|| "CONNECTOR_WEBHOOK_REGISTER_FAILED".to_string()),
            message: connector_details
                .and_then(|details| details.message.clone())
                .unwrap_or_else(|| "Connector webhook registration failed".to_string()),
            reason: connector_details.and_then(|details| details.reason.clone()),
            status_code,
            attempt_status: None,
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        }));
    }

    let metadata = response
        .metadata
        .clone()
        .map(|metadata| {
            serde_json::from_str(&metadata.expose())
                .map(common_utils::pii::SecretSerdeValue::new)
                .change_context(
                    external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceError::ResponseDeserializationFailed,
                )
        })
        .transpose()?;

    Ok(Ok(ConnectorWebhookRegisterResponse {
        identifier: scope.clone(),
        status: match response.status() {
            payments_grpc::ConnectorWebhookRegistrationStatus::Success => {
                common_enums::WebhookRegistrationStatus::Success
            }
            payments_grpc::ConnectorWebhookRegistrationStatus::Failure
            | payments_grpc::ConnectorWebhookRegistrationStatus::Unspecified => {
                common_enums::WebhookRegistrationStatus::Failure
            }
        },
        connector_webhook_id: response.connector_webhook_id.clone(),
        connector_webhook_secret: response.connector_webhook_secret.clone(),
        error_code: None,
        error_message: None,
        metadata,
    }))
}
