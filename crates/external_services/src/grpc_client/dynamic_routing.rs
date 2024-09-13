use common_utils::{
    errors::CustomResult, ext_traits::OptionExt, id_type, transformers::ForeignFrom,
};
use error_stack::ResultExt;
use hyperswitch_interfaces::api::api_models::routing::{
    CurrentBlockThreshold, DynamicRoutingConfig, DynamicRoutingConfigBody, RoutableConnectorChoice,
    RoutableConnectorChoiceWithStatus,
};
use serde;
use std::fmt::Debug;
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

pub type DRResult<T> = CustomResult<T, DRError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum DRError {
    #[error("Error buling the gRPC Client for communication")]
    ClientBuildingFailed,
    #[error("Error getting a response from the gRPC Server")]
    GrpcServerResponseFailure,
}

// Struct consists of all the services provided by the client
#[derive(Debug, Clone)]
pub struct RoutingStrategy {
    pub success_rate_client: Option<SuccessRateCalculatorClient<Channel>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct DynamicRoutingClientConfig {
    pub host: String,
    pub port: u16,
    pub enabled: bool,
}

// establish connection with the server
impl DynamicRoutingClientConfig {
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

// The trait Success Based Dynamic Routing would have the functions required to support the calculation and updation window
#[async_trait::async_trait]
pub trait SuccessBasedDynamicRouting: dyn_clone::DynClone + Send + Sync {
    async fn calculate_success_rate(
        &self,
        id: id_type::ProfileId,
        dynamic_routing_config: DynamicRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DRResult<CalSuccessRateResponse>;

    async fn update_success_rate(
        &self,
        id: id_type::ProfileId,
        dynamic_routing_config: DynamicRoutingConfig,
        response: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DRResult<UpdateSuccessRateWindowResponse>;
}

#[async_trait::async_trait]
impl SuccessBasedDynamicRouting for SuccessRateCalculatorClient<Channel> {
    async fn calculate_success_rate(
        &self,
        id: id_type::ProfileId,
        dynamic_routing_config: DynamicRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DRResult<CalSuccessRateResponse> {
        let params = dynamic_routing_config
            .params
            .map(|vec| {
                if !vec.is_empty() {
                    vec.into_iter()
                        .map(|param| param.to_string())
                        .collect::<Vec<_>>()
                        .join(":")
                } else {
                    String::default()
                }
            })
            .get_required_value("Vector of params")
            .change_context(DRError::ClientBuildingFailed)?;

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
            .change_context(DRError::GrpcServerResponseFailure)?
            .into_inner();

        Ok(response)
    }

    async fn update_success_rate(
        &self,
        id: id_type::ProfileId,
        dynamic_routing_config: DynamicRoutingConfig,
        label_input: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DRResult<UpdateSuccessRateWindowResponse> {
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
                if !vec.is_empty() {
                    vec.into_iter()
                        .map(|param| param.to_string())
                        .collect::<Vec<_>>()
                        .join(":")
                } else {
                    String::default()
                }
            })
            .get_required_value("Vector of params")
            .change_context(DRError::ClientBuildingFailed)?;

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
            .change_context(DRError::GrpcServerResponseFailure)?
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

impl ForeignFrom<DynamicRoutingConfigBody> for UpdateSuccessRateWindowConfig {
    fn foreign_from(config: DynamicRoutingConfigBody) -> Self {
        Self {
            max_aggregates_size: config.max_aggregates_size.unwrap_or_default(),
            current_block_threshold: config
                .current_block_threshold
                .map(ForeignFrom::foreign_from),
        }
    }
}

impl ForeignFrom<DynamicRoutingConfigBody> for CalSuccessRateConfig {
    fn foreign_from(config: DynamicRoutingConfigBody) -> Self {
        Self {
            min_aggregates_size: config.min_aggregates_size.unwrap_or_default(),
            default_success_rate: config.default_success_rate.unwrap_or_default(),
        }
    }
}
