use std::fmt::Debug;

use api_models::routing::{
    CurrentBlockThreshold, RoutableConnectorChoice, RoutableConnectorChoiceWithStatus,
    SuccessBasedRoutingConfig, SuccessBasedRoutingConfigBody,
};
use common_utils::{errors::CustomResult, ext_traits::OptionExt, transformers::ForeignTryFrom};
use error_stack::ResultExt;
use http_body_util::combinators::UnsyncBoxBody;
use hyper::body::Bytes;
use hyper_util::client::legacy::connect::HttpConnector;
use serde;
use success_rate::{
    success_rate_calculator_client::SuccessRateCalculatorClient, CalSuccessRateConfig,
    CalSuccessRateRequest, CalSuccessRateResponse,
    CurrentBlockThreshold as DynamicCurrentThreshold, LabelWithStatus,
    UpdateSuccessRateWindowConfig, UpdateSuccessRateWindowRequest, UpdateSuccessRateWindowResponse,
};
use tonic::Status;
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
    /// The required input is missing
    #[error("Missing Required Field : {field} for building the Dynamic Routing Request")]
    MissingRequiredField {
        /// The required field name
        field: String,
    },
    /// Error from Dynamic Routing Server
    #[error("Error from Dynamic Routing Server : {0}")]
    SuccessRateBasedRoutingFailure(String),
}

type Client = hyper_util::client::legacy::Client<HttpConnector, UnsyncBoxBody<Bytes, Status>>;

/// Type that consists of all the services provided by the client
#[derive(Debug, Clone)]
pub struct RoutingStrategy {
    /// success rate service for Dynamic Routing
    pub success_rate_client: Option<SuccessRateCalculatorClient<Client>>,
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
        /// Service name
        service: String,
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
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .http2_only(true)
                .build_http();
        let success_rate_client = match self {
            Self::Enabled { host, port } => {
                let uri = format!("http://{}:{}", host, port).parse::<tonic::transport::Uri>()?;
                Some(SuccessRateCalculatorClient::with_origin(client, uri))
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
        id: String,
        success_rate_based_config: SuccessBasedRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DynamicRoutingResult<CalSuccessRateResponse>;
    /// To update the success rate with the given label
    async fn update_success_rate(
        &self,
        id: String,
        success_rate_based_config: SuccessBasedRoutingConfig,
        response: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DynamicRoutingResult<UpdateSuccessRateWindowResponse>;
}

#[async_trait::async_trait]
impl SuccessBasedDynamicRouting for SuccessRateCalculatorClient<Client> {
    async fn calculate_success_rate(
        &self,
        id: String,
        success_rate_based_config: SuccessBasedRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DynamicRoutingResult<CalSuccessRateResponse> {
        let params = success_rate_based_config
            .params
            .map(|vec| {
                vec.into_iter().fold(String::new(), |mut acc_str, params| {
                    if !acc_str.is_empty() {
                        acc_str.push(':')
                    }
                    acc_str.push_str(params.to_string().as_str());
                    acc_str
                })
            })
            .get_required_value("params")
            .change_context(DynamicRoutingError::MissingRequiredField {
                field: "params".to_string(),
            })?;

        let labels = label_input
            .into_iter()
            .map(|conn_choice| conn_choice.to_string())
            .collect::<Vec<_>>();

        let config = success_rate_based_config
            .config
            .map(ForeignTryFrom::foreign_try_from)
            .transpose()?;

        let request = tonic::Request::new(CalSuccessRateRequest {
            id,
            params,
            labels,
            config,
        });

        let mut client = self.clone();

        let response = client
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
                vec.into_iter().fold(String::new(), |mut acc_str, params| {
                    if !acc_str.is_empty() {
                        acc_str.push(':')
                    }
                    acc_str.push_str(params.to_string().as_str());
                    acc_str
                })
            })
            .get_required_value("params")
            .change_context(DynamicRoutingError::MissingRequiredField {
                field: "params".to_string(),
            })?;

        let request = tonic::Request::new(UpdateSuccessRateWindowRequest {
            id,
            params,
            labels_with_status,
            config,
        });

        let mut client = self.clone();

        let response = client
            .update_success_rate_window(request)
            .await
            .change_context(DynamicRoutingError::SuccessRateBasedRoutingFailure(
                "Failed to update the success rate window".to_string(),
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
