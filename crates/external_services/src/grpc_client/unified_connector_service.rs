use std::collections::{HashMap, HashSet};

use common_enums::connector_enums::Connector;
use common_utils::{consts as common_utils_consts, errors::CustomResult, types::Url};
use error_stack::ResultExt;
pub use hyperswitch_interfaces::unified_connector_service::transformers::UnifiedConnectorServiceError;
use masking::{PeekInterface, Secret};
use router_env::logger;
use tokio::time::{timeout, Duration};
use tonic::{
    metadata::{MetadataMap, MetadataValue},
    transport::Uri,
};
use unified_connector_service_client::payments::{
    self as payments_grpc, payment_service_client::PaymentServiceClient,
    PaymentServiceAuthorizeResponse, PaymentServiceTransformRequest,
    PaymentServiceTransformResponse,
};

use crate::{
    consts,
    grpc_client::{GrpcClientSettings, GrpcHeadersUcs},
    utils::deserialize_hashset,
};

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
    /// Base URL of the gRPC Server
    pub base_url: Url,

    /// Contains the connection timeout duration in seconds
    pub connection_timeout: u64,

    /// Set of external services/connectors available for the unified connector service
    #[serde(default, deserialize_with = "deserialize_hashset")]
    pub ucs_only_connectors: HashSet<Connector>,

    /// Set of connectors for which psync is disabled in unified connector service
    #[serde(default, deserialize_with = "deserialize_hashset")]
    pub ucs_psync_disabled_connectors: HashSet<Connector>,
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

    /// Optional auth_key_map used for authentication.
    pub auth_key_map:
        Option<HashMap<common_enums::enums::Currency, common_utils::pii::SecretSerdeValue>>,

    /// Id of the merchant.
    pub merchant_id: Secret<String>,
}

/// External Vault Proxy Related Metadata
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum ExternalVaultProxyMetadata {
    /// VGS proxy data variant
    VgsMetadata(VgsMetadata),
}

/// VGS proxy data
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct VgsMetadata {
    /// External vault url
    pub proxy_url: Url,
    /// CA certificates to verify the vault server
    pub certificate: Secret<String>,
}

