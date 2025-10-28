//! UCS Context Implementation
//!
//! This module implements the UcsContext trait for the router crate,
//! providing concrete types for authentication, headers, and lineage IDs.

use external_services::grpc_client::{unified_connector_service::ConnectorAuthMetadata, GrpcHeadersUcs, LineageIds};
use hyperswitch_interfaces::unified_connector_service::UcsContext;

/// Context for UCS GRPC calls containing auth, headers, and lineage IDs
#[derive(Debug, Clone)]
pub struct RouterUcsContext {
    auth: ConnectorAuthMetadata,
    headers: GrpcHeadersUcs,
    lineage_ids: LineageIds,
}

impl RouterUcsContext {
    /// Create a new UCS context
    pub fn new(
        auth: ConnectorAuthMetadata,
        headers: GrpcHeadersUcs,
        lineage_ids: LineageIds,
    ) -> Self {
        Self {
            auth,
            headers,
            lineage_ids,
        }
    }
}

impl UcsContext for RouterUcsContext {
    type AuthMetadata = ConnectorAuthMetadata;
    type GrpcHeaders = GrpcHeadersUcs;
    type LineageIds = LineageIds;

    fn auth(&self) -> Self::AuthMetadata {
        self.auth.clone()
    }

    fn headers(self) -> Self::GrpcHeaders {
        self.headers
    }

    fn lineage_ids(&self) -> &Self::LineageIds {
        &self.lineage_ids
    }
}