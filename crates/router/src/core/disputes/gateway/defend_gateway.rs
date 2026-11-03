use std::marker::PhantomData;

use async_trait::async_trait;
use common_enums::ExecutionPath;
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::DisputesFlowData, router_flow_types::dispute,
};
use hyperswitch_interfaces::api::gateway as interface_gateway;

use super::{
    DirectDisputeGateway, DisputeGatewayExecutionContext, DisputeGatewayFlow, UcsDisputeGateway,
};
use crate::{
    core::{payments::helpers, unified_connector_service},
    errors::RouterResult,
    routes::SessionState,
    types::{self, domain},
};

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

impl
    interface_gateway::DisputeFlowGateway<
        SessionState,
        DisputesFlowData,
        types::DefendDisputeRequestData,
        types::DefendDisputeResponse,
        DisputeGatewayExecutionContext,
    > for dispute::Defend
{
    fn get_dispute_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn interface_gateway::DisputeGateway<
            SessionState,
            DisputesFlowData,
            Self,
            types::DefendDisputeRequestData,
            types::DefendDisputeResponse,
            DisputeGatewayExecutionContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => Box::new(DirectDisputeGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => {
                Box::new(UcsDisputeGateway::<Self>(PhantomData))
            }
        }
    }
}
