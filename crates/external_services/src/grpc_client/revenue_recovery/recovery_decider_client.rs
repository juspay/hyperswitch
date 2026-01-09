use std::fmt::Debug;

use common_utils::errors::CustomResult;
use error_stack::{Report, ResultExt};
use router_env::logger;

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

/// Recovery Decider Error
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
        recovery_headers: super::GrpcRecoveryHeaders,
    ) -> RecoveryDeciderResult<DeciderResponse>;
}

dyn_clone::clone_trait_object!(RecoveryDeciderClientInterface);

/// Configuration for the Recovery Decider gRPC client.
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct RecoveryDeciderClientConfig {
    /// Base URL of the Recovery Decider service
    pub base_url: String,
}

impl RecoveryDeciderClientConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), RecoveryDeciderError> {
        use common_utils::fp_utils::when;

        when(self.base_url.is_empty(), || {
            Err(RecoveryDeciderError::ConfigError(
                "Recovery Decider base URL cannot be empty when configuration is provided"
                    .to_string(),
            ))
        })
    }

    /// create a connection
    pub fn get_recovery_decider_connection(
        &self,
        hyper_client: Client,
    ) -> Result<DeciderClient<Client>, Report<RecoveryDeciderError>> {
        let uri = self
            .base_url
            .parse::<tonic::transport::Uri>()
            .map_err(Report::from)
            .change_context(RecoveryDeciderError::ConfigError(format!(
                "Invalid URI: {}",
                self.base_url
            )))?;

        let service_client = DeciderClient::with_origin(hyper_client, uri);

        Ok(service_client)
    }
}

#[async_trait::async_trait]
impl RecoveryDeciderClientInterface for DeciderClient<Client> {
    async fn decide_on_retry(
        &mut self,
        request_payload: DeciderRequest,
        recovery_headers: super::GrpcRecoveryHeaders,
    ) -> RecoveryDeciderResult<DeciderResponse> {
        let request =
            super::create_revenue_recovery_grpc_request(request_payload, recovery_headers);

        logger::debug!(decider_request =?request);

        let grpc_response = self
            .decide(request)
            .await
            .change_context(RecoveryDeciderError::ServiceError(
                "Decider service call failed".to_string(),
            ))?
            .into_inner();

        logger::debug!(grpc_decider_response =?grpc_response);

        Ok(grpc_response)
    }
}
