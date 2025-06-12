/// Dyanimc Routing Client interface implementation
#[cfg(feature = "dynamic_routing")]
pub mod dynamic_routing;
/// gRPC based Heath Check Client interface implementation
#[cfg(feature = "dynamic_routing")]
pub mod health_check_client;
/// gRPC based Trainer Client interface implementation
#[cfg(feature = "v2")]
pub mod recovery_trainer_client;

use std::{fmt::Debug, sync::Arc};

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
use recovery_trainer_client::{TrainerClientConfig, TrainerClientInterface};
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
    #[cfg(feature = "v2")]
    #[allow(missing_docs)]
    pub trainer_client: Box<dyn TrainerClientInterface>,
}

/// Type that contains the configs required to construct a  gRPC client with its respective services.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct GrpcClientSettings {
    #[cfg(feature = "dynamic_routing")]
    /// Configs for Dynamic Routing Client
    pub dynamic_routing_client: DynamicRoutingClientConfig,
    #[cfg(feature = "v2")]
    /// Configs for Trainer Client
    pub trainer_client: TrainerClientConfig,
}

impl GrpcClientSettings {
    /// # Panics
    ///
    /// This function will panic if it fails to establish a connection with the gRPC server.
    /// This function will be called at service startup.
    #[allow(clippy::expect_used)]
    /// Asynchronously constructs and returns a shared instance of all enabled gRPC clients.
    ///
    /// Initializes and connects the required gRPC clients based on enabled features, including dynamic routing, health check, and trainer clients. Panics if any client connection fails.
    ///
    /// # Returns
    /// An `Arc<GrpcClients>` containing all successfully connected gRPC client interfaces.
    ///
    /// # Panics
    /// Panics if any gRPC client connection cannot be established.
    ///
    /// # Examples
    ///
    /// ```
    /// let settings = GrpcClientSettings::default();
    /// let clients = settings.get_grpc_client_interface().await;
    /// // Use `clients` to access the enabled gRPC services.
    /// ```
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
        let trainer_client = self
            .trainer_client
            .get_trainer_service_client(client.clone())
            .map(|client| {
                #[allow(clippy::as_conversions)]
                {
                    Box::new(client) as Box<dyn TrainerClientInterface>
                }
            })
            .expect("Failed to establish a connection with the Trainer Server");

        Arc::new(GrpcClients {
            #[cfg(feature = "dynamic_routing")]
            dynamic_routing: dynamic_routing_connection,
            #[cfg(feature = "dynamic_routing")]
            health_client,
            #[cfg(feature = "v2")]
            trainer_client,
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

#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
/// Trait to add necessary headers to the tonic Request
pub(crate) trait AddHeaders {
    /// Add necessary header fields to the tonic Request
    fn add_headers_to_grpc_request(&mut self, headers: GrpcHeaders);
}

#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
impl<T> AddHeaders for tonic::Request<T> {
    #[track_caller]
    /// Adds tenant and optional request ID headers to the gRPC request metadata.
    ///
    /// Parses and appends the tenant ID and, if present, the request ID from the provided `GrpcHeaders` to the request's metadata. Invalid header values are ignored and logged as warnings.
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

#[cfg(any(feature = "dynamic_routing", feature = "v2"))]
/// Constructs a tonic gRPC request with the specified message and attaches provided gRPC headers.
///
/// The function creates a new `tonic::Request` containing the given message and injects metadata headers such as tenant ID and optional request ID from the provided `GrpcHeaders`.
///
/// # Examples
///
/// ```
/// let headers = GrpcHeaders {
///     tenant_id: "tenant-123".to_string(),
///     request_id: Some("req-456".to_string()),
/// };
/// let message = MyGrpcMessage::default();
/// let request = create_grpc_request(message, headers);
/// assert_eq!(request.metadata().get("x-tenant-id").unwrap(), "tenant-123");
/// ```
pub(crate) fn create_grpc_request<T: Debug>(message: T, headers: GrpcHeaders) -> tonic::Request<T> {
    let mut request = tonic::Request::new(message);
    request.add_headers_to_grpc_request(headers);

    logger::info!(request=?request);

    request
}
