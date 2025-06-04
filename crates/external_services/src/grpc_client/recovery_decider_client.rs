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
pub mod decider {
    tonic::include_proto!("decider");
}

use common_utils::custom_serde::prost_timestamp::SerializableTimestamp;
use decider::decider_client::DeciderClient;
pub use decider::DeciderRequest;

// This is the struct that will be serialized/deserialized by Actix
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeciderResponseForSerde {
    pub retry_flag: bool,
    // This field will use the custom serde logic defined in build.rs
    // via field_type and field_attribute
    pub retry_time: Option<SerializableTimestamp>,
}

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
    client: DeciderClient<Client>,
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

        let service_client = DeciderClient::with_origin(hyper_client, uri);

        Ok(Self {
            client: service_client,
        })
    }

    #[allow(clippy::too_many_arguments, missing_docs)]
    pub async fn decide_on_retry(
        &mut self,
        first_error_message: String,
        billing_state: String,
        card_funding: String,
        card_network: String,
        card_issuer: String,
        start_time: Option<prost_types::Timestamp>,
        end_time: Option<prost_types::Timestamp>,
        headers: GrpcHeaders,
    ) -> RecoveryDeciderResult<DeciderResponseForSerde> {
        let request_data = DeciderRequest {
            first_error_message,
            billing_state,
            card_funding,
            card_network,
            card_issuer,
            start_time,
            end_time,
        };
        let request = grpc_client::create_grpc_request(request_data, headers);

        logger::debug!(decider_request =?request);

        let grpc_response = self
            .client
            .decide(request)
            .await
            .map_err(|status| {
                logger::error!(grpc_error =?status, "Decider service call failed");
                RecoveryDeciderError::ServiceError(status.message().to_string())
            })?
            .into_inner();

        logger::debug!(grpc_decider_response =?grpc_response);

        // Map to our Serde-compatible struct
        let response_for_serde = DeciderResponseForSerde {
            retry_flag: grpc_response.retry_flag,
            retry_time: grpc_response.retry_time.map(SerializableTimestamp::from),
        };

        Ok(response_for_serde)
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

impl common_utils::events::ApiEventMetric for DeciderResponseForSerde {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl common_utils::events::ApiEventMetric for DeciderRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}
