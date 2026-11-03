//! Gateway execution for dispute actions.

pub mod accept_gateway;
pub mod defend_gateway;
pub mod evidence_gateway;

use std::{fmt::Debug, marker::PhantomData};

use async_trait::async_trait;
use common_enums::ExecutionPath;
use hyperswitch_domain_models::router_data_v2::flow_common_types::DisputesFlowData;
use hyperswitch_interfaces::{
    api::gateway as interface_gateway, connector_integration_interface::RouterDataConversion,
};

use crate::{
    core::{errors::ConnectorErrorExt, payments::helpers, unified_connector_service},
    errors::RouterResult,
    routes::SessionState,
    services,
    types::{self, domain},
};

#[derive(Clone)]
pub struct DisputeGatewayContext {
    pub execution_path: ExecutionPath,
    pub rollout_result: helpers::RolloutExecutionResult,
    pub merchant_connector_account: helpers::MerchantConnectorAccountType,
    pub connector_transaction_id: Option<String>,
}

#[derive(Clone)]
pub struct DisputeGatewayExecutionContext {
    processor: domain::Processor,
    dispute: DisputeGatewayContext,
}

impl interface_gateway::GatewayContext for DisputeGatewayExecutionContext {
    fn execution_path(&self) -> ExecutionPath {
        self.dispute.execution_path
    }

    fn execution_mode(&self) -> common_enums::ExecutionMode {
        self.dispute.execution_path.get_execution_mode()
    }
}

impl DisputeGatewayContext {
    fn rollout_settings(&self) -> unified_connector_service::kill_switch::RolloutSettings {
        let mut settings = self.rollout_result.rollout_settings();
        settings.execution_mode = self.execution_path.get_execution_mode();
        settings
    }
}

#[async_trait]
pub trait DisputeGatewayFlow<Req, Resp>: Clone + Debug + Send + Sync + Sized + 'static
where
    Req: Debug + Clone + Send + Sync + 'static,
    Resp: Debug + Clone + Send + Sync + 'static,
{
    async fn execute_ucs(
        state: &SessionState,
        processor: &domain::Processor,
        router_data: types::RouterData<Self, Req, Resp>,
        connector_transaction_id: Option<String>,
        rollout_settings: unified_connector_service::kill_switch::RolloutSettings,
        merchant_connector_account: helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::RouterData<Self, Req, Resp>>;
}

pub struct DirectDisputeGateway;

#[async_trait]
impl<Flow, Req, Resp>
    interface_gateway::DisputeGateway<
        SessionState,
        DisputesFlowData,
        Flow,
        Req,
        Resp,
        DisputeGatewayExecutionContext,
    > for DirectDisputeGateway
where
    Flow: Clone + Debug + Send + Sync + 'static,
    Req: Debug + Clone + Send + Sync + 'static,
    Resp: Debug + Clone + Send + Sync + 'static,
    DisputesFlowData: RouterDataConversion<Flow, Req, Resp>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        connector_integration: services::BoxedDisputeConnectorIntegrationInterface<Flow, Req, Resp>,
        router_data: &types::RouterData<Flow, Req, Resp>,
        _context: DisputeGatewayExecutionContext,
    ) -> RouterResult<types::RouterData<Flow, Req, Resp>> {
        services::execute_connector_processing_step(
            state,
            connector_integration,
            router_data,
            common_enums::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_dispute_failed_response()
    }
}

pub struct UcsDisputeGateway<Flow>(PhantomData<Flow>);

#[async_trait]
impl<Flow, Req, Resp>
    interface_gateway::DisputeGateway<
        SessionState,
        DisputesFlowData,
        Flow,
        Req,
        Resp,
        DisputeGatewayExecutionContext,
    > for UcsDisputeGateway<Flow>
where
    Flow: DisputeGatewayFlow<Req, Resp>,
    Req: Debug + Clone + Send + Sync + 'static,
    Resp: Debug + Clone + Send + Sync + 'static,
    DisputesFlowData: RouterDataConversion<Flow, Req, Resp>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: services::BoxedDisputeConnectorIntegrationInterface<
            Flow,
            Req,
            Resp,
        >,
        router_data: &types::RouterData<Flow, Req, Resp>,
        context: DisputeGatewayExecutionContext,
    ) -> RouterResult<types::RouterData<Flow, Req, Resp>> {
        let rollout_settings = context.dispute.rollout_settings();
        Flow::execute_ucs(
            state,
            &context.processor,
            router_data.clone(),
            context.dispute.connector_transaction_id,
            rollout_settings,
            context.dispute.merchant_connector_account,
        )
        .await
    }
}

/// Run a dispute action through the shared gateway executor.
pub async fn execute_dispute_gateway<Flow, Req, Resp>(
    state: &SessionState,
    ucs_state: &SessionState,
    processor: &domain::Processor,
    router_data: types::RouterData<Flow, Req, Resp>,
    connector_integration: services::BoxedDisputeConnectorIntegrationInterface<Flow, Req, Resp>,
    context: DisputeGatewayContext,
) -> RouterResult<types::RouterData<Flow, Req, Resp>>
where
    Flow: DisputeGatewayFlow<Req, Resp>
        + interface_gateway::DisputeFlowGateway<
            SessionState,
            DisputesFlowData,
            Req,
            Resp,
            DisputeGatewayExecutionContext,
        >,
    Req: Debug + Clone + Send + Sync + 'static,
    Resp: Debug + Clone + Send + Sync + 'static,
    DisputesFlowData: RouterDataConversion<Flow, Req, Resp>,
{
    interface_gateway::execute_dispute_gateway(
        state,
        ucs_state,
        connector_integration,
        &router_data,
        DisputeGatewayExecutionContext {
            processor: processor.clone(),
            dispute: context,
        },
    )
    .await
}
