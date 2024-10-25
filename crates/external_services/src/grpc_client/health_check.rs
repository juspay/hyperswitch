use std::fmt::Debug;

use common_utils::{errors::CustomResult, fp_utils};
use error_stack::ResultExt;

#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions
)]
pub mod health_check {
    tonic::include_proto!("grpc.health.v1");
}

pub use health_check::{
    health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
    HealthCheckResponse,
};

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

/// Health Check CLient type
#[derive(Debug, Clone)]
pub struct HealthCheckClient;

impl HealthCheckClient {
    /// Perform health check for all services involved
    pub async fn perform_health_check(&self, config: &GrpcClientSettings) -> HealthCheckResult<()> {
        #[cfg(feature = "dynamic_routing")]
        {
            let dynamic_routing_config = &config.dynamic_routing_client;
            let connection = match dynamic_routing_config {
                DynamicRoutingClientConfig::Enabled { host, port } => Some((host.clone(), *port)),
                _ => None,
            }
            .ok_or(HealthCheckError::MissingFields)?;

            let uri = format!("http://{}:{}", connection.0, connection.1);
            let channel = tonic::transport::Endpoint::new(uri)
                .map_err(|err| HealthCheckError::ConnectionError(err.to_string()))?
                .connect()
                .await
                .map_err(|err| HealthCheckError::ConnectionError(err.to_string()))?;
            let mut client = HealthClient::new(channel);

            let request = tonic::Request::new(HealthCheckRequest {
                service: "dynamo".to_string(),
            });

            let response = client
                .check(request)
                .await
                .change_context(HealthCheckError::ConnectionError(
                    "error calling dynamic routing service".to_string(),
                ))?
                .into_inner();

            #[allow(clippy::as_conversions)]
            let expected_status = ServingStatus::Serving as i32;

            fp_utils::when(response.status != expected_status, || {
                Err(HealthCheckError::InvalidStatus)
            })?;

            Ok(())
        }
    }
}
