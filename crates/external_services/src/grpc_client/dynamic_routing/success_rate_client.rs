use api_models::routing::{
    CurrentBlockThreshold, RoutableConnectorChoice, RoutableConnectorChoiceWithStatus,
    SuccessBasedRoutingConfig, SuccessBasedRoutingConfigBody,
};
use common_utils::{ext_traits::OptionExt, transformers::ForeignTryFrom};
use error_stack::ResultExt;
use router_env::logger;
pub use success_rate::{
    success_rate_calculator_client::SuccessRateCalculatorClient, CalSuccessRateConfig,
    CalSuccessRateRequest, CalSuccessRateResponse,
    CurrentBlockThreshold as DynamicCurrentThreshold, InvalidateWindowsRequest,
    InvalidateWindowsResponse, LabelWithStatus, UpdateSuccessRateWindowConfig,
    UpdateSuccessRateWindowRequest, UpdateSuccessRateWindowResponse,
};
#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions
)]
pub mod success_rate {
    tonic::include_proto!("success_rate");
}
use crate::grpc_client::GrpcHeaders;

use super::{Client, DynamicRoutingError, DynamicRoutingResult};
/// The trait Success Based Dynamic Routing would have the functions required to support the calculation and updation window
#[async_trait::async_trait]
pub trait SuccessBasedDynamicRouting: dyn_clone::DynClone + Send + Sync {
    /// To calculate the success rate for the list of chosen connectors
    async fn calculate_success_rate(
        &self,
        id: String,
        success_rate_based_config: SuccessBasedRoutingConfig,
        params: String,
        label_input: Vec<RoutableConnectorChoice>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<CalSuccessRateResponse>;
    /// To update the success rate with the given label
    async fn update_success_rate(
        &self,
        id: String,
        success_rate_based_config: SuccessBasedRoutingConfig,
        params: String,
        response: Vec<RoutableConnectorChoiceWithStatus>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<UpdateSuccessRateWindowResponse>;
    /// To invalidates the success rate routing keys
    async fn invalidate_success_rate_routing_keys(
        &self,
        id: String,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<InvalidateWindowsResponse>;
}

#[async_trait::async_trait]
impl SuccessBasedDynamicRouting for SuccessRateCalculatorClient<Client> {
    async fn calculate_success_rate(
        &self,
        id: String,
        success_rate_based_config: SuccessBasedRoutingConfig,
        params: String,
        label_input: Vec<RoutableConnectorChoice>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<CalSuccessRateResponse> {
        let labels = label_input
            .into_iter()
            .map(|conn_choice| conn_choice.to_string())
            .collect::<Vec<_>>();

        let config = success_rate_based_config
            .config
            .map(ForeignTryFrom::foreign_try_from)
            .transpose()?;

        let mut request = tonic::Request::new(CalSuccessRateRequest {
            id,
            params,
            labels,
            config,
        });

        headers
            .tenant_id
            .parse()
            .map(|tenant_id| request.metadata_mut().append("x-tenant-id", tenant_id))
            .inspect_err(|err| logger::warn!(header_parse_error=?err,"invalid x-tenant-id received in calculate_success_rate"))
            .ok();

        headers.request_id.map(|request_id| {
            request_id
                .parse()
                .map(|request_id| request.metadata_mut().append("x-request-id", request_id))
                .inspect_err(|err| {
                    logger::warn!(header_parse_error=?err,"invalid x-request-id received in calculate_success_rate")
                })
                .ok();
        });

        let response = self
            .clone()
            .fetch_success_rate(request)
            .await
            .change_context(DynamicRoutingError::SuccessRateBasedRoutingFailure(
                "Failed to fetch the success rate".to_string(),
            ))?
            .into_inner();

        Ok(response)
    }

    async fn update_success_rate(
        &self,
        id: String,
        success_rate_based_config: SuccessBasedRoutingConfig,
        params: String,
        label_input: Vec<RoutableConnectorChoiceWithStatus>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<UpdateSuccessRateWindowResponse> {
        let config = success_rate_based_config
            .config
            .map(ForeignTryFrom::foreign_try_from)
            .transpose()?;

        let labels_with_status = label_input
            .into_iter()
            .map(|conn_choice| LabelWithStatus {
                label: conn_choice.routable_connector_choice.to_string(),
                status: conn_choice.status,
            })
            .collect();

        let mut request = tonic::Request::new(UpdateSuccessRateWindowRequest {
            id,
            params,
            labels_with_status,
            config,
        });

        headers
            .tenant_id
            .parse()
            .map(|tenant_id| request.metadata_mut().append("x-tenant-id", tenant_id))
            .inspect_err(|err| logger::warn!(header_parse_error=?err,"invalid x-tenant-id received in update_success_rate"))
            .ok();

        headers.request_id.map(|request_id| {
            request_id
                .parse()
                .map(|request_id| request.metadata_mut().append("x-request-id", request_id))
                .inspect_err(|err| {
                    logger::warn!(header_parse_error=?err,"invalid x-request-id received in update_success_rate")
                })
                .ok();
        });

        let response = self
            .clone()
            .update_success_rate_window(request)
            .await
            .change_context(DynamicRoutingError::SuccessRateBasedRoutingFailure(
                "Failed to update the success rate window".to_string(),
            ))?
            .into_inner();

        Ok(response)
    }
    async fn invalidate_success_rate_routing_keys(
        &self,
        id: String,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<InvalidateWindowsResponse> {
        let mut request = tonic::Request::new(InvalidateWindowsRequest { id });

        headers
            .tenant_id
            .parse()
            .map(|tenant_id| request.metadata_mut().append("x-tenant-id", tenant_id))
            .inspect_err(|err| logger::warn!(header_parse_error=?err,"invalid x-tenant-id received in invalidate_success_rate_routing_keys"))
            .ok();

        headers.request_id.map(|request_id| {
            request_id
                .parse()
                .map(|request_id| request.metadata_mut().append("x-request-id", request_id))
                .inspect_err(|err| {
                    logger::warn!(header_parse_error=?err,"invalid x-request-id received in invalidate_success_rate_routing_keys")
                })
                .ok();
        });

        let response = self
            .clone()
            .invalidate_windows(request)
            .await
            .change_context(DynamicRoutingError::SuccessRateBasedRoutingFailure(
                "Failed to invalidate the success rate routing keys".to_string(),
            ))?
            .into_inner();
        Ok(response)
    }
}

impl ForeignTryFrom<CurrentBlockThreshold> for DynamicCurrentThreshold {
    type Error = error_stack::Report<DynamicRoutingError>;
    fn foreign_try_from(current_threshold: CurrentBlockThreshold) -> Result<Self, Self::Error> {
        Ok(Self {
            duration_in_mins: current_threshold.duration_in_mins,
            max_total_count: current_threshold
                .max_total_count
                .get_required_value("max_total_count")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "max_total_count".to_string(),
                })?,
        })
    }
}

impl ForeignTryFrom<SuccessBasedRoutingConfigBody> for UpdateSuccessRateWindowConfig {
    type Error = error_stack::Report<DynamicRoutingError>;
    fn foreign_try_from(config: SuccessBasedRoutingConfigBody) -> Result<Self, Self::Error> {
        Ok(Self {
            max_aggregates_size: config
                .max_aggregates_size
                .get_required_value("max_aggregate_size")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "max_aggregates_size".to_string(),
                })?,
            current_block_threshold: config
                .current_block_threshold
                .map(ForeignTryFrom::foreign_try_from)
                .transpose()?,
        })
    }
}

impl ForeignTryFrom<SuccessBasedRoutingConfigBody> for CalSuccessRateConfig {
    type Error = error_stack::Report<DynamicRoutingError>;
    fn foreign_try_from(config: SuccessBasedRoutingConfigBody) -> Result<Self, Self::Error> {
        Ok(Self {
            min_aggregates_size: config
                .min_aggregates_size
                .get_required_value("min_aggregate_size")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "min_aggregates_size".to_string(),
                })?,
            default_success_rate: config
                .default_success_rate
                .get_required_value("default_success_rate")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "default_success_rate".to_string(),
                })?,
        })
    }
}
