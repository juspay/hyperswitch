//! UcsStateProvider implementation for SessionState
//!
//! This module implements the UcsStateProvider trait for SessionState,
//! enabling the trait-based UCS architecture to work with the router's state.

use common_enums::ExecutionMode;
use common_utils::errors::CustomResult;
use external_services::grpc_client::{
    GrpcHeadersUcsBuilderInitial, LineageIds, UnifiedConnectorServiceClient,
};
use hyperswitch_interfaces::{errors::ConnectorError, unified_connector_service::UcsStateProvider};

use crate::routes::SessionState;

use super::ucs_context::RouterUcsContext;

impl UcsStateProvider for SessionState {
    type GrpcClient = UnifiedConnectorServiceClient;
    type Context = RouterUcsContext;
    type ContextBuilder = GrpcHeadersUcsBuilderInitial;
    type LineageIds = LineageIds;

    fn get_ucs_client(
        &self,
    ) -> CustomResult<&UnifiedConnectorServiceClient, ConnectorError> {
        self.grpc_client
            .unified_connector_service_client
            .as_ref()
            .ok_or(ConnectorError::ProcessingStepFailed(Some(
                "Failed to fetch Unified Connector Service client"
                    .to_string()
                    .into(),
            )))
            .map_err(Into::into)
    }

    fn get_context_builder(&self, execution_mode: ExecutionMode) -> Self::ContextBuilder {
        self.get_grpc_headers_ucs(execution_mode)
    }
}