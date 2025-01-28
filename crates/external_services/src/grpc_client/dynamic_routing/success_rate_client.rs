use api_models::routing::{
    CurrentBlockThreshold, RoutableConnectorChoice, RoutableConnectorChoiceWithStatus,
    SuccessBasedRoutingConfig, SuccessBasedRoutingConfigBody, SuccessRateSpecificityLevel,
};
use common_utils::{ext_traits::OptionExt, transformers::ForeignTryFrom};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};
pub use success_rate::{
    success_rate_calculator_client::SuccessRateCalculatorClient, CalGlobalSuccessRateConfig,
    CalGlobalSuccessRateRequest, CalGlobalSuccessRateResponse, CalSuccessRateConfig,
    CalSuccessRateRequest, CalSuccessRateResponse,
    CurrentBlockThreshold as DynamicCurrentThreshold, InvalidateWindowsRequest,
    InvalidateWindowsResponse, LabelWithStatus,
    SuccessRateSpecificityLevel as ProtoSpecificityLevel, UpdateSuccessRateWindowConfig,
    UpdateSuccessRateWindowRequest, UpdateSuccessRateWindowResponse,
};
#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::use_self
)]
pub mod success_rate {
    tonic::include_proto!("success_rate");
}
use super::{Client, DynamicRoutingError, DynamicRoutingResult};
use crate::grpc_client::{self, GrpcHeaders};
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
    /// To calculate both global and merchant specific success rate for the list of chosen connectors
    async fn calculate_entity_and_global_success_rate(
        &self,
        id: String,
        success_rate_based_config: SuccessBasedRoutingConfig,
        params: String,
        label_input: Vec<RoutableConnectorChoice>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<CalGlobalSuccessRateResponse>;
}

#[async_trait::async_trait]
impl SuccessBasedDynamicRouting for SuccessRateCalculatorClient<Client> {
    #[instrument(skip_all)]
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

        let request = grpc_client::create_grpc_request(
            CalSuccessRateRequest {
                id,
                params,
                labels,
                config,
            },
            headers,
        );

        let response = self
            .clone()
            .fetch_success_rate(request)
            .await
            .change_context(DynamicRoutingError::SuccessRateBasedRoutingFailure(
                "Failed to fetch the success rate".to_string(),
            ))?
            .into_inner();

        logger::info!(dynamic_routing_response=?response);

        Ok(response)
    }

    #[instrument(skip_all)]
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
            .clone()
            .into_iter()
            .map(|conn_choice| LabelWithStatus {
                label: conn_choice.routable_connector_choice.to_string(),
                status: conn_choice.status,
            })
            .collect();

        let global_labels_with_status = label_input
            .into_iter()
            .map(|conn_choice| LabelWithStatus {
                label: conn_choice.routable_connector_choice.connector.to_string(),
                status: conn_choice.status,
            })
            .collect();

        let request = grpc_client::create_grpc_request(
            UpdateSuccessRateWindowRequest {
                id,
                params,
                labels_with_status,
                config,
                global_labels_with_status,
            },
            headers,
        );

        let response = self
            .clone()
            .update_success_rate_window(request)
            .await
            .change_context(DynamicRoutingError::SuccessRateBasedRoutingFailure(
                "Failed to update the success rate window".to_string(),
            ))?
            .into_inner();

        logger::info!(dynamic_routing_response=?response);

        Ok(response)
    }

    #[instrument(skip_all)]
    async fn invalidate_success_rate_routing_keys(
        &self,
        id: String,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<InvalidateWindowsResponse> {
        let request = grpc_client::create_grpc_request(InvalidateWindowsRequest { id }, headers);

        let response = self
            .clone()
            .invalidate_windows(request)
            .await
            .change_context(DynamicRoutingError::SuccessRateBasedRoutingFailure(
                "Failed to invalidate the success rate routing keys".to_string(),
            ))?
            .into_inner();

        logger::info!(dynamic_routing_response=?response);

        Ok(response)
    }

    async fn calculate_entity_and_global_success_rate(
        &self,
        id: String,
        success_rate_based_config: SuccessBasedRoutingConfig,
        params: String,
        label_input: Vec<RoutableConnectorChoice>,
        headers: GrpcHeaders,
    ) -> DynamicRoutingResult<CalGlobalSuccessRateResponse> {
        let labels = label_input
            .clone()
            .into_iter()
            .map(|conn_choice| conn_choice.to_string())
            .collect::<Vec<_>>();

        let global_labels = label_input
            .into_iter()
            .map(|conn_choice| conn_choice.connector.to_string())
            .collect::<Vec<_>>();

        let config = success_rate_based_config
            .config
            .map(ForeignTryFrom::foreign_try_from)
            .transpose()?;

        let request = grpc_client::create_grpc_request(
            CalGlobalSuccessRateRequest {
                entity_id: id,
                entity_params: params,
                entity_labels: labels,
                global_labels,
                config,
            },
            headers,
        );

        let response = self
            .clone()
            .fetch_entity_and_global_success_rate(request)
            .await
            .change_context(DynamicRoutingError::SuccessRateBasedRoutingFailure(
                "Failed to fetch the entity and global success rate".to_string(),
            ))?
            .into_inner();

        logger::info!(dynamic_routing_response=?response);

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
            specificity_level: match config.specificity_level {
                SuccessRateSpecificityLevel::Merchant => Some(ProtoSpecificityLevel::Entity.into()),
                SuccessRateSpecificityLevel::Global => Some(ProtoSpecificityLevel::Global.into()),
            },
        })
    }
}

impl ForeignTryFrom<SuccessBasedRoutingConfigBody> for CalGlobalSuccessRateConfig {
    type Error = error_stack::Report<DynamicRoutingError>;
    fn foreign_try_from(config: SuccessBasedRoutingConfigBody) -> Result<Self, Self::Error> {
        Ok(Self {
            entity_min_aggregates_size: config
                .min_aggregates_size
                .get_required_value("min_aggregate_size")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "min_aggregates_size".to_string(),
                })?,
            entity_default_success_rate: config
                .default_success_rate
                .get_required_value("default_success_rate")
                .change_context(DynamicRoutingError::MissingRequiredField {
                    field: "default_success_rate".to_string(),
                })?,
        })
    }
}
