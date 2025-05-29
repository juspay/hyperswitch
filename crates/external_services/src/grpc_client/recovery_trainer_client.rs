use common_utils::errors::CustomResult;
use error_stack::{Report, ResultExt};
use router_env::logger;

use super::Client;
use crate::grpc_client::{self, GrpcHeaders};

#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::use_self
)]
pub mod recovery_trainer {
    tonic::include_proto!("recovery_trainer");
}

use recovery_trainer::recovery_trainer_service_client::RecoveryTrainerServiceClient;
pub use recovery_trainer::{RecoveryTrainerRequest, RecoveryTrainerResponse};

#[allow(missing_docs)]
pub type RecoveryTrainerResult<T> = CustomResult<T, RecoveryTrainerError>;

#[allow(missing_docs)]
#[derive(Debug, Clone, thiserror::Error)]
pub enum RecoveryTrainerError {
    /// Error establishing gRPC connection
    #[error("Failed to establish connection with Recovery Trainer service: {0}")]
    ConnectionError(String),
    /// Error received from the gRPC service
    #[error("Recovery Trainer service returned an error: {0}")]
    ServiceError(String),
    /// Missing configuration for the client
    #[error("Recovery Trainer client configuration is missing or invalid")]
    ConfigError(String),
}

#[allow(missing_docs)]
/// Client for interacting with the Recovery Trainer gRPC service.
#[derive(Debug, Clone)]
pub struct RecoveryTrainerClient {
    client: RecoveryTrainerServiceClient<Client>,
}

impl RecoveryTrainerClient {
    #[allow(missing_docs)]
    pub async fn get_recovery_trainer_connection(
        config: RecoveryTrainerClientConfig,
        hyper_client: Client,
    ) -> Result<Self, Report<RecoveryTrainerError>> {
        let host = config.host;
        let port = config.port;

        if host.is_empty() {
            return Err(Report::new(RecoveryTrainerError::ConfigError(
                "Host is not configured for Recovery Trainer client".to_string(),
            )));
        }

        let uri_string = format!("http://{}:{}", host, port);
        let uri = uri_string
            .parse::<tonic::transport::Uri>()
            .map_err(Report::from)
            .change_context_lazy(|| {
                RecoveryTrainerError::ConfigError(format!("Invalid URI: {}", uri_string))
            })?;

        let service_client = RecoveryTrainerServiceClient::with_origin(hyper_client, uri);

        Ok(Self {
            client: service_client,
        })
    }

    #[allow(clippy::too_many_arguments, missing_docs)]
    pub async fn get_trainer_time(
        &mut self,
        first_error_message: String,
        billing_state: String,
        card_funding: String,
        card_network: String,
        card_issuer: String,
        txn_time: i64,
        headers: GrpcHeaders,
    ) -> RecoveryTrainerResult<RecoveryTrainerResponse> {
        let request = grpc_client::create_grpc_request(
            RecoveryTrainerRequest {
                first_error_message,
                billing_state,
                card_funding,
                card_network,
                card_issuer,
                txn_time,
            },
            headers,
        );

        logger::debug!(recovery_trainer_request =?request);

        let response = self
            .client
            .should_retry(request)
            .await
            .map_err(|status| {
                logger::error!(grpc_error =?status, "Recovery Trainer gRPC call failed");
                RecoveryTrainerError::ServiceError(status.message().to_string())
            })?
            .into_inner();

        logger::debug!(recovery_trainer_response =?response);

        Ok(response)
    }
}

#[allow(missing_docs)]
/// Configuration for the Recovery Trainer gRPC client.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RecoveryTrainerClientConfig {
    pub host: String,
    pub port: u16,
}

impl Default for RecoveryTrainerClientConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(), // Default host for recovery trainer
            port: 50052,                   // Default port for recovery trainer
        }
    }
}
