//! The one place a gateway decides whether a UCS call goes over the network
//! (SaaS deployments, daily-deployable UCS) or in-process (self-hosted
//! deployments shipping one stable HS+UCS release, no separate UCS service to
//! run). Everything upstream of this enum — request building, response
//! parsing, the gateway call sites themselves — is unchanged either way; only
//! this module and its construction at startup know two transports exist.
//!
//! Adding a method here means adding it to both
//! [`UnifiedConnectorServiceClient`] and
//! [`unified_connector_service_library::LibraryConnectorService`] with the
//! same signature (mirroring the same request/response proto types — see that
//! crate's docs for why that's guaranteed, not just a convention to remember)
//! and one match arm here. That's the only place a flow's wiring needs to
//! exist twice; the flow's actual logic never does.

use super::unified_connector_service::{
    ConnectorAuthMetadata, GrpcHeadersUcs, UnifiedConnectorServiceClient,
    UnifiedConnectorServiceResult,
};
// Not `grpc_api_types` directly: `external_services` only has
// `unified-connector-service-client` (a pure re-export of `grpc_api_types`,
// see that crate's one-line lib.rs) as a direct dependency. Same type either
// way — `unified_connector_service_library` depends on `grpc-api-types`
// directly instead, since that's a direct dependency there.
use unified_connector_service_client::payments as payments_grpc;

/// Either transport to the Unified Connector Service. Constructed once at
/// startup from `[grpc_client.unified_connector_service]`'s `mode` — see that
/// config table's docs — and held by callers exactly as they'd hold a bare
/// `UnifiedConnectorServiceClient` today.
pub enum UcsTransport {
    /// UCS as a separately-deployed gRPC service (SaaS deployments).
    Grpc(UnifiedConnectorServiceClient),
    /// UCS running in-process (self-hosted deployments).
    Library(unified_connector_service_library::LibraryConnectorService),
}

impl UcsTransport {
    /// Performs Payment Authorize.
    pub async fn payment_authorize(
        &self,
        request: payments_grpc::PaymentServiceAuthorizeRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceAuthorizeResponse>,
    > {
        match self {
            Self::Grpc(client) => {
                client
                    .payment_authorize(request, connector_auth_metadata, grpc_headers)
                    .await
            }
            Self::Library(service) => {
                service
                    .payment_authorize(request, connector_auth_metadata, grpc_headers)
                    .await
            }
        }
    }

    // Every other UnifiedConnectorServiceClient method gets the same two-line
    // match arm here once unified_connector_service_library implements it —
    // see that crate's LibraryConnectorService for the current status.
}
