use std::fmt::Debug;

use common_utils::{
    errors::CustomResult, ext_traits::OptionExt, id_type, transformers::ForeignFrom,
};
use error_stack::ResultExt;
use hyperswitch_interfaces::api::api_models::routing::{
    CurrentBlockThreshold, RoutableConnectorChoice, RoutableConnectorChoiceWithStatus,
    SuccessBasedRoutingConfig, SuccessBasedRoutingConfigBody,
};
use serde;
use success_rate::{
    success_rate_calculator_client::SuccessRateCalculatorClient, CalSuccessRateConfig,
    CalSuccessRateRequest, CalSuccessRateResponse,
    CurrentBlockThreshold as DynamicCurrentThreshold, LabelWithStatus,
    UpdateSuccessRateWindowConfig, UpdateSuccessRateWindowRequest, UpdateSuccessRateWindowResponse,
};
use tonic::transport::Channel;
#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions
)]
pub mod success_rate {
    tonic::include_proto!("success_rate");
}
/// Result type for Dynamic Routing
pub type DynamicRoutingResult<T> = CustomResult<T, DynamicRoutingError>;

/// Dynamic Routing Errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum DynamicRoutingError {
    /// Error buliding the Dynamic Routing Client Request as params was missing
    #[error("Error buliding the Dynamic Routing Client Request as params was missing")]
    MissingRequiredParamToBuildRequest,
    /// Error getting a response from the gRPC Server
    #[error("Error getting a response from the Dynamic Routing Server")]
    GrpcServerResponseFailure,
}

/// Struct consists of all the services provided by the client
#[derive(Debug, Clone)]
pub struct RoutingStrategy {
    /// success rate service for Dynamic Routing
    pub success_rate_client: Option<SuccessRateCalculatorClient<Channel>>,
}

/// Contains the Dynamic Routing Client Config
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct DynamicRoutingClientConfig {
    /// The host for the client
    pub host: String,
    /// The port of the client
    pub port: u16,
    /// Booolean value for establishment of connection with the server
    pub enabled: bool,
}

impl DynamicRoutingClientConfig {
    /// establish connection with the server
    pub async fn get_dynamic_routing_connection(
        self,
    ) -> Result<RoutingStrategy, Box<dyn std::error::Error>> {
        let success_rate_client = if self.enabled {
            let uri = format!("http://{}:{}", self.host, self.port);
            let channel = tonic::transport::Endpoint::new(uri)?.connect().await?;
            Some(SuccessRateCalculatorClient::new(channel))
        } else {
            None
        };
        Ok(RoutingStrategy {
            success_rate_client,
        })
    }
}

/// The trait Success Based Dynamic Routing would have the functions required to support the calculation and updation window
#[async_trait::async_trait]
pub trait SuccessBasedDynamicRouting: dyn_clone::DynClone + Send + Sync {
    /// To calculate the success rate for the list of chosen connectors
    async fn calculate_success_rate(
        &self,
        id: id_type::ProfileId,
        dynamic_routing_config: SuccessBasedRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DynamicRoutingResult<CalSuccessRateResponse>;
    /// To update the success rate with the given label
    async fn update_success_rate(
        &self,
        id: id_type::ProfileId,
        dynamic_routing_config: SuccessBasedRoutingConfig,
        response: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DynamicRoutingResult<UpdateSuccessRateWindowResponse>;
}

#[async_trait::async_trait]
impl SuccessBasedDynamicRouting for SuccessRateCalculatorClient<Channel> {
    async fn calculate_success_rate(
        &self,
        id: id_type::ProfileId,
        dynamic_routing_config: SuccessBasedRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DynamicRoutingResult<CalSuccessRateResponse> {
        let params = dynamic_routing_config
            .params
            .map(|vec| {
                vec.into_iter().fold(String::new(), |mut acc_vec, params| {
                    if !acc_vec.is_empty() {
                        acc_vec.push(':')
                    }
                    acc_vec.push_str(params.to_string().as_str());
                    acc_vec
                })
            })
            .get_required_value("Vector of params")
            .change_context(DynamicRoutingError::MissingRequiredParamToBuildRequest)?;

        let labels = label_input
            .into_iter()
            .map(|conn_choice| conn_choice.to_string())
            .collect::<Vec<_>>();

        let config = dynamic_routing_config.config.map(ForeignFrom::foreign_from);

        let request = tonic::Request::new(CalSuccessRateRequest {
            id: id.get_string_repr().to_owned(),
            params,
            labels,
            config,
        });

        let mut client = self.clone();

        let response = client
            .fetch_success_rate(request)
            .await
            .change_context(DynamicRoutingError::GrpcServerResponseFailure)?
            .into_inner();

        Ok(response)
    }

    async fn update_success_rate(
        &self,
        id: id_type::ProfileId,
        dynamic_routing_config: SuccessBasedRoutingConfig,
        label_input: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DynamicRoutingResult<UpdateSuccessRateWindowResponse> {
        let config = dynamic_routing_config.config.map(ForeignFrom::foreign_from);

        let labels_with_status = label_input
            .into_iter()
            .map(|conn_choice| LabelWithStatus {
                label: conn_choice.routable_connector_choice.to_string(),
                status: conn_choice.status,
            })
            .collect();

        let params = dynamic_routing_config
            .params
            .map(|vec| {
                vec.into_iter().fold(String::new(), |mut acc_vec, params| {
                    if !acc_vec.is_empty() {
                        acc_vec.push(':')
                    }
                    acc_vec.push_str(params.to_string().as_str());
                    acc_vec
                })
            })
            .get_required_value("Vector of params")
            .change_context(DynamicRoutingError::MissingRequiredParamToBuildRequest)?;

        let request = tonic::Request::new(UpdateSuccessRateWindowRequest {
            id: id.get_string_repr().to_owned(),
            params,
            labels_with_status,
            config,
        });

        let mut client = self.clone();

        let response = client
            .update_success_rate_window(request)
            .await
            .change_context(DynamicRoutingError::GrpcServerResponseFailure)?
            .into_inner();

        Ok(response)
    }
}

impl ForeignFrom<CurrentBlockThreshold> for DynamicCurrentThreshold {
    fn foreign_from(current_threshold: CurrentBlockThreshold) -> Self {
        Self {
            duration_in_mins: current_threshold.duration_in_mins,
            max_total_count: current_threshold.max_total_count.unwrap_or_default(),
        }
    }
}

impl ForeignFrom<SuccessBasedRoutingConfigBody> for UpdateSuccessRateWindowConfig {
    fn foreign_from(config: SuccessBasedRoutingConfigBody) -> Self {
        Self {
            max_aggregates_size: config.max_aggregates_size.unwrap_or_default(),
            current_block_threshold: config
                .current_block_threshold
                .map(ForeignFrom::foreign_from),
        }
    }
}

impl ForeignFrom<SuccessBasedRoutingConfigBody> for CalSuccessRateConfig {
    fn foreign_from(config: SuccessBasedRoutingConfigBody) -> Self {
        Self {
            min_aggregates_size: config.min_aggregates_size.unwrap_or_default(),
            default_success_rate: config.default_success_rate.unwrap_or_default(),
        }
    }
}
