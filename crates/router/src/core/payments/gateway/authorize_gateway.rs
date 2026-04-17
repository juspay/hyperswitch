use std::str::FromStr;

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, id_type, request::Request, ucs_types};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{router_data::RouterData, router_flow_types as domain};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
    unified_connector_service::transformers::UnifiedConnectorServiceError,
};
use hyperswitch_masking::ExposeInterface as UcsMaskingExposeInterface;
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    core::{
        payments::gateway::context::RouterGatewayContext,
        unified_connector_service::{
            self, handle_unified_connector_service_response_for_payment_authorize,
            handle_unified_connector_service_response_for_recurring_payment_charge,
        },
    },
    routes::SessionState,
    services::logger,
    types::{self, transformers::ForeignTryFrom, MinorUnit},
};

// =============================================================================
// PaymentGateway Implementation for domain::Authorize
// =============================================================================

/// Implementation of PaymentGateway for api::PSync flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        Self,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Authorize
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<Self, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<Self, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        let merchant_connector_account = context.merchant_connector_account;
        let processor = &context.processor;
        let lineage_ids = context.lineage_ids;
        let header_payload = context.header_payload;
        let unified_connector_service_execution_mode = context.execution_mode;
        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        // Check if this is a MIT payment (MIT with mandate_id or MandatePayment)
        let is_mit_payment = router_data.request.mandate_id.is_some()
            || matches!(
                router_data.request.payment_method_data,
                hyperswitch_domain_models::payment_method_data::PaymentMethodData::MandatePayment
            );

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                processor,
                router_data.connector.clone(),
            )
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct request metadata")?;

        let merchant_reference_id = unified_connector_service::parse_merchant_reference_id(
            header_payload
                .x_reference_id
                .as_deref()
                .unwrap_or(router_data.payment_id.as_str()),
        )
        .map(ucs_types::UcsReferenceId::Payment);
        let resource_id = id_type::PaymentResourceId::from_str(router_data.attempt_id.as_str())
            .inspect_err(
                |err| logger::warn!(error=?err, "Invalid Payment AttemptId for UCS resource id"),
            )
            .ok()
            .map(ucs_types::UcsResourceId::PaymentAttempt);

        let grpc_headers = state
            .get_grpc_headers_ucs(unified_connector_service_execution_mode)
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(merchant_reference_id)
            .resource_id(resource_id)
            .lineage_ids(lineage_ids);

        let updated_router_data = if is_mit_payment {
            logger::info!(
                "Granular Gateway: Detected MIT payment, calling UCS recurring_payment_charge endpoint"
            );

            let recurring_payment_charge_request =
                payments_grpc::RecurringPaymentServiceChargeRequest::foreign_try_from(router_data)
                    .change_context(ConnectorError::RequestEncodingFailed)
                    .attach_printable("Failed to construct Recurring Payment Charge Request")?;

            Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
                router_data.clone(),
                state,
                recurring_payment_charge_request,
                grpc_headers,
                unified_connector_service_execution_mode,
                |mut router_data, recurring_payment_charge_request, grpc_headers| async move {
                    let response = match Box::pin(client.recurring_payment_charge(
                        recurring_payment_charge_request,
                        connector_auth_metadata,
                        grpc_headers,
                    ))
                    .await
                    {
                        Ok(resp) => resp,
                        Err(report) => {
                            // Check if this is a connector error (4xx/5xx from connector via UCS)
                            if let UnifiedConnectorServiceError::ConnectorError {
                                code,
                                message,
                                status_code,
                                reason,
                            } = report.current_context()
                            {
                                logger::info!(
                                    "Connector error via UCS for recurring charge (status {}): {} - {}",
                                    status_code,
                                    code,
                                    message
                                );
                                router_data.response =
                                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                                        code: code.clone(),
                                        message: message.clone(),
                                        reason: reason.clone(),
                                        status_code: *status_code,
                                        attempt_status: None,
                                        connector_transaction_id: None,
                                        connector_response_reference_id: None,
                                        network_decline_code: None,
                                        network_advice_code: None,
                                        network_error_message: None,
                                        connector_metadata: None,
                                    });
                                return Ok((
                                    router_data,
                                    (),
                                    payments_grpc::RecurringPaymentServiceChargeResponse::default(),
                                ));
                            }
                            // UCS validation errors (4xx) - propagate as Err
                            // so the API layer returns proper HTTP 4xx response
                            return Err(report.attach_printable("Failed to charge recurring payment"));
                        }
                    };

                    let recurring_payment_charge_response = response.into_inner();

                    let ucs_data =
                        handle_unified_connector_service_response_for_recurring_payment_charge(
                            recurring_payment_charge_response.clone(),
                            router_data.status,
                        )
                        .attach_printable("Failed to deserialize UCS response")?;

                    let router_data_response = match ucs_data.router_data_response {
                        Ok((response, status)) => {
                            router_data.status = status;
                            Ok(response)
                        }
                        Err(err) => {
                            logger::debug!("Error in UCS router data response");
                            if let Some(attempt_status) = err.attempt_status {
                                router_data.status = attempt_status;
                            }
                            Err(err)
                        }
                    };
                    router_data.response = router_data_response;

                    router_data.amount_captured = recurring_payment_charge_response.captured_amount;
                    router_data.minor_amount_captured = recurring_payment_charge_response
                        .captured_amount
                        .map(MinorUnit::new);
                    router_data.raw_connector_response = recurring_payment_charge_response
                        .raw_connector_response
                        .clone()
                        .map(|raw_connector_response| raw_connector_response.expose().into());
                    router_data.connector_http_status_code = Some(ucs_data.status_code);

                    ucs_data.connector_customer_id.map(|connector_customer_id| {
                        router_data.connector_customer = Some(connector_customer_id);
                    });

                    ucs_data.connector_response.map(|connector_response| {
                        router_data.connector_response = Some(connector_response);
                    });

                    Ok((router_data, (), recurring_payment_charge_response))
                },
            ))
            .await
            .map(|(router_data, _)| router_data)
            .map_err(|report| {
                convert_ucs_error_to_connector_error(report)
            })?
        } else {
            logger::debug!("Granular Gateway: Regular authorize flow");
            let granular_authorize_request =
                payments_grpc::PaymentServiceAuthorizeRequest::foreign_try_from((
                    router_data,
                    call_connector_action,
                ))
                .change_context(ConnectorError::RequestEncodingFailed)
                .attach_printable("Failed to construct Payment Authorize Request")?;

            Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
                router_data.clone(),
                state,
                granular_authorize_request,
                grpc_headers,
                unified_connector_service_execution_mode,
                |mut router_data, granular_authorize_request, grpc_headers| async move {
                    let response = match Box::pin(client.payment_authorize(
                        granular_authorize_request,
                        connector_auth_metadata,
                        grpc_headers,
                    ))
                    .await
                    {
                        Ok(resp) => resp,
                        Err(report) => {
                            // Check if this is a connector error (4xx/5xx from connector via UCS)
                            // If so, set it as router_data.response = Err(ErrorResponse) and return Ok
                            // This matches how direct connector errors are handled
                            if let UnifiedConnectorServiceError::ConnectorError {
                                code,
                                message,
                                status_code,
                                reason,
                            } = report.current_context()
                            {
                                logger::info!(
                                    "Connector error via UCS (status {}): {} - {}",
                                    status_code,
                                    code,
                                    message
                                );
                                router_data.response =
                                    Err(hyperswitch_domain_models::router_data::ErrorResponse {
                                        code: code.clone(),
                                        message: message.clone(),
                                        reason: reason.clone(),
                                        status_code: *status_code,
                                        attempt_status: None,
                                        connector_transaction_id: None,
                                        connector_response_reference_id: None,
                                        network_decline_code: None,
                                        network_advice_code: None,
                                        network_error_message: None,
                                        connector_metadata: None,
                                    });
                                // Return Ok with router_data containing the error response
                                // This ensures the connector error flows through the normal
                                // response handling path (same as direct connector errors)
                                return Ok((
                                    router_data,
                                    (),
                                    payments_grpc::PaymentServiceAuthorizeResponse::default(),
                                ));
                            }
                            // For UCS validation errors (TonicInvalidArgument, etc.)
                            // propagate as Err so they become HTTP 4xx at the API layer
                            return Err(report.attach_printable("Failed to authorize payment"));
                        }
                    };

                    let payment_authorize_response = response.into_inner();

                    let ucs_data = handle_unified_connector_service_response_for_payment_authorize(
                        payment_authorize_response.clone(),
                        router_data.status,
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                    let router_data_response = match ucs_data.router_data_response {
                        Ok((response, status)) => {
                            router_data.status = status;
                            Ok(response)
                        }
                        Err(err) => {
                            logger::debug!("Error in UCS router data response");
                            if let Some(attempt_status) = err.attempt_status {
                                router_data.status = attempt_status;
                            }
                            Err(err)
                        }
                    };
                    router_data.response = router_data_response;

                    router_data.amount_captured = payment_authorize_response.captured_amount;
                    router_data.minor_amount_captured = payment_authorize_response
                        .captured_amount
                        .map(MinorUnit::new);
                    router_data.minor_amount_capturable = payment_authorize_response
                        .capturable_amount
                        .map(MinorUnit::new);
                    router_data.raw_connector_response = payment_authorize_response
                        .raw_connector_response
                        .clone()
                        .map(|raw_connector_response| raw_connector_response.expose().into());
                    router_data.connector_http_status_code = Some(ucs_data.status_code);

                    ucs_data.connector_response.map(|connector_response| {
                        router_data.connector_response = Some(connector_response);
                    });

                    Ok((router_data, (), payment_authorize_response))
                },
            ))
            .await
            .map(|(router_data, _)| router_data)
            .map_err(|report| convert_ucs_error_to_connector_error(report))?
        };

        Ok(updated_router_data)
    }
}

