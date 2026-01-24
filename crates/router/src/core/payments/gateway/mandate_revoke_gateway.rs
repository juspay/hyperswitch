use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::RouterData, router_flow_types::mandate_revoke::MandateRevoke,
};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    core::{
        payments::gateway::context::RouterGatewayContext, unified_connector_service,
        unified_connector_service::handle_unified_connector_service_response_for_mandate_revoke,
    },
    routes::SessionState,
    types::{self, transformers::ForeignTryFrom},
};

// Implementation of PaymentGateway for domain::MandateRevoke
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        Self,
        types::MandateRevokeRequestData,
        types::MandateRevokeResponseData,
        RouterGatewayContext,
    > for MandateRevoke
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            Self,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
        >,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
        >,
        router_data: &RouterData<
            Self,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<Self, types::MandateRevokeRequestData, types::MandateRevokeResponseData>,
        ConnectorError,
    > {
        let merchant_connector_account = context.merchant_connector_account;
        let platform = context.platform;
        let lineage_ids = context.lineage_ids;
        let unified_connector_service_execution_mode = context.execution_mode;

        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        let mandate_revoke_request =
            payments_grpc::PaymentServiceRevokeMandateRequest::foreign_try_from(router_data)
                .change_context(ConnectorError::RequestEncodingFailed)
                .attach_printable("Failed to construct Mandate Revoke Request")?;

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                &platform,
                router_data.connector.clone(),
            )
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct request metadata")?;

        let header_payload = state
            .get_grpc_headers_ucs(unified_connector_service_execution_mode)
            .lineage_ids(lineage_ids)
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(None);

        Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
            router_data.clone(),
            state,
            mandate_revoke_request,
            header_payload,
            |mut router_data, mandate_revoke_request, grpc_headers| async move {
                let response = Box::pin(client.mandate_revoke(
                    mandate_revoke_request,
                    connector_auth_metadata,
                    grpc_headers,
                ))
                .await
                .attach_printable("Failed to revoke mandate")?;

                let mandate_revoke_response = response.into_inner();

                let (mandate_revoke_response_data, status_code) =
                    handle_unified_connector_service_response_for_mandate_revoke(
                        mandate_revoke_response.clone(),
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                router_data.response = mandate_revoke_response_data;
                router_data.connector_http_status_code = Some(status_code);

                Ok((router_data, (), mandate_revoke_response))
            },
        ))
        .await
        .map(|(router_data, _)| router_data)
        .change_context(ConnectorError::ResponseHandlingFailed)
    }
}

impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::MandateRevokeRequestData,
        types::MandateRevokeResponseData,
        RouterGatewayContext,
    > for MandateRevoke
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            Self,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
        >,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
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
