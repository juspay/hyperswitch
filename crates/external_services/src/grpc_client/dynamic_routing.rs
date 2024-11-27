use std::fmt::Debug;

use common_utils::errors::CustomResult;
use router_env::logger;
use serde;
/// Elimination Routing Client Interface Implementation
pub mod elimination_rate;
/// Success Routing Client Interface Implementation
pub mod success_rate;

use super::Client;
pub use elimination_rate::EliminationAnalyserClient;
pub use success_rate::SuccessRateCalculatorClient;
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
    /// Error from Dynamic Routing Server while perfrming success_rate analysis
    #[error("Error from Dynamic Routing Server while perfrming success_rate analysis : {0}")]
    SuccessRateBasedRoutingFailure(String),
    /// Error from Dynamic Routing Server while perfrming elimination
    #[error("Error from Dynamic Routing Server while perfrming elimination : {0}")]
    EliminationRateRoutingFailure(String),
}

/// Type that consists of all the services provided by the client
#[derive(Debug, Clone)]
pub struct RoutingStrategy {
    /// success rate service for Dynamic Routing
    pub success_rate_client: Option<SuccessRateCalculatorClient<Client>>,
    /// elimination service for Dynamic Routing
    pub elimination_rate_client: Option<EliminationAnalyserClient<Client>>,
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
        client: Client,
    ) -> Result<RoutingStrategy, Box<dyn std::error::Error>> {
        let (success_rate_client, elimination_rate_client) = match self {
            Self::Enabled { host, port, .. } => {
                let uri = format!("http://{}:{}", host, port).parse::<tonic::transport::Uri>()?;
                logger::info!("Connection established with dynamic routing gRPC Server");
                (
                    Some(SuccessRateCalculatorClient::with_origin(
                        client.clone(),
                        uri.clone(),
                    )),
                    Some(EliminationAnalyserClient::with_origin(client, uri)),
                )
            }
            Self::Disabled => (None, None),
        };
        Ok(RoutingStrategy {
            success_rate_client,
            elimination_rate_client,
        })
    }
}
