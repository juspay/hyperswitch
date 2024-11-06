/// Dyanimc Routing Client interface implementation
#[cfg(feature = "dynamic_routing")]
pub mod dynamic_routing;
/// gRPC based Heath Check Client interface implementation
#[cfg(feature = "dynamic_routing")]
pub mod health_check_client;
use std::{fmt::Debug, sync::Arc};

#[cfg(feature = "dynamic_routing")]
use dynamic_routing::{DynamicRoutingClientConfig, RoutingStrategy};
#[cfg(feature = "dynamic_routing")]
use health_check_client::HealthCheckClient;
use serde;

/// Struct contains all the gRPC Clients
#[derive(Debug, Clone)]
pub struct GrpcClients {
    /// The routing client
    #[cfg(feature = "dynamic_routing")]
    pub dynamic_routing: RoutingStrategy,
    /// Health Check client for all gRPC services
    #[cfg(feature = "dynamic_routing")]
    pub health_client: HealthCheckClient,
}
/// Type that contains the configs required to construct a  gRPC client with its respective services.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct GrpcClientSettings {
    #[cfg(feature = "dynamic_routing")]
    /// Configs for Dynamic Routing Client
    pub dynamic_routing_client: DynamicRoutingClientConfig,
}

impl GrpcClientSettings {
    /// # Panics
    ///
    /// This function will panic if it fails to establish a connection with the gRPC server.
    /// This function will be called at service startup.
    #[allow(clippy::expect_used)]
    pub async fn get_grpc_client_interface(&self) -> Arc<GrpcClients> {
        #[cfg(feature = "dynamic_routing")]
        let dynamic_routing_connection = self
            .dynamic_routing_client
            .clone()
            .get_dynamic_routing_connection()
            .await
            .expect("Failed to establish a connection with the Dynamic Routing Server");

        #[cfg(feature = "dynamic_routing")]
        let health_client = HealthCheckClient::build_connections(self)
            .await
            .expect("Failed to build gRPC connections");

        Arc::new(GrpcClients {
            #[cfg(feature = "dynamic_routing")]
            dynamic_routing: dynamic_routing_connection,
            #[cfg(feature = "dynamic_routing")]
            health_client,
        })
    }
}
