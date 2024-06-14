pub mod flow_common_types;

use std::{marker::PhantomData, ops::Deref};

pub use flow_common_types::*;

use crate::router_data::{ConnectorAuthType, ErrorResponse};

#[derive(Debug, Clone)]
pub struct RouterDataNew<Flow, ResourceCommonData, FlowSpecificRequest, FlowSpecificResponse> {
    pub flow: PhantomData<Flow>,
    pub resource_common_data: ResourceCommonData,
    pub connector_auth_type: ConnectorAuthType,
    /// Contains flow-specific data required to construct a request and send it to the connector.
    pub request: FlowSpecificRequest,
    /// Contains flow-specific data that the connector responds with.
    pub response: Result<FlowSpecificResponse, ErrorResponse>,
}

impl<Flow, ResourceCommonData, FlowSpecificRequest, FlowSpecificResponse> Deref
    for RouterDataNew<Flow, ResourceCommonData, FlowSpecificRequest, FlowSpecificResponse>
{
    type Target = ResourceCommonData;
    fn deref(&self) -> &Self::Target {
        &self.resource_common_data
    }
}
