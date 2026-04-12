use common_enums::ExecutionPath;
use hyperswitch_domain_models::router_flow_types as domain;
use hyperswitch_interfaces::{
    api::gateway as payment_gateway, connector_integration_interface::RouterDataConversion,
};

use crate::{core::payments::gateway::context::RouterGatewayContext, routes::SessionState, types};

// =============================================================================
// FlowGateway Implementation for domain::GenerateQr
// =============================================================================

/// Implementation of FlowGateway for GenerateQr
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::GenerateQrRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::GenerateQr
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::GenerateQrRequestData, types::PaymentsResponseData>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::GenerateQrRequestData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            // GenerateQr currently only supports DirectGateway
            // UCS support can be added when the gRPC generate_qr methods are implemented
            ExecutionPath::Direct
            | ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => {
                Box::new(payment_gateway::DirectGateway)
            }
        }
    }
}
