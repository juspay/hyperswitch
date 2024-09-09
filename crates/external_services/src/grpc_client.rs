use common_utils::errors::CustomResult;
pub use dynamic_routing::success_rate_calculator_client::SuccessRateCalculatorClient;
use dynamic_routing::{CalSuccessRateConfig, UpdateSuccessRateWindowRequest};
pub use dynamic_routing::{
    CalSuccessRateRequest, CalSuccessRateResponse, UpdateSuccessRateWindowResponse,
};
use error_stack::ResultExt;
use hyperswitch_interfaces::api::api_models::routing::RoutableConnectorChoice;
use router_env::logger;
use std::{fmt::Debug, sync::Arc};
use tokio::sync::Mutex;
use tonic::transport::Channel;
#[allow(missing_docs)]
pub mod dynamic_routing {
    tonic::include_proto!("success_rate");
}
use serde;

pub type DRResult<T> = CustomResult<T, DRError>;

// The trait Success Based Dynamic Routing would have all the functions required to support the calculation and updation window
#[async_trait::async_trait]
pub trait SuccessBasedDynamicRouting: dyn_clone::DynClone + Send + Sync {
    type Label;
    async fn calculate_success_rate(
        &self,
        label_input: Vec<Self::Label>,
    ) -> DRResult<CalSuccessRateResponse>;

    async fn update_rate_of_change_calculated_for_factor(
        &self,
        response: Vec<Self::Label>,
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
    pub async fn get_grpc_client_interface(&self) -> GrpcClients {
        let grpc_connect = self
            .dynamic_routing_client
            .clone()
            .get_dynamic_routing_connection()
            .await
            .expect("Failed to establish a connection with the gRPC Server");
        logger::debug!("Connection established with Grpc Server");
        let grpc_struct = GrpcClients {
            dynamic_routing: grpc_connect,
        };
        grpc_struct
    }
}

#[derive(Debug, Clone)]
pub struct GrpcClients {
    pub dynamic_routing: RoutingStrategy,
}

#[derive(Debug, Clone)]
pub struct RoutingStrategy {
    pub success_rate_client: Arc<Mutex<SuccessRateCalculatorClient<Channel>>>,
}

impl DynamicRoutingClientConfig {
    pub async fn get_dynamic_routing_connection(
        self,
    ) -> Result<RoutingStrategy, Box<dyn std::error::Error>> {
        let uri = format!("http://{}:{}", self.host, self.port);
        let channel = tonic::transport::Endpoint::new(uri)?.connect().await?;
        let success_rate_client = Arc::new(Mutex::new(SuccessRateCalculatorClient::new(channel)));
        Ok(RoutingStrategy {
            success_rate_client,
        })
    }
}
#[async_trait::async_trait]
impl SuccessBasedDynamicRouting for RoutingStrategy {
    type Label = RoutableConnectorChoice;
    async fn calculate_success_rate(
        &self,
        _label_input: Vec<Self::Label>,
    ) -> DRResult<CalSuccessRateResponse> {
        let config = CalSuccessRateConfig {
            min_aggregates_size: 3,
            default_success_rate: 100.0,
        };
        // call the db to populate the other fields once this function is called
        let request = tonic::Request::new(CalSuccessRateRequest {
            id: "ab".to_string(),
            params: "".to_string(),
            labels: vec![],
            config: None,
        });

        let mut client = self.success_rate_client.lock().await;
        let response = client
            .fetch_success_rate(request)
            .await
            .change_context(DRError::ClientBuildingFailed)?
            .into_inner();
        Ok(response)
    }
    async fn update_rate_of_change_calculated_for_factor(
        &self,
        response: Vec<Self::Label>,
    ) -> DRResult<UpdateSuccessRateWindowResponse> {
        let request = tonic::Request::new(UpdateSuccessRateWindowRequest {
            id: todo!(),
            params: todo!(),
            labels_with_status: todo!(),
            config: todo!(),
        });
        let mut client = self.success_rate_client.lock().await;
        let response = client
            .update_success_rate_window(request)
            .await
            .change_context(DRError::ClientBuildingFailed)?
            .into_inner();
        Ok(response)
    }
}
