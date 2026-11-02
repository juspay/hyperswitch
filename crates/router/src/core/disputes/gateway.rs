//! Gateway execution for dispute actions.

use std::fmt::Debug;

use async_trait::async_trait;
use common_enums::{ExecutionMode, ExecutionPath};
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::DisputesFlowData, router_flow_types::dispute,
};
use hyperswitch_interfaces::connector_integration_interface::RouterDataConversion;

use crate::{
    core::{errors::ConnectorErrorExt, payments::helpers, unified_connector_service},
    errors::RouterResult,
    routes::SessionState,
    services,
    types::{self, domain},
};

pub struct DisputeGatewayContext {
    pub execution_path: ExecutionPath,
    pub rollout_result: helpers::RolloutExecutionResult,
    pub merchant_connector_account: helpers::MerchantConnectorAccountType,
    pub connector_transaction_id: Option<String>,
}

impl DisputeGatewayContext {
    fn rollout_settings(&self) -> unified_connector_service::kill_switch::RolloutSettings {
        let mut settings = self.rollout_result.rollout_settings();
        settings.execution_mode = ExecutionMode::Primary;
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

/// Selects the UCS or direct connector gateway for one dispute action.
pub async fn execute_dispute_gateway<Flow, Req, Resp>(
    state: &SessionState,
    ucs_state: &SessionState,
    processor: &domain::Processor,
    router_data: types::RouterData<Flow, Req, Resp>,
    connector_integration: services::BoxedDisputeConnectorIntegrationInterface<Flow, Req, Resp>,
    context: DisputeGatewayContext,
) -> RouterResult<types::RouterData<Flow, Req, Resp>>
where
    Flow: DisputeGatewayFlow<Req, Resp>,
    Req: Debug + Clone + Send + Sync + 'static,
    Resp: Debug + Clone + Send + Sync + 'static,
    DisputesFlowData: RouterDataConversion<Flow, Req, Resp>,
{
    match context.execution_path {
        ExecutionPath::UnifiedConnectorService => {
            let rollout_settings = context.rollout_settings();
            Flow::execute_ucs(
                ucs_state,
                processor,
                router_data,
                context.connector_transaction_id,
                rollout_settings,
                context.merchant_connector_account,
            )
            .await
        }
        ExecutionPath::Direct | ExecutionPath::ShadowUnifiedConnectorService => {
            services::execute_connector_processing_step(
                state,
                connector_integration,
                &router_data,
                common_enums::CallConnectorAction::Trigger,
                None,
                None,
            )
            .await
            .to_dispute_failed_response()
        }
    }
}

#[async_trait]
impl DisputeGatewayFlow<types::AcceptDisputeRequestData, types::AcceptDisputeResponse>
    for dispute::Accept
{
    async fn execute_ucs(
        state: &SessionState,
        processor: &domain::Processor,
        router_data: types::AcceptDisputeRouterData,
        connector_transaction_id: Option<String>,
        rollout_settings: unified_connector_service::kill_switch::RolloutSettings,
        merchant_connector_account: helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::AcceptDisputeRouterData> {
        unified_connector_service::call_unified_connector_service_for_dispute_accept(
            state,
            processor,
            router_data,
            connector_transaction_id,
            rollout_settings,
            merchant_connector_account,
        )
        .await
        .map_err(|error| error.attach_printable("Failed while calling accept dispute via UCS"))
    }
}

#[async_trait]
impl DisputeGatewayFlow<types::SubmitEvidenceRequestData, types::SubmitEvidenceResponse>
    for dispute::Evidence
{
    async fn execute_ucs(
        state: &SessionState,
        processor: &domain::Processor,
        router_data: types::SubmitEvidenceRouterData,
        connector_transaction_id: Option<String>,
        rollout_settings: unified_connector_service::kill_switch::RolloutSettings,
        merchant_connector_account: helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::SubmitEvidenceRouterData> {
        unified_connector_service::call_unified_connector_service_for_dispute_submit_evidence(
            state,
            processor,
            router_data,
            connector_transaction_id,
            rollout_settings,
            merchant_connector_account,
        )
        .await
        .map_err(|error| error.attach_printable("Failed while calling submit evidence via UCS"))
    }
}

#[async_trait]
impl DisputeGatewayFlow<types::DefendDisputeRequestData, types::DefendDisputeResponse>
    for dispute::Defend
{
    async fn execute_ucs(
        state: &SessionState,
        processor: &domain::Processor,
        router_data: types::DefendDisputeRouterData,
        connector_transaction_id: Option<String>,
        rollout_settings: unified_connector_service::kill_switch::RolloutSettings,
        merchant_connector_account: helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::DefendDisputeRouterData> {
        unified_connector_service::call_unified_connector_service_for_dispute_defend(
            state,
            processor,
            router_data,
            connector_transaction_id,
            rollout_settings,
            merchant_connector_account,
        )
        .await
        .map_err(|error| error.attach_printable("Failed while calling defend dispute via UCS"))
    }
}
