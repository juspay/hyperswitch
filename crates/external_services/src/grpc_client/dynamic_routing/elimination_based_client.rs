use api_models::routing::{
    EliminationAnalyserConfig as EliminationConfig, RoutableConnectorChoice,
    RoutableConnectorChoiceWithBucketName,
};
use common_utils::{ext_traits::OptionExt, transformers::ForeignTryFrom};
pub use elimination_rate::{
    elimination_analyser_client::EliminationAnalyserClient, EliminationBucketConfig,
    EliminationRequest, EliminationResponse, InvalidateBucketRequest, InvalidateBucketResponse,
    LabelWithBucketName, UpdateEliminationBucketRequest, UpdateEliminationBucketResponse,
};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};
#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::use_self
)]
pub mod elimination_rate {
    tonic::include_proto!("elimination");
}

use super::{Client, DynamicRoutingError, DynamicRoutingResult};
use crate::grpc_client::{self, GrpcHeaders};

/// The trait Elimination Based Routing would have the functions required to support performance, calculation and invalidation bucket
#[async_trait::async_trait]
pub trait EliminationBasedRouting: dyn_clone::DynClone + Send + Sync {
    /// To perform the elimination based routing for the list of connectors
    async fn perform_elimination_routing(
        &self,
        id: String,
        params: String,
        labels: Vec<RoutableConnectorChoice>,
        configs: Option<EliminationConfig>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<EliminationResponse>;
    /// To update the bucket size and ttl for list of connectors with its respective bucket name
    async fn update_elimination_bucket_config(
        &self,
        id: String,
        params: String,
        report: Vec<RoutableConnectorChoiceWithBucketName>,
        config: Option<EliminationConfig>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<UpdateEliminationBucketResponse>;
    /// To invalidate the previous id's bucket
    async fn invalidate_elimination_bucket(
        &self,
        id: String,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<InvalidateBucketResponse>;
}

#[async_trait::async_trait]
impl EliminationBasedRouting for EliminationAnalyserClient<Client> {
    #[instrument(skip_all)]
    async fn perform_elimination_routing(
        &self,
        id: String,
        params: String,
        label_input: Vec<RoutableConnectorChoice>,
        configs: Option<EliminationConfig>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<EliminationResponse> {
        let labels = label_input
            .into_iter()
            .map(|conn_choice| conn_choice.to_string())
            .collect::<Vec<_>>();

        let config = configs.map(ForeignTryFrom::foreign_try_from).transpose()?;

        let request = grpc_client::create_grpc_request(
            EliminationRequest {
                id,
                params,
                labels,
                config,
            },
            headers,
        );

        let response = self
            .clone()
            .get_elimination_status(request)
            .await
            .change_context(DynamicRoutingError::EliminationRateRoutingFailure(
                "Failed to perform the elimination analysis".to_string(),
            ))?
            .into_inner();

        logger::info!(dynamic_routing_response=?response);

        Ok(response)
    }

    #[instrument(skip_all)]
    async fn update_elimination_bucket_config(
        &self,
        id: String,
        params: String,
        report: Vec<RoutableConnectorChoiceWithBucketName>,
        configs: Option<EliminationConfig>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<UpdateEliminationBucketResponse> {
        let config = configs.map(ForeignTryFrom::foreign_try_from).transpose()?;

        let labels_with_bucket_name = report
            .into_iter()
            .map(|conn_choice_with_bucket| LabelWithBucketName {
                label: conn_choice_with_bucket
                    .routable_connector_choice
                    .to_string(),
                bucket_name: conn_choice_with_bucket.bucket_name,
            })
            .collect::<Vec<_>>();

        let request = grpc_client::create_grpc_request(
            UpdateEliminationBucketRequest {
                id,
                params,
                labels_with_bucket_name,
                config,
            },
            headers,
        );

        let response = self
            .clone()
            .update_elimination_bucket(request)
            .await
            .change_context(DynamicRoutingError::EliminationRateRoutingFailure(
                "Failed to update the elimination bucket".to_string(),
            ))?
            .into_inner();

        logger::info!(dynamic_routing_response=?response);

        Ok(response)
    }

    #[instrument(skip_all)]
    async fn invalidate_elimination_bucket(
        &self,
        id: String,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<InvalidateBucketResponse> {
        let request = grpc_client::create_grpc_request(InvalidateBucketRequest { id }, headers);

        let response = self
            .clone()
            .invalidate_bucket(request)
            .await
            .change_context(DynamicRoutingError::EliminationRateRoutingFailure(
                "Failed to invalidate the elimination bucket".to_string(),
            ))?
            .into_inner();

        logger::info!(dynamic_routing_response=?response);

        Ok(response)
    }
}

impl ForeignTryFrom<EliminationConfig> for EliminationBucketConfig {
    type Error = error_stack::Report<DynamicRoutingError>;
    fn foreign_try_from(config: EliminationConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            bucket_size: config
                .bucket_size
                .get_required_value("bucket_size")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "bucket_size".to_string(),
                })?,
            bucket_leak_interval_in_secs: config
                .bucket_leak_interval_in_secs
                .get_required_value("bucket_leak_interval_in_secs")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "bucket_leak_interval_in_secs".to_string(),
                })?,
        })
    }
}
