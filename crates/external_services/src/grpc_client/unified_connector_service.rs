use std::collections::{HashMap, HashSet};

use common_enums::{connector_enums::Connector, ConnectorType};
use common_utils::{consts as common_utils_consts, errors::CustomResult, types::Url};
use error_stack::ResultExt;
pub use hyperswitch_interfaces::unified_connector_service::transformers::UnifiedConnectorServiceError;
use hyperswitch_masking::{PeekInterface, Secret};
use router_env::logger;
use tokio::time::Duration;
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

/// The transport under every UCS service client. Under the `deja` feature it is
/// the gRPC egress boundary wrapper — every unary rpc (payment, refund, …) is
/// recorded/substituted at the wire level with rank-2 span-path identity;
/// otherwise it is the raw tonic channel. Threading this alias through the
/// client fields + their construction keeps the feature-off build byte-identical.
#[cfg(feature = "deja")]
pub type UcsChannel = super::deja_transport::DejaGrpcTransport<tonic::transport::Channel>;
/// The transport under every UCS service client (raw tonic channel; see the
/// `deja`-gated definition above for the recording/substituting variant).
#[cfg(not(feature = "deja"))]
pub type UcsChannel = tonic::transport::Channel;
/// Contains the  Unified Connector Service client
#[derive(Debug, Clone)]
pub struct UnifiedConnectorServiceClient {
    /// The Payment Service Client
    pub payment_service_client: payments_grpc::payment_service_client::PaymentServiceClient<UcsChannel>,
    /// The Refund Service Client
    pub refund_service_client: payments_grpc::refund_service_client::RefundServiceClient<UcsChannel>,
    /// The Event Service Client
    pub event_service_client: payments_grpc::event_service_client::EventServiceClient<UcsChannel>,
    /// The Recurring Payment Service Client
    pub recurring_payment_service_client: payments_grpc::recurring_payment_service_client::RecurringPaymentServiceClient<UcsChannel>,
    /// The Dispute Service Client
    pub dispute_service_client: payments_grpc::dispute_service_client::DisputeServiceClient<UcsChannel>,
    /// The Payment Method Service Client
    pub payment_method_service_client: payments_grpc::payment_method_service_client::PaymentMethodServiceClient<UcsChannel>,
    /// The Customer Service Client
    pub customer_service_client: payments_grpc::customer_service_client::CustomerServiceClient<UcsChannel>,
    /// The Merchant Authentication Service Client
    pub merchant_authentication_service_client:
        payments_grpc::merchant_authentication_service_client::MerchantAuthenticationServiceClient<UcsChannel>,
    /// The Payment Method Authentication Service Client
    pub payment_method_authentication_service_client:
        payments_grpc::payment_method_authentication_service_client::PaymentMethodAuthenticationServiceClient<UcsChannel>,
        /// The Payout Service Client
    pub payout_service_client: payments_grpc::payout_service_client::PayoutServiceClient<UcsChannel>,
    /// The Surcharge Service Client
    pub surcharge_service_client: payments_grpc::surcharge_service_client::SurchargeServiceClient<UcsChannel>,
    /// Standard gRPC health client (grpc.health.v1) over the same shared channel, used by the
    /// router health endpoint to prove UCS is reachable from this pod. Gated like the module it
    /// comes from: the health proto is only compiled under `dynamic_routing`.
    #[cfg(feature = "dynamic_routing")]
    pub health_client: super::health_check_client::HealthClient<UcsChannel>,
}

/// Contains the Unified Connector Service Client config
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct UnifiedConnectorServiceClientConfig {
    /// Base URL of the gRPC Server
    pub base_url: Url,

    /// Contains the connection timeout duration in seconds
    pub connection_timeout: UcsConnectionTimeoutInSeconds,

    /// Per-RPC timeout (seconds) for calls to the unified connector service.
    #[serde(default)]
    pub request_timeout: UcsRequestTimeoutInSeconds,

    /// HTTP/2 PING keepalive interval (seconds) on the shared channel. PINGs are sent while idle.
    #[serde(default = "default_ucs_keep_alive_interval_secs")]
    pub keep_alive_interval_secs: u64,

    /// Time (seconds) to wait for a keepalive PING acknowledgement before the connection is
    /// treated as dead and re-established.
    #[serde(default = "default_ucs_keep_alive_timeout_secs")]
    pub keep_alive_timeout_secs: u64,

    /// TCP keepalive idle time (seconds) on the shared channel.
    #[serde(default = "default_ucs_tcp_keepalive_secs")]
    pub tcp_keepalive_secs: u64,

    /// Set of external services/connectors available for the unified connector service
    #[serde(default, deserialize_with = "deserialize_hashset")]
    pub ucs_only_connectors: HashSet<Connector>,

    /// Set of connectors for which psync is disabled in unified connector service
    #[serde(default, deserialize_with = "deserialize_hashset")]
    pub ucs_psync_disabled_connectors: HashSet<Connector>,
}

