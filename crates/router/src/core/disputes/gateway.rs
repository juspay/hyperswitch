//! Gateway execution for dispute actions.

use std::fmt::Debug;

use async_trait::async_trait;
use common_enums::ExecutionPath;
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::DisputesFlowData, router_flow_types::dispute,
};
use hyperswitch_interfaces::{
    api_client::{self, ApiClientWrapper},
    connector_integration_interface::RouterDataConversion,
    helpers::{self as comparison_helpers, GetComparisonServiceConfig},
};
use hyperswitch_masking::Secret;
use router_env::tracing::Instrument;

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

async fn execute_direct_dispute_gateway<Flow, Req, Resp>(
    state: &SessionState,
    connector_integration: services::BoxedDisputeConnectorIntegrationInterface<Flow, Req, Resp>,
    router_data: &types::RouterData<Flow, Req, Resp>,
) -> RouterResult<types::RouterData<Flow, Req, Resp>>
where
    Flow: Clone + Debug + Send + Sync + 'static,
    Req: Debug + Clone + Send + Sync + 'static,
    Resp: Debug + Clone + Send + Sync + 'static,
    DisputesFlowData: RouterDataConversion<Flow, Req, Resp>,
{
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

fn dispute_comparison_value<Flow, Req, Resp>(
    result: &RouterResult<types::RouterData<Flow, Req, Resp>>,
) -> serde_json::Value
where
    Resp: Debug,
{
    match result {
        Ok(data) => serde_json::json!({
            "connector_http_status_code": data.connector_http_status_code,
            "response": format!("{:?}", data.response),
        }),
        Err(error) => serde_json::json!({ "error": format!("{error:?}") }),
    }
}

/// Selects the execution path for one dispute action.
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
        ExecutionPath::Direct => {
            execute_direct_dispute_gateway(state, connector_integration, &router_data).await
        }
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
        ExecutionPath::ShadowUnifiedConnectorService => {
            let direct_result =
                execute_direct_dispute_gateway(state, connector_integration, &router_data).await;
            let direct_for_compare = dispute_comparison_value(&direct_result);
            let shadow_state = ucs_state.clone();
            let shadow_processor = processor.clone();
            let shadow_router_data = router_data.clone();
            let rollout_settings = context.rollout_settings();
            let connector_transaction_id = context.connector_transaction_id;
            let merchant_connector_account = context.merchant_connector_account;
            let connector_name = router_data.connector.clone();
            let merchant_id = router_data.merchant_id.clone();
            let payment_method = router_data.payment_method;
            let payment_method_type = router_data.payment_method_type;

            tokio::spawn(
                async move {
                    let shadow_result = Flow::execute_ucs(
                        &shadow_state,
                        &shadow_processor,
                        shadow_router_data,
                        connector_transaction_id,
                        rollout_settings,
                        merchant_connector_account,
                    )
                    .await;
                    if let Err(error) = &shadow_result {
                        router_env::logger::warn!(
                            ?error,
                            execution_mode = "shadow",
                            primary_impact = false,
                            "UCS dispute shadow execution failed"
                        );
                    }
                    if let Some(config) = shadow_state.get_comparison_service_config() {
                        let comparison_data = comparison_helpers::ComparisonData {
                            hyperswitch_data: Secret::new(direct_for_compare),
                            unified_connector_service_data: Secret::new(dispute_comparison_value(
                                &shadow_result,
                            )),
                        };
                        if let Err(error) = comparison_helpers::send_comparison_data(
                            &shadow_state,
                            comparison_data,
                            config,
                            connector_name,
                            api_client::get_flow_name::<Flow>().ok(),
                            shadow_state.get_request_id_str(),
                            Some(&merchant_id),
                            Some(payment_method),
                            payment_method_type,
                        )
                        .await
                        {
                            router_env::logger::warn!(
                                ?error,
                                "Failed to send dispute shadow comparison"
                            );
                        }
                    }
                }
                .instrument(router_env::tracing::Span::current()),
            );
            direct_result
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
