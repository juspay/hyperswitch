use std::fmt::Debug;

use common_utils::{errors::CustomResult, ext_traits::AsyncExt, fp_utils};
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
    pub async fn perform_health_check(&self, config: &GrpcClientSettings) -> HealthCheckResult<()> {
        let dynamic_routing_config = &config.dynamic_routing_client;
        let connection = match dynamic_routing_config {
            DynamicRoutingClientConfig::Enabled { host, port } => Some((host.clone(), *port)),
            _ => None,
        };

        let response: Option<HealthCheckResponse> = connection
            .async_map(|conn| self.get_response_from_grpc_service(conn.0, conn.1))
            .await
            .transpose()
            .change_context(HealthCheckError::ConnectionError(
                "error calling dynamic routing service".to_string(),
            ))?;

        #[allow(clippy::as_conversions)]
        let expected_status = ServingStatus::Serving as i32;

        if let Some(resp) = response {
            fp_utils::when(resp.status != expected_status, || {
                Err(HealthCheckError::InvalidStatus)
            })?;
        }

        Ok(())
    }

    async fn get_response_from_grpc_service(
        &self,
        host: String,
        port: u16,
    ) -> HealthCheckResult<HealthCheckResponse> {
        let uri = format!("http://{}:{}", host, port);
        let channel = tonic::transport::Endpoint::new(uri)
            .map_err(|err| HealthCheckError::ConnectionError(err.to_string()))?
            .connect()
            .await
            .map_err(|err| HealthCheckError::ConnectionError(err.to_string()))?;
        let mut client = HealthClient::new(channel);

        let request = tonic::Request::new(HealthCheckRequest {
            service: "dynamo".to_string(), // check this in review
        });

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
