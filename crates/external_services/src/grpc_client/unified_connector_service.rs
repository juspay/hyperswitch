use common_utils::{errors::CustomResult, types::Url};
use error_stack::ResultExt;
use router_env::logger;
use tonic::metadata::{MetadataMap, MetadataValue};
use unified_connector_service_client::payments::{
    self as payments_grpc, payment_service_client::PaymentServiceClient,
    PaymentServiceAuthorizeResponse,
};

use crate::{consts, grpc_client::GrpcClientSettings};

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

/// Result type for Dynamic Routing
pub type UnifiedConnectorServiceResult<T> = CustomResult<T, UnifiedConnectorServiceError>;
/// Contains the  Unified Connector Service client
#[derive(Debug, Clone)]
pub struct UnifiedConnectorServiceClient {
    /// The Unified Connector Service Client
    pub client: PaymentServiceClient<tonic::transport::Channel>,
}

/// Contains the Unified Connector Service Client config
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct UnifiedConnectorServiceClientConfig {
    /// Contains the Base URL for the gRPC server
    pub base_url: Url,
}

/// Contains the Connector Auth Type and related authentication data.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ConnectorAuthMetadata {
    /// Name of the connector (e.g., "stripe", "paypal").
    pub connector_name: String,

    /// Type of authentication used (e.g., "HeaderKey", "BodyKey", "SignatureKey").
    pub auth_type: String,

    /// Optional API key used for authentication.
    pub api_key: Option<String>,

    /// Optional additional key used by some authentication types.
    pub key1: Option<String>,

    /// Optional API secret used for signature or secure authentication.
    pub api_secret: Option<String>,

    /// Optional tenant ID for multi-tenant systems.
    pub tenant_id: String,

    /// Optional merchant ID for the request
    pub merchant_id: String,
}

impl UnifiedConnectorServiceClient {
    /// Builds the connection to the gRPC service
    pub async fn build_connections(config: &GrpcClientSettings) -> Option<Self> {
        match &config.unified_connector_service {
            Some(unified_connector_service_client_config) => {
                match PaymentServiceClient::connect(
                    unified_connector_service_client_config
                        .base_url
                        .clone()
                        .get_string_repr()
                        .to_owned(),
                )
                .await
                {
                    Ok(unified_connector_service_client) => Some(Self {
                        client: unified_connector_service_client,
                    }),
                    Err(err) => {
                        logger::error!(error = ?err, "Failed to connect to Unified Connector Service");
                        None
                    }
                }
            }
            None => None,
        }
    }

    /// Performs Payment Authorize
    pub async fn payment_authorize(
        &self,
        payment_authorize_request: payments_grpc::PaymentServiceAuthorizeRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
    ) -> UnifiedConnectorServiceResult<tonic::Response<PaymentServiceAuthorizeResponse>> {
        let mut request = tonic::Request::new(payment_authorize_request);

        let metadata = MetadataMap::try_from(connector_auth_metadata)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .authorize(request)
            .await
            .change_context(UnifiedConnectorServiceError::ConnectionError(
                "Failed to authorize payment through Unified Connector Service".to_owned(),
            ))
            .inspect_err(|error| logger::error!(?error))
    }

    /// Performs Payment Sync/Get
    pub async fn payment_get(
        &self,
        payment_get_request: payments_grpc::PaymentServiceGetRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PaymentServiceGetResponse>>
    {
        let mut request = tonic::Request::new(payment_get_request);

        let metadata = MetadataMap::try_from(connector_auth_metadata)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .get(request)
            .await
            .change_context(UnifiedConnectorServiceError::ConnectionError(
                "Failed to get payment through Unified Connector Service".to_owned(),
            ))
            .inspect_err(|error| logger::error!(?error))
    }
}

impl TryFrom<ConnectorAuthMetadata> for MetadataMap {
    type Error = UnifiedConnectorServiceError;

    fn try_from(meta: ConnectorAuthMetadata) -> Result<Self, Self::Error> {
        let mut metadata = Self::new();
        let parse =
            |key: &str, value: &str| -> Result<MetadataValue<_>, UnifiedConnectorServiceError> {
                value.parse::<MetadataValue<_>>().map_err(|error| {
                    logger::error!(?error);
                    UnifiedConnectorServiceError::HeaderInjectionFailed(key.to_string())
                })
            };

        metadata.append(
            consts::UCS_HEADER_CONNECTOR,
            parse("connector", &meta.connector_name)?,
        );
        metadata.append(
            consts::UCS_HEADER_AUTH_TYPE,
            parse("auth_type", &meta.auth_type)?,
        );

        if let Some(api_key) = meta.api_key {
            metadata.append(consts::UCS_HEADER_API_KEY, parse("api_key", &api_key)?);
        }
        if let Some(key1) = meta.key1 {
            metadata.append(consts::UCS_HEADER_KEY1, parse("key1", &key1)?);
        }
        if let Some(api_secret) = meta.api_secret {
            metadata.append(
                consts::UCS_HEADER_API_SECRET,
                parse("api_secret", &api_secret)?,
            );
        }

        metadata.append(
            consts::UCS_AUTH_HEADER_MERCHANT_ID_KEY,
            parse("merchant_id", &meta.merchant_id)?,
        );

        metadata.append(
            consts::UCS_AUTH_HEADER_TENANT_ID_KEY,
            parse("tenant_id", &meta.tenant_id)?,
        );

        Ok(metadata)
    }
}