impl UnifiedConnectorServiceClient {
    /// Builds the connection to the gRPC service
    pub async fn build_connections(config: &GrpcClientSettings) -> Option<Self> {
        match &config.unified_connector_service {
            Some(unified_connector_service_client_config) => {
                let uri: Uri = match unified_connector_service_client_config
                    .base_url
                    .get_string_repr()
                    .parse()
                {
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
                    Ok(Ok(client)) => {
                        logger::info!("Successfully connected to Unified Connector Service");
                        Some(Self { client })
                    }
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
            None => {
                router_env::logger::error!(?config.unified_connector_service, "Unified Connector Service config is missing");
                None
            }
        }
    }

    /// Performs Payment Authorize
    pub async fn payment_authorize(
        &self,
        payment_authorize_request: payments_grpc::PaymentServiceAuthorizeRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<PaymentServiceAuthorizeResponse>> {
        let mut request = tonic::Request::new(payment_authorize_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;

        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .authorize(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentAuthorizeFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_authorize",
                    connector_name=?connector_name,
                    "UCS payment authorize gRPC call failed"
                )
            })
    }

    /// Performs Payment Sync/Get
    pub async fn payment_get(
        &self,
        payment_get_request: payments_grpc::PaymentServiceGetRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PaymentServiceGetResponse>>
    {
        let mut request = tonic::Request::new(payment_get_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .get(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentGetFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_get",
                    connector_name=?connector_name,
                    "UCS payment get/sync gRPC call failed"
                )
            })
    }

    /// Performs Payment Setup Mandate
    pub async fn payment_setup_mandate(
        &self,
        payment_register_request: payments_grpc::PaymentServiceRegisterRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PaymentServiceRegisterResponse>>
    {
        let mut request = tonic::Request::new(payment_register_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .register(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentRegisterFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_setup_mandate",
                    connector_name=?connector_name,
                    "UCS payment setup mandate gRPC call failed"
                )
            })
    }

    /// Performs Payment repeat (MIT - Merchant Initiated Transaction).
    pub async fn payment_repeat(
        &self,
        payment_repeat_request: payments_grpc::PaymentServiceRepeatEverythingRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceRepeatEverythingResponse>,
    > {
        let mut request = tonic::Request::new(payment_repeat_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .repeat_everything(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentRepeatEverythingFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_repeat",
                    connector_name=?connector_name,
                    "UCS payment repeat gRPC call failed"
                )
            })
    }

    /// Transforms incoming webhook through UCS
    pub async fn transform_incoming_webhook(
        &self,
        webhook_transform_request: PaymentServiceTransformRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<PaymentServiceTransformResponse>> {
        let mut request = tonic::Request::new(webhook_transform_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .transform(request)
            .await
            .change_context(UnifiedConnectorServiceError::WebhookTransformFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="transform_incoming_webhook",
                    connector_name=?connector_name,
                    "UCS webhook transform gRPC call failed"
                )
            })
    }
}

/// Build the gRPC Headers for Unified Connector Service Request
pub fn build_unified_connector_service_grpc_headers(
    meta: ConnectorAuthMetadata,
    grpc_headers: GrpcHeadersUcs,
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
    if let Some(auth_key_map) = meta.auth_key_map {
        let auth_key_map_str = serde_json::to_string(&auth_key_map).map_err(|error| {
            logger::error!(?error);
            UnifiedConnectorServiceError::ParsingFailed
        })?;
        metadata.append(
            consts::UCS_HEADER_AUTH_KEY_MAP,
            parse("auth_key_map", &auth_key_map_str)?,
        );
    }

    metadata.append(
        common_utils_consts::X_MERCHANT_ID,
        parse(common_utils_consts::X_MERCHANT_ID, meta.merchant_id.peek())?,
    );

    if let Some(external_vault_proxy_metadata) = grpc_headers.external_vault_proxy_metadata {
        metadata.append(
            consts::UCS_HEADER_EXTERNAL_VAULT_METADATA,
            parse("external_vault_metadata", &external_vault_proxy_metadata)?,
        );
    };

    let lineage_ids_str = grpc_headers
        .lineage_ids
        .get_url_encoded_string()
        .map_err(|err| {
            logger::error!(?err);
            UnifiedConnectorServiceError::HeaderInjectionFailed(consts::UCS_LINEAGE_IDS.to_string())
        })?;
    metadata.append(
        consts::UCS_LINEAGE_IDS,
        parse(consts::UCS_LINEAGE_IDS, &lineage_ids_str)?,
    );

    if let Some(reference_id) = grpc_headers.merchant_reference_id {
        metadata.append(
            consts::UCS_HEADER_REFERENCE_ID,
            parse(
                consts::UCS_HEADER_REFERENCE_ID,
                reference_id.get_string_repr(),
            )?,
        );
    };

    if let Some(request_id) = grpc_headers.request_id {
        metadata.append(
            common_utils_consts::X_REQUEST_ID,
            parse(common_utils_consts::X_REQUEST_ID, &request_id)?,
        );
    };

    if let Some(shadow_mode) = grpc_headers.shadow_mode {
        metadata.append(
            common_utils_consts::X_UNIFIED_CONNECTOR_SERVICE_MODE,
            parse(
                common_utils_consts::X_UNIFIED_CONNECTOR_SERVICE_MODE,
                &shadow_mode.to_string(),
            )?,
        );
    }

    if let Err(err) = grpc_headers
        .tenant_id
        .parse()
        .map(|tenant_id| metadata.append(common_utils_consts::TENANT_HEADER, tenant_id))
    {
        logger::error!(
            header_parse_error=?err,
            tenant_id=?grpc_headers.tenant_id,
            "Failed to parse tenant_id header for UCS gRPC request: {}",
            common_utils_consts::TENANT_HEADER
        );
    }

    Ok(metadata)
}
