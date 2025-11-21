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

use crate::{
    core::{
        payments::gateway::context::RouterGatewayContext,
        unified_connector_service::{
            self, handle_unified_connector_service_response_for_create_order,
        },
    },
    routes::SessionState,
    services::logger,
    types::{self, transformers::ForeignTryFrom},
};

// =============================================================================
// PaymentGateway Implementation for domain::CreateOrder
// =============================================================================

/// Implementation of PaymentGateway for api::CreateOrder flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        Self,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::CreateOrder
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::CreateOrderRequestData, types::PaymentsResponseData>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<Self, types::CreateOrderRequestData, types::PaymentsResponseData>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<Self, types::CreateOrderRequestData, types::PaymentsResponseData>,
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

        let create_order_request =
            payments_grpc::PaymentServiceCreateOrderRequest::foreign_try_from(router_data)
                .change_context(ConnectorError::RequestEncodingFailed)
                .attach_printable("Failed to construct Payment Create Order Request")?;

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                &platform,
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
        let header_payload = state
            .get_grpc_headers_ucs(unified_connector_service_execution_mode)
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(merchant_reference_id)
            .lineage_ids(lineage_ids);
        let updated_router_data = Box::pin(unified_connector_service::ucs_logging_wrapper_new(
            router_data.clone(),
            state,
            create_order_request,
            header_payload,
            |mut router_data, create_order_request, grpc_headers| async move {
                let response = Box::pin(client.payment_create_order(
                    create_order_request,
                    connector_auth_metadata,
                    grpc_headers,
                ))
                .await
                .attach_printable("Failed to create order")?;

                let create_order_response = response.into_inner();

                let (router_data_response, _status_code) =
                    handle_unified_connector_service_response_for_create_order(
                        create_order_response.clone(),
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                let router_data_response = router_data_response.map(|(response, status)| {
                    router_data.status = status;
                    response
                });
                router_data.response = router_data_response;
                Ok((router_data, create_order_response))
            },
        ))
        .await
        .change_context(ConnectorError::ResponseHandlingFailed)?;

        Ok(updated_router_data)
    }
}

/// Implementation of FlowGateway for api::CreateOrder
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::CreateOrder
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::CreateOrderRequestData, types::PaymentsResponseData>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::CreateOrderRequestData,
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
