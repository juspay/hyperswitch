//! PaymentGateway implementation for api::SetupMandate flow
//!
//! This module implements the PaymentGateway trait for the SetupMandate flow,
//! handling mandate registration via the payment_setup_mandate GRPC endpoint.

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use hyperswitch_domain_models::{
    router_flow_types as domain,
    router_data::RouterData,
};
use hyperswitch_interfaces::{
    api::{self, gateway as payment_gateway},
    api_client::ApiClientWrapper,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
    unified_connector_service::UcsFlowExecutor,
};
use crate::core::payments::gateway::RouterGatewayContext;

// use super::{
//     ucs_execution_context::RouterUcsExecutionContext,
//     ucs_executors::SetupMandateUcsExecutor,
// };
use crate::{
    routes::SessionState,
    types,
};

// /// Gateway struct for api::SetupMandate flow
// #[derive(Debug, Clone, Copy)]
// pub struct SetupMandateGateway;

/// Implementation of PaymentGateway for api::SetupMandate flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::SetupMandate
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::SetupMandate,
            RCD,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<
            domain::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<domain::SetupMandate, types::SetupMandateRequestData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        // Create execution context
        // let execution_context = RouterUcsExecutionContext::new(
        //     &context.merchant_context,
        //     &context.header_payload,
        //     context.lineage_ids,
        //     &context.merchant_connector_account,
        //     context.execution_mode,
        // );

        // // Execute payment_setup_mandate GRPC call using trait-based executor
        // let updated_router_data = SetupMandateUcsExecutor::execute_ucs_flow(
        //     state,
        //     router_data,
        //     execution_context,
        // )
        // .await?;

        // Ok(updated_router_data)
        todo!()
    }
}

/// Implementation of FlowGateway for api::SetupMandate
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::SetupMandate
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => {
                Box::new(payment_gateway::DirectGateway)
            }
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => {
                Box::new(domain::SetupMandate)
            }
        }
    }
}

