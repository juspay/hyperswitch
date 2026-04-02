use common_enums::ExecutionPath;
use hyperswitch_domain_models::router_flow_types as domain;
use hyperswitch_interfaces::{
    api::gateway as payment_gateway, connector_integration_interface::RouterDataConversion,
};

use crate::{core::payments::gateway::context::RouterGatewayContext, routes::SessionState, types};

// =============================================================================
// FlowGateway Implementation for domain::PushNotification
// =============================================================================

/// Implementation of FlowGateway for PushNotification
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PushNotificationRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PushNotification
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PushNotificationRequestData, types::PaymentsResponseData>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PushNotificationRequestData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            // PushNotification currently only supports DirectGateway
            // UCS support can be added when the gRPC push_notification methods are implemented
            ExecutionPath::Direct
            | ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => {
                Box::new(payment_gateway::DirectGateway)
            }
        }
    }
}
