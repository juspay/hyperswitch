use std::str::FromStr;

use async_trait::async_trait;
use common_enums::ExecutionPath;
use common_utils::{id_type, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client::LineageIds;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails;
use hyperswitch_domain_models::{
    platform::Processor, router_data::RouterData,
    router_data_v2::flow_common_types::DisputesFlowData, router_flow_types::dispute,
    router_request_types::AcceptDisputeRequestData, router_response_types::AcceptDisputeResponse,
};
use hyperswitch_interfaces::api::gateway as interface_gateway;
use router_env::{instrument, tracing};
use unified_connector_service_client::payments as payments_grpc;

use super::{DirectDisputeGateway, DisputeGatewayExecutionContext};
use crate::{
    core::{errors, payments::helpers::MerchantConnectorAccountType, unified_connector_service},
    errors::RouterResult,
    routes::SessionState,
    services,
    types::{self, transformers::ForeignTryFrom},
};

#[async_trait]
impl
    interface_gateway::DisputeGateway<
        SessionState,
        DisputesFlowData,
        Self,
        AcceptDisputeRequestData,
        AcceptDisputeResponse,
        DisputeGatewayExecutionContext,
    > for dispute::Accept
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: services::BoxedDisputeConnectorIntegrationInterface<
            Self,
            AcceptDisputeRequestData,
            AcceptDisputeResponse,
        >,
        router_data: &types::AcceptDisputeRouterData,
        context: DisputeGatewayExecutionContext,
    ) -> RouterResult<types::AcceptDisputeRouterData> {
        let rollout_settings = context.dispute.rollout_settings();
        call_unified_connector_service_for_dispute_accept(
            state,
            &context.processor,
            router_data.clone(),
            context.dispute.connector_transaction_id,
            rollout_settings,
            context.dispute.merchant_connector_account,
        )
        .await
        .map_err(|error| error.attach_printable("Failed while calling accept via UCS"))
    }
}

impl
    interface_gateway::DisputeFlowGateway<
        SessionState,
        DisputesFlowData,
        AcceptDisputeRequestData,
        AcceptDisputeResponse,
        DisputeGatewayExecutionContext,
    > for dispute::Accept
{
    fn get_dispute_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn interface_gateway::DisputeGateway<
            SessionState,
            DisputesFlowData,
            Self,
            AcceptDisputeRequestData,
            AcceptDisputeResponse,
            DisputeGatewayExecutionContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => Box::new(DirectDisputeGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => Box::new(Self),
        }
    }
}

/// Execute a dispute accept using DisputeService.Accept.
#[instrument(skip_all)]
async fn call_unified_connector_service_for_dispute_accept(
    state: &SessionState,
    processor: &Processor,
    router_data: RouterData<dispute::Accept, AcceptDisputeRequestData, AcceptDisputeResponse>,
    connector_transaction_id: Option<String>,
    rollout_settings: unified_connector_service::kill_switch::RolloutSettings,
    #[cfg(feature = "v1")] merchant_connector_account: MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: MerchantConnectorAccountTypeDetails,
) -> RouterResult<RouterData<dispute::Accept, AcceptDisputeRequestData, AcceptDisputeResponse>> {
    let ucs_client = unified_connector_service::get_ucs_client(state)?;
    let connector_auth_metadata =
        unified_connector_service::build_unified_connector_service_auth_metadata(
            merchant_connector_account,
            processor.get_account().get_id(),
            router_data.connector.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to build UCS auth metadata for dispute accept")?;
    let grpc_request = payments_grpc::DisputeServiceAcceptRequest::foreign_try_from((
        &router_data,
        connector_transaction_id,
    ))
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to transform router data to UCS dispute accept request")?;

    let merchant_id = processor.get_account().get_id().clone();
    let profile_id = id_type::ProfileId::from_str(merchant_id.get_string_repr())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert merchant_id to profile_id for UCS dispute accept")?;
    let lineage_ids = LineageIds::new(merchant_id, profile_id);
    let merchant_reference_id =
        id_type::PaymentReferenceId::from_str(router_data.payment_id.as_str())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert payment_id to UCS reference id")
            .map(ucs_types::UcsReferenceId::Payment)
            .map(Some)?;
    // UcsResourceId has no dispute variant; the dispute is identified by the request body.
    let grpc_header_builder = state
        .get_grpc_headers_ucs(rollout_settings.execution_mode)
        .payment_method(Some(router_data.payment_method))
        .payment_method_type(router_data.payment_method_type)
        .lineage_ids(lineage_ids)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_reference_id)
        .resource_id(None);

    Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
        router_data,
        state,
        grpc_request,
        grpc_header_builder,
        rollout_settings.clone(),
        |mut router_data, grpc_request, grpc_headers| async move {
            // UCS connector errors are handled by the wrapper — see `ucs_logging_wrapper_granular`.
            let grpc_response = ucs_client
                .dispute_accept(grpc_request, connector_auth_metadata, grpc_headers)
                .await
                .attach_printable("UCS dispute accept execution failed")?
                .into_inner();
            let (router_data_response, status_code) =
                unified_connector_service::handle_unified_connector_service_response_for_dispute_accept(grpc_response.clone())
                    .attach_printable("Failed to transform UCS dispute accept response")?;

            router_data.connector_http_status_code = Some(status_code);
            router_data.response = router_data_response;

            Ok((router_data, (), grpc_response))
        },
    ))
    .await
    .map(|(router_data, _flow_response)| router_data)
    .change_context(errors::ApiErrorResponse::InternalServerError)
}
