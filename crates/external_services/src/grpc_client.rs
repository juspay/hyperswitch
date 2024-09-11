use std::fmt::Debug;

pub use common_utils::errors::CustomResult;
use common_utils::transformers::ForeignFrom;
pub use dynamic_routing::{
    success_rate_calculator_client::SuccessRateCalculatorClient, CalSuccessRateConfig,
    CalSuccessRateRequest, CalSuccessRateResponse,
    CurrentBlockThreshold as DynamicCurrentThreshold, LabelWithStatus,
    UpdateSuccessRateWindowConfig, UpdateSuccessRateWindowRequest, UpdateSuccessRateWindowResponse,
};
use error_stack::ResultExt;
use hyperswitch_interfaces::api::api_models::routing::{
    CurrentBlockThreshold, DynamicRoutingConfig, DynamicRoutingConfigId, RoutableConnectorChoice,
    RoutableConnectorChoiceWithStatus,
};
use router_env::logger;
use tonic::transport::Channel;
#[allow(
    missing_docs,
    unused_qualifications,
    clippy::unwrap_used,
    clippy::as_conversions
)]
pub mod dynamic_routing {
    tonic::include_proto!("success_rate");
}
use serde;

pub type DRResult<T> = CustomResult<T, DRError>;

// The trait Success Based Dynamic Routing would have all the functions required to support the calculation and updation window
#[async_trait::async_trait]
pub trait SuccessBasedDynamicRouting: dyn_clone::DynClone + Send + Sync {
    async fn calculate_success_rate(
        &self,
        dynamic_routing_config: DynamicRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DRResult<CalSuccessRateResponse>;

    async fn update_rate_of_change_calculated_for_factor(
        &self,
        dynamic_routing_config: DynamicRoutingConfig,
        response: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DRResult<UpdateSuccessRateWindowResponse>;
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DRError {
    #[error("Error buling the gRPC Client for communication")]
    ClientBuildingFailed,
}
/// Struct that contains the settings required to construct an Grpc client.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct GrpcClientSettings {
    pub dynamic_routing_client: DynamicRoutingClientConfig,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct DynamicRoutingClientConfig {
    pub host: String,
    pub port: u16,
}

impl GrpcClientSettings {
    /// # Panics
    ///
    /// This function will panic if it fails to establish a connection with the gRPC server.
    #[allow(clippy::expect_used)]
    pub async fn get_grpc_client_interface(&self) -> GrpcClients {
        let grpc_connect = self
            .dynamic_routing_client
            .clone()
            .get_dynamic_routing_connection()
            .await
            .expect("Failed to establish a connection with the gRPC Server");
        logger::debug!("Connection established with Grpc Server");
        GrpcClients {
            dynamic_routing: grpc_connect,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GrpcClients {
    pub dynamic_routing: RoutingStrategy,
}

#[derive(Debug, Clone)]
pub struct RoutingStrategy {
    pub success_rate_client: SuccessRateCalculatorClient<Channel>,
}

impl DynamicRoutingClientConfig {
    pub async fn get_dynamic_routing_connection(
        self,
    ) -> Result<RoutingStrategy, Box<dyn std::error::Error>> {
        let uri = format!("http://{}:{}", self.host, self.port);
        let channel = tonic::transport::Endpoint::new(uri)?.connect().await?;
        let success_rate_client = SuccessRateCalculatorClient::new(channel);
        Ok(RoutingStrategy {
            success_rate_client,
        })
    }
}
#[async_trait::async_trait]
impl SuccessBasedDynamicRouting for RoutingStrategy {
    async fn calculate_success_rate(
        &self,
        dynamic_routing_config: DynamicRoutingConfig,
        label_input: Vec<RoutableConnectorChoice>,
    ) -> DRResult<CalSuccessRateResponse> {
        let labels = label_input
            .into_iter()
            .map(|conn_choice| conn_choice.to_string())
            .collect::<Vec<_>>();
        let config = if let Some(config) = dynamic_routing_config.config {
            Some(CalSuccessRateConfig {
                min_aggregates_size: config.min_aggregates_size.unwrap_or_default(),
                default_success_rate: config.default_success_rate.unwrap_or_default(),
            })
        } else {
            None
        };
        let params = dynamic_routing_config
            .params
            .map(|vec| {
                vec.into_iter()
                    .map(|param| param.to_string())
                    .collect::<Vec<String>>()
                    .join(":")
            })
            .unwrap_or_default();
        let id = match dynamic_routing_config.id {
            DynamicRoutingConfigId::MerchantId(m_id) => m_id.get_string_repr().to_owned(),
            DynamicRoutingConfigId::ProfileId(p_id) => p_id.get_string_repr().to_owned(),
        };
        let request = tonic::Request::new(CalSuccessRateRequest {
            id,
            params,
            labels,
            config,
        });

        let mut client = self.success_rate_client.clone();
        let response = client
            .fetch_success_rate(request)
            .await
            .change_context(DRError::ClientBuildingFailed)?
            .into_inner();
        Ok(response)
    }
    async fn update_rate_of_change_calculated_for_factor(
        &self,
        dynamic_routing_config: DynamicRoutingConfig,
        label_input: Vec<RoutableConnectorChoiceWithStatus>,
    ) -> DRResult<UpdateSuccessRateWindowResponse> {
        let config = if let Some(config) = dynamic_routing_config.config {
            Some(UpdateSuccessRateWindowConfig {
                max_aggregates_size: config.max_aggregates_size.unwrap_or_default(),
                current_block_threshold: config
                    .current_block_threshold
                    .map(ForeignFrom::foreign_from),
            })
        } else {
            None
        };
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
                vec.into_iter()
                    .map(|param| param.to_string())
                    .collect::<Vec<String>>()
                    .join(":")
            })
            .unwrap_or_default();
        let id = match dynamic_routing_config.id {
            DynamicRoutingConfigId::MerchantId(m_id) => m_id.get_string_repr().to_owned(),
            DynamicRoutingConfigId::ProfileId(p_id) => p_id.get_string_repr().to_owned(),
        };
        let request = tonic::Request::new(UpdateSuccessRateWindowRequest {
            id,
            params,
            labels_with_status,
            config,
        });
        let mut client = self.success_rate_client.clone();
        let response = client
            .update_success_rate_window(request)
            .await
            .change_context(DRError::ClientBuildingFailed)?
            .into_inner();
        Ok(response)
    }
}
impl ForeignFrom<CurrentBlockThreshold> for DynamicCurrentThreshold {
    fn foreign_from(current_threshold: CurrentBlockThreshold) -> Self {
        DynamicCurrentThreshold {
            duration_in_mins: current_threshold.duration_in_mins,
            max_total_count: current_threshold.max_total_count.unwrap_or_default(),
        }
    }
}
