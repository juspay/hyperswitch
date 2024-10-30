use std::fmt::Debug;

use api_models::health_check::{HealthCheckMap, HealthCheckServices};
use common_utils::{errors::CustomResult, ext_traits::AsyncExt};
use error_stack::ResultExt;
pub use health_check::{
    health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
    HealthCheckResponse,
};

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

use super::{DynamicRoutingClientConfig, GrpcClientSettings};

/// Result type for Dynamic Routing
pub type HealthCheckResult<T> = CustomResult<T, HealthCheckError>;
/// Dynamic Routing Errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum HealthCheckError {
    /// The required input is missing
    #[error("Missing Host and Port for building the Health check connection")]
    MissingFields,
    /// Error from gRPC Server
    #[error("Error from gRPC Server : {0}")]
    ConnectionError(String),
    /// status is invalid
    #[error("Invalid Status from server")]
    InvalidStatus,
}

/// Health Check Client type
#[derive(Debug, Clone)]
pub struct HealthCheckClient;

impl HealthCheckClient {
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

        // SAFETY : This is a safe cast as there exists a valid
        // integer value for this variant
        #[allow(clippy::as_conversions)]
        let expected_status = ServingStatus::Serving as i32;

        let mut service_map = HealthCheckMap::new();

        let health_check_succeed = connection
            .async_map(|conn| self.get_response_from_grpc_service(conn.0, conn.1, conn.2))
            .await
            .transpose()
            .change_context(HealthCheckError::ConnectionError(
                "error calling dynamic routing service".to_string(),
            ))
            .ok()
            .flatten()
            .is_some_and(|resp| resp.status == expected_status);

        service_map.insert(
            HealthCheckServices::DynamicRoutingService,
            health_check_succeed,
        );

        Ok(service_map)
    }

    async fn get_response_from_grpc_service(
        &self,
        host: String,
        port: u16,
        service: String,
    ) -> HealthCheckResult<HealthCheckResponse> {
        let uri = format!("http://{}:{}", host, port);
        let channel = tonic::transport::Endpoint::new(uri)
            .map_err(|err| HealthCheckError::ConnectionError(err.to_string()))?
            .connect()
            .await
            .map_err(|err| HealthCheckError::ConnectionError(err.to_string()))?;
        let mut client = HealthClient::new(channel);

        let request = tonic::Request::new(HealthCheckRequest { service });

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
