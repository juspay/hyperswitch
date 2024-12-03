use api_models::routing::{
    EliminationAnalyserConfig as EliminationConfig, RoutableConnectorChoice,
    RoutableConnectorChoiceWithBucketName,
};
use common_utils::{ext_traits::OptionExt, transformers::ForeignTryFrom};
pub use elimination_rate::{
    elimination_analyser_client::EliminationAnalyserClient, EliminationAnalyserConfig,
    EliminationRequest, EliminationResponse, InvalidateBucketRequest, InvalidateBucketResponse,
    LabelWithBucketName, UpdateEliminationBucketConfig, UpdateEliminationBucketRequest,
    UpdateEliminationBucketResponse,
};
use error_stack::ResultExt;
#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions
)]
pub mod elimination_rate {
    tonic::include_proto!("elimination");
}

use super::{Client, DynamicRoutingError, DynamicRoutingResult};

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
    ) -> DynamicRoutingResult<EliminationResponse>;
    /// To update the bucket size and ttl for list of connectors with its respective bucket name
    async fn update_elimination_bucket_config(
        &self,
        id: String,
        params: String,
        report: Vec<RoutableConnectorChoiceWithBucketName>,
        config: Option<EliminationConfig>,
    ) -> DynamicRoutingResult<UpdateEliminationBucketResponse>;
    /// To invalidate the previous id's bucket
    async fn invalidate_elimination_bucket(
        &self,
        id: String,
    ) -> DynamicRoutingResult<InvalidateBucketResponse>;
}

#[async_trait::async_trait]
impl EliminationBasedRouting for EliminationAnalyserClient<Client> {
    async fn perform_elimination_routing(
        &self,
        id: String,
        params: String,
        label_input: Vec<RoutableConnectorChoice>,
        configs: Option<EliminationConfig>,
    ) -> DynamicRoutingResult<EliminationResponse> {
        let labels = label_input
            .into_iter()
            .map(|conn_choice| conn_choice.to_string())
            .collect::<Vec<_>>();

        let config = configs.map(ForeignTryFrom::foreign_try_from).transpose()?;

        let request = tonic::Request::new(EliminationRequest {
            id,
            params,
            labels,
            config,
        });

        let response = self
            .clone()
            .perform_elimination(request)
            .await
            .change_context(DynamicRoutingError::EliminationRateRoutingFailure(
                "Failed to perform the elimination analysis".to_string(),
            ))?
            .into_inner();

        Ok(response)
    }

    async fn update_elimination_bucket_config(
        &self,
        id: String,
        params: String,
        report: Vec<RoutableConnectorChoiceWithBucketName>,
        configs: Option<EliminationConfig>,
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

        let request = tonic::Request::new(UpdateEliminationBucketRequest {
            id
            params,
            labels_with_bucket_name,
            config,
        });

        let response = self
            .clone()
            .update_elimination_bucket(request)
            .await
            .change_context(DynamicRoutingError::EliminationRateRoutingFailure(
                "Failed to update the elimination bucket".to_string(),
            ))?
            .into_inner();
        Ok(response)
    }
    async fn invalidate_elimination_bucket(
        &self,
        id: String,
    ) -> DynamicRoutingResult<InvalidateBucketResponse> {
        let request = tonic::Request::new(InvalidateBucketRequest { id });

        let response = self
            .clone()
            .invalidate_bucket(request)
            .await
            .change_context(DynamicRoutingError::EliminationRateRoutingFailure(
                "Failed to invalidate the elimination bucket".to_string(),
            ))?
            .into_inner();
        Ok(response)
    }
}

impl ForeignTryFrom<EliminationConfig> for EliminationAnalyserConfig {
    type Error = error_stack::Report<DynamicRoutingError>;
    fn foreign_try_from(config: EliminationConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            bucket_size: config
                .bucket_size
                .get_required_value("bucket_size")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "bucket_size".to_string(),
                })?,
            bucket_ttl_in_mins: config
                .bucket_ttl_in_mins
                .get_required_value("bucket_ttl_in_mins")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "bucket_ttl_in_mins".to_string(),
                })?,
        })
    }
}
impl ForeignTryFrom<EliminationConfig> for UpdateEliminationBucketConfig {
    type Error = error_stack::Report<DynamicRoutingError>;
    fn foreign_try_from(config: EliminationConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            bucket_size: config
                .bucket_size
                .get_required_value("bucket_size")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "bucket_size".to_string(),
                })?,
            bucket_ttl_in_mins: config
                .bucket_ttl_in_mins
                .get_required_value("bucket_ttl_in_mins")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "bucket_ttl_in_mins".to_string(),
                })?,
        })
    }
}
