use rust_grpc_client::payments::{
    self as payments_grpc, payment_service_client::PaymentServiceClient, PaymentsAuthorizeResponse,
};

use crate::grpc_client::GrpcClientSettings;

/// Contains the  Unified Connector Service client
#[derive(Debug, Clone)]
pub struct UnifiedConnectorService {
    /// The Unified Connector Service Client
    pub unified_connector_service_client: PaymentServiceClient<tonic::transport::Channel>,
}

/// Contains the Unified Connector Service Client config
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct UnifiedConnectorServiceClientConfig {
    /// Contains the Base URL for the gRPC server
    pub base_url: Option<String>,
}

impl UnifiedConnectorService {
    /// Builds the connection to the gRPC service
    pub async fn build_connections(
        config: &GrpcClientSettings,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        if let Some(base_url) = &config.unified_connector_service_client.base_url {
            Ok(Some(Self {
                unified_connector_service_client: PaymentServiceClient::connect(base_url.clone())
                    .await
                    .expect("Failed to establish a connection with the Unified Connector Service"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Performs Payment Authorize
    pub async fn payment_authorize(
        &self,
        request: tonic::Request<payments_grpc::PaymentsAuthorizeRequest>,
    ) -> Result<tonic::Response<PaymentsAuthorizeResponse>, tonic::Status> {
        self.unified_connector_service_client
            .clone()
            .payment_authorize(request)
            .await
    }
}
