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

/// Represents the response from the decider service, suitable for HTTP JSON serialization.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeciderResponseForSerde {
    /// Flag indicating if a retry is recommended.
    pub retry_flag: bool,
    /// The recommended time for a retry, if applicable.
    // This field uses a custom wrapper `SerializableTimestamp` for Serde compatibility.
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

#[async_trait::async_trait]
#[allow(missing_docs)]
#[allow(clippy::too_many_arguments)]
pub trait RecoveryDeciderClientInterface: dyn_clone::DynClone + Send + Sync + std::fmt::Debug {
    #[allow(missing_docs)]
    #[allow(clippy::too_many_arguments)]
    async fn decide_on_retry(
        &mut self,
        first_error_message: String,
        billing_state: String,
        card_funding: String,
        card_network: String,
        card_issuer: String,
        start_time: Option<prost_types::Timestamp>,
        end_time: Option<prost_types::Timestamp>,
        retry_count: f64,
        headers: GrpcHeaders,
    ) -> RecoveryDeciderResult<DeciderResponseForSerde>;
}

dyn_clone::clone_trait_object!(RecoveryDeciderClientInterface);

#[allow(missing_docs)]
/// Configuration for the Recovery Decider gRPC client.
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct RecoveryDeciderClientConfig {
    pub host: String,
    pub port: u16,
}

impl RecoveryDeciderClientConfig {
    #[allow(missing_docs)]
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
        .change_context_lazy(|| {
            RecoveryDeciderError::ConfigError(format!("Invalid URI: {}", uri_string))
        })?;

    let service_client = DeciderClient::with_origin(hyper_client, uri);

    Ok(service_client)
}
}

#[async_trait::async_trait]
impl RecoveryDeciderClientInterface for DeciderClient<Client> {
    #[allow(clippy::too_many_arguments, missing_docs)]
    async fn decide_on_retry(
        &mut self,
        first_error_message: String,
        billing_state: String,
        card_funding: String,
        card_network: String,
        card_issuer: String,
        start_time: Option<prost_types::Timestamp>,
        end_time: Option<prost_types::Timestamp>,
        retry_count: f64,
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
            retry_count,
        };
        let request = grpc_client::create_grpc_request(request_data, headers);

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

        // Map to our Serde-compatible struct
        let response_for_serde = DeciderResponseForSerde {
            retry_flag: grpc_response.retry_flag,
            retry_time: grpc_response.retry_time.map(SerializableTimestamp::from),
        };

        Ok(response_for_serde)
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
