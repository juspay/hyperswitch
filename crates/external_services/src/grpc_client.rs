/// Dyanimc Routing Client interface implementation
#[cfg(feature = "dynamic_routing")]
pub mod dynamic_routing;
/// gRPC based Heath Check Client interface implementation
#[cfg(feature = "dynamic_routing")]
pub mod health_check_client;
/// gRPC based Recovery Trainer Client interface implementation
#[cfg(feature = "v2")]
pub mod recovery_decider_client;

use std::{fmt::Debug, sync::Arc};

#[cfg(feature = "v2")]
use common_enums::enums;
#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
use common_utils::consts;
#[cfg(feature = "dynamic_routing")]
use dynamic_routing::{DynamicRoutingClientConfig, RoutingStrategy};
#[cfg(feature = "dynamic_routing")]
use health_check_client::HealthCheckClient;
#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
use http_body_util::combinators::UnsyncBoxBody;
#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
use hyper::body::Bytes;
#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
use hyper_util::client::legacy::connect::HttpConnector;
#[cfg(feature = "v2")]
use recovery_decider_client::{RecoveryDeciderClientConfig, RecoveryDeciderClientInterface};
#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
use router_env::logger;
use serde;
#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
use tonic::Status;

#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
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
    #[cfg(feature = "v2")]
    pub recovery_decider_client: Box<dyn RecoveryDeciderClientInterface>,
}

/// Type that contains the configs required to construct a  gRPC client with its respective services.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct GrpcClientSettings {
    #[cfg(feature = "dynamic_routing")]
    /// Configs for Dynamic Routing Client
    pub dynamic_routing_client: DynamicRoutingClientConfig,
    #[cfg(feature = "v2")]
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
        #[cfg(any(feature = "dynamic_routing", feature = "v2"))]
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

        #[cfg(feature = "v2")]
        let recovery_decider_client = self
            .recovery_decider_client
            .get_recovery_decider_connection(client.clone())
            .map(|client| -> Box<dyn RecoveryDeciderClientInterface> {
                // Explicit return type for closure
                Box::new(client)
            })
            .expect("Failed to establish a connection with the Recovery Decider Server");

        Arc::new(GrpcClients {
            #[cfg(feature = "dynamic_routing")]
            dynamic_routing: dynamic_routing_connection,
            #[cfg(feature = "dynamic_routing")]
            health_client,
            #[cfg(feature = "v2")]
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

/// Contains recovery grpc headers
#[derive(Debug)]
pub struct GrpcRecoveryHeaders {
    /// Merchant id
    pub merchant_id: String,
    /// Version
    pub version: Option<enums::RecoveryModelVersion>,
}

#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
/// Trait to add necessary headers to the tonic Request
pub(crate) trait AddHeaders {
    /// Add necessary header fields to the tonic Request
    fn add_headers_to_grpc_request(&mut self, headers: GrpcHeaders);
}

#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
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

#[cfg(feature = "v2")]
/// Trait to add necessary recovery headers to the tonic Request
pub(crate) trait AddRecoveryHeaders {
    /// Add necessary recovery header fields to the tonic Request
    fn add_recovery_headers(&mut self, headers: GrpcRecoveryHeaders);
}

#[cfg(feature = "v2")]
impl<T> AddRecoveryHeaders for tonic::Request<T> {
    #[track_caller]
    fn add_recovery_headers(&mut self, headers: GrpcRecoveryHeaders) {
        headers
            .merchant_id
            .parse()
            .map(|merchant_id_val| {
                self.metadata_mut()
                    .append(consts::MERCHANT_ID_HEADER, merchant_id_val);
            })
            .inspect_err(|err| {
                logger::warn!(header_parse_error=?err, "invalid {} received", consts::MERCHANT_ID_HEADER)
            })
            .ok();

        if let Some(version) = headers.version {
            version
                .to_string()
                .parse()
                .map(|version_val| {
                    self.metadata_mut()
                        .append(consts::RECOVERY_MODEL_VERSION_HEADER, version_val);
                })
                .inspect_err(|err| {
                    logger::warn!(header_parse_error=?err, "invalid {} received", consts::RECOVERY_MODEL_VERSION_HEADER)
                })
                .ok();
        }
    }
}

#[cfg(feature = "v2")]
pub(crate) fn create_recovery_decider_grpc_request<T: Debug>(
    message: T,
    recovery_headers: GrpcRecoveryHeaders,
) -> tonic::Request<T> {
    let mut request = tonic::Request::new(message);
    request.add_recovery_headers(recovery_headers);
    logger::info!(message_sent_to_recovery_decider_server=?request);
    request
}
