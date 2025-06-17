use common_utils::errors::CustomResult;
use error_stack::{Report, ResultExt};
use router_env::logger;

use super::Client;

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

///Trainer result
pub type TrainerResult<T> = CustomResult<T, TrainerError>;

/// Trainer errors
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
/// Trainer Client  
pub trait TrainerClientInterface: dyn_clone::DynClone + Send + Sync + std::fmt::Debug {
    /// triggers training
    async fn get_training(
        &mut self,
        model_version_tag: String,
        enable_incremental_learning: bool,
    ) -> TrainerResult<TriggerTrainingResponse>;

    /// gets training job status
    async fn get_the_training_job_status(
        &mut self,
        job_id: String,
    ) -> TrainerResult<GetTrainingJobStatusResponse>;
}

dyn_clone::clone_trait_object!(TrainerClientInterface);

#[async_trait::async_trait]
impl TrainerClientInterface for TrainerServiceClient<Client> {
    async fn get_training(
        &mut self,
        model_version_tag: String,
        enable_incremental_learning: bool,
    ) -> TrainerResult<TriggerTrainingResponse> {
        let request_data = TriggerTrainingRequest {
            model_version_tag,
            enable_incremental_learning,
        };
        let request = tonic::Request::new(request_data);

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

    async fn get_the_training_job_status(
        &mut self,
        job_id: String,
    ) -> TrainerResult<GetTrainingJobStatusResponse> {
        let request_data = GetTrainingJobStatusRequest { job_id };
        let request = tonic::Request::new(request_data);

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

/// Trainer client config
#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct TrainerClientConfig {
    pub host: String,
    pub port: u16,
}

impl TrainerClientConfig {
    /// creates connection
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
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl common_utils::events::ApiEventMetric for TriggerTrainingRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl common_utils::events::ApiEventMetric for GetTrainingJobStatusResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}