/// Connection timeout for the Unified Connector Service in seconds.
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct UcsConnectionTimeoutInSeconds(u64);

impl UcsConnectionTimeoutInSeconds {
    /// Return the timeout as a [`Duration`].
    pub fn as_duration(self) -> Duration {
        Duration::from_secs(self.0)
    }
}

/// Per-RPC request timeout for the Unified Connector Service in seconds.
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct UcsRequestTimeoutInSeconds(u64);

impl Default for UcsRequestTimeoutInSeconds {
    fn default() -> Self {
        Self(consts::DEFAULT_UCS_REQUEST_TIMEOUT_SECS)
    }
}

impl UcsRequestTimeoutInSeconds {
    /// Return the timeout as a [`Duration`].
    pub fn as_duration(self) -> Duration {
        Duration::from_secs(self.0)
    }
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

/// Type of the vault connector
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultConnectorType {
    /// Proxy vault - forwards requests through a proxy (e.g., VGS forward proxy)
    Proxy,
    /// Transformation vault - transforms/tokenizes data (e.g., HyperswitchVault)
    Transformation,
}

/// Authentication credentials for vault connectors
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct VaultConnectorAuth {
    /// API key for authenticating with the vault connector
    pub api_key: Secret<String>,
    /// API secret for authenticating with the vault connector
    pub profile_id: Secret<String>,
}

/// External Vault Proxy Related Metadata
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum ExternalVaultProxyMetadata {
    /// VGS proxy data variant
    VgsMetadata(VgsMetadata),
    /// HyperswitchVault data variant
    HyperswitchVaultMetadata(HyperswitchVaultMetadata),
}

/// Complete external vault proxy configuration to be serialized and sent to UCS
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ExternalVaultProxyConfig {
    /// Type of the vault connector (e.g., Proxy or Transformation)
    pub vault_connector_type: VaultConnectorType,
    /// Name/ID of the vault connector (e.g., "vgs", "hyperswitch_vault")
    pub vault_connector_id: Option<String>,
    /// Metadata specific to the vault connector type
    pub metadata: ExternalVaultProxyMetadata,
}

/// HyperswitchVault proxy data
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HyperswitchVaultMetadata {
    /// External vault url
    pub vault_endpoint: Url,
    /// Authentication data for the vault connector
    pub vault_auth_data: VaultConnectorAuth,
}

fn default_ucs_keep_alive_interval_secs() -> u64 {
    consts::DEFAULT_UCS_KEEP_ALIVE_INTERVAL_SECS
}

fn default_ucs_keep_alive_timeout_secs() -> u64 {
    consts::DEFAULT_UCS_KEEP_ALIVE_TIMEOUT_SECS
}

fn default_ucs_tcp_keepalive_secs() -> u64 {
    consts::DEFAULT_UCS_TCP_KEEPALIVE_SECS
}

/// Failure to build the Unified Connector Service client from configuration.
///
/// Fatal at startup by design: a configured UCS that cannot be set up must stop the pod from
/// starting rather than silently degrading every payment on this pod to the direct connector
/// path, which is not an option for `ucs_only_connectors`.
#[derive(Debug, thiserror::Error)]
pub enum UcsClientBuildError {
    /// The configured `base_url` is not a valid URI
    #[error("Failed to parse Unified Connector Service base_url as a URI: {0}")]
    InvalidUri(String),
    /// The transport boundary did not yield a usable channel
    #[error("Unified Connector Service transport could not be constructed")]
    TransportUnavailable,
}

/// Builds the single shared channel to the Unified Connector Service.
///
/// One lazily connected HTTP/2 connection carries every UCS service client, so no client-specific
/// channel can sit idle for hours. HTTP/2 PING keepalive runs while idle: a connection that dies
/// without a GOAWAY is detected within `keep_alive_interval + keep_alive_timeout` and rebuilt by
/// tonic's reconnect layer before the next RPC, instead of the next RPC failing with
/// `transport error`. `connect_lazy` performs no I/O here; the first RPC (or the router health
/// check) opens the connection, and a failed dial fails only that RPC and is retried on the next.
fn build_ucs_channel(
    uri: Uri,
    config: &UnifiedConnectorServiceClientConfig,
) -> tonic::transport::Channel {
    tonic::transport::Channel::builder(uri)
        .connect_timeout(config.connection_timeout.as_duration())
        .timeout(config.request_timeout.as_duration())
        .http2_keep_alive_interval(Duration::from_secs(config.keep_alive_interval_secs))
        .keep_alive_timeout(Duration::from_secs(config.keep_alive_timeout_secs))
        .keep_alive_while_idle(true)
        .tcp_keepalive(Some(Duration::from_secs(config.tcp_keepalive_secs)))
        // A single connection now carries every UCS service client. Each previously had its own
        // default 64 KiB HTTP/2 connection window, so let hyper size the windows from measured
        // bandwidth rather than capping throughput on the shared connection.
        .http2_adaptive_window(true)
        .connect_lazy()
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
    /// Builds every UCS service client over one shared channel.
    ///
    /// Returns `Ok(None)` when UCS is not configured. Returns `Err` when it is configured but the
    /// client cannot be built; the caller treats that as fatal at startup.
    pub async fn build_connections(
        config: &GrpcClientSettings,
    ) -> Result<Option<Self>, UcsClientBuildError> {
        let Some(ucs_config) = &config.unified_connector_service else {
            logger::info!("Unified Connector Service is not configured; client not built");
            return Ok(None);
        };

        let uri: Uri = ucs_config.base_url.get_string_repr().parse().map_err(
            |err: tonic::codegen::http::uri::InvalidUri| {
                logger::error!(error = ?err, "Failed to parse URI for Unified Connector Service");
                UcsClientBuildError::InvalidUri(err.to_string())
            },
        )?;

        let channel = build_ucs_channel(uri, ucs_config);

        // deja: the same channel handed to the gRPC egress boundary so every unary rpc is
        // recorded/substituted at the wire level. Under replay no transport is connected at all.
        #[cfg(feature = "deja")]
        let transport: UcsChannel =
            super::deja_transport::connect_or_substitute(|| async { Some(channel) })
                .await
                .ok_or(UcsClientBuildError::TransportUnavailable)?;

        #[cfg(not(feature = "deja"))]
        let transport: UcsChannel = channel;

        logger::info!(
            keep_alive_interval_secs = ucs_config.keep_alive_interval_secs,
            keep_alive_timeout_secs = ucs_config.keep_alive_timeout_secs,
            tcp_keepalive_secs = ucs_config.tcp_keepalive_secs,
            "Unified Connector Service clients built over one shared lazily-connected channel"
        );

        Ok(Some(Self {
            payment_service_client: payments_grpc::payment_service_client::PaymentServiceClient::new(
                transport.clone(),
            ),
            refund_service_client: payments_grpc::refund_service_client::RefundServiceClient::new(
                transport.clone(),
            ),
            event_service_client: payments_grpc::event_service_client::EventServiceClient::new(
                transport.clone(),
            ),
            recurring_payment_service_client:
                payments_grpc::recurring_payment_service_client::RecurringPaymentServiceClient::new(
                    transport.clone(),
                ),
            dispute_service_client: payments_grpc::dispute_service_client::DisputeServiceClient::new(
                transport.clone(),
            ),
            payment_method_service_client:
                payments_grpc::payment_method_service_client::PaymentMethodServiceClient::new(
                    transport.clone(),
                ),
            customer_service_client: payments_grpc::customer_service_client::CustomerServiceClient::new(
                transport.clone(),
            ),
            merchant_authentication_service_client:
                payments_grpc::merchant_authentication_service_client::MerchantAuthenticationServiceClient::new(
                    transport.clone(),
                ),
            payment_method_authentication_service_client:
                payments_grpc::payment_method_authentication_service_client::PaymentMethodAuthenticationServiceClient::new(
                    transport.clone(),
                ),
            payout_service_client: payments_grpc::payout_service_client::PayoutServiceClient::new(
                transport.clone(),
            ),
            surcharge_service_client:
                payments_grpc::surcharge_service_client::SurchargeServiceClient::new(
                    transport.clone(),
                ),
            #[cfg(feature = "dynamic_routing")]
            health_client: super::health_check_client::HealthClient::new(transport.clone()),
        }))
    }

    /// Standard gRPC health check (grpc.health.v1 `Check`, empty service name) against UCS over
    /// the shared channel. `Ok(true)` means UCS reported `SERVING`.
    #[cfg(feature = "dynamic_routing")]
    pub async fn health_check(&self) -> Result<bool, tonic::Status> {
        let request = tonic::Request::new(super::health_check_client::HealthCheckRequest {
            service: String::new(),
        });
        let response = self.health_client.clone().check(request).await?;
        Ok(response.into_inner().status() == super::health_check_client::ServingStatus::Serving)
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_method_tokenize",
                    connector_name=?connector_name,
                    "UCS payment_method_tokenize gRPC call failed"
                )
            })
    }

    /// Performs Payment Method Refresh
    pub async fn payment_method_refresh(
        &self,
        payment_method_refresh_request: payments_grpc::PaymentMethodServiceRefreshRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
        timeout: Duration,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PaymentMethodServiceRefreshResponse>,
    > {
        let mut request = tonic::Request::new(payment_method_refresh_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;
        request.set_timeout(timeout);

        self.payment_method_service_client
            .clone()
            .refresh(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payment_method_refresh",
                    connector_name=?connector_name,
                    "UCS payment_method_refresh gRPC call failed"
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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

        Box::pin(
            self.payment_method_authentication_service_client
                .clone()
                .authenticate(request),
        )
        .await
        .map_err(|error| {
            error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                &error,
                &connector_name,
            ))
        })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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

        // Box the authorize future: the merged UCS proto request types are large enough
        // to trip clippy::large_futures when this is awaited inline.
        Box::pin(self.payment_service_client.clone().authorize(request))
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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

        Box::pin(self.payment_service_client.clone().setup_recurring(request))
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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

        Box::pin(
            self.recurring_payment_service_client
                .clone()
                .charge(request),
        )
        .await
        .map_err(|error| {
            error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                &error,
                &connector_name,
            ))
        })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="incoming_webhook_handle_event",
                    connector_name=?connector_name,
                    "UCS incoming webhook handle event gRPC call failed"
                )
            })
    }

    /// Phase 1 of the two-phase UCS webhook API.
    ///
    /// Parses the raw inbound webhook payload to extract a typed `EventReference` and the
    /// `WebhookEventType`, without requiring credentials or making any outbound connector
    /// call. The caller uses the returned reference to resolve the merchant-connector
    /// account (and thus the webhook secret) before invoking `incoming_webhook_handle_event`.
    pub async fn incoming_webhook_parse_event(
        &self,
        incoming_webhook_parse_event_request: payments_grpc::EventServiceParseRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::EventServiceParseResponse>>
    {
        let mut request = tonic::Request::new(incoming_webhook_parse_event_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.event_service_client
            .clone()
            .parse_event(request)
            .await
            .change_context(UnifiedConnectorServiceError::IncomingWebhookParseEventFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="incoming_webhook_parse_event",
                    connector_name=?connector_name,
                    "UCS incoming webhook parse event gRPC call failed"
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
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
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="refund_get",
                    connector_name=?connector_name,
                    "UCS refund get gRPC call failed"
                )
            })
    }

    /// Voids or reverses a refund before connector settlement.
    pub async fn refund_void_post_refund(
        &self,
        request_data: payments_grpc::RefundServiceVoidPostRefundRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::RefundResponse>> {
        let mut request = tonic::Request::new(request_data);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata =
            build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)?;
        *request.metadata_mut() = metadata;

        self.refund_service_client
            .clone()
            .void_post_refund(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="refund_void_post_refund",
                    connector_name=?connector_name,
                    "UCS refund void post-refund gRPC call failed"
                )
            })
    }

    /// Performs Payout Create
    pub async fn payout_create(
        &self,
        payout_create_request: payments_grpc::PayoutServiceCreateRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PayoutServiceCreateResponse>>
    {
        let mut request = tonic::Request::new(payout_create_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_payouts(
            connector_auth_metadata,
            grpc_headers,
        )?;

        *request.metadata_mut() = metadata;

        self.payout_service_client
            .clone()
            .create(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payout_create",
                    connector_name=?connector_name,
                    "UCS payout create gRPC call failed"
                )
            })
    }

    /// Performs Create Access Token Granular
    ///
    /// The connector type determines which UCS connector header namespace is used.
    pub async fn create_access_token(
        &self,
        create_access_token_request: payments_grpc::MerchantAuthenticationServiceCreateServerAuthenticationTokenRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
        connector_type: ConnectorType,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<
            payments_grpc::MerchantAuthenticationServiceCreateServerAuthenticationTokenResponse,
        >,
    > {
        let mut request = tonic::Request::new(create_access_token_request);
        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_connector_type(
            connector_auth_metadata,
            grpc_headers,
            connector_type,
        )?;
        *request.metadata_mut() = metadata;

        self.merchant_authentication_service_client
            .clone()
            .create_server_authentication_token(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="create_server_authentication_token",
                    connector_name=?connector_name,
                    "UCS create server authentication token gRPC call failed"
                )
            })
    }

    /// Performs Payout Transfer
    pub async fn payout_transfer(
        &self,
        payout_transfer_request: payments_grpc::PayoutServiceTransferRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PayoutServiceTransferResponse>>
    {
        let mut request = tonic::Request::new(payout_transfer_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_payouts(
            connector_auth_metadata,
            grpc_headers,
        )?;

        *request.metadata_mut() = metadata;

        self.payout_service_client
            .clone()
            .transfer(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payout_transfer",
                    connector_name=?connector_name,
                    "UCS payout transfer gRPC call failed"
                )
            })
    }

    /// Performs Payout Eligibility (e.g. SEPA VoP / payee verification).
    pub async fn payout_eligibility(
        &self,
        payout_eligibility_request: payments_grpc::PayoutMethodEligibilityRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PayoutMethodEligibilityResponse>,
    > {
        let mut request = tonic::Request::new(payout_eligibility_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_payouts(
            connector_auth_metadata,
            grpc_headers,
        )?;

        *request.metadata_mut() = metadata;

        self.payout_service_client
            .clone()
            .eligibility(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payout_eligibility",
                    connector_name=?connector_name,
                    "UCS payout eligibility gRPC call failed"
                )
            })
    }

    /// Performs Payout Get
    pub async fn payout_get(
        &self,
        payout_get_request: payments_grpc::PayoutServiceGetRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PayoutServiceGetResponse>>
    {
        let mut request = tonic::Request::new(payout_get_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_payouts(
            connector_auth_metadata,
            grpc_headers,
        )?;

        *request.metadata_mut() = metadata;

        self.payout_service_client
            .clone()
            .get(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payout_get",
                    connector_name=?connector_name,
                    "UCS payout get gRPC call failed"
                )
            })
    }

    /// Performs Payout Void
    pub async fn payout_void(
        &self,
        payout_void_request: payments_grpc::PayoutServiceVoidRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PayoutServiceVoidResponse>>
    {
        let mut request = tonic::Request::new(payout_void_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_payouts(
            connector_auth_metadata,
            grpc_headers,
        )?;

        *request.metadata_mut() = metadata;

        self.payout_service_client
            .clone()
            .void(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payout_void",
                    connector_name=?connector_name,
                    "UCS payout void gRPC call failed"
                )
            })
    }

    /// Performs Payout Stage
    pub async fn payout_stage(
        &self,
        payout_stage_request: payments_grpc::PayoutServiceStageRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::PayoutServiceStageResponse>>
    {
        let mut request = tonic::Request::new(payout_stage_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_payouts(
            connector_auth_metadata,
            grpc_headers,
        )?;

        *request.metadata_mut() = metadata;

        self.payout_service_client
            .clone()
            .stage(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payout_stage",
                    connector_name=?connector_name,
                    "UCS payout stage gRPC call failed"
                )
            })
    }

    /// Performs Payout Create Recipient
    pub async fn payout_create_recipient(
        &self,
        payout_create_recipient_request: payments_grpc::PayoutServiceCreateRecipientRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PayoutServiceCreateRecipientResponse>,
    > {
        let mut request = tonic::Request::new(payout_create_recipient_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_payouts(
            connector_auth_metadata,
            grpc_headers,
        )?;

        *request.metadata_mut() = metadata;

        self.payout_service_client
            .clone()
            .create_recipient(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payout_create_recipient",
                    connector_name=?connector_name,
                    "UCS payout create recipient gRPC call failed"
                )
            })
    }

    /// Performs Payout Enroll Disburse Account
    pub async fn payout_enroll_disburse_account(
        &self,
        payout_enroll_disburse_account_request: payments_grpc::PayoutServiceEnrollDisburseAccountRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::PayoutServiceEnrollDisburseAccountResponse>,
    > {
        let mut request = tonic::Request::new(payout_enroll_disburse_account_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_payouts(
            connector_auth_metadata,
            grpc_headers,
        )?;

        *request.metadata_mut() = metadata;

        self.payout_service_client
            .clone()
            .enroll_disburse_account(request)
            .await
            .map_err(|error| {
                error_stack::Report::new(UnifiedConnectorServiceError::from_grpc_error(
                    &error,
                    &connector_name,
                ))
            })
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="payout_enroll_disburse_account",
                    connector_name=?connector_name,
                    "UCS payout enroll disburse account gRPC call failed"
                )
            })
    }

    /// Performs surcharge calculation via the Surcharge Service
    pub async fn surcharge_calculate(
        &self,
        surcharge_calculate_request: payments_grpc::SurchargeServiceCalculateRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<payments_grpc::SurchargeServiceCalculateResponse>,
    > {
        let mut request = tonic::Request::new(surcharge_calculate_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_surcharge(
            connector_auth_metadata,
            grpc_headers,
        )?;

        *request.metadata_mut() = metadata;

        self.surcharge_service_client
            .clone()
            .calculate(request)
            .await
            .change_context(UnifiedConnectorServiceError::SurchargeCalculateFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="surcharge_calculate",
                    connector_name=?connector_name,
                    "UCS surcharge_calculate gRPC call failed"
                )
            })
    }

    /// Performs notify connector via the Event Service
    pub async fn notify_connector(
        &self,
        notify_connector_request: payments_grpc::NotifyConnectorRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: GrpcHeadersUcs,
        event_type: payments_grpc::NotifyEventType,
    ) -> UnifiedConnectorServiceResult<tonic::Response<payments_grpc::NotifyConnectorResponse>>
    {
        let mut request = tonic::Request::new(notify_connector_request);

        let connector_name = connector_auth_metadata.connector_name.clone();
        let metadata = build_unified_connector_service_grpc_headers_for_notify_connector(
            connector_auth_metadata,
            grpc_headers,
            event_type,
        )?;

        *request.metadata_mut() = metadata;

        self.event_service_client
            .clone()
            .notify_connector(request)
            .await
            .change_context(UnifiedConnectorServiceError::NotifyConnectorFailure)
            .inspect_err(|error| {
                logger::error!(
                    grpc_error=?error,
                    method="notify_connector",
                    connector_name=?connector_name,
                    "UCS notify_connector gRPC call failed"
                )
            })
    }
}