/// Implementation of FlowGateway for api::PSync
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Authorize
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => Box::new(payment_gateway::DirectGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => Box::new(Self),
        }
    }
}

/// Maps a `UnifiedConnectorServiceError` to an (error_code, error_message, http_status_code) tuple.
///
/// This is used to convert UCS validation errors (non-connector errors) into proper HTTP error
/// responses instead of blanket 500s. The mapping follows the tonic gRPC status → HTTP status
/// convention:
///
/// - InvalidArgument / FailedPrecondition → 400
/// - Unauthenticated → 401
/// - PermissionDenied → 403
/// - NotFound → 404
/// - AlreadyExists → 409
/// - Unimplemented → 501
/// - Unavailable → 503
/// - DeadlineExceeded → 504
/// - Internal / others → 500
fn map_ucs_error_to_response(error: &UnifiedConnectorServiceError) -> (String, String, u16) {
    match error {
        UnifiedConnectorServiceError::TonicInvalidArgument { message } => {
            ("UCS_400".to_string(), message.clone(), 400)
        }
        UnifiedConnectorServiceError::TonicNotFound { message } => {
            ("UCS_404".to_string(), message.clone(), 404)
        }
        UnifiedConnectorServiceError::TonicAlreadyExists { message } => {
            ("UCS_409".to_string(), message.clone(), 409)
        }
        UnifiedConnectorServiceError::TonicPermissionDenied { message } => {
            ("UCS_403".to_string(), message.clone(), 403)
        }
        UnifiedConnectorServiceError::TonicUnauthenticated { message } => {
            ("UCS_401".to_string(), message.clone(), 401)
        }
        UnifiedConnectorServiceError::TonicFailedPrecondition { message } => {
            ("UCS_400".to_string(), message.clone(), 400)
        }
        UnifiedConnectorServiceError::TonicUnimplemented { message } => {
            ("UCS_501".to_string(), message.clone(), 501)
        }
        UnifiedConnectorServiceError::TonicUnavailable { message } => {
            ("UCS_503".to_string(), message.clone(), 503)
        }
        UnifiedConnectorServiceError::TonicDeadlineExceeded { message } => {
            ("UCS_504".to_string(), message.clone(), 504)
        }
        UnifiedConnectorServiceError::TonicInternal { message } => {
            ("UCS_500".to_string(), message.clone(), 500)
        }
        // All other legacy/generic UCS errors → 500
        other => ("UCS_500".to_string(), format!("{other}"), 500),
    }
}

