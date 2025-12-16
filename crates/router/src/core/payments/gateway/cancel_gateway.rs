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
// PaymentGateway Implementation for domain::Void
// =============================================================================

/// Implementation of PaymentGateway for api::Void flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        Self,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Void
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PaymentsCancelData, types::PaymentsResponseData>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            types::PaymentsCancelData
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<Self, types::PaymentsCancelData, types::PaymentsResponseData>,
        call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<Self, types::PaymentsCancelData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        let payment_void_request =
            payments_grpc::PaymentServiceVoidRequest::foreign_try_from(&*self)
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to construct Payment Void Request")?;

        let connector_auth_metadata = build_unified_connector_service_auth_metadata(
            merchant_connector_account,
            platform,
            self.connector.clone(),
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

        let (updated_router_data, _) = Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
            self.clone(),
            state,
            payment_void_request,
            header_payload,
            |mut router_data, payment_void_request, grpc_headers| async move {
                let response = client
                    .payment_cancel(payment_void_request, connector_auth_metadata, grpc_headers)
                    .await
                    .attach_printable("Failed to Cancel payment")?;

                let payment_void_response = response.into_inner();

                let (router_data_response, status_code) =
                    handle_unified_connector_service_response_for_payment_cancel(
                        payment_void_response.clone(),
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                let router_data_response = router_data_response.map(|(response, status)| {
                    router_data.status = status;
                    response
                });
                router_data.response = router_data_response;
                router_data.connector_http_status_code = Some(status_code);

                Ok((router_data, (), payment_void_response))
            },
        ))
        .await?;

        *self = updated_router_data;
        Ok(())
    }
}

/// Implementation of FlowGateway for api::PSync
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsCancelData
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Void
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PaymentsCancelData types::PaymentsResponseData>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsCancelData
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
