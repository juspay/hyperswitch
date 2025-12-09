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
    refund_service_client::RefundServiceClient, PaymentServiceAuthorizeResponse,
    PaymentServiceRefundRequest, PaymentServiceTransformRequest, PaymentServiceTransformResponse,
    RefundResponse, RefundServiceGetRequest,
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
    /// The Refund Service Client
    pub refund_client: RefundServiceClient<tonic::transport::Channel>,
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

    /// Optional second additional key used by multi-auth authentication types.
    pub key2: Option<Secret<String>>,

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

                let payment_client_result = timeout(
                    Duration::from_secs(unified_connector_service_client_config.connection_timeout),
                    PaymentServiceClient::connect(uri.clone()),
                )
                .await;

                let refund_client_result = timeout(
                    Duration::from_secs(unified_connector_service_client_config.connection_timeout),
                    RefundServiceClient::connect(uri),
                )
                .await;

                match (payment_client_result, refund_client_result) {
                    (Ok(Ok(client)), Ok(Ok(refund_client))) => {
                        logger::info!("Successfully connected to Unified Connector Service");
                        Some(Self {
                            client,
                            refund_client,
                        })
                    }
                    (Ok(Err(err)), _) | (_, Ok(Err(err))) => {
                        logger::error!(error = ?err, "Failed to connect to Unified Connector Service");
                        None
                    }
                    (Err(err), _) | (_, Err(err)) => {
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

    /// Performs Payment Method Token Create
    pub async fn payment_method_token_create(
        &self,
        pm_token_create_request: payments_grpc::PaymentServiceCreatePaymentMethodTokenRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceCreatePaymentMethodTokenResponse>,
    > {
        let mut request = tonic::Request::new(pm_token_create_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .create_payment_method_token(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentMethodTokenCreateFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_payment_method_token",
                    connector_name=?connector_name,
                    "UCS create_payment_method_token gRPC call failed"
                )
            })
    }

    /// Performs Payment Granular Authorize
    pub async fn payment_authorize_granular(
        &self,
        payment_authorize_only_request: payments_grpc::PaymentServiceAuthorizeOnlyRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<PaymentServiceAuthorizeResponse>> {
        let mut request = tonic::Request::new(payment_authorize_only_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .authorize_only(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentAuthorizeGranularFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="authorize_only",
                    connector_name=?connector_name,
                    "UCS authorize_only gRPC call failed"
                )
            })
    }

    /// Performs Create Connector Customer Granular
    pub async fn create_connector_customer(
        &self,
        create_customer_request: payments_grpc::PaymentServiceCreateConnectorCustomerRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceCreateConnectorCustomerResponse>,
    > {
        let mut request = tonic::Request::new(create_customer_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .create_connector_customer(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentConnectorCustomerCreateFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_connector_customer_granular",
                    connector_name=?connector_name,
                    "UCS create connector customer granular gRPC call failed"
                )
            })
    }

    /// Performs Payment Create Order
    pub async fn payment_create_order(
        &self,
        payment_create_order_request: payments_grpc::PaymentServiceCreateOrderRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceCreateOrderResponse>,
    > {
        let mut request = tonic::Request::new(payment_create_order_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .create_order(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentCreateOrderFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_order",
                    connector_name=?connector_name,
                    "UCS create_order gRPC call failed"
                )
            })
    }

    /// Performs Payment Pre Authenticate
    pub async fn payment_pre_authenticate(
        &self,
        payment_pre_authenticate_request: payments_grpc::PaymentServicePreAuthenticateRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServicePreAuthenticateResponse>,
    > {
        let mut request = tonic::Request::new(payment_pre_authenticate_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;

        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .pre_authenticate(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentPreAuthenticateFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_pre_authenticate",
                    connector_name=?connector_name,
                    "UCS payment pre authenticate gRPC call failed"
                )
            })
    }

    /// Performs Payment Authenticate
    pub async fn payment_authenticate(
        &self,
        payment_authenticate_request: payments_grpc::PaymentServiceAuthenticateRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceAuthenticateResponse>,
    > {
        let mut request = tonic::Request::new(payment_authenticate_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;

        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .authenticate(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentAuthenticateFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_pre_authenticate",
                    connector_name=?connector_name,
                    "UCS payment pre authenticate gRPC call failed"
                )
            })
    }

    /// Performs Payment Session token create
    pub async fn payment_session_token_create(
        &self,
        payment_create_session_token_request: payments_grpc::PaymentServiceCreateSessionTokenRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceCreateSessionTokenResponse>,
    > {
        let mut request = tonic::Request::new(payment_create_session_token_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .create_session_token(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentCreateSessionTokenFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_session_token",
                    connector_name=?connector_name,
                    "UCS payment create_session_token gRPC call failed"
                )
            })
    }

    /// Performs Payment Post Authenticate
    pub async fn payment_post_authenticate(
        &self,
        payment_post_authenticate_request: payments_grpc::PaymentServicePostAuthenticateRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServicePostAuthenticateResponse>,
    > {
        let mut request = tonic::Request::new(payment_post_authenticate_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;

        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .post_authenticate(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentPostAuthenticateFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_post_authenticate",
                    connector_name=?connector_name,
                    "UCS payment post authenticate gRPC call failed"
                )
            })
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

    /// Performs Payment Capture
    pub async fn payment_capture(
        &self,
        payment_capture_request: payments_grpc::PaymentServiceCaptureRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PaymentServiceCaptureResponse>>
    {
        let mut request = tonic::Request::new(payment_capture_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .capture(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentCaptureFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_capture",
                    connector_name=?connector_name,
                    "UCS payment capture gRPC call failed"
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

    /// Performs Payment Setup Mandate
    pub async fn payment_setup_mandate_granular(
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
            .register_only(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentRegisterFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_setup_mandate_granular",
                    connector_name=?connector_name,
                    "UCS payment granular setup mandate gRPC call failed"
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

    /// Performs Payment Cancel/Void
    pub async fn payment_cancel(
        &self,
        payment_void_request: payments_grpc::PaymentServiceVoidRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PaymentServiceVoidResponse>>
    {
        let mut request = tonic::Request::new(payment_void_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .void(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentCancelFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_cancel",
                    connector_name=?connector_name,
                    "UCS payment cancel gRPC call failed"
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

    /// Performs Payment Refund through PaymentService.Refund
    pub async fn payment_refund(
        &self,
        payment_refund_request: PaymentServiceRefundRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<RefundResponse>> {
        let mut request = tonic::Request::new(payment_refund_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .refund(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentRefundFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_refund",
                    connector_name=?connector_name,
                    "UCS payment refund gRPC call failed"
                )
            })
    }

    /// Performs Refund Sync through RefundService.Get
    pub async fn refund_sync(
        &self,
        refund_sync_request: RefundServiceGetRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<RefundResponse>> {
        let mut request = tonic::Request::new(refund_sync_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.refund_client
            .clone()
            .get(request)
            .await
            .change_context(UnifiedConnectorServiceError::RefundSyncFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="refund_sync",
                    connector_name=?connector_name,
                    "UCS refund sync gRPC call failed"
                )
            })
    }

    /// Performs Create Access Token Granular
    pub async fn create_access_token(
        &self,
        create_access_token_request: payments_grpc::PaymentServiceCreateAccessTokenRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceCreateAccessTokenResponse>,
    > {
        let mut request = tonic::Request::new(create_access_token_request);
        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.client
            .clone()
            .create_access_token(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentCreateAccessTokenFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_access_token",
                    connector_name=?connector_name,
                    "UCS create access token granular gRPC call failed"
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
    if let Some(key2) = meta.key2 {
        metadata.append(consts::UCS_HEADER_KEY2, parse("key2", key2.peek())?);
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

    if let Some(ref request_id) = grpc_headers.request_id {
        metadata.append(
            common_utils_consts::X_REQUEST_ID,
            parse(common_utils_consts::X_REQUEST_ID, request_id.as_str())?,
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
