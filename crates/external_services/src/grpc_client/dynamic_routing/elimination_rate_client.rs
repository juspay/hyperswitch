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
use router_env::logger;
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
use crate::grpc_client::GrpcHeaders;

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

        let mut request = tonic::Request::new(EliminationRequest {
            id,
            params,
            labels,
            config,
        });

        headers
            .tenant_id
            .parse()
            .map(|tenant_id| request.metadata_mut().append("x-tenant-id", tenant_id))
            .inspect_err(|err| logger::warn!(header_parse_error=?err,"invalid x-tenant-id received in perform_elimination_routing"))
            .ok();

        headers.request_id.map(|request_id| {
            request_id
                .parse()
                .map(|request_id| request.metadata_mut().append("x-request-id", request_id))
                .inspect_err(|err| {
                    logger::warn!(header_parse_error=?err,"invalid x-request-id received in perform_elimination_routing")
                })
                .ok();
        });

        let response = self
            .clone()
            .get_elimination_status(request)
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

        let mut request = tonic::Request::new(UpdateEliminationBucketRequest {
            id,
            params,
            labels_with_bucket_name,
            config,
        });

        headers
            .tenant_id
            .parse()
            .map(|tenant_id| request.metadata_mut().append("x-tenant-id", tenant_id))
            .inspect_err(|err| logger::warn!(header_parse_error=?err,"invalid x-tenant-id received in update_elimination_bucket_config"))
            .ok();

        headers.request_id.map(|request_id| {
            request_id
                .parse()
                .map(|request_id| request.metadata_mut().append("x-request-id", request_id))
                .inspect_err(|err| {
                    logger::warn!(header_parse_error=?err,"invalid x-request-id received in update_elimination_bucket_config")
                })
                .ok();
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
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<InvalidateBucketResponse> {
        let mut request = tonic::Request::new(InvalidateBucketRequest { id });

        headers
            .tenant_id
            .parse()
            .map(|tenant_id| request.metadata_mut().append("x-tenant-id", tenant_id))
            .inspect_err(|err| logger::warn!(header_parse_error=?err,"invalid x-tenant-id received in invalidate_elimination_bucket"))
            .ok();

        headers.request_id.map(|request_id| {
            request_id
                .parse()
                .map(|request_id| request.metadata_mut().append("x-request-id", request_id))
                .inspect_err(|err| {
                    logger::warn!(header_parse_error=?err,"invalid x-request-id received in invalidate_elimination_bucket")
                })
                .ok();
        });

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
