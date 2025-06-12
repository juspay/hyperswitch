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
pub mod trainer {
    tonic::include_proto!("trainer"); // Corresponds to package name in .proto
}

use trainer::trainer_service_client::TrainerServiceClient;
pub use trainer::{
    GetTrainingJobStatusRequest, GetTrainingJobStatusResponse, JobStatus, TriggerTrainingRequest,
    TriggerTrainingResponse,
};

#[allow(missing_docs)]
pub type TrainerResult<T> = CustomResult<T, TrainerError>;

#[allow(missing_docs)]
#[derive(Debug, Clone, thiserror::Error)]
pub enum TrainerError {
    #[error("Failed to establish connection with Trainer service: {0}")]
    ConnectionError(String),
    #[error("Trainer service returned an error: {0}")]
    ServiceError(String),
    #[error("Trainer client configuration is missing or invalid")]
    ConfigError(String),
}

#[async_trait::async_trait]
#[allow(missing_docs)]
pub trait TrainerClientInterface: dyn_clone::DynClone + Send + Sync + std::fmt::Debug {
    async fn get_training(
        &mut self,
        model_version_tag: String,
        enable_incremental_learning: bool,
        headers: GrpcHeaders,
    ) -> TrainerResult<TriggerTrainingResponse>;

    async fn get_the_training_job_status(
        &mut self,
        job_id: String,
        headers: GrpcHeaders,
    ) -> TrainerResult<GetTrainingJobStatusResponse>;
}

dyn_clone::clone_trait_object!(TrainerClientInterface);

#[async_trait::async_trait]
impl TrainerClientInterface for TrainerServiceClient<Client> {
    /// Triggers a training job on the Trainer service with the specified model version and incremental learning option.
    ///
    /// # Parameters
    /// - `model_version_tag`: The version tag of the model to be trained.
    /// - `enable_incremental_learning`: Whether to enable incremental learning for this training job.
    ///
    /// # Returns
    /// A result containing the response from the Trainer service if successful, or a `TrainerError` if the operation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut client = TrainerServiceClient::connect("http://localhost:50051").await.unwrap();
    /// let headers = GrpcHeaders::default();
    /// let response = client
    ///     .get_training("v1.2.3".to_string(), true, headers)
    ///     .await
    ///     .unwrap();
    /// assert!(response.job_id.len() > 0);
    /// ```
    async fn get_training(
        &mut self,
        model_version_tag: String,
        enable_incremental_learning: bool,
        headers: GrpcHeaders,
    ) -> TrainerResult<TriggerTrainingResponse> {
        let request_data = TriggerTrainingRequest {
            model_version_tag,
            enable_incremental_learning,
        };
        let request = grpc_client::create_grpc_request(request_data, headers);

        logger::debug!(trainer_trigger_training_request =?request);

        let response = self
            .trigger_training(request)
            .await
            .map_err(|status| {
                logger::error!(grpc_error =?status, "Trainer service TriggerTraining call failed");
                TrainerError::ServiceError(status.message().to_string())
            })?
            .into_inner();

        Ok(response)
    }

    /// Retrieves the status of a training job from the Trainer service by job ID.
    ///
    /// Sends a gRPC request to fetch the current status of the specified training job.
    ///
    /// # Parameters
    /// - `job_id`: The unique identifier of the training job whose status is being queried.
    ///
    /// # Returns
    /// A result containing the training job status response on success, or a `TrainerError` if the request fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut client = TrainerServiceClient::connect("http://localhost:50051").await.unwrap();
    /// let headers = GrpcHeaders::default();
    /// let response = client.get_the_training_job_status("job-123".to_string(), headers).await;
    /// assert!(response.is_ok());
    /// ```
    async fn get_the_training_job_status(
        &mut self,
        job_id: String,
        headers: GrpcHeaders,
    ) -> TrainerResult<GetTrainingJobStatusResponse> {
        let request_data = GetTrainingJobStatusRequest { job_id };
        let request = grpc_client::create_grpc_request(request_data, headers);

        logger::debug!(trainer_get_status_request =?request);

        let response = self
            .get_training_job_status(request)
            .await
            .map_err(|status| {
                logger::error!(grpc_error =?status, "Trainer service GetTrainingJobStatus call failed");
                TrainerError::ServiceError(status.message().to_string())
            })?
            .into_inner();

        logger::debug!(trainer_get_status_response =?response);
        Ok(response)
    }
}

#[allow(missing_docs)]
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct TrainerClientConfig {
    pub host: String,
    pub port: u16,
}

impl TrainerClientConfig {
    #[allow(missing_docs)]
    /// Creates a `TrainerServiceClient` using the provided HTTP client and the configuration's host and port.
    ///
    /// Returns an error if the host is empty or the constructed URI is invalid.
    pub fn get_trainer_service_client(
        &self,
        hyper_client: Client,
    ) -> Result<TrainerServiceClient<Client>, Report<TrainerError>> {
        let host = &self.host;
        let port = self.port;

        if host.is_empty() {
            return Err(Report::new(TrainerError::ConfigError(
                "Host is not configured for Trainer client".to_string(),
            )));
        }

        let uri_string = format!("http://{}:{}", host, port);
        let uri = uri_string
            .parse::<tonic::transport::Uri>()
            .map_err(Report::from)
            .change_context_lazy(|| {
                TrainerError::ConfigError(format!("Invalid URI: {}", uri_string))
            })?;

        let service_client = TrainerServiceClient::with_origin(hyper_client, uri);
        Ok(service_client)
    }
}

impl common_utils::events::ApiEventMetric for TriggerTrainingResponse {
    /// Returns the API event type associated with this request or response.
    ///
    /// Always returns `Some(ApiEventsType::Miscellaneous)`.
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl common_utils::events::ApiEventMetric for TriggerTrainingRequest {
    /// Returns the API event type associated with this request or response.
    ///
    /// Always returns `Some(ApiEventsType::Miscellaneous)`.
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl common_utils::events::ApiEventMetric for GetTrainingJobStatusResponse {
    /// Returns the API event type associated with this request or response.
    ///
    /// Always returns `Some(ApiEventsType::Miscellaneous)`.
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}