fn convert_ucs_error_to_connector_error(
    report: Report<UnifiedConnectorServiceError>,
) -> Report<ConnectorError> {
    let ucs_error = report.current_context();

    // Check if this is a UCS validation error (tonic 4xx equivalent)
    match ucs_error {
        UnifiedConnectorServiceError::TonicInvalidArgument { .. }
        | UnifiedConnectorServiceError::TonicNotFound { .. }
        | UnifiedConnectorServiceError::TonicAlreadyExists { .. }
        | UnifiedConnectorServiceError::TonicPermissionDenied { .. }
        | UnifiedConnectorServiceError::TonicFailedPrecondition { .. }
        | UnifiedConnectorServiceError::TonicUnimplemented { .. } => {
            let (_code, _message, status_code) = map_ucs_error_to_response(ucs_error);
            let error_body = serde_json::json!({
                "code": _code,
                "message": _message,
                "status_code": status_code,
                "type": "ucs_validation_error"
            });
            report.change_context(ConnectorError::ProcessingStepFailed(Some(
                bytes::Bytes::from(error_body.to_string()),
            )))
        }
        UnifiedConnectorServiceError::TonicUnauthenticated { .. } => {
            report.change_context(ConnectorError::FailedToObtainAuthType)
        }
        // Server errors and other failures → generic handling
        _ => report.change_context(ConnectorError::ResponseHandlingFailed),
    }
}
