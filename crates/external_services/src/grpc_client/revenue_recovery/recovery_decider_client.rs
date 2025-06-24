#![cfg(all(feature = "revenue_recovery", feature = "v2"))]

use std::fmt::Debug;

use common_utils::errors::CustomResult;
use error_stack::{Report, ResultExt};
use router_env::logger;
use serde;

use crate::grpc_client::Client;

#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::use_self
)]
pub mod decider {
    tonic::include_proto!("decider");
}

use decider::decider_client::DeciderClient;
pub use decider::{DeciderRequest, DeciderResponse};

/// Recovery Decider result
pub type RecoveryDeciderResult<T> = CustomResult<T, RecoveryDeciderError>;

#[allow(missing_docs)]
#[derive(Debug, Clone, thiserror::Error)]
pub enum RecoveryDeciderError {
    /// Error establishing gRPC connection
    #[error("Failed to establish connection with Recovery Decider service: {0}")]
    ConnectionError(String),
    /// Error received from the gRPC service
    #[error("Recovery Decider service returned an error: {0}")]
    ServiceError(String),
    /// Missing configuration for the client
    #[error("Recovery Decider client configuration is missing or invalid")]
    ConfigError(String),
}

/// Recovery Decider Client type
#[async_trait::async_trait]
pub trait RecoveryDeciderClientInterface: dyn_clone::DynClone + Send + Sync + Debug {
    /// fn to call gRPC service
    async fn decide_on_retry(
        &mut self,
        request_payload: DeciderRequest,
        recovery_headers: super::common::GrpcRecoveryHeaders,
    ) -> RecoveryDeciderResult<DeciderResponse>;
}

dyn_clone::clone_trait_object!(RecoveryDeciderClientInterface);

/// Configuration for the Recovery Decider gRPC client.
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct RecoveryDeciderClientConfig {
    /// Host
    pub host: String,
    /// port number
    pub port: u16,
}

impl RecoveryDeciderClientConfig {
    /// create a connection
    pub fn get_recovery_decider_connection(
        &self,
        hyper_client: Client,
    ) -> Result<DeciderClient<Client>, Report<RecoveryDeciderError>> {
        let host = &self.host;
        let port = self.port;

        if host.is_empty() {
            return Err(Report::new(RecoveryDeciderError::ConfigError(
                "Host is not configured for Recovery Decider client".to_string(),
            )));
        }

        let uri_string = format!("http://{}:{}", host, port);
        let uri = uri_string
            .parse::<tonic::transport::Uri>()
            .map_err(Report::from)
            .change_context(RecoveryDeciderError::ConfigError(format!(
                "Invalid URI: {}",
                uri_string
            )))?;

        let service_client = DeciderClient::with_origin(hyper_client, uri);

        Ok(service_client)
    }
}

#[async_trait::async_trait]
impl RecoveryDeciderClientInterface for DeciderClient<Client> {
    /// collects the request from HS and sends it to recovery decider gRPC service
    async fn decide_on_retry(
        &mut self,
        request_payload: DeciderRequest,
        recovery_headers: super::common::GrpcRecoveryHeaders,
    ) -> RecoveryDeciderResult<DeciderResponse> {
        let request =
            super::common::create_revenue_recovery_grpc_request(request_payload, recovery_headers);

        logger::debug!(decider_request =?request);

        let grpc_response = self
            .decide(request)
            .await
            .map_err(|status| {
                logger::error!(grpc_error =?status, "Decider service call failed");
                RecoveryDeciderError::ServiceError(status.message().to_string())
            })?
            .into_inner();

        logger::debug!(grpc_decider_response =?grpc_response);

        Ok(grpc_response)
    }
}
