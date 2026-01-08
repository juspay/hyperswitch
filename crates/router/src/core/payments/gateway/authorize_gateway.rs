use std::str::FromStr;

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, id_type, request::Request, ucs_types};
use error_stack::ResultExt;
use hyperswitch_domain_models::{router_data::RouterData, router_flow_types as domain};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use unified_connector_service_client::payments as payments_grpc;
use unified_connector_service_masking::ExposeInterface as UcsMaskingExposeInterface;

use crate::{
    core::{
        payments::gateway::context::RouterGatewayContext,
        unified_connector_service::{
            self, handle_unified_connector_service_response_for_payment_authorize,
            handle_unified_connector_service_response_for_payment_repeat,
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
        let platform = context.platform;
        let lineage_ids = context.lineage_ids;
        let header_payload = context.header_payload;
        let unified_connector_service_execution_mode = context.execution_mode;
        let merchant_order_reference_id = header_payload.x_reference_id.clone();
        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        // Check if this is a repeat payment (MIT with mandate_id or MandatePayment)
        let is_repeat_payment = router_data.request.mandate_id.is_some()
            || matches!(
                router_data.request.payment_method_data,
                hyperswitch_domain_models::payment_method_data::PaymentMethodData::MandatePayment
            );

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                &platform,
                router_data.connector.clone(),
            )
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct request metadata")?;

        let merchant_reference_id = header_payload
            .x_reference_id
            .clone()
            .or(merchant_order_reference_id)
            .map(|id| id_type::PaymentReferenceId::from_str(id.as_str()))
            .transpose()
            .inspect_err(|err| logger::warn!(error=?err, "Invalid Merchant ReferenceId found"))
            .ok()
            .flatten()
            .map(ucs_types::UcsReferenceId::Payment);

        let grpc_headers = state
            .get_grpc_headers_ucs(unified_connector_service_execution_mode)
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(merchant_reference_id)
            .lineage_ids(lineage_ids);

        let updated_router_data = if is_repeat_payment {
            logger::info!(
                "Granular Gateway: Detected repeat payment, calling UCS RepeatPayment endpoint"
            );

            let payment_repeat_request =
                payments_grpc::PaymentServiceRepeatEverythingRequest::foreign_try_from(router_data)
                    .change_context(ConnectorError::RequestEncodingFailed)
                    .attach_printable("Failed to construct Payment Repeat Request")?;

            Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
                router_data.clone(),
                state,
                payment_repeat_request,
                grpc_headers,
                |mut router_data, payment_repeat_request, grpc_headers| async move {
                    logger::debug!("Calling UCS payment_repeat gRPC method");
                    let response = Box::pin(client.payment_repeat(
                        payment_repeat_request,
                        connector_auth_metadata,
                        grpc_headers,
                    ))
                    .await
                    .attach_printable("Failed to repeat payment")?;

                    let payment_repeat_response = response.into_inner();

                    let ucs_data = handle_unified_connector_service_response_for_payment_repeat(
                        payment_repeat_response.clone(),
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                    let router_data_response = match ucs_data.router_data_response {
                        Ok((response, status)) => {
                            router_data.status = status;
                            Ok(response)
                        }
                        Err(err) => {
                            logger::debug!("Error in UCS router data response");
                            if let Some(attempt_status) = err.attempt_status.clone() {
                                router_data.status = attempt_status;
                            }
                            Err(err)
                        }
                    };
                    router_data.response = router_data_response;

                    router_data.amount_captured = payment_repeat_response.captured_amount;
                    router_data.minor_amount_captured = payment_repeat_response
                        .minor_captured_amount
                        .map(MinorUnit::new);
                    router_data.raw_connector_response = payment_repeat_response
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

                    Ok((router_data, (), payment_repeat_response))
                },
            ))
            .await
            .map(|(router_data, _)| router_data)
            .change_context(ConnectorError::ResponseHandlingFailed)?
        } else {
            logger::debug!("Granular Gateway: Regular authorize flow");
            let granular_authorize_request =
                payments_grpc::PaymentServiceAuthorizeOnlyRequest::foreign_try_from((
                    router_data,
                    call_connector_action,
                ))
                .change_context(ConnectorError::RequestEncodingFailed)
                .attach_printable("Failed to construct Payment Get Request")?;

            Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
                router_data.clone(),
                state,
                granular_authorize_request,
                grpc_headers,
                |mut router_data, granular_authorize_request, grpc_headers| async move {
                    let response = Box::pin(client.payment_authorize_granular(
                        granular_authorize_request,
                        connector_auth_metadata,
                        grpc_headers,
                    ))
                    .await
                    .attach_printable("Failed to get payment")?;

                    let payment_authorize_response = response.into_inner();

                    let ucs_data = handle_unified_connector_service_response_for_payment_authorize(
                        payment_authorize_response.clone(),
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                    let router_data_response = match ucs_data.router_data_response {
                        Ok((response, status)) => {
                            router_data.status = status;
                            Ok(response)
                        }
                        Err(err) => {
                            logger::debug!("Error in UCS router data response");
                            if let Some(attempt_status) = err.attempt_status.clone() {
                                router_data.status = attempt_status;
                            }
                            Err(err)
                        }
                    };
                    router_data.response = router_data_response;

                    router_data.amount_captured = payment_authorize_response.captured_amount;
                    router_data.minor_amount_captured = payment_authorize_response
                        .minor_captured_amount
                        .map(MinorUnit::new);
                    router_data.minor_amount_capturable = payment_authorize_response
                        .minor_capturable_amount
                        .map(MinorUnit::new);
                    router_data.raw_connector_response = payment_authorize_response
                        .raw_connector_response
                        .clone()
                        .map(|raw_connector_response| raw_connector_response.expose().into());
                    router_data.connector_http_status_code = Some(ucs_data.status_code);

                    ucs_data.connector_response.map(|customer_response| {
                        router_data.connector_response = Some(customer_response);
                    });

                    Ok((router_data, (), payment_authorize_response))
                },
            ))
            .await
            .map(|(router_data, _)| router_data)
            .change_context(ConnectorError::ResponseHandlingFailed)?
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
