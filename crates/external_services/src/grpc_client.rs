/// Dyanimc Routing Client interface implementation
#[cfg(feature = "dynamic_routing")]
pub mod dynamic_routing;
/// gRPC based Heath Check Client interface implementation
#[cfg(feature = "dynamic_routing")]
pub mod health_check_client;
/// gRPC based Recovery Trainer Client interface implementation
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
pub mod revenue_recovery;

use std::{fmt::Debug, sync::Arc};

#[cfg(feature = "dynamic_routing")]
use common_utils::consts;
#[cfg(feature = "dynamic_routing")]
use dynamic_routing::{DynamicRoutingClientConfig, RoutingStrategy};
#[cfg(feature = "dynamic_routing")]
use health_check_client::HealthCheckClient;
#[cfg(any(
    feature = "dynamic_routing",
    all(feature = "v2", feature = "revenue_recovery")
))]
use http_body_util::combinators::UnsyncBoxBody;
#[cfg(any(
    feature = "dynamic_routing",
    all(feature = "v2", feature = "revenue_recovery")
))]
use hyper::body::Bytes;
#[cfg(any(
    feature = "dynamic_routing",
    all(feature = "v2", feature = "revenue_recovery")
))]
use hyper_util::client::legacy::connect::HttpConnector;
#[cfg(feature = "dynamic_routing")]
use router_env::logger;
use serde;
#[cfg(any(
    feature = "dynamic_routing",
    all(feature = "v2", feature = "revenue_recovery")
))]
use tonic::Status;

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
pub use self::revenue_recovery::recovery_decider_client::{
    DeciderRequest, DeciderResponse, RecoveryDeciderClientConfig, RecoveryDeciderClientInterface,
    RecoveryDeciderError, RecoveryDeciderResult,
};

#[cfg(any(
    feature = "dynamic_routing",
    all(feature = "v2", feature = "revenue_recovery")
))]
/// Hyper based Client type for maintaining connection pool for all gRPC services
pub type Client = hyper_util::client::legacy::Client<HttpConnector, UnsyncBoxBody<Bytes, Status>>;

/// Struct contains all the gRPC Clients
#[derive(Debug, Clone)]
pub struct GrpcClients {
    /// The routing client
    #[cfg(feature = "dynamic_routing")]
    pub dynamic_routing: RoutingStrategy,
    /// Health Check client for all gRPC services
    #[cfg(feature = "dynamic_routing")]
    pub health_client: HealthCheckClient,
    /// Recovery Decoder Client
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    pub recovery_decider_client: Box<dyn RecoveryDeciderClientInterface>,
}

/// Type that contains the configs required to construct a  gRPC client with its respective services.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct GrpcClientSettings {
    #[cfg(feature = "dynamic_routing")]
    /// Configs for Dynamic Routing Client
    pub dynamic_routing_client: DynamicRoutingClientConfig,
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    /// Configs for Recovery Decider Client
    pub recovery_decider_client: RecoveryDeciderClientConfig,
}

impl GrpcClientSettings {
    /// # Panics
    ///
    /// This function will panic if it fails to establish a connection with the gRPC server.
    /// This function will be called at service startup.
    #[allow(clippy::expect_used)]
    pub async fn get_grpc_client_interface(&self) -> Arc<GrpcClients> {
        // Define the hyper client if any gRPC feature is enabled
        #[cfg(any(
            feature = "dynamic_routing",
            all(feature = "v2", feature = "revenue_recovery")
        ))]
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .http2_only(true)
                .build_http();

        #[cfg(feature = "dynamic_routing")]
        let dynamic_routing_connection = self
            .dynamic_routing_client
            .clone()
            .get_dynamic_routing_connection(client.clone())
            .await
            .expect("Failed to establish a connection with the Dynamic Routing Server");

        #[cfg(feature = "dynamic_routing")]
        let health_client = HealthCheckClient::build_connections(self, client.clone())
            .await
            .expect("Failed to build gRPC connections");

        #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
        let recovery_decider_client = self
            .recovery_decider_client
            .get_recovery_decider_connection(client.clone())
            .map(|client| -> Box<dyn RecoveryDeciderClientInterface> { Box::new(client) })
            .expect("Failed to establish a connection with the Recovery Decider Server");

        Arc::new(GrpcClients {
            #[cfg(feature = "dynamic_routing")]
            dynamic_routing: dynamic_routing_connection,
            #[cfg(feature = "dynamic_routing")]
            health_client,
            #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
            recovery_decider_client,
        })
    }
}

/// Contains grpc headers
#[derive(Debug)]
pub struct GrpcHeaders {
    /// Tenant id
    pub tenant_id: String,
    /// Request id
    pub request_id: Option<String>,
}

#[cfg(feature = "dynamic_routing")]
/// Trait to add necessary headers to the tonic Request
pub(crate) trait AddHeaders {
    /// Add necessary header fields to the tonic Request
    fn add_headers_to_grpc_request(&mut self, headers: GrpcHeaders);
}

#[cfg(feature = "dynamic_routing")]
impl<T> AddHeaders for tonic::Request<T> {
    #[track_caller]
    fn add_headers_to_grpc_request(&mut self, headers: GrpcHeaders) {
        headers.tenant_id
            .parse()
            .map(|tenant_id| {
                self
                    .metadata_mut()
                    .append(consts::TENANT_HEADER, tenant_id)
            })
            .inspect_err(
                |err| logger::warn!(header_parse_error=?err,"invalid {} received",consts::TENANT_HEADER),
            )
            .ok();

        headers.request_id.map(|request_id| {
            request_id
                .parse()
                .map(|request_id| {
                    self
                        .metadata_mut()
                        .append(consts::X_REQUEST_ID, request_id)
                })
                .inspect_err(
                    |err| logger::warn!(header_parse_error=?err,"invalid {} received",consts::X_REQUEST_ID),
                )
                .ok();
        });
    }
}

#[cfg(feature = "dynamic_routing")]
pub(crate) fn create_grpc_request<T: Debug>(message: T, headers: GrpcHeaders) -> tonic::Request<T> {
    let mut request = tonic::Request::new(message);
    request.add_headers_to_grpc_request(headers);

    logger::info!(request=?request);

    request
}
