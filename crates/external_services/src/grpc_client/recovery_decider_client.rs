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
pub mod recovery_decider {
    tonic::include_proto!("recovery_decider");
}

use recovery_decider::recovery_decider_service_client::RecoveryDeciderServiceClient;
pub use recovery_decider::{RecoveryDeciderRequest, RecoveryDeciderResponse};

#[allow(missing_docs)]
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

#[allow(missing_docs)]
/// Client for interacting with the Recovery Decider gRPC service.
#[derive(Debug, Clone)]
pub struct RecoveryDeciderClient {
    client: RecoveryDeciderServiceClient<Client>,
}

impl RecoveryDeciderClient {
    #[allow(missing_docs)]
    pub async fn get_recovery_decider_connection(
        config: RecoveryDeciderClientConfig,
        hyper_client: Client,
    ) -> Result<Self, Report<RecoveryDeciderError>> {
        let host = config.host;
        let port = config.port;

        if host.is_empty() {
            return Err(Report::new(RecoveryDeciderError::ConfigError(
                "Host is not configured for Recovery Decider client".to_string(),
            )));
        }

        let uri_string = format!("http://{}:{}", host, port);
        let uri = uri_string
            .parse::<tonic::transport::Uri>()
            .map_err(Report::from)
            .change_context_lazy(|| {
                RecoveryDeciderError::ConfigError(format!("Invalid URI: {}", uri_string))
            })?;

        let service_client = RecoveryDeciderServiceClient::with_origin(hyper_client, uri);

        Ok(Self {
            client: service_client,
        })
    }

    #[allow(clippy::too_many_arguments, missing_docs)]
    pub async fn get_decider(
        &mut self,
        first_error_message: String,
        billing_state: String,
        card_funding: String,
        card_network: String,
        card_issuer: String,
        txn_time: i64,
        headers: GrpcHeaders,
    ) -> RecoveryDeciderResult<RecoveryDeciderResponse> {
        let request = grpc_client::create_grpc_request(
            RecoveryDeciderRequest {
                first_error_message,
                billing_state,
                card_funding,
                card_network,
                card_issuer,
                txn_time,
            },
            headers,
        );

        logger::debug!(recovery_decider_response =?request);

        let response = self
            .client
            .should_retry(request)
            .await
            .map_err(|status| {
                logger::error!(grpc_error =?status, "Recovery Decider gRPC call failed");
                RecoveryDeciderError::ServiceError(status.message().to_string())
            })?
            .into_inner();

        logger::debug!(recovery_decider_response =?response);

        Ok(response)
    }
}

#[allow(missing_docs)]
/// Configuration for the Recovery Decider gRPC client.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RecoveryDeciderClientConfig {
    pub host: String,
    pub port: u16,
}

impl Default for RecoveryDeciderClientConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(), // Default host for recovery Decider
            port: 50052,                   // Default port for recovery Decider
        }
    }
}

impl common_utils::events::ApiEventMetric for RecoveryDeciderResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl common_utils::events::ApiEventMetric for RecoveryDeciderRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}
