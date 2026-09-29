//! In-process ("library mode") transport for the Unified Connector Service.
//!
//! For self-hosted deployments that ship a stable HS+UCS release and don't
//! want to run and operate UCS as a second service. Every flow here mirrors
//! one method on `external_services::grpc_client::unified_connector_service::
//! UnifiedConnectorServiceClient` (the gRPC-mode transport): same request/
//! response proto types (this crate's `grpc-api-types` dependency is pinned to
//! the branch/tag `external_services`'s own UCS dependency uses — see this
//! crate's `Cargo.toml`), same `ConnectorAuthMetadata` / `GrpcHeadersUcs`
//! inputs, same `UnifiedConnectorServiceResult` return shape — so a gateway
//! can hold either transport behind one small enum
//! (`external_services::grpc_client::unified_connector_service_transport::
//! UcsTransport`) and never duplicate request-building.
//!
//! What genuinely differs from gRPC mode, and why that's fine here:
//! - No network round-trip: `hyperswitch_payments_client::ConnectorClient`'s
//!   split `_build_request` / `execute_connector_request` / `_parse_response`
//!   methods are called directly, in-process.
//! - No UCS-side Kafka/otel event for the connector call: that only exists in
//!   UCS's own `execute_connector_processing_step`, which this path never
//!   goes through. HS's own event emission (wrapped around the calls below by
//!   the caller) is the only audit trail — nothing duplicated.
//! - Connector base URLs and other UCS-side settings come from a `ucs_env::
//!   configs::Config` this process builds from its own toml and hands to
//!   `hyperswitch_payments_client`'s runtime config override once at startup
//!   (`init`, below) — never from UCS's embedded sandbox/production defaults.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use error_stack::{Report, ResultExt};
use external_services::grpc_client::unified_connector_service::{
    build_unified_connector_service_grpc_headers, ConnectorAuthMetadata,
    UnifiedConnectorServiceResult,
};
use hyperswitch_interfaces::unified_connector_service::transformers::UnifiedConnectorServiceError;
use hyperswitch_masking::PeekInterface;
use hyperswitch_payments_client::{ConnectorClient, SdkError};

/// One-time startup wiring: point every `ConnectorClient` this process ever
/// constructs at `config` instead of the two sandbox/production defaults
/// embedded in the UCS crates. Must run before the first connector call in
/// library mode; call from the same place gRPC mode builds its
/// `UnifiedConnectorServiceClient`.
///
/// Returns `Err` if called more than once (a logic error in the caller, not a
/// runtime condition to recover from) — see
/// `connector_service_ffi::handlers::payments::set_runtime_config`.
pub fn init(config: Arc<ucs_env::configs::Config>) -> Result<(), Report<UnifiedConnectorServiceError>> {
    hyperswitch_payments_client::set_runtime_config(config)
        .map_err(|_| Report::new(UnifiedConnectorServiceError::ConnectionError(
            "unified_connector_service_library::init called more than once".to_string(),
        )))
}

