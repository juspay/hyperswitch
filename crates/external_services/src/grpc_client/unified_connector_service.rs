use std::collections::{HashMap, HashSet};

use common_enums::connector_enums::Connector;
use common_utils::{consts as common_utils_consts, errors::CustomResult, types::Url};
use error_stack::ResultExt;
pub use hyperswitch_interfaces::unified_connector_service::transformers::UnifiedConnectorServiceError;
use hyperswitch_masking::{PeekInterface, Secret};
use router_env::logger;
use tokio::time::{timeout, Duration};
use tonic::{
    metadata::{MetadataMap, MetadataValue},
    transport::Uri,
};
use unified_connector_service_client::payments as payments_grpc;

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
    /// The Payment Service Client
    pub payment_service_client: payments_grpc::payment_service_client::PaymentServiceClient<tonic::transport::Channel>,
    /// The Refund Service Client
    pub refund_service_client: payments_grpc::refund_service_client::RefundServiceClient<tonic::transport::Channel>,
    /// The Event Service Client
    pub event_service_client: payments_grpc::event_service_client::EventServiceClient<tonic::transport::Channel>,
    /// The Recurring Payment Service Client
    pub recurring_payment_service_client: payments_grpc::recurring_payment_service_client::RecurringPaymentServiceClient<tonic::transport::Channel>,
    /// The Dispute Service Client
    pub dispute_service_client: payments_grpc::dispute_service_client::DisputeServiceClient<tonic::transport::Channel>,
    /// The Payment Method Service Client
    pub payment_method_service_client: payments_grpc::payment_method_service_client::PaymentMethodServiceClient<tonic::transport::Channel>,
    /// The Customer Service Client
    pub customer_service_client: payments_grpc::customer_service_client::CustomerServiceClient<tonic::transport::Channel>,
    /// The Merchant Authentication Service Client
    pub merchant_authentication_service_client:
        payments_grpc::merchant_authentication_service_client::MerchantAuthenticationServiceClient<tonic::transport::Channel>,
    /// The Payment Method Authentication Service Client
    pub payment_method_authentication_service_client:
        payments_grpc::payment_method_authentication_service_client::PaymentMethodAuthenticationServiceClient<tonic::transport::Channel>,
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

    /// Connector-specific configuration (JSON serialized) for UCS.
    pub connector_config: Option<Secret<String>>,
}

/// External Vault Proxy Related Metadata
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum ExternalVaultProxyMetadata {
    /// VGS proxy data variant
    VgsMetadata(VgsMetadata),
}