/// Build the gRPC Headers for Unified Connector Service Request
pub fn build_unified_connector_service_grpc_headers(
    meta: ConnectorAuthMetadata,
    grpc_headers: GrpcHeadersUcs,
) -> Result<MetadataMap, UnifiedConnectorServiceError> {
    build_grpc_headers_internal(meta, grpc_headers, consts::UCS_HEADER_CONNECTOR)
}

/// Build the gRPC Headers for Unified Connector Service Payout Request
/// Uses `x-payout-connector` instead of `x-connector` so the UCS server can
/// route payout traffic via a separate `PayoutConnectorEnum` without affecting
/// payment flows.
pub fn build_unified_connector_service_grpc_headers_for_payouts(
    meta: ConnectorAuthMetadata,
    grpc_headers: GrpcHeadersUcs,
) -> Result<MetadataMap, UnifiedConnectorServiceError> {
    build_grpc_headers_internal(meta, grpc_headers, consts::UCS_HEADER_PAYOUT_CONNECTOR)
}

fn build_unified_connector_service_grpc_headers_for_connector_type(
    meta: ConnectorAuthMetadata,
    grpc_headers: GrpcHeadersUcs,
    connector_type: ConnectorType,
) -> Result<MetadataMap, UnifiedConnectorServiceError> {
    let connector_header_key = match connector_type {
        ConnectorType::PaymentProcessor => consts::UCS_HEADER_CONNECTOR,
        ConnectorType::PayoutProcessor => consts::UCS_HEADER_PAYOUT_CONNECTOR,
        ConnectorType::SurchargeProcessor => consts::UCS_HEADER_SURCHARGE_CONNECTOR,
        connector_type => {
            return Err(
                UnifiedConnectorServiceError::RequestEncodingFailedWithReason(format!(
                    "unsupported UCS connector type for connector headers: {connector_type:?}"
                )),
            )
        }
    };

    build_grpc_headers_internal(meta, grpc_headers, connector_header_key)
}

