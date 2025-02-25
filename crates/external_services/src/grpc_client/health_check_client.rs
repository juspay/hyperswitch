use std::{collections::HashMap, fmt::Debug};

use api_models::health_check::{HealthCheckMap, HealthCheckServices};
use common_utils::{errors::CustomResult, ext_traits::AsyncExt};
use error_stack::ResultExt;
pub use health_check::{
    health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
    HealthCheckResponse,
};
use router_env::logger;

#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::use_self
)]
pub mod health_check {
    tonic::include_proto!("grpc.health.v1");
}

use super::{Client, DynamicRoutingClientConfig, GrpcClientSettings};

/// Result type for Dynamic Routing
pub type HealthCheckResult<T> = CustomResult<T, HealthCheckError>;
/// Dynamic Routing Errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum HealthCheckError {
    /// The required input is missing
    #[error("Missing fields: {0} for building the Health check connection")]
    MissingFields(String),
    /// Error from gRPC Server
    #[error("Error from gRPC Server : {0}")]
    ConnectionError(String),
    /// status is invalid
    #[error("Invalid Status from server")]
    InvalidStatus,
}

/// Health Check Client type
#[derive(Debug, Clone)]
pub struct HealthCheckClient {
    /// Health clients for all gRPC based services
    pub clients: HashMap<HealthCheckServices, HealthClient<Client>>,
}

impl HealthCheckClient {
    /// Build connections to all gRPC services
    pub async fn build_connections(
        config: &GrpcClientSettings,
        client: Client,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let dynamic_routing_config = &config.dynamic_routing_client;
        let connection = match dynamic_routing_config {
            DynamicRoutingClientConfig::Enabled {
                host,
                port,
                service,
            } => Some((host.clone(), *port, service.clone())),
            _ => None,
        };

        let mut client_map = HashMap::new();

        if let Some(conn) = connection {
            let uri = format!("http://{}:{}", conn.0, conn.1).parse::<tonic::transport::Uri>()?;
            let health_client = HealthClient::with_origin(client, uri);

            client_map.insert(HealthCheckServices::DynamicRoutingService, health_client);
        }

        Ok(Self {
            clients: client_map,
        })
    }
    /// Perform health check for all services involved
    pub async fn perform_health_check(
        &self,
        config: &GrpcClientSettings,
    ) -> HealthCheckResult<HealthCheckMap> {
        let dynamic_routing_config = &config.dynamic_routing_client;
        let connection = match dynamic_routing_config {
            DynamicRoutingClientConfig::Enabled {
                host,
                port,
                service,
            } => Some((host.clone(), *port, service.clone())),
            _ => None,
        };

        let health_client = self
            .clients
            .get(&HealthCheckServices::DynamicRoutingService);

        // SAFETY : This is a safe cast as there exists a valid
        // integer value for this variant
        #[allow(clippy::as_conversions)]
        let expected_status = ServingStatus::Serving as i32;

        let mut service_map = HealthCheckMap::new();

        let health_check_succeed = connection
            .as_ref()
            .async_map(|conn| self.get_response_from_grpc_service(conn.2.clone(), health_client))
            .await
            .transpose()
            .change_context(HealthCheckError::ConnectionError(
                "error calling dynamic routing service".to_string(),
            ))
            .map_err(|err| logger::error!(error=?err))
            .ok()
            .flatten()
            .is_some_and(|resp| resp.status == expected_status);

        connection.and_then(|_conn| {
            service_map.insert(
                HealthCheckServices::DynamicRoutingService,
                health_check_succeed,
            )
        });

        Ok(service_map)
    }

    async fn get_response_from_grpc_service(
        &self,
        service: String,
        client: Option<&HealthClient<Client>>,
    ) -> HealthCheckResult<HealthCheckResponse> {
        let request = tonic::Request::new(HealthCheckRequest { service });

        let mut client = client
            .ok_or(HealthCheckError::MissingFields(
                "[health_client]".to_string(),
            ))?
            .clone();

        let response = client
            .check(request)
            .await
            .change_context(HealthCheckError::ConnectionError(
                "Failed to call dynamic routing service".to_string(),
            ))?
            .into_inner();

        Ok(response)
    }
}