/// Builds a gRPC client with timeout handling
#[macro_export]
macro_rules! build_grpc_client {
    ($client:ty, $name:expr, $uri:expr, $timeout:expr) => {{
        match timeout(
            Duration::from_secs($timeout),
            <$client>::connect($uri.clone()),
        )
        .await
        {
            Ok(Ok(client)) => client,
            Ok(Err(err)) => {
                router_env::logger::error!(
                    "Failed to connect to Unified Connector Service for {}: {:?}",
                    $name,
                    err
                );
                return None;
            }
            Err(err) => {
                router_env::logger::error!(
                    "Connection to Unified Connector Service timed out for {}: {:?}",
                    $name,
                    err
                );
                return None;
            }
        }
    }};
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

                let timeout = unified_connector_service_client_config.connection_timeout;

                let payment_service_client = build_grpc_client!(
                    payments_grpc::payment_service_client::PaymentServiceClient<
                        tonic::transport::Channel,
                    >,
                    "payment_service_client",
                    uri,
                    timeout
                );

                let refund_service_client = build_grpc_client!(
                    payments_grpc::refund_service_client::RefundServiceClient<
                        tonic::transport::Channel,
                    >,
                    "refund_service_client",
                    uri,
                    timeout
                );

                let event_service_client = build_grpc_client!(
                    payments_grpc::event_service_client::EventServiceClient<
                        tonic::transport::Channel,
                    >,
                    "event_service_client",
                    uri,
                    timeout
                );

                let recurring_payment_service_client = build_grpc_client!(
                    payments_grpc::recurring_payment_service_client::RecurringPaymentServiceClient<
                        tonic::transport::Channel,
                    >,
                    "recurring_payment_service_client",
                    uri,
                    timeout
                );

                let dispute_service_client = build_grpc_client!(
                    payments_grpc::dispute_service_client::DisputeServiceClient<
                        tonic::transport::Channel,
                    >,
                    "dispute_service_client",
                    uri,
                    timeout
                );

                let payment_method_service_client = build_grpc_client!(
                    payments_grpc::payment_method_service_client::PaymentMethodServiceClient<
                        tonic::transport::Channel,
                    >,
                    "payment_method_service_client",
                    uri,
                    timeout
                );

                let customer_service_client = build_grpc_client!(
                    payments_grpc::customer_service_client::CustomerServiceClient<
                        tonic::transport::Channel,
                    >,
                    "customer_service_client",
                    uri,
                    timeout
                );

                let merchant_authentication_service_client = build_grpc_client!(
                    payments_grpc::merchant_authentication_service_client::MerchantAuthenticationServiceClient<tonic::transport::Channel>,
                    "merchant_authentication_service_client",
                    uri,
                    timeout
                );

                let payment_method_authentication_service_client = build_grpc_client!(
                    payments_grpc::payment_method_authentication_service_client::PaymentMethodAuthenticationServiceClient<tonic::transport::Channel>,
                    "payment_method_authentication_service_client",
                    uri,
                    timeout
                );

                logger::info!("Successfully connected to Unified Connector Service");

                Some(Self {
                    payment_service_client,
                    refund_service_client,
                    event_service_client,
                    recurring_payment_service_client,
                    dispute_service_client,
                    payment_method_service_client,
                    customer_service_client,
                    merchant_authentication_service_client,
                    payment_method_authentication_service_client,
                })
            }
            None => {
                router_env::logger::error!(?config.unified_connector_service, "Unified Connector Service config is missing");
                None
            }
        }
    }

    /// Performs Payment Method Tokenize
    pub async fn payment_method_tokenize(
        &self,
        payment_method_tokenize_request: payments_grpc::PaymentMethodServiceTokenizeRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentMethodServiceTokenizeResponse>,
    > {
        let mut request = tonic::Request::new(payment_method_tokenize_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.payment_method_service_client
            .clone()
            .tokenize(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentMethodTokenizeFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_method_tokenize",
                    connector_name=?connector_name,
                    "UCS payment_method_tokenize gRPC call failed"
                )
            })
    }

    /// Performs SDK Session Token Create
    pub async fn create_sdk_session_token(
        &self,
        create_sdk_session_token_request: payments_grpc::MerchantAuthenticationServiceCreateClientAuthenticationTokenRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<
            payments_grpc::MerchantAuthenticationServiceCreateClientAuthenticationTokenResponse,
        >,
    > {
        let mut request = tonic::Request::new(create_sdk_session_token_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.merchant_authentication_service_client
            .clone()
            .create_client_authentication_token(request)
            .await
            .change_context(UnifiedConnectorServiceError::CreateSdkSessionTokenFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_client_authentication_token",
                    connector_name=?connector_name,
                    "UCS create client authentication token gRPC call failed"
                )
            })
    }

    /// Performs Payment Incremental Authorization
    pub async fn payment_incremental_authorization(
        &self,
        payment_incremental_authorization_request: payments_grpc::PaymentServiceIncrementalAuthorizationRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceIncrementalAuthorizationResponse>,
    > {
        let mut request = tonic::Request::new(payment_incremental_authorization_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.payment_service_client
            .clone()
            .incremental_authorization(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentIncrementalAuthorizationFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_incremental_authorization",
                    connector_name=?connector_name,
                    "UCS payment_incremental_authorization gRPC call failed"
                )
            })
    }

    /// Performs Create Connector Customer
    pub async fn create_connector_customer(
        &self,
        create_connector_customer_request: payments_grpc::CustomerServiceCreateRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::CustomerServiceCreateResponse>>
    {
        let mut request = tonic::Request::new(create_connector_customer_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.customer_service_client
            .clone()
            .create(request)
            .await
            .change_context(UnifiedConnectorServiceError::CreateConnectorCustomerFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_connector_customer",
                    connector_name=?connector_name,
                    "UCS create connector customer gRPC call failed"
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

        self.payment_service_client
            .clone()
            .create_order(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentCreateOrderFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_create_order",
                    connector_name=?connector_name,
                    "UCS payment_create_order gRPC call failed"
                )
            })
    }

    /// Performs Payment Pre Authenticate
    pub async fn payment_pre_authenticate(
        &self,
        payment_pre_authenticate_request: payments_grpc::PaymentMethodAuthenticationServicePreAuthenticateRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentMethodAuthenticationServicePreAuthenticateResponse>,
    > {
        let mut request = tonic::Request::new(payment_pre_authenticate_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;

        *request.metadata_mut() = metadata;

        self.payment_method_authentication_service_client
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
        payment_authenticate_request: payments_grpc::PaymentMethodAuthenticationServiceAuthenticateRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentMethodAuthenticationServiceAuthenticateResponse>,
    > {
        let mut request = tonic::Request::new(payment_authenticate_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;

        *request.metadata_mut() = metadata;

        self.payment_method_authentication_service_client
            .clone()
            .authenticate(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentAuthenticateFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_authenticate",
                    connector_name=?connector_name,
                    "UCS payment authenticate gRPC call failed"
                )
            })
    }

    /// Performs Session token create
    pub async fn create_session_token(
        &self,
        create_session_token_request: payments_grpc::MerchantAuthenticationServiceCreateServerSessionAuthenticationTokenRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::MerchantAuthenticationServiceCreateServerSessionAuthenticationTokenResponse>,
    >{
        let mut request = tonic::Request::new(create_session_token_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.merchant_authentication_service_client
            .clone()
            .create_server_session_authentication_token(request)
            .await
            .change_context(UnifiedConnectorServiceError::CreateSessionTokenFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_server_session_authentication_token",
                    connector_name=?connector_name,
                    "UCS createm server session authentication token gRPC call failed"
                )
            })
    }

    /// Performs Payment Post Authenticate
    pub async fn payment_post_authenticate(
        &self,
        payment_post_authenticate_request: payments_grpc::PaymentMethodAuthenticationServicePostAuthenticateRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentMethodAuthenticationServicePostAuthenticateResponse>,
    > {
        let mut request = tonic::Request::new(payment_post_authenticate_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;

        *request.metadata_mut() = metadata;

        self.payment_method_authentication_service_client
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
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceAuthorizeResponse>,
    > {
        let mut request = tonic::Request::new(payment_authorize_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;

        *request.metadata_mut() = metadata;

        self.payment_service_client
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

        self.payment_service_client
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

        self.payment_service_client
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

    /// Performs Payment Setup Recurring/Mandate
    pub async fn payment_setup_recurring(
        &self,
        payment_setup_recurring_request: payments_grpc::PaymentServiceSetupRecurringRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentServiceSetupRecurringResponse>,
    > {
        let mut request = tonic::Request::new(payment_setup_recurring_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.payment_service_client
            .clone()
            .setup_recurring(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentSetupRecurringFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_setup_recurring",
                    connector_name=?connector_name,
                    "UCS payment setup recurring gRPC call failed"
                )
            })
    }

    /// Performs recurring payment (MIT - Merchant Initiated Transaction).
    pub async fn recurring_payment_charge(
        &self,
        recurring_payment_charge_request: payments_grpc::RecurringPaymentServiceChargeRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::RecurringPaymentServiceChargeResponse>,
    > {
        let mut request = tonic::Request::new(recurring_payment_charge_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.recurring_payment_service_client
            .clone()
            .charge(request)
            .await
            .change_context(UnifiedConnectorServiceError::RecurringPaymentChargeFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="recurring_payment_charge",
                    connector_name=?connector_name,
                    "UCS recurring payment charge gRPC call failed"
                )
            })
    }

    /// Performs Payment Cancel/Void
    pub async fn payment_void(
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

        self.payment_service_client
            .clone()
            .void(request)
            .await
            .change_context(UnifiedConnectorServiceError::PaymentVoidFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_void",
                    connector_name=?connector_name,
                    "UCS payment void gRPC call failed"
                )
            })
    }

    /// Incoming webhook handle
    pub async fn incoming_webhook_handle_event(
        &self,
        incoming_webhook_handle_event_request: payments_grpc::EventServiceHandleRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::EventServiceHandleResponse>>
    {
        let mut request = tonic::Request::new(incoming_webhook_handle_event_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.event_service_client
            .clone()
            .handle_event(request)
            .await
            .change_context(UnifiedConnectorServiceError::IncomingWebhookHandleEventFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="incoming_webhook_handle_event",
                    connector_name=?connector_name,
                    "UCS incoming webhook handle event gRPC call failed"
                )
            })
    }

    /// Performs Payment Refund
    pub async fn payment_refund(
        &self,
        payment_refund_request: payments_grpc::PaymentServiceRefundRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::RefundResponse>> {
        let mut request = tonic::Request::new(payment_refund_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.payment_service_client
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

    /// Performs Refund Sync/Get
    pub async fn refund_get(
        &self,
        refund_get_request: payments_grpc::RefundServiceGetRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::RefundResponse>> {
        let mut request = tonic::Request::new(refund_get_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.refund_service_client
            .clone()
            .get(request)
            .await
            .change_context(UnifiedConnectorServiceError::RefundSyncFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="refund_get",
                    connector_name=?connector_name,
                    "UCS refund get gRPC call failed"
                )
            })
    }

    /// Performs Create Access Token Granular
    pub async fn create_access_token(
        &self,
        create_access_token_request: payments_grpc::MerchantAuthenticationServiceCreateServerAuthenticationTokenRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<
            payments_grpc::MerchantAuthenticationServiceCreateServerAuthenticationTokenResponse,
        >,
    > {
        let mut request = tonic::Request::new(create_access_token_request);
        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.merchant_authentication_service_client
            .clone()
            .create_server_authentication_token(request)
            .await
            .change_context(UnifiedConnectorServiceError::CreateAccessTokenFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_server_authentication_token",
                    connector_name=?connector_name,
                    "UCS create server authentication token gRPC call failed"
                )
            })
    }
}

/// Build the gRPC Headers for Unified Connector Service Request
pub fn build_unified_connector_service_grpc_headers(
    meta: ConnectorAuthMetadata,
    grpc_headers: GrpcHeadersUcs,
) -> Result<MetadataMap, UnifiedConnectorServiceError> {
    // Destructure grpc_headers to ensure all fields are handled
    let GrpcHeadersUcs {
        tenant_id,
        request_id,
        lineage_ids,
        external_vault_proxy_metadata,
        merchant_reference_id,
        resource_id,
        shadow_mode,
        config_override,
    } = grpc_headers;

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

    // Add connector-specific config header if available
    if let Some(connector_config) = meta.connector_config {
        metadata.append(
            consts::UCS_HEADER_CONNECTOR_CONFIG,
            parse("connector_config", connector_config.peek())?,
        );
    }

    metadata.append(
        common_utils_consts::X_MERCHANT_ID,
        parse(common_utils_consts::X_MERCHANT_ID, meta.merchant_id.peek())?,
    );

    if let Some(external_vault_proxy_metadata) = external_vault_proxy_metadata {
        metadata.append(
            consts::UCS_HEADER_EXTERNAL_VAULT_METADATA,
            parse("external_vault_metadata", &external_vault_proxy_metadata)?,
        );
    };

    let lineage_ids_str = lineage_ids.get_url_encoded_string().map_err(|err| {
        logger::error!(?err);
        UnifiedConnectorServiceError::HeaderInjectionFailed(consts::UCS_LINEAGE_IDS.to_string())
    })?;
    metadata.append(
        consts::UCS_LINEAGE_IDS,
        parse(consts::UCS_LINEAGE_IDS, &lineage_ids_str)?,
    );

    if let Some(reference_id) = merchant_reference_id {
        metadata.append(
            consts::UCS_HEADER_REFERENCE_ID,
            parse(
                consts::UCS_HEADER_REFERENCE_ID,
                reference_id.get_string_repr(),
            )?,
        );
    };

    if let Some(resource_id) = resource_id {
        metadata.append(
            consts::UCS_HEADER_RESOURCE_ID,
            parse(
                consts::UCS_HEADER_RESOURCE_ID,
                resource_id.get_string_repr(),
            )?,
        );
    };

    if let Some(ref request_id) = request_id {
        metadata.append(
            common_utils_consts::X_REQUEST_ID,
            parse(common_utils_consts::X_REQUEST_ID, request_id.as_str())?,
        );
    };

    if let Some(shadow_mode) = shadow_mode {
        metadata.append(
            common_utils_consts::X_UNIFIED_CONNECTOR_SERVICE_MODE,
            parse(
                common_utils_consts::X_UNIFIED_CONNECTOR_SERVICE_MODE,
                &shadow_mode.to_string(),
            )?,
        );
    }

    if let Some(config_override) = config_override {
        metadata.append(
            common_utils_consts::X_CONFIG_OVERRIDE,
            parse(common_utils_consts::X_CONFIG_OVERRIDE, &config_override)?,
        );
    }

    if let Err(err) = tenant_id
        .parse()
        .map(|tenant_id| metadata.append(common_utils_consts::TENANT_HEADER, tenant_id))
    {
        logger::error!(
            header_parse_error=?err,
            tenant_id=?tenant_id,
            "Failed to parse tenant_id header for UCS gRPC request: {}",
            common_utils_consts::TENANT_HEADER
        );
    }

    Ok(metadata)
}
