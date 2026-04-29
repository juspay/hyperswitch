use std::str::FromStr;

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, id_type, request::Request, ucs_types};
use error_stack::ResultExt;
use hyperswitch_domain_models::{router_data::RouterData, router_flow_types as domain};
use hyperswitch_interfaces::{
    api::gateway as payout_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    core::{payments::gateway::context::RouterGatewayContext, unified_connector_service},
    routes::SessionState,
    services::logger,
    types::{self, transformers::ForeignTryFrom},
};

// =============================================================================
// PayoutGateway Implementation for domain::PoCancel
// =============================================================================

#[async_trait]
impl<RCD>
    payout_gateway::PayoutGateway<
        SessionState,
        RCD,
        Self,
        types::PayoutsData,
        types::PayoutsResponseData,
        RouterGatewayContext,
    > for domain::PoCancel
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PayoutsData, types::PayoutsResponseData>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            types::PayoutsData,
            types::PayoutsResponseData,
        >,
        router_data: &RouterData<Self, types::PayoutsData, types::PayoutsResponseData>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<Self, types::PayoutsData, types::PayoutsResponseData>,
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

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                processor,
                router_data.connector.clone(),
            )
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct request metadata")?;

        let merchant_reference_id = unified_connector_service::parse_merchant_payout_reference_id(
            header_payload
                .x_reference_id
                .as_deref()
                .unwrap_or(router_data.request.payout_id.get_string_repr()),
        )
        .map(ucs_types::UcsReferenceId::Payout);
        let resource_id = id_type::PayoutResourceId::from_str(router_data.attempt_id.as_str())
            .inspect_err(|err| logger::warn!(error = ?err, "Invalid Payout Resource Id found"))
            .ok()
            .map(ucs_types::UcsResourceId::PayoutAttempt);

        let grpc_headers = state
            .get_grpc_headers_ucs(unified_connector_service_execution_mode)
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(merchant_reference_id)
            .resource_id(resource_id)
            .lineage_ids(lineage_ids);

        logger::debug!("Granular Gateway: Payout cancel flow");
        let granular_payout_void_request =
            payments_grpc::PayoutServiceVoidRequest::foreign_try_from(router_data)
                .change_context(ConnectorError::RequestEncodingFailed)
                .attach_printable("Failed to construct Payout Void Request")?;

        let updated_router_data =
            Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
                router_data.clone(),
                state,
                granular_payout_void_request,
                grpc_headers,
                unified_connector_service_execution_mode,
                |mut router_data, granular_payout_void_request, grpc_headers| async move {
                    let response = Box::pin(client.payout_void(
                        granular_payout_void_request,
                        connector_auth_metadata,
                        grpc_headers,
                    ))
                    .await
                    .attach_printable("Failed to void payout")?;

                    let payout_void_response = response.into_inner();

                    let ucs_data = types::UcsPayoutVoidResponseData::foreign_try_from((
                        payout_void_response.clone(),
                        common_enums::PayoutStatus::Cancelled,
                    ))
                    .attach_printable("Failed to deserialize UCS response")?;

                    let router_data_response = ucs_data.router_data_response.inspect_err(|_| {
                        logger::debug!("Error in UCS router data response");
                    });
                    router_data.response = router_data_response;

                    router_data.connector_http_status_code = Some(ucs_data.status_code);

                    Ok((router_data, (), payout_void_response))
                },
            ))
            .await
            .map(|(router_data, _)| router_data)
            .change_context(ConnectorError::ResponseHandlingFailed)?;

        Ok(updated_router_data)
    }
}

impl<RCD>
    payout_gateway::PayoutFlowGateway<
        SessionState,
        RCD,
        types::PayoutsData,
        types::PayoutsResponseData,
        RouterGatewayContext,
    > for domain::PoCancel
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PayoutsData, types::PayoutsResponseData>,
{
    fn get_payout_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payout_gateway::PayoutGateway<
            SessionState,
            RCD,
            Self,
            types::PayoutsData,
            types::PayoutsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => Box::new(payout_gateway::DirectGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => Box::new(Self),
        }
    }
}
