use common_utils::{errors::CustomResult, types::Url};
use error_stack::ResultExt;
use router_env::logger;
use rust_grpc_client::payments::{
    self as payments_grpc, payment_service_client::PaymentServiceClient, PaymentsAuthorizeResponse,
};

use crate::grpc_client::GrpcClientSettings;

/// Result type for Dynamic Routing
pub type UnifiedConnectorServiceResult<T> = CustomResult<T, UnifiedConnectorServiceError>;

/// Unified Connector Service error variants
#[derive(Debug, Clone, thiserror::Error)]
pub enum UnifiedConnectorServiceError {
    /// Error occurred while communicating with the gRPC server.
    #[error("Error from gRPC Server : {0}")]
    ConnectionError(String),

    /// Failed to encode the request to the unified connector service.
    #[error("Failed to encode unified connector service request")]
    RequestEncodingFailed,

    /// Request encoding failed due to a specific reason.
    #[error("Request encoding failed : {0}")]
    RequestEncodingFailedWithReason(String),

    /// Failed to deserialize the response from the connector.
    #[error("Failed to deserialize connector response")]
    ResponseDeserializationFailed,

    /// The connector name provided is invalid or unrecognized.
    #[error("An invalid connector name was provided")]
    InvalidConnectorName,

    /// Connector name is missing
    #[error("Connector name is missing")]
    MissingConnectorName,

    /// A required field was missing in the request.
    #[error("Missing required field: {field_name}")]
    MissingRequiredField {
        /// Missing Field
        field_name: &'static str,
    },

    /// Multiple required fields were missing in the request.
    #[error("Missing required fields: {field_names:?}")]
    MissingRequiredFields {
        /// Missing Fields
        field_names: Vec<&'static str>,
    },

    /// The requested step or feature is not yet implemented.
    #[error("This step has not been implemented for: {0}")]
    NotImplemented(String),

    /// Parsing of some value or input failed.
    #[error("Parsing failed")]
    ParsingFailed,

    /// Data format provided is invalid
    #[error("Invalid Data format")]
    InvalidDataFormat {
        /// Field Name for which data is invalid
        field_name: &'static str,
    },

    /// Failed to obtain authentication type
    #[error("Failed to obtain authentication type")]
    FailedToObtainAuthType,

    /// Failed to inject metadata into request headers
    #[error("Failed to inject metadata into request headers: {0}")]
    HeaderInjectionFailed(String),
}
/// Contains the  Unified Connector Service client
#[derive(Debug, Clone)]
pub struct UnifiedConnectorService {
    /// The Unified Connector Service Client
    pub unified_connector_service_client: PaymentServiceClient<tonic::transport::Channel>,
}

/// Contains the Unified Connector Service Client config
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct UnifiedConnectorServiceClientConfig {
    /// Contains the Base URL for the gRPC server
    pub base_url: Url,
}

impl UnifiedConnectorService {
    /// Builds the connection to the gRPC service
    pub async fn build_connections(
        config: &GrpcClientSettings,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        match &config.unified_connector_service_client {
            Some(unified_connector_service_client) => {
                match PaymentServiceClient::connect(
                    unified_connector_service_client
                        .base_url
                        .clone()
                        .get_string_repr()
                        .to_owned(),
                )
                .await
                {
                    Ok(unified_connector_service_client) => Ok(Some(Self {
                        unified_connector_service_client,
                    })),
                    Err(_) => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    /// Performs Payment Authorize
    pub async fn payment_authorize(
        &self,
        request: tonic::Request<payments_grpc::PaymentsAuthorizeRequest>,
    ) -> UnifiedConnectorServiceResult<tonic::Response<PaymentsAuthorizeResponse>> {
        self.unified_connector_service_client
            .clone()
            .payment_authorize(request)
            .await
            .change_context(UnifiedConnectorServiceError::ConnectionError(
                "Failed to authorize payment through Unified Connector Service".to_owned(),
            ))
            .map_err(|err| {
                logger::error!(error=?err);
                err
            })
    }
}