/// Builds a `ucs_env::configs::Config` for [`init`] from HS's own settings.
///
/// Deliberately narrow: only the proxy/egress fields HS already owns for its
/// own outbound connector calls are translated here (see the field-by-field
/// mapping this was designed against). Connector base URLs and every other
/// UCS-side setting are left at `Config`'s own defaults — that's UCS's
/// domain knowledge to ship, not a deployment concern for HS to override.
///
/// `hs_proxy` is `hyperswitch_interfaces::types::Proxy` (HS's existing
/// `[proxy]` toml struct); `environment` selects which of UCS's embedded
/// connector-URL sets `Config::new_with_config_path` would otherwise have
/// picked, kept here for parity even though this path supplies its own
/// config file rather than an embedded one.
pub fn config_from_hs_proxy_settings(
    ucs_toml_path: &std::path::Path,
    hs_proxy: &hyperswitch_interfaces::types::Proxy,
) -> Result<ucs_env::configs::Config, Report<UnifiedConnectorServiceError>> {
    let mut config = ucs_env::configs::Config::new_with_config_path(Some(ucs_toml_path.to_path_buf()))
        .change_context(UnifiedConnectorServiceError::ConnectionError(
            "failed to load UCS config for library mode".to_string(),
        ))?;

    let mut proxies = HashMap::new();
    if hs_proxy.http_url.is_some() || hs_proxy.https_url.is_some() {
        proxies.insert(
            "primary".to_string(),
            domain_types::types::Proxy {
                http_url: hs_proxy.http_url.clone(),
                https_url: hs_proxy.https_url.clone(),
                ca_cert: hs_proxy
                    .mitm_ca_certificate
                    .as_ref()
                    .map(|secret| hyperswitch_masking::ExposeInterface::expose(secret.clone())),
            },
        );
    }
    config.proxy = domain_types::types::ProxyConfig {
        idle_pool_connection_timeout: hs_proxy.idle_pool_connection_timeout,
        // HS has no equivalent setting today (see the mapping this was
        // designed against) — left at whatever the loaded toml set, which
        // defaults to UCS's own DEFAULT_CONNECTOR_REQUEST_TIMEOUT_SECS.
        connector_request_timeout: config.proxy.connector_request_timeout,
        // NOTE: HS's `bypass_proxy_hosts` is a comma-separated list of HOSTS;
        // UCS's `bypass_urls` is matched as a full-URL exact string (see
        // `external_services::service::call_connector_api`). Passing hosts
        // through unconverted here would silently stop bypassing working —
        // left empty until that's resolved (either expand hosts to full
        // connector URLs here, or fix UCS to match by host).
        bypass_urls: Vec::new(),
        proxies,
    };

    Ok(config)
}

/// One `ConnectorClient` per distinct connector credential set, so repeated
/// calls for the same merchant connector account reuse the same instance
/// (and its pooled HTTP client) instead of rebuilding one per call.
#[derive(Default)]
pub struct LibraryConnectorService {
    clients: RwLock<HashMap<String, Arc<ConnectorClient>>>,
}

impl LibraryConnectorService {
    pub fn new() -> Self {
        Self::default()
    }

    fn client_for(
        &self,
        connector_auth_metadata: &ConnectorAuthMetadata,
    ) -> UnifiedConnectorServiceResult<Arc<ConnectorClient>> {
        // Cache key mirrors what actually changes the client's behavior: which
        // connector, and which credentials (merchant_id disambiguates two
        // merchants on the same connector with different credentials).
        let cache_key = format!(
            "{}:{}",
            connector_auth_metadata.connector_name,
            connector_auth_metadata.merchant_id.peek()
        );

        if let Some(client) = self
            .clients
            .read()
            .ok()
            .and_then(|cache| cache.get(&cache_key).cloned())
        {
            return Ok(client);
        }

        let connector_config = connector_config_from_auth_metadata(connector_auth_metadata)?;
        let client = Arc::new(ConnectorClient::new(connector_config, None).map_err(|error| {
            Report::new(UnifiedConnectorServiceError::ConnectionError(format!(
                "failed to construct library-mode ConnectorClient: {error:?}"
            )))
        })?);

        if let Ok(mut cache) = self.clients.write() {
            cache.insert(cache_key, client.clone());
        }
        Ok(client)
    }

    /// Performs Payment Authorize — mirrors
    /// `UnifiedConnectorServiceClient::payment_authorize`'s signature exactly,
    /// so a caller holding either transport behind `UcsTransport` (below)
    /// calls this the same way it would the gRPC client.
    pub async fn payment_authorize(
        &self,
        request: grpc_api_types::payments::PaymentServiceAuthorizeRequest,
        connector_auth_metadata: ConnectorAuthMetadata,
        grpc_headers: external_services::grpc_client::GrpcHeadersUcs,
    ) -> UnifiedConnectorServiceResult<
        tonic::Response<grpc_api_types::payments::PaymentServiceAuthorizeResponse>,
    > {
        let connector_name = connector_auth_metadata.connector_name.clone();
        let client = self.client_for(&connector_auth_metadata)?;
        let metadata = grpc_metadata_to_string_map(connector_auth_metadata, grpc_headers)?;

        let response = client
            .authorize(request, &metadata, None)
            .await
            .map_err(|error| sdk_error_to_ucs_error(&connector_name, "payment_authorize", error))?;

        Ok(tonic::Response::new(response))
    }

