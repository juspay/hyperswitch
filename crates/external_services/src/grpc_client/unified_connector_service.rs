use common_utils::{consts as common_utils_consts, errors::CustomResult};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use router_env::logger;
use tokio::time::{timeout, Duration};
use tonic::{
    metadata::{MetadataMap, MetadataValue},
    transport::Uri,
};
use unified_connector_service_client::payments::{
    self as payments_grpc, payment_service_client::PaymentServiceClient,
    PaymentServiceAuthorizeResponse,
};

use crate::{
    consts,
    grpc_client::{GrpcClientSettings, GrpcHeaders},
};

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

    /// Failed to perform Payment Authorize from gRPC Server
    #[error("Failed to perform Payment Authorize from gRPC Server")]
    PaymentAuthorizeFailure,

    /// Failed to perform Payment Get from gRPC Server
    #[error("Failed to perform Payment Get from gRPC Server")]
    PaymentGetFailure,
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
    /// Host for the gRPC Client
    pub host: String,

    /// Port of the gRPC Client
    pub port: u16,

    /// Contains the connection timeout duration in seconds
    pub connection_timeout: u64,
}

/// Contains the Connector Auth Type and related authentication data.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ConnectorAuthMetadata {
    /// Name of the connector (e.g., "stripe", "paypal").
    pub connector_name: String,

    /// Type of authentication used (e.g., "HeaderKey", "BodyKey", "SignatureKey").
    pub auth_type: String,

    /// Optional API key used for authentication.
    pub api_key: Option<Secret<String>>,

    /// Optional additional key used by some authentication types.
    pub key1: Option<Secret<String>>,

    /// Optional API secret used for signature or secure authentication.
    pub api_secret: Option<Secret<String>>,

    /// Id of the merchant.
    pub merchant_id: Secret<String>,
}

impl UnifiedConnectorServiceClient {
    /// Builds the connection to the gRPC service
    pub async fn build_connections(config: &GrpcClientSettings) -> Option<Self> {
        match &config.unified_connector_service {
            Some(unified_connector_service_client_config) => {
                let uri_str = format!(
                    "https://{}:{}",
                    unified_connector_service_client_config.host,
                    unified_connector_service_client_config.port
                );

                let uri: Uri = match uri_str.parse() {
                    Ok(parsed_uri) => parsed_uri,
                    Err(err) => {
                        logger::error!(error = ?err, "Failed to parse URI for Unified Connector Service");
                        return None;
                    }
                };

                let connect_result = timeout(
                    Duration::from_secs(unified_connector_service_client_config.connection_timeout),
                    PaymentServiceClient::connect(uri),
                )
                .await;

                match connect_result {
                    Ok(Ok(client)) => Some(Self { client }),
                    Ok(Err(err)) => {
                        logger::error!(error = ?err, "Failed to connect to Unified Connector Service");
                        None
                    }
                    Err(err) => {
                        logger::error!(error = ?err, "Connection to Unified Connector Service timed out");
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
        grpc_headers: GrpcHeaders,
    ) -> UnifiedConnectorServiceResult<tonic::Response<PaymentServiceAuthorizeResponse>> {
        let mut request = tonic::Request::new(payment_authorize_request);

        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .authorize(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentAuthorizeFailure)
            .inspect_err(|error| logger::error!(?error))
    }

    /// Performs Payment Sync/Get
    pub async fn payment_get(
        &self,
        payment_get_request: payments_grpc::PaymentServiceGetRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeaders,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PaymentServiceGetResponse>>
    {
        let mut request = tonic::Request::new(payment_get_request);

        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .get(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentGetFailure)
            .inspect_err(|error| logger::error!(?error))
    }
}

/// Build the gRPC Headers for Unified Connector Service Request
pub fn build_unified_connector_service_grpc_headers(
    meta: ConnectorAuthMetadata,
    grpc_headers: GrpcHeaders,
) -> Result<MetadataMap, UnifiedConnectorServiceError> {
    let mut metadata = MetadataMap::new();
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
        metadata.append(
            consts::UCS_HEADER_API_KEY,
            parse("api_key", api_key.peek())?,
        );
    }
    if let Some(key1) = meta.key1 {
        metadata.append(consts::UCS_HEADER_KEY1, parse("key1", key1.peek())?);
    }
    if let Some(api_secret) = meta.api_secret {
        metadata.append(
            consts::UCS_HEADER_API_SECRET,
            parse("api_secret", api_secret.peek())?,
        );
    }

    metadata.append(
        common_utils_consts::X_MERCHANT_ID,
        parse(common_utils_consts::X_MERCHANT_ID, meta.merchant_id.peek())?,
    );

    grpc_headers.tenant_id
            .parse()
            .map(|tenant_id| {
                metadata.append(
                    common_utils_consts::TENANT_HEADER,
                    tenant_id)
            })
            .inspect_err(
                |err| logger::warn!(header_parse_error=?err,"invalid {} received",common_utils_consts::TENANT_HEADER),
            )
            .ok();

    Ok(metadata)
}
