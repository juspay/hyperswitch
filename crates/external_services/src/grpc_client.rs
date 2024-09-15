#[cfg(feature = "dynamic_routing")]
pub mod dynamic_routing;
#[cfg(feature = "dynamic_routing")]
use crate::grpc_client::dynamic_routing::{DynamicRoutingClientConfig, RoutingStrategy};
use router_env::logger;
use serde;
use std::fmt::Debug;

// Struct contains all the gRPC Clients
#[derive(Debug, Clone)]
pub struct GrpcClients {
    #[cfg(feature = "dynamic_routing")]
    pub dynamic_routing: RoutingStrategy,
}
/// Struct that contains the settings required to construct an Grpc client.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct GrpcClientSettings {
    #[cfg(feature = "dynamic_routing")]
    pub dynamic_routing_client: DynamicRoutingClientConfig,
}

impl GrpcClientSettings {
    /// # Panics
    ///
    /// This function will panic if it fails to establish a connection with the gRPC server.
    /// This function will be called at service startup.
    #[allow(clippy::expect_used)]
    pub async fn get_grpc_client_interface(&self) -> GrpcClients {
        #[cfg(feature = "dynamic_routing")]
        let dynamic_routing_connection = self
            .dynamic_routing_client
            .clone()
            .get_dynamic_routing_connection()
            .await
            .expect("Failed to establish a connection with the gRPC Server");

        logger::info!("Connection established with Grpc Server");

        GrpcClients {
            #[cfg(feature = "dynamic_routing")]
            dynamic_routing: dynamic_routing_connection,
        }
    }
}
