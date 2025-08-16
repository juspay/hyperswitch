pub mod flow_common_types;

use std::{marker::PhantomData, ops::Deref};

use common_utils::id_type;
#[cfg(feature = "frm")]
pub use flow_common_types::FrmFlowData;
#[cfg(feature = "payouts")]
pub use flow_common_types::PayoutFlowData;
pub use flow_common_types::{
    AccessTokenFlowData, AuthenticationTokenFlowData, DisputesFlowData,
    ExternalAuthenticationFlowData, ExternalVaultProxyFlowData, FilesFlowData,
    MandateRevokeFlowData, PaymentFlowData, RefundFlowData, UasFlowData, VaultConnectorFlowData,
    WebhookSourceVerifyData,
};

use crate::router_data::{ConnectorAuthType, ErrorResponse};

#[derive(Debug, Clone)]
pub struct RouterDataV2<Flow, ResourceCommonData, FlowSpecificRequest, FlowSpecificResponse> {
    pub flow: PhantomData<Flow>,
    pub tenant_id: id_type::TenantId,
    pub resource_common_data: ResourceCommonData,
    pub connector_auth_type: ConnectorAuthType,
    /// Contains flow-specific data required to construct a request and send it to the connector.
    pub request: FlowSpecificRequest,
    /// Contains flow-specific data that the connector responds with.
    pub response: Result<FlowSpecificResponse, ErrorResponse>,
}

impl<Flow, ResourceCommonData, FlowSpecificRequest, FlowSpecificResponse> Deref
    for RouterDataV2<Flow, ResourceCommonData, FlowSpecificRequest, FlowSpecificResponse>
{
    type Target = ResourceCommonData;
    fn deref(&self) -> &Self::Target {
        &self.resource_common_data
    }
}