fn build_grpc_headers_internal(
    meta: ConnectorAuthMetadata,
    grpc_headers: GrpcHeadersUcs,
    connector_header_key: &'static str,
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
        proxy_name,
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
        connector_header_key,
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

    if let Some(proxy_name) = proxy_name {
        metadata.append(
            common_utils_consts::X_PROXY_NAME,
            parse(common_utils_consts::X_PROXY_NAME, proxy_name)?,
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

/// Build gRPC headers for UCS surcharge requests.
/// Same as [`build_unified_connector_service_grpc_headers`] but uses
/// `x-surcharge-connector` instead of `x-connector` for the connector name,
/// since surcharge uses a distinct connector type.
pub fn build_unified_connector_service_grpc_headers_for_surcharge(
    meta: ConnectorAuthMetadata,
    grpc_headers: GrpcHeadersUcs,
) -> Result<MetadataMap, UnifiedConnectorServiceError> {
    let mut metadata = build_unified_connector_service_grpc_headers(meta.clone(), grpc_headers)?;

    metadata.remove(consts::UCS_HEADER_CONNECTOR);

    // Add the surcharge-specific connector header
    let connector_name = meta.connector_name.clone();
    let surcharge_connector_value =
        connector_name
            .parse::<MetadataValue<_>>()
            .map_err(|error| {
                logger::error!(?error);
                UnifiedConnectorServiceError::HeaderInjectionFailed(
                    consts::UCS_HEADER_SURCHARGE_CONNECTOR.to_string(),
                )
            })?;
    metadata.append(
        consts::UCS_HEADER_SURCHARGE_CONNECTOR,
        surcharge_connector_value,
    );

    Ok(metadata)
}

/// Build gRPC headers for UCS notify requests.
/// Build gRPC headers for UCS notify requests.
/// Same as [`build_unified_connector_service_grpc_headers`] but uses
/// `x-surcharge-connector` for surcharge events and `x-connector` for other events.
pub fn build_unified_connector_service_grpc_headers_for_notify_connector(
    meta: ConnectorAuthMetadata,
    grpc_headers: GrpcHeadersUcs,
    event_type: payments_grpc::NotifyEventType,
) -> Result<MetadataMap, UnifiedConnectorServiceError> {
    let mut metadata = build_unified_connector_service_grpc_headers(meta.clone(), grpc_headers)?;

    // Remove the default connector header
    metadata.remove(consts::UCS_HEADER_CONNECTOR);

    // Choose header based on event type
    let is_surcharge_event = matches!(
        event_type,
        payments_grpc::NotifyEventType::SurchargePaymentSucceeded
            | payments_grpc::NotifyEventType::SurchargeRefundSucceeded
    );

    let connector_name = meta.connector_name.clone();
    let connector_value = connector_name
        .parse::<MetadataValue<_>>()
        .map_err(|error| {
            logger::error!(?error);
            if is_surcharge_event {
                UnifiedConnectorServiceError::HeaderInjectionFailed(
                    consts::UCS_HEADER_SURCHARGE_CONNECTOR.to_string(),
                )
            } else {
                UnifiedConnectorServiceError::HeaderInjectionFailed(
                    consts::UCS_HEADER_CONNECTOR.to_string(),
                )
            }
        })?;

    if is_surcharge_event {
        metadata.append(consts::UCS_HEADER_SURCHARGE_CONNECTOR, connector_value);
    } else {
        metadata.append(consts::UCS_HEADER_CONNECTOR, connector_value);
    }

    Ok(metadata)
}
