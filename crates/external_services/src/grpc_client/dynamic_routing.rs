use std::fmt::Debug;

use api_models::routing::{
    CurrentBlockThreshold, RoutableConnectorChoice, RoutableConnectorChoiceWithStatus,
    SuccessBasedRoutingConfig, SuccessBasedRoutingConfigBody,
};
use common_utils::{
    errors::CustomResult, ext_traits::OptionExt, id_type, transformers::ForeignTryFrom,
};
use error_stack::ResultExt;
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
    /// Error building the Dynamic Routing Client Request as params was missing
    #[error("Error building the Dynamic Routing Client Request as params was missing")]
    MissingRequiredParam,
    /// Error getting a response from the gRPC Server
    #[error("Error getting a response from the Dynamic Routing Server")]
    SuccessBasedResponseFailure(String),
    /// Error building the Dynamic Routing Client Request as max count was missing
    #[error("Error building the Dynamic Routing Client Request as max count was missing")]
    MissingRequiredMaxTotalCount,
    /// Error building the Dynamic Routing Client Request as min aggregate size was missing
    #[error("Error building the Dynamic Routing Client Request as min aggregate size was missing")]
    MissingRequiredMinAggregate,
    /// Error building the Dynamic Routing Client Request as max aggregate size was missing
    #[error("Error building the Dynamic Routing Client Request as max aggregate size was missing")]
    MissingRequiredMaxAggregateSize,
    /// Error building the Dynamic Routing Client Request as default success rate was missing
    #[error(
        "Error building the Dynamic Routing Client Request as default success rate was missing"
    )]
    MissingRequiredDefaultSuccessRate,
}

/// Struct consists of all the services provided by the client
#[derive(Debug, Clone)]
pub struct RoutingStrategy {
    /// success rate service for Dynamic Routing
    pub success_rate_client: Option<SuccessRateCalculatorClient<Channel>>,
}

/// Contains the Dynamic Routing Client Config
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
#[serde(untagged)]
pub enum DynamicRoutingClientConfig {
    /// If the dynamic routing client config has been enabled
    Enabled {
        /// The host for the client
        host: String,
        /// The port of the client
        port: u16,
    },
    #[default]
    /// If the dynamic routing client config has been disabled
    Disabled,
}

impl DynamicRoutingClientConfig {
    /// establish connection with the server
    pub async fn get_dynamic_routing_connection(
        self,
    ) -> Result<RoutingStrategy, Box<dyn std::error::Error>> {
        let success_rate_client = match self {
            Self::Enabled { host, port } => {
                let uri = format!("http://{}:{}", host, port);
                let channel = tonic::transport::Endpoint::new(uri)?.connect().await?;
                Some(SuccessRateCalculatorClient::new(channel))
            }
            Self::Disabled => None,
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
        success_rate_based_config: SuccessBasedRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DynamicRoutingResult<CalSuccessRateResponse>;
    /// To update the success rate with the given label
    async fn update_success_rate(
        &self,
        id: id_type::ProfileId,
        success_rate_based_config: SuccessBasedRoutingConfig,
        response: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DynamicRoutingResult<UpdateSuccessRateWindowResponse>;
}

#[async_trait::async_trait]
impl SuccessBasedDynamicRouting for SuccessRateCalculatorClient<Channel> {
    async fn calculate_success_rate(
        &self,
        id: id_type::ProfileId,
        success_rate_based_config: SuccessBasedRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DynamicRoutingResult<CalSuccessRateResponse> {
        let params = success_rate_based_config
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
            .change_context(DynamicRoutingError::MissingRequiredParam)?;

        let labels = label_input
            .into_iter()
            .map(|conn_choice| conn_choice.to_string())
            .collect::<Vec<_>>();

        let config = success_rate_based_config
            .config
            .map(ForeignTryFrom::foreign_try_from)
            .transpose()?;

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
            .change_context(DynamicRoutingError::SuccessBasedResponseFailure(
                "Failed to fetch the success rate".to_string(),
            ))?
            .into_inner();

        Ok(response)
    }

    async fn update_success_rate(
        &self,
        id: id_type::ProfileId,
        success_rate_based_config: SuccessBasedRoutingConfig,
        label_input: Vec<RoutableConnectorChoiceWithStatus>,
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

        let params = success_rate_based_config
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
            .change_context(DynamicRoutingError::MissingRequiredParam)?;

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
            .change_context(DynamicRoutingError::SuccessBasedResponseFailure(
                "Failed to update the successs rate window".to_string(),
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
                .get_required_value("Max Total Count")
                .change_context(DynamicRoutingError::MissingRequiredMaxTotalCount)?,
        })
    }
}

impl ForeignTryFrom<SuccessBasedRoutingConfigBody> for UpdateSuccessRateWindowConfig {
    type Error = error_stack::Report<DynamicRoutingError>;
    fn foreign_try_from(config: SuccessBasedRoutingConfigBody) -> Result<Self, Self::Error> {
        Ok(Self {
            max_aggregates_size: config
                .max_aggregates_size
                .get_required_value("Max Aggregate Size")
                .change_context(DynamicRoutingError::MissingRequiredMaxAggregateSize)?,
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
                .get_required_value("Min Agggregate Size")
                .change_context(DynamicRoutingError::MissingRequiredMinAggregate)?,
            default_success_rate: config
                .default_success_rate
                .get_required_value("Default Success Rate")
                .change_context(DynamicRoutingError::MissingRequiredDefaultSuccessRate)?,
        })
    }
}