    // Every other flow (payment_get, payment_capture, payment_void,
    // payment_refund, refund_get, payout_*, frm_*, tokenize, ...) follows this
    // exact shape: client_for() + grpc_metadata_to_string_map() +
    // client.<flow>(request, &metadata, None) + sdk_error_to_ucs_error().
    // Left for a follow-up pass (mechanical, one per
    // `UnifiedConnectorServiceClient` method) rather than guessed here.
}

/// Converts `ConnectorAuthMetadata` + `GrpcHeadersUcs` into the plain
/// string-keyed metadata map `ConnectorClient`'s methods expect — by
/// building the exact same `tonic::metadata::MetadataMap` the gRPC transport
/// sends on the wire (via the existing, already-correct
/// `build_unified_connector_service_grpc_headers`) and converting that map's
/// entries to strings. This is the only place this crate re-derives anything
/// gRPC-mode already does, and it re-derives the *conversion*, not the
/// header-building logic itself.
fn grpc_metadata_to_string_map(
    connector_auth_metadata: ConnectorAuthMetadata,
    grpc_headers: external_services::grpc_client::GrpcHeadersUcs,
) -> UnifiedConnectorServiceResult<HashMap<String, String>> {
    let metadata_map =
        build_unified_connector_service_grpc_headers(connector_auth_metadata, grpc_headers)
            .map_err(|error| Report::new(error))?;

    Ok(metadata_map
        .iter()
        .filter_map(|entry| match entry {
            tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                value.to_str().ok().map(|v| (key.to_string(), v.to_string()))
            }
            tonic::metadata::KeyAndValueRef::Binary(_, _) => None,
        })
        .collect())
}

/// Builds the `ConnectorConfig` proto value `ConnectorClient::new` needs from
/// `ConnectorAuthMetadata`'s flat auth fields.
///
/// NOT YET IMPLEMENTED: `ConnectorSpecificConfig` is a `oneof` with a distinct
/// message per connector (`CybersourceConfig`, `AdyenConfig`, ... — over 100
/// variants, see `payment.proto`), each with its own field names for what
/// `ConnectorAuthMetadata` represents generically as `api_key`/`key1`/`key2`/
/// `api_secret`. Mapping every connector by hand here would mean guessing
/// field names I haven't verified per connector — exactly the kind of
/// unreliable code this crate should not ship. This needs either:
///   (a) a generated per-connector mapping (this codebase already generates
///       comparable per-flow/per-connector boilerplate elsewhere — see
///       `scripts/generators/code/generate.py` on the UCS side), or
///   (b) confirming whether UCS's grpc-server already exposes a generic
///       "auth fields -> ConnectorSpecificConfig" constructor by connector
///       name that this could call instead of reimplementing.
fn connector_config_from_auth_metadata(
    connector_auth_metadata: &ConnectorAuthMetadata,
) -> UnifiedConnectorServiceResult<grpc_api_types::payments::ConnectorConfig> {
    Err(Report::new(UnifiedConnectorServiceError::MissingRequiredField {
        field_name: std::borrow::Cow::Owned(format!(
            "connector_config_from_auth_metadata is not yet implemented for connector '{}' — see doc comment",
            connector_auth_metadata.connector_name
        )),
    }))
}

fn sdk_error_to_ucs_error(
    connector_name: &str,
    method: &str,
    error: SdkError,
) -> Report<UnifiedConnectorServiceError> {
    let debug_repr = format!("{error:?}");
    router_env::logger::error!(
        sdk_error = %debug_repr,
        method,
        connector_name,
        "UCS library-mode call failed"
    );
    match error {
        SdkError::IntegrationError { error_message, .. } => {
            Report::new(UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                error_message,
            ))
        }
        SdkError::NetworkError { .. } => {
            Report::new(UnifiedConnectorServiceError::ConnectionError(debug_repr))
        }
        _ => Report::new(UnifiedConnectorServiceError::ResponseDeserializationFailed),
    }
}
