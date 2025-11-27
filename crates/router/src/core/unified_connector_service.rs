use std::{str::FromStr, time::Instant};

use api_models::admin;
#[cfg(feature = "v2")]
use base64::Engine;
use common_enums::{
    connector_enums::Connector, AttemptStatus, CallConnectorAction, ConnectorIntegrationType,
    ExecutionMode, ExecutionPath, GatewaySystem, PaymentMethodType, ShadowRolloutAvailability,
    UcsAvailability,
};
#[cfg(feature = "v2")]
use common_utils::consts::BASE64_ENGINE;
use common_utils::{
    consts::X_FLOW_NAME,
    errors::CustomResult,
    ext_traits::ValueExt,
    id_type,
    request::{Method, RequestBuilder, RequestContent},
};
use diesel_models::types::FeatureMetadata;
use error_stack::ResultExt;
use external_services::{
    grpc_client::{
        unified_connector_service::{ConnectorAuthMetadata, UnifiedConnectorServiceError},
        LineageIds,
    },
    http_client,
};
use hyperswitch_connectors::utils::CardData;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::merchant_connector_account::{
    ExternalVaultConnectorMetadata, MerchantConnectorAccountTypeDetails,
};
use hyperswitch_domain_models::{
    platform::Platform,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds,
    router_request_types::RefundsData,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
};
use masking::{ExposeInterface, PeekInterface, Secret};
use router_env::{instrument, logger, tracing};
use unified_connector_service_cards::CardNumber;
use unified_connector_service_client::payments::{
    self as payments_grpc, payment_method::PaymentMethod, CardDetails, ClassicReward,
    CryptoCurrency, EVoucher, PaymentServiceAuthorizeResponse,
};

#[cfg(feature = "v2")]
use crate::types::api::enums as api_enums;
use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payments::{
            helpers::{
                self, is_ucs_enabled, should_execute_based_on_rollout,
                MerchantConnectorAccountType, ProxyOverride,
            },
            OperationSessionGetters, OperationSessionSetters,
        },
        utils::get_flow_name,
    },
    events::connector_api_logs::ConnectorEvent,
    headers::{CONTENT_TYPE, X_REQUEST_ID},
    routes::SessionState,
    types::{
        transformers::{ForeignFrom, ForeignTryFrom},
        UcsAuthorizeResponseData, UcsRepeatPaymentResponseData, UcsSetupMandateResponseData,
    },
};

pub mod transformers;

pub async fn get_access_token_from_ucs_response(
    session_state: &SessionState,
    platform: &Platform,
    connector_name: &str,
    merchant_connector_id: Option<&id_type::MerchantConnectorAccountId>,
    creds_identifier: Option<String>,
    ucs_state: Option<&unified_connector_service_client::payments::ConnectorState>,
) -> Option<AccessToken> {
    let ucs_access_token = ucs_state
        .and_then(|state| state.access_token.as_ref())
        .map(AccessToken::foreign_from)?;

    let merchant_id = platform.get_processor().get_account().get_id();

    let merchant_connector_id_or_connector_name = merchant_connector_id
        .map(|mca_id| mca_id.get_string_repr().to_string())
        .or(creds_identifier.map(|id| id.to_string()))
        .unwrap_or(connector_name.to_string());

    let key = common_utils::access_token::get_default_access_token_key(
        merchant_id,
        merchant_connector_id_or_connector_name,
    );

    if let Ok(Some(cached_token)) = session_state.store.get_access_token(key).await {
        if cached_token.token.peek() == ucs_access_token.token.peek() {
            return None;
        }
    }

    Some(ucs_access_token)
}

pub async fn set_access_token_for_ucs(
    state: &SessionState,
    platform: &Platform,
    connector_name: &str,
    access_token: AccessToken,
    merchant_connector_id: Option<&id_type::MerchantConnectorAccountId>,
    creds_identifier: Option<String>,
) -> Result<(), errors::StorageError> {
    let merchant_id = platform.get_processor().get_account().get_id();

    let merchant_connector_id_or_connector_name = merchant_connector_id
        .map(|mca_id| mca_id.get_string_repr().to_string())
        .or(creds_identifier.map(|id| id.to_string()))
        .unwrap_or(connector_name.to_string());

    let key = common_utils::access_token::get_default_access_token_key(
        merchant_id,
        &merchant_connector_id_or_connector_name,
    );

    let modified_access_token = AccessToken {
        expires: access_token
            .expires
            .saturating_sub(consts::REDUCE_ACCESS_TOKEN_EXPIRY_TIME.into()),
        ..access_token
    };

    logger::debug!(
        access_token_expiry_after_modification = modified_access_token.expires,
        merchant_id = ?merchant_id,
        connector_name = connector_name,
        merchant_connector_id_or_connector_name = merchant_connector_id_or_connector_name
    );

    if let Err(access_token_set_error) = state
        .store
        .set_access_token(key, modified_access_token)
        .await
    {
        // If we are not able to set the access token in redis, the error should just be logged and proceed with the payment
        // Payments should not fail, once the access token is successfully created
        // The next request will create new access token, if required
        logger::error!(access_token_set_error=?access_token_set_error, "Failed to store UCS access token");
    }

    Ok(())
}

// Re-export webhook transformer types for easier access
pub use transformers::{WebhookTransformData, WebhookTransformationStatus};

/// Type alias for return type used by unified connector service response handlers
type UnifiedConnectorServiceResult = CustomResult<
    (
        Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
        u16,
    ),
    UnifiedConnectorServiceError,
>;

/// Type alias for return type used by unified connector service refund response handlers
type UnifiedConnectorServiceRefundResult =
    CustomResult<(Result<RefundsResponseData, ErrorResponse>, u16), UnifiedConnectorServiceError>;

/// Checks if a config key exists and returns its percentage if present
/// Returns (key_exists, rollout_percentage)
async fn get_rollout_config_info(state: &SessionState, config_key: &str) -> (bool, Option<f64>) {
    let db = state.store.as_ref();

    match db.find_config_by_key(config_key).await {
        Ok(rollout_config) => {
            // Key exists, try to parse percentage
            let percentage =
                match serde_json::from_str::<helpers::RolloutConfig>(&rollout_config.config) {
                    Ok(config) => Some(config.rollout_percent),
                    Err(_) => {
                        // Fallback to legacy format (simple float)
                        rollout_config.config.parse::<f64>().ok()
                    }
                };
            (true, percentage)
        }
        Err(_) => (false, None), // Key doesn't exist
    }
}

/// Checks if the Unified Connector Service (UCS) is available for use
async fn check_ucs_availability(state: &SessionState) -> UcsAvailability {
    let is_client_available = state.grpc_client.unified_connector_service_client.is_some();

    let is_enabled = is_ucs_enabled(state, consts::UCS_ENABLED).await;

    match (is_client_available, is_enabled) {
        (true, true) => {
            router_env::logger::debug!("UCS is available and enabled");
            UcsAvailability::Enabled
        }
        _ => {
            router_env::logger::debug!(
                "UCS client is {} and UCS is {} in configuration",
                if is_client_available {
                    "available"
                } else {
                    "not available"
                },
                if is_enabled { "enabled" } else { "not enabled" }
            );
            UcsAvailability::Disabled
        }
    }
}

/// Determines the connector integration type based on UCS configuration or on both
async fn determine_connector_integration_type(
    state: &SessionState,
    connector: Connector,
    config_key: &str,
) -> RouterResult<ConnectorIntegrationType> {
    match state.conf.grpc_client.unified_connector_service.as_ref() {
        Some(ucs_config) => {
            let is_ucs_only = ucs_config.ucs_only_connectors.contains(&connector);
            let rollout_result = should_execute_based_on_rollout(state, config_key).await?;

            if is_ucs_only || rollout_result.should_execute {
                router_env::logger::debug!(
                    connector = ?connector,
                    ucs_only_list = is_ucs_only,
                    rollout_enabled = rollout_result.should_execute,
                    "Using UcsConnector"
                );
                Ok(ConnectorIntegrationType::UcsConnector)
            } else {
                router_env::logger::debug!(
                    connector = ?connector,
                    "Using DirectConnector - not in ucs_only_list and rollout not enabled"
                );
                Ok(ConnectorIntegrationType::DirectConnector)
            }
        }
        None => {
            router_env::logger::debug!(
                connector = ?connector,
                "UCS config not present, using DirectConnector"
            );
            Ok(ConnectorIntegrationType::DirectConnector)
        }
    }
}

pub async fn should_call_unified_connector_service<F: Clone, T, R, D>(
    state: &SessionState,
    platform: &Platform,
    router_data: &RouterData<F, T, R>,
    payment_data: Option<&D>,
    call_connector_action: CallConnectorAction,
    shadow_ucs_call_connector_action: Option<CallConnectorAction>,
) -> RouterResult<(ExecutionPath, SessionState)>
where
    D: OperationSessionGetters<F>,
    R: Send + Sync + Clone,
{
    // Extract context information
    let merchant_id = platform
        .get_processor()
        .get_account()
        .get_id()
        .get_string_repr();

    let connector_name = &router_data.connector;
    let connector_enum = Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
        .attach_printable_lazy(|| format!("Failed to parse connector name: {connector_name}"))?;

    let flow_name = get_flow_name::<F>()?;

    // Check UCS availability using idiomatic helper
    let ucs_availability = check_ucs_availability(state).await;

    let (rollout_key, shadow_rollout_key) = build_rollout_keys(
        merchant_id,
        connector_name,
        &flow_name,
        router_data.payment_method,
    );

    // Determine connector integration type
    let connector_integration_type =
        determine_connector_integration_type(state, connector_enum, &rollout_key).await?;

    // Extract previous gateway from payment data
    let previous_gateway = payment_data.and_then(extract_gateway_system_from_payment_intent);

    // Check rollout key availability and shadow key presence (optimized to reduce DB calls)
    let rollout_result = should_execute_based_on_rollout(state, &rollout_key).await?;
    let (shadow_key_exists, _shadow_percentage) =
        get_rollout_config_info(state, &shadow_rollout_key).await;

    // Simplified decision logic: Shadow takes priority, then rollout, then direct
    let shadow_rollout_availability = if shadow_key_exists {
        // Block 1: Shadow key exists - check if it's enabled
        let shadow_percentage = _shadow_percentage.unwrap_or(0.0);

        if shadow_percentage != 0.0 {
            router_env::logger::debug!( shadow_key = %shadow_rollout_key, shadow_percentage = shadow_percentage, "Shadow key enabled, using shadow mode for comparison" );
            ShadowRolloutAvailability::IsAvailable
        } else {
            router_env::logger::debug!(
                shadow_key = %shadow_rollout_key,
                shadow_percentage = shadow_percentage,
                rollout_enabled = rollout_result.should_execute,
                "Shadow key exists but disabled (0.0%), falling back to rollout or direct"
            );
            // Shadow disabled, result is the same regardless of rollout status
            ShadowRolloutAvailability::NotAvailable
        }
    } else if rollout_result.should_execute {
        // Block 2: No shadow key, but rollout is enabled - use primary UCS
        router_env::logger::debug!( rollout_key = %rollout_key, "No shadow key, rollout enabled, using primary UCS mode" );
        ShadowRolloutAvailability::NotAvailable
    } else {
        // Block 3: Neither shadow nor rollout enabled - use direct
        router_env::logger::debug!( rollout_key = %rollout_key, shadow_key = %shadow_rollout_key, "Neither shadow nor rollout enabled, using Direct mode" );
        ShadowRolloutAvailability::NotAvailable
    };

    // Single decision point using pattern matching
    let (gateway_system, execution_path) = if ucs_availability == UcsAvailability::Disabled {
        match call_connector_action {
            CallConnectorAction::UCSConsumeResponse(_)
            | CallConnectorAction::UCSHandleResponse(_) => {
                Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("CallConnectorAction UCSHandleResponse/UCSConsumeResponse received but UCS is disabled. These actions are only valid in UCS gateway")?
            }
            CallConnectorAction::Avoid
            | CallConnectorAction::Trigger
            | CallConnectorAction::HandleResponse(_)
            | CallConnectorAction::StatusUpdate { .. } => {
                router_env::logger::debug!("UCS is disabled, using Direct gateway");
                (GatewaySystem::Direct, ExecutionPath::Direct)
            }
        }
    } else {
        match call_connector_action {
            CallConnectorAction::UCSConsumeResponse(_)
            | CallConnectorAction::UCSHandleResponse(_) => {
                router_env::logger::info!("CallConnectorAction UCSHandleResponse/UCSConsumeResponse received, using UCS gateway");
                (
                    GatewaySystem::UnifiedConnectorService,
                    ExecutionPath::UnifiedConnectorService,
                )
            }
            CallConnectorAction::HandleResponse(_) => {
                router_env::logger::info!(
                    "CallConnectorAction HandleResponse received, using Direct gateway"
                );
                if shadow_ucs_call_connector_action.is_some() {
                    (
                        GatewaySystem::Direct,
                        ExecutionPath::ShadowUnifiedConnectorService,
                    )
                } else {
                    (GatewaySystem::Direct, ExecutionPath::Direct)
                }
            }
            CallConnectorAction::Trigger
            | CallConnectorAction::Avoid
            | CallConnectorAction::StatusUpdate { .. } => {
                // UCS is enabled, call decide function
                decide_execution_path(
                    connector_integration_type,
                    previous_gateway,
                    shadow_rollout_availability,
                )?
            }
        }
    };

    router_env::logger::info!(
        "Payment gateway decision: gateway={:?}, execution_path={:?} - merchant_id={}, connector={}, flow={}",
        gateway_system,
        execution_path,
        merchant_id,
        connector_name,
        flow_name
    );

    // Handle proxy configuration for Shadow UCS flows
    let session_state = match execution_path {
        ExecutionPath::ShadowUnifiedConnectorService => {
            // For shadow UCS, use rollout_result for proxy configuration since it takes priority
            match &rollout_result.proxy_override {
                Some(proxy_override) => {
                    router_env::logger::debug!(
                        proxy_override = ?proxy_override,
                        "Creating updated session state with proxy configuration for Shadow UCS"
                    );
                    create_updated_session_state_with_proxy(state.clone(), proxy_override)
                }
                None => {
                    router_env::logger::debug!(
                        "No proxy override available for Shadow UCS, using original state"
                    );
                    state.clone()
                }
            }
        }
        _ => {
            // For Direct and UCS flows, use original state
            state.clone()
        }
    };

    Ok((execution_path, session_state))
}

/// Creates a new SessionState with proxy configuration updated from the override
fn create_updated_session_state_with_proxy(
    state: SessionState,
    proxy_override: &ProxyOverride,
) -> SessionState {
    let mut updated_state = state;

    // Create updated configuration with proxy overrides
    let mut updated_conf = (*updated_state.conf).clone();

    // Update proxy URLs with overrides, falling back to existing values
    if let Some(ref http_url) = proxy_override.http_url {
        updated_conf.proxy.http_url = Some(http_url.clone());
    }
    if let Some(ref https_url) = proxy_override.https_url {
        updated_conf.proxy.https_url = Some(https_url.clone());
    }

    updated_state.conf = std::sync::Arc::new(updated_conf);

    updated_state
}

fn decide_execution_path(
    connector_type: ConnectorIntegrationType,
    previous_gateway: Option<GatewaySystem>,
    shadow_rollout_enabled: ShadowRolloutAvailability,
) -> RouterResult<(GatewaySystem, ExecutionPath)> {
    match (connector_type, previous_gateway, shadow_rollout_enabled) {
        // Case 1: DirectConnector with no previous gateway and no shadow rollout
        // This is a fresh payment request for a direct connector - use direct gateway
        (
            ConnectorIntegrationType::DirectConnector,
            None,
            ShadowRolloutAvailability::NotAvailable,
        ) => Ok((GatewaySystem::Direct, ExecutionPath::Direct)),

        // Case 2: DirectConnector previously used Direct gateway, no shadow rollout
        // Continue using the same direct gateway for consistency
        (
            ConnectorIntegrationType::DirectConnector,
            Some(GatewaySystem::Direct),
            ShadowRolloutAvailability::NotAvailable,
        ) => Ok((GatewaySystem::Direct, ExecutionPath::Direct)),

        // Case 3: DirectConnector previously used UCS, but now switching back to Direct (no shadow)
        // Migration scenario: UCS was used before, but now we're reverting to Direct
        (
            ConnectorIntegrationType::DirectConnector,
            Some(GatewaySystem::UnifiedConnectorService),
            ShadowRolloutAvailability::NotAvailable,
        ) => Ok((GatewaySystem::Direct, ExecutionPath::Direct)),

        // Case 4: UcsConnector configuration, but previously used Direct gateway (no shadow)
        // Maintain Direct for backward compatibility - don't switch mid-transaction
        (
            ConnectorIntegrationType::UcsConnector,
            Some(GatewaySystem::Direct),
            ShadowRolloutAvailability::NotAvailable,
        ) => Ok((GatewaySystem::Direct, ExecutionPath::Direct)),

        // Case 5: DirectConnector with no previous gateway, shadow rollout enabled
        // Use Direct as primary, but also execute UCS in shadow mode for comparison
        (
            ConnectorIntegrationType::DirectConnector,
            None,
            ShadowRolloutAvailability::IsAvailable,
        ) => Ok((
            GatewaySystem::Direct,
            ExecutionPath::ShadowUnifiedConnectorService,
        )),

        // Case 6: DirectConnector previously used Direct, shadow rollout enabled
        // Continue with Direct as primary, execute UCS in shadow mode for testing
        (
            ConnectorIntegrationType::DirectConnector,
            Some(GatewaySystem::Direct),
            ShadowRolloutAvailability::IsAvailable,
        ) => Ok((
            GatewaySystem::Direct,
            ExecutionPath::ShadowUnifiedConnectorService,
        )),

        // Case 7: DirectConnector previously used UCS, shadow rollout enabled
        // Revert to Direct as primary, but keep UCS in shadow mode for comparison
        (
            ConnectorIntegrationType::DirectConnector,
            Some(GatewaySystem::UnifiedConnectorService),
            ShadowRolloutAvailability::IsAvailable,
        ) => Ok((
            GatewaySystem::Direct,
            ExecutionPath::ShadowUnifiedConnectorService,
        )),

        // Case 8: UcsConnector configuration, previously used Direct, shadow rollout enabled
        // Maintain Direct as primary for transaction consistency, shadow UCS for testing
        (
            ConnectorIntegrationType::UcsConnector,
            Some(GatewaySystem::Direct),
            ShadowRolloutAvailability::IsAvailable,
        ) => Ok((
            GatewaySystem::Direct,
            ExecutionPath::ShadowUnifiedConnectorService,
        )),

        // Case 9a: UcsConnector with no previous gateway and shadow rollout enabled
        // Fresh payment for UCS-enabled connector with shadow mode - use shadow UCS
        (ConnectorIntegrationType::UcsConnector, None, ShadowRolloutAvailability::IsAvailable) => {
            Ok((
                GatewaySystem::Direct,
                ExecutionPath::ShadowUnifiedConnectorService,
            ))
        }

        // Case 9b: UcsConnector with no previous gateway and no shadow rollout
        // Fresh payment for a UCS-enabled connector - use UCS as primary
        (ConnectorIntegrationType::UcsConnector, None, ShadowRolloutAvailability::NotAvailable) => {
            Ok((
                GatewaySystem::UnifiedConnectorService,
                ExecutionPath::UnifiedConnectorService,
            ))
        }

        // Case 10: UcsConnector previously used UCS (regardless of shadow rollout)
        // Continue using UCS for consistency in the payment flow
        (
            ConnectorIntegrationType::UcsConnector,
            Some(GatewaySystem::UnifiedConnectorService),
            _,
        ) => Ok((
            GatewaySystem::UnifiedConnectorService,
            ExecutionPath::UnifiedConnectorService,
        )),
    }
}

/// Build rollout keys based on flow type - include payment method for payments, skip for refunds
fn build_rollout_keys(
    merchant_id: &str,
    connector_name: &str,
    flow_name: &str,
    payment_method: common_enums::PaymentMethod,
) -> (String, String) {
    // Detect if this is a refund flow based on flow name
    let is_refund_flow = matches!(flow_name, "Execute" | "RSync");

    let rollout_key = if is_refund_flow {
        // Refund flows: UCS_merchant_connector_flow (e.g., UCS_merchant123_stripe_Execute)
        format!(
            "{}_{}_{}_{}",
            consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
            merchant_id,
            connector_name,
            flow_name
        )
    } else {
        // Payment flows: UCS_merchant_connector_paymentmethod_flow (e.g., UCS_merchant123_stripe_card_Authorize)
        let payment_method_str = payment_method.to_string();
        format!(
            "{}_{}_{}_{}_{}",
            consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
            merchant_id,
            connector_name,
            payment_method_str,
            flow_name
        )
    };

    let shadow_rollout_key = format!("{rollout_key}_shadow");
    (rollout_key, shadow_rollout_key)
}

/// Extracts the gateway system from the payment intent's feature metadata
/// Returns None if metadata is missing, corrupted, or doesn't contain gateway_system
fn extract_gateway_system_from_payment_intent<F: Clone, D>(
    payment_data: &D,
) -> Option<GatewaySystem>
where
    D: OperationSessionGetters<F>,
{
    #[cfg(feature = "v1")]
    {
        payment_data
            .get_payment_intent()
            .feature_metadata
            .as_ref()
            .and_then(|metadata| {
                // Try to parse the JSON value as FeatureMetadata
                // Log errors but don't fail the flow for corrupted metadata
                match serde_json::from_value::<FeatureMetadata>(metadata.clone()) {
                    Ok(feature_metadata) => feature_metadata.gateway_system,
                    Err(err) => {
                        router_env::logger::warn!(
                            "Failed to parse feature_metadata for gateway_system extraction: {}",
                            err
                        );
                        None
                    }
                }
            })
    }
    #[cfg(feature = "v2")]
    {
        None // V2 does not use feature metadata for gateway system tracking
    }
}

/// Updates the payment intent's feature metadata to track the gateway system being used
#[cfg(feature = "v1")]
pub fn update_gateway_system_in_feature_metadata<F: Clone, D>(
    payment_data: &mut D,
    gateway_system: GatewaySystem,
) -> RouterResult<()>
where
    D: OperationSessionGetters<F> + OperationSessionSetters<F>,
{
    let mut payment_intent = payment_data.get_payment_intent().clone();

    let existing_metadata = payment_intent.feature_metadata.as_ref();

    let mut feature_metadata = existing_metadata
        .and_then(|metadata| serde_json::from_value::<FeatureMetadata>(metadata.clone()).ok())
        .unwrap_or_default();

    feature_metadata.gateway_system = Some(gateway_system);

    let updated_metadata = serde_json::to_value(feature_metadata)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize feature metadata")?;

    payment_intent.feature_metadata = Some(updated_metadata.clone());
    payment_data.set_payment_intent(payment_intent);

    Ok(())
}

pub async fn should_call_unified_connector_service_for_webhooks(
    state: &SessionState,
    platform: &Platform,
    connector_name: &str,
) -> RouterResult<ExecutionPath> {
    // Extract context information
    let merchant_id = platform
        .get_processor()
        .get_account()
        .get_id()
        .get_string_repr();

    let connector_enum = Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
        .attach_printable_lazy(|| format!("Failed to parse connector name: {}", connector_name))?;

    let flow_name = "Webhooks";

    // Check UCS availability using idiomatic helper
    let ucs_availability = check_ucs_availability(state).await;

    // Build rollout keys - webhooks don't use payment method, so use a simplified key format
    let rollout_key = format!(
        "{}_{}_{}_{}",
        consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
        merchant_id,
        connector_name,
        flow_name
    );
    let shadow_rollout_key = format!("{rollout_key}_shadow");

    // Determine connector integration type
    let connector_integration_type =
        determine_connector_integration_type(state, connector_enum, &rollout_key).await?;

    // For webhooks, there is no previous gateway system to consider (webhooks are stateless)
    let previous_gateway = None;

    // Check both rollout keys to determine priority based on shadow percentage
    let rollout_result = should_execute_based_on_rollout(state, &rollout_key).await?;
    let shadow_rollout_result = should_execute_based_on_rollout(state, &shadow_rollout_key).await?;

    // Get shadow percentage to determine priority
    let (_shadow_key_exists, shadow_percentage) =
        get_rollout_config_info(state, &shadow_rollout_key).await;

    let shadow_rollout_availability =
        if shadow_rollout_result.should_execute && shadow_percentage.unwrap_or(0.0) != 0.0 {
            // Shadow is present and percentage is non-zero, use shadow
            router_env::logger::debug!(
                shadow_percentage = shadow_percentage.unwrap_or(0.0),
                "Shadow rollout is present with non-zero percentage for webhooks, using shadow"
            );
            ShadowRolloutAvailability::IsAvailable
        } else if rollout_result.should_execute {
            // Either shadow is 0.0 or not present, use rollout if available
            router_env::logger::debug!(
                shadow_percentage = shadow_percentage.unwrap_or(0.0),
                "Shadow rollout is 0.0 or not present for webhooks, using rollout"
            );
            ShadowRolloutAvailability::IsAvailable
        } else {
            ShadowRolloutAvailability::NotAvailable
        };

    // Use the same decision logic as payments, with no call_connector_action to consider
    let (gateway_system, execution_path) = if ucs_availability == UcsAvailability::Disabled {
        router_env::logger::debug!("UCS is disabled for webhooks, using Direct gateway");
        (GatewaySystem::Direct, ExecutionPath::Direct)
    } else {
        // UCS is enabled, use decide function with no previous gateway for webhooks
        decide_execution_path(
            connector_integration_type,
            previous_gateway,
            shadow_rollout_availability,
        )?
    };

    router_env::logger::info!(
        "Webhook gateway decision: gateway={:?}, execution_path={:?} - merchant_id={}, connector={}, flow={}",
        gateway_system,
        execution_path,
        merchant_id,
        connector_name,
        flow_name
    );

    Ok(execution_path)
}

pub fn build_unified_connector_service_payment_method(
    payment_method_data: hyperswitch_domain_models::payment_method_data::PaymentMethodData,
    payment_method_type: Option<PaymentMethodType>,
) -> CustomResult<payments_grpc::PaymentMethod, UnifiedConnectorServiceError> {
    match payment_method_data {
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Card(card) => {
            let card_exp_month = card
                .get_card_expiry_month_2_digit()
                .attach_printable("Failed to extract 2-digit expiry month from card")
                .change_context(UnifiedConnectorServiceError::InvalidDataFormat {
                    field_name: "card_exp_month",
                })?
                .peek()
                .to_string();

            let card_network = card
                .card_network
                .clone()
                .map(payments_grpc::CardNetwork::foreign_try_from)
                .transpose()?;

            let card_details = CardDetails {
                card_number: Some(
                    CardNumber::from_str(&card.card_number.get_card_no()).change_context(
                        UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                            "Failed to parse card number".to_string(),
                        ),
                    )?,
                ),
                card_exp_month: Some(card_exp_month.into()),
                card_exp_year: Some(card.card_exp_year.expose().into()),
                card_cvc: Some(card.card_cvc.expose().into()),
                card_holder_name: card.card_holder_name.map(|name| name.expose().into()),
                card_issuer: card.card_issuer.clone(),
                card_network: card_network.map(|card_network| card_network.into()),
                card_type: card.card_type.clone(),
                bank_code: card.bank_code.clone(),
                nick_name: card.nick_name.map(|n| n.expose()),
                card_issuing_country_alpha2: card.card_issuing_country.clone(),
            };

            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::Card(card_details)),
            })
        }
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Upi(upi_data) => {
            let upi_type = match upi_data {
                hyperswitch_domain_models::payment_method_data::UpiData::UpiCollect(
                    upi_collect_data,
                ) => {
                    let upi_details = payments_grpc::UpiCollect {
                        vpa_id: upi_collect_data.vpa_id.map(|vpa| vpa.expose().into()),
                    };
                    PaymentMethod::UpiCollect(upi_details)
                }
                hyperswitch_domain_models::payment_method_data::UpiData::UpiIntent(_) => {
                    let upi_details = payments_grpc::UpiIntent { app_name: None };
                    PaymentMethod::UpiIntent(upi_details)
                }
                hyperswitch_domain_models::payment_method_data::UpiData::UpiQr(_) => {
                    let upi_details = payments_grpc::UpiQr {};
                    PaymentMethod::UpiQr(upi_details)
                }
            };

            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(upi_type),
            })
        }
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankRedirect(
            bank_redirect_data,
        ) => match bank_redirect_data {
            hyperswitch_domain_models::payment_method_data::BankRedirectData::OpenBankingUk {
                issuer,
                country,
            } => {
                let open_banking_uk = payments_grpc::OpenBankingUk {
                    issuer: issuer.map(|issuer| issuer.to_string()),
                    country: country.map(|country| country.to_string()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::OpenBankingUk(open_banking_uk)),
                })
            }
            _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                "Unimplemented bank redirect type: {bank_redirect_data:?}"
            ))
            .into()),
        },
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Reward => {
            match payment_method_type {
                Some(PaymentMethodType::ClassicReward) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::ClassicReward(ClassicReward {})),
                }),
                Some(PaymentMethodType::Evoucher) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::EVoucher(EVoucher {})),
                }),
                None | Some(_) => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                    "Unimplemented payment method subtype: {payment_method_type:?}"
                ))
                .into()),
            }
        }
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Wallet(wallet_data) => {
            match wallet_data {
                hyperswitch_domain_models::payment_method_data::WalletData::Mifinity(
                    mifinity_data,
                ) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Mifinity(payments_grpc::MifinityWallet {
                        date_of_birth: Some(mifinity_data.date_of_birth.peek().to_string().into()),
                        language_preference: mifinity_data.language_preference,
                    })),
                }),
                hyperswitch_domain_models::payment_method_data::WalletData::ApplePay(
                    apple_pay_wallet_data
                ) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::ApplePay(payments_grpc::AppleWallet {
                        payment_data: Some(payments_grpc::apple_wallet::PaymentData {
                            payment_data: Some(payments_grpc::apple_wallet::payment_data::PaymentData::foreign_try_from(&apple_pay_wallet_data.payment_data)?),
                        }),
                        payment_method: Some(payments_grpc::apple_wallet::PaymentMethod {
                            display_name: apple_pay_wallet_data.payment_method.display_name,
                            network: apple_pay_wallet_data.payment_method.network,
                            r#type: apple_pay_wallet_data.payment_method.pm_type,
                        }),
                        transaction_identifier: apple_pay_wallet_data.transaction_identifier,
                    })),
                }),
                hyperswitch_domain_models::payment_method_data::WalletData::GooglePay(
                    google_pay_wallet_data,
                ) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::GooglePay(payments_grpc::GoogleWallet {
                        r#type: google_pay_wallet_data.pm_type,
                        description: google_pay_wallet_data.description,
                        info: Some(payments_grpc::google_wallet::PaymentMethodInfo {
                            card_network: google_pay_wallet_data.info.card_network,
                            card_details: google_pay_wallet_data.info.card_details,
                            assurance_details: google_pay_wallet_data.info.assurance_details.map(|details| {
                                payments_grpc::google_wallet::payment_method_info::AssuranceDetails {
                                    card_holder_authenticated: details.card_holder_authenticated,
                                    account_verified: details.account_verified,
                                }
                            }),
                        }),
                        tokenization_data: Some(payments_grpc::google_wallet::TokenizationData {
                            tokenization_data: Some(payments_grpc::google_wallet::tokenization_data::TokenizationData::foreign_try_from(&google_pay_wallet_data.tokenization_data)?),
                        }),
                    })),
                }),
                _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                    "Unimplemented payment method subtype: {payment_method_type:?}"
                ))
                .into()),
            }
        }
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Crypto(crypto_data) => {
            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::Crypto(CryptoCurrency {
                    pay_currency: crypto_data.pay_currency.clone(),
                    network: crypto_data.network.clone(),
                })),
            })
        }
        _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
            "Unimplemented payment method: {payment_method_data:?}"
        ))
        .into()),
    }
}

pub fn build_unified_connector_service_payment_method_for_external_proxy(
    payment_method_data: hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData,
    payment_method_type: Option<PaymentMethodType>,
) -> CustomResult<payments_grpc::PaymentMethod, UnifiedConnectorServiceError> {
    match payment_method_data {
        hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::Card(
            external_vault_card,
        ) => {
            let card_network = external_vault_card
                .card_network
                .clone()
                .map(payments_grpc::CardNetwork::foreign_try_from)
                .transpose()?;
            let card_details = CardDetails {
                card_number: Some(CardNumber::from_str(external_vault_card.card_number.peek()).change_context(
                    UnifiedConnectorServiceError::RequestEncodingFailedWithReason("Failed to parse card number".to_string())
                )?),
                card_exp_month: Some(external_vault_card.card_exp_month.expose().into()),
                card_exp_year: Some(external_vault_card.card_exp_year.expose().into()),
                card_cvc: Some(external_vault_card.card_cvc.expose().into()),
                card_holder_name: external_vault_card.card_holder_name.map(|name| name.expose().into()),
                card_issuer: external_vault_card.card_issuer.clone(),
                card_network: card_network.map(|card_network| card_network.into()),
                card_type: external_vault_card.card_type.clone(),
                bank_code: external_vault_card.bank_code.clone(),
                nick_name: external_vault_card.nick_name.map(|n| n.expose()),
                card_issuing_country_alpha2: external_vault_card.card_issuing_country.clone(),
            };
            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::CardProxy(card_details)),
            })
        }
        hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData::VaultToken(_) => {
            Err(UnifiedConnectorServiceError::NotImplemented(format!(
                        "Unimplemented payment method subtype: {payment_method_type:?}"
            ))
            .into())
        }
    }
}

/// Gets the UCS client from session state
fn get_ucs_client(
    state: &SessionState,
) -> RouterResult<
    &external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceClient,
> {
    state
        .grpc_client
        .unified_connector_service_client
        .as_ref()
        .ok_or_else(|| {
            error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("UCS client is not available")
        })
}

pub fn build_unified_connector_service_auth_metadata(
    #[cfg(feature = "v1")] merchant_connector_account: MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: MerchantConnectorAccountTypeDetails,
    platform: &Platform,
) -> CustomResult<ConnectorAuthMetadata, UnifiedConnectorServiceError> {
    #[cfg(feature = "v1")]
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(UnifiedConnectorServiceError::FailedToObtainAuthType)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    #[cfg(feature = "v2")]
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .change_context(UnifiedConnectorServiceError::FailedToObtainAuthType)
        .attach_printable("Failed to obtain ConnectorAuthType")?;

    let connector_name = {
        #[cfg(feature = "v1")]
        {
            merchant_connector_account
                .get_connector_name()
                .ok_or(UnifiedConnectorServiceError::MissingConnectorName)
                .attach_printable("Missing connector name")?
        }

        #[cfg(feature = "v2")]
        {
            merchant_connector_account
                .get_connector_name()
                .map(|connector| connector.to_string())
                .ok_or(UnifiedConnectorServiceError::MissingConnectorName)
                .attach_printable("Missing connector name")?
        }
    };

    let merchant_id = platform
        .get_processor()
        .get_account()
        .get_id()
        .get_string_repr();

    match &auth_type {
        ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_SIGNATURE_KEY.to_string(),
            api_key: Some(api_key.clone()),
            key1: Some(key1.clone()),
            key2: None,
            api_secret: Some(api_secret.clone()),
            auth_key_map: None,
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        ConnectorAuthType::BodyKey { api_key, key1 } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_BODY_KEY.to_string(),
            api_key: Some(api_key.clone()),
            key1: Some(key1.clone()),
            key2: None,
            api_secret: None,
            auth_key_map: None,
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        ConnectorAuthType::HeaderKey { api_key } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_HEADER_KEY.to_string(),
            api_key: Some(api_key.clone()),
            key1: None,
            key2: None,
            api_secret: None,
            auth_key_map: None,
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        ConnectorAuthType::CurrencyAuthKey { auth_key_map } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_CURRENCY_AUTH_KEY.to_string(),
            api_key: None,
            key1: None,
            key2: None,
            api_secret: None,
            auth_key_map: Some(auth_key_map.clone()),
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        ConnectorAuthType::MultiAuthKey {
            api_key,
            key1,
            api_secret,
            key2,
        } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_MULTI_KEY.to_string(),
            api_key: Some(api_key.clone()),
            key1: Some(key1.clone()),
            key2: Some(key2.clone()),
            api_secret: Some(api_secret.clone()),
            auth_key_map: None,
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        _ => Err(UnifiedConnectorServiceError::FailedToObtainAuthType)
            .attach_printable("Unsupported ConnectorAuthType for header injection"),
    }
}

#[cfg(feature = "v2")]
pub fn build_unified_connector_service_external_vault_proxy_metadata(
    external_vault_merchant_connector_account: MerchantConnectorAccountTypeDetails,
) -> CustomResult<String, UnifiedConnectorServiceError> {
    let external_vault_metadata = external_vault_merchant_connector_account
        .get_metadata()
        .ok_or(UnifiedConnectorServiceError::ParsingFailed)
        .attach_printable("Failed to obtain ConnectorMetadata")?;

    let connector_name = external_vault_merchant_connector_account
        .get_connector_name()
        .map(|connector| connector.to_string())
        .ok_or(UnifiedConnectorServiceError::MissingConnectorName)
        .attach_printable("Missing connector name")?; // always get the connector name from this call

    let external_vault_connector = api_enums::VaultConnectors::from_str(&connector_name)
        .change_context(UnifiedConnectorServiceError::InvalidConnectorName)
        .attach_printable("Failed to parse Vault connector")?;

    let unified_service_vault_metdata = match external_vault_connector {
        api_enums::VaultConnectors::Vgs => {
            let vgs_metadata: ExternalVaultConnectorMetadata = external_vault_metadata
                .expose()
                .parse_value("ExternalVaultConnectorMetadata")
                .change_context(UnifiedConnectorServiceError::ParsingFailed)
                .attach_printable("Failed to parse Vgs connector metadata")?;

            Some(external_services::grpc_client::unified_connector_service::ExternalVaultProxyMetadata::VgsMetadata(
                external_services::grpc_client::unified_connector_service::VgsMetadata {
                    proxy_url: vgs_metadata.proxy_url,
                    certificate: vgs_metadata.certificate,
                }
            ))
        }
        api_enums::VaultConnectors::HyperswitchVault | api_enums::VaultConnectors::Tokenex => None,
    };

    match unified_service_vault_metdata {
        Some(metdata) => {
            let external_vault_metadata_bytes = serde_json::to_vec(&metdata)
                .change_context(UnifiedConnectorServiceError::ParsingFailed)
                .attach_printable("Failed to convert External vault metadata to bytes")?;

            Ok(BASE64_ENGINE.encode(&external_vault_metadata_bytes))
        }
        None => Err(UnifiedConnectorServiceError::NotImplemented(
            "External vault proxy metadata is not supported for {connector_name}".to_string(),
        )
        .into()),
    }
}

pub fn handle_unified_connector_service_response_for_payment_authorize(
    response: PaymentServiceAuthorizeResponse,
) -> CustomResult<UcsAuthorizeResponseData, UnifiedConnectorServiceError> {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(
            response.clone(),
        )?;

    let connector_customer_id =
        extract_connector_customer_id_from_ucs_state(response.state.as_ref());
    let connector_response =
        extract_connector_response_from_ucs(response.connector_response.as_ref());

    Ok(UcsAuthorizeResponseData {
        router_data_response,
        status_code,
        connector_customer_id,
        connector_response,
    })
}

pub fn handle_unified_connector_service_response_for_create_connector_customer(
    response: payments_grpc::PaymentServiceCreateConnectorCustomerResponse,
) -> CustomResult<(Result<PaymentsResponseData, ErrorResponse>, u16), UnifiedConnectorServiceError>
{
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let connector_customer_result =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((connector_customer_result, status_code))
}

pub fn handle_unified_connector_service_response_for_create_order(
    response: payments_grpc::PaymentServiceCreateOrderResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_post_authenticate(
    response: payments_grpc::PaymentServicePostAuthenticateResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_method_token_create(
    response: payments_grpc::PaymentServiceCreatePaymentMethodTokenResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_authenticate(
    response: payments_grpc::PaymentServiceAuthenticateResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_pre_authenticate(
    response: payments_grpc::PaymentServicePreAuthenticateResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_capture(
    response: payments_grpc::PaymentServiceCaptureResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_register(
    response: payments_grpc::PaymentServiceRegisterResponse,
) -> CustomResult<UcsSetupMandateResponseData, UnifiedConnectorServiceError> {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(
            response.clone(),
        )?;

    let connector_customer_id =
        extract_connector_customer_id_from_ucs_state(response.state.as_ref());

    Ok(UcsSetupMandateResponseData {
        router_data_response,
        status_code,
        connector_customer_id,
    })
}

pub fn handle_unified_connector_service_response_for_session_token_create(
    response: payments_grpc::PaymentServiceCreateSessionTokenResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_repeat(
    response: payments_grpc::PaymentServiceRepeatEverythingResponse,
) -> CustomResult<UcsRepeatPaymentResponseData, UnifiedConnectorServiceError> {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(
            response.clone(),
        )?;

    let connector_customer_id =
        extract_connector_customer_id_from_ucs_state(response.state.as_ref());
    let connector_response =
        extract_connector_response_from_ucs(response.connector_response.as_ref());

    Ok(UcsRepeatPaymentResponseData {
        router_data_response,
        status_code,
        connector_customer_id,
        connector_response,
    })
}

/// Extracts connector_customer_id from UCS state
pub fn extract_connector_customer_id_from_ucs_state(
    ucs_state: Option<&payments_grpc::ConnectorState>,
) -> Option<String> {
    ucs_state.and_then(|state| {
        state
            .connector_customer_id
            .as_ref()
            .map(|id| id.to_string())
    })
}

/// Extracts connector_response from UCS response
pub fn extract_connector_response_from_ucs(
    connector_response: Option<&payments_grpc::ConnectorResponseData>,
) -> Option<hyperswitch_domain_models::router_data::ConnectorResponseData> {
    connector_response.and_then(|data| {
        <hyperswitch_domain_models::router_data::ConnectorResponseData as hyperswitch_interfaces::helpers::ForeignTryFrom<payments_grpc::ConnectorResponseData>>::foreign_try_from(data.clone())
            .map_err(|e| {
                logger::warn!(
                    error=?e,
                    "Failed to deserialize connector_response from UCS"
                );
                e
            })
            .ok()
    })
}

pub fn handle_unified_connector_service_response_for_refund_execute(
    response: payments_grpc::RefundResponse,
) -> UnifiedConnectorServiceRefundResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response: Result<RefundsResponseData, ErrorResponse> =
        Result::<RefundsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_refund_sync(
    response: payments_grpc::RefundResponse,
) -> UnifiedConnectorServiceRefundResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<RefundsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_cancel(
    response: payments_grpc::PaymentServiceVoidResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

/// Handles the unified connector service response for create access token
pub fn handle_unified_connector_service_response_for_create_access_token(
    response: payments_grpc::PaymentServiceCreateAccessTokenResponse,
) -> CustomResult<(Result<AccessToken, ErrorResponse>, u16), UnifiedConnectorServiceError> {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let access_token_result = Result::<AccessToken, ErrorResponse>::foreign_try_from(response)?;

    Ok((access_token_result, status_code))
}

pub fn build_webhook_secrets_from_merchant_connector_account(
    #[cfg(feature = "v1")] merchant_connector_account: &MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: &MerchantConnectorAccountTypeDetails,
) -> CustomResult<Option<payments_grpc::WebhookSecrets>, UnifiedConnectorServiceError> {
    // Extract webhook credentials from merchant connector account
    // This depends on how webhook secrets are stored in the merchant connector account

    #[cfg(feature = "v1")]
    let webhook_details = merchant_connector_account
        .get_webhook_details()
        .map_err(|_| UnifiedConnectorServiceError::FailedToObtainAuthType)?;

    #[cfg(feature = "v2")]
    let webhook_details = match merchant_connector_account {
        MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(mca) => {
            mca.connector_webhook_details.as_ref()
        }
        MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => None,
    };

    match webhook_details {
        Some(details) => {
            // Parse the webhook details JSON to extract secrets
            let webhook_details: admin::MerchantConnectorWebhookDetails = details
                .clone()
                .parse_value("MerchantConnectorWebhookDetails")
                .change_context(UnifiedConnectorServiceError::FailedToObtainAuthType)
                .attach_printable("Failed to parse MerchantConnectorWebhookDetails")?;

            // Build gRPC WebhookSecrets from parsed details
            Ok(Some(payments_grpc::WebhookSecrets {
                secret: webhook_details.merchant_secret.expose().to_string(),
                additional_secret: webhook_details
                    .additional_secret
                    .map(|secret| secret.expose().to_string()),
            }))
        }
        None => Ok(None),
    }
}

/// High-level abstraction for calling UCS webhook transformation
/// This provides a clean interface similar to payment flow UCS calls
pub async fn call_unified_connector_service_for_webhook(
    state: &SessionState,
    platform: &Platform,
    connector_name: &str,
    body: &actix_web::web::Bytes,
    request_details: &hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails<'_>,
    merchant_connector_account: Option<
        &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    >,
) -> RouterResult<(
    api_models::webhooks::IncomingWebhookEvent,
    bool,
    WebhookTransformData,
)> {
    let ucs_client = state
        .grpc_client
        .unified_connector_service_client
        .as_ref()
        .ok_or_else(|| {
            error_stack::report!(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("UCS client is not available for webhook processing")
        })?;

    // Build webhook secrets from merchant connector account
    let webhook_secrets = merchant_connector_account.and_then(|mca| {
        #[cfg(feature = "v1")]
        let mca_type = MerchantConnectorAccountType::DbVal(Box::new(mca.clone()));
        #[cfg(feature = "v2")]
        let mca_type =
            MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(mca.clone()));

        build_webhook_secrets_from_merchant_connector_account(&mca_type)
            .map_err(|e| {
                logger::warn!(
                    build_error=?e,
                    connector_name=connector_name,
                    "Failed to build webhook secrets from merchant connector account in call_unified_connector_service_for_webhook"
                );
                e
            })
            .ok()
            .flatten()
    });

    // Build UCS transform request using new webhook transformers
    let transform_request = transformers::build_webhook_transform_request(
        body,
        request_details,
        webhook_secrets,
        platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
        connector_name,
    )?;

    // Build connector auth metadata
    let connector_auth_metadata = merchant_connector_account
        .map(|mca| {
            #[cfg(feature = "v1")]
            let mca_type = MerchantConnectorAccountType::DbVal(Box::new(mca.clone()));
            #[cfg(feature = "v2")]
            let mca_type = MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
                mca.clone(),
            ));

            build_unified_connector_service_auth_metadata(mca_type, platform)
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to build UCS auth metadata")?
        .ok_or_else(|| {
            error_stack::report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                "Missing merchant connector account for UCS webhook transformation",
            )
        })?;
    let profile_id = merchant_connector_account
        .as_ref()
        .map(|mca| mca.profile_id.clone())
        .unwrap_or(consts::PROFILE_ID_UNAVAILABLE.clone());
    // Build gRPC headers
    let grpc_headers = state
        .get_grpc_headers_ucs(ExecutionMode::Primary)
        .lineage_ids(LineageIds::new(
            platform.get_processor().get_account().get_id().clone(),
            profile_id,
        ))
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .build();

    // Make UCS call - client availability already verified
    match ucs_client
        .transform_incoming_webhook(transform_request, connector_auth_metadata, grpc_headers)
        .await
    {
        Ok(response) => {
            let transform_response = response.into_inner();
            let transform_data = transformers::transform_ucs_webhook_response(transform_response)?;

            // UCS handles everything internally - event type, source verification, decoding
            Ok((
                transform_data.event_type,
                transform_data.source_verified,
                transform_data,
            ))
        }
        Err(err) => {
            // When UCS is configured, we don't fall back to direct connector processing
            Err(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable(format!("UCS webhook processing failed: {err}"))
        }
    }
}

/// Extract webhook content from UCS response for further processing
/// This provides a helper function to extract specific data from UCS responses
pub fn extract_webhook_content_from_ucs_response(
    transform_data: &WebhookTransformData,
) -> Option<&unified_connector_service_client::payments::WebhookResponseContent> {
    transform_data.webhook_content.as_ref()
}

/// UCS Event Logging Wrapper Function
/// This function wraps UCS calls with comprehensive event logging.
/// It logs the actual gRPC request/response data, timing, and error information.
#[instrument(skip_all, fields(connector_name, flow_type, payment_id))]
pub async fn ucs_logging_wrapper<T, F, Fut, Req, Resp, GrpcReq, GrpcResp, FlowOutput>(
    router_data: RouterData<T, Req, Resp>,
    state: &SessionState,
    grpc_request: GrpcReq,
    grpc_header_builder: external_services::grpc_client::GrpcHeadersUcsBuilderFinal,
    handler: F,
) -> RouterResult<(RouterData<T, Req, Resp>, FlowOutput)>
where
    T: std::fmt::Debug + Clone + Send + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    GrpcReq: serde::Serialize,
    GrpcResp: serde::Serialize,
    F: FnOnce(
            RouterData<T, Req, Resp>,
            GrpcReq,
            external_services::grpc_client::GrpcHeadersUcs,
        ) -> Fut
        + Send,
    Fut: std::future::Future<Output = RouterResult<(RouterData<T, Req, Resp>, FlowOutput, GrpcResp)>>
        + Send,
{
    tracing::Span::current().record("connector_name", &router_data.connector);
    tracing::Span::current().record("flow_type", std::any::type_name::<T>());
    tracing::Span::current().record("payment_id", &router_data.payment_id);

    // Capture request data for logging
    let connector_name = router_data.connector.clone();
    let payment_id = router_data.payment_id.clone();
    let merchant_id = router_data.merchant_id.clone();
    let refund_id = router_data.refund_id.clone();
    let dispute_id = router_data.dispute_id.clone();
    let grpc_header = grpc_header_builder.build();
    // Log the actual gRPC request with masking
    let grpc_request_body = masking::masked_serialize(&grpc_request)
        .unwrap_or_else(|_| serde_json::json!({"error": "failed_to_serialize_grpc_request"}));

    // Update connector call count metrics for UCS operations
    crate::routes::metrics::CONNECTOR_CALL_COUNT.add(
        1,
        router_env::metric_attributes!(
            ("connector", connector_name.clone()),
            (
                "flow",
                std::any::type_name::<T>()
                    .split("::")
                    .last()
                    .unwrap_or_default()
            ),
        ),
    );

    // Execute UCS function and measure timing
    let start_time = Instant::now();
    let result = handler(router_data, grpc_request, grpc_header).await;
    let external_latency = start_time.elapsed().as_millis();

    // Create and emit connector event after UCS call
    let (status_code, response_body, router_result) = match result {
        Ok((updated_router_data, flow_output, grpc_response)) => {
            let status = updated_router_data
                .connector_http_status_code
                .unwrap_or(200);

            // Log the actual gRPC response with masking
            let grpc_response_body = masking::masked_serialize(&grpc_response).unwrap_or_else(
                |_| serde_json::json!({"error": "failed_to_serialize_grpc_response"}),
            );

            (
                status,
                Some(grpc_response_body),
                Ok((updated_router_data, flow_output)),
            )
        }
        Err(error) => {
            // Update error metrics for UCS calls
            crate::routes::metrics::CONNECTOR_ERROR_RESPONSE_COUNT.add(
                1,
                router_env::metric_attributes!(("connector", connector_name.clone(),)),
            );

            let error_body = serde_json::json!({
                "error": error.to_string(),
                "error_type": "ucs_call_failed"
            });
            (500, Some(error_body), Err(error))
        }
    };

    let mut connector_event = ConnectorEvent::new(
        state.tenant.tenant_id.clone(),
        connector_name,
        std::any::type_name::<T>(),
        grpc_request_body,
        "grpc://unified-connector-service".to_string(),
        Method::Post,
        payment_id,
        merchant_id,
        state.request_id.as_ref(),
        external_latency,
        refund_id,
        dispute_id,
        status_code,
    );

    // Set response body based on status code
    if let Some(body) = response_body {
        match status_code {
            400..=599 => {
                connector_event.set_error_response_body(&body);
            }
            _ => {
                connector_event.set_response_body(&body);
            }
        }
    }

    // Emit event
    state.event_handler.log_event(&connector_event);

    // Set external latency on router data
    router_result.map(|mut router_data| {
        router_data.0.external_latency =
            Some(router_data.0.external_latency.unwrap_or(0) + external_latency);
        router_data
    })
}

/// new UCS Event Logging Wrapper Function with UCS error response
/// This function wraps UCS calls with comprehensive event logging.
/// It logs the actual gRPC request/response data, timing, and error information.
#[instrument(skip_all, fields(connector_name, flow_type, payment_id))]
pub async fn ucs_logging_wrapper_new<T, F, Fut, Req, Resp, GrpcReq, GrpcResp>(
    router_data: RouterData<T, Req, Resp>,
    state: &SessionState,
    grpc_request: GrpcReq,
    grpc_header_builder: external_services::grpc_client::GrpcHeadersUcsBuilderFinal,
    handler: F,
) -> CustomResult<RouterData<T, Req, Resp>, UnifiedConnectorServiceError>
where
    T: std::fmt::Debug + Clone + Send + 'static,
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
    GrpcReq: serde::Serialize,
    GrpcResp: serde::Serialize,
    F: FnOnce(
            RouterData<T, Req, Resp>,
            GrpcReq,
            external_services::grpc_client::GrpcHeadersUcs,
        ) -> Fut
        + Send,
    Fut: std::future::Future<
            Output = CustomResult<
                (RouterData<T, Req, Resp>, GrpcResp),
                UnifiedConnectorServiceError,
            >,
        > + Send,
{
    tracing::Span::current().record("connector_name", &router_data.connector);
    tracing::Span::current().record("flow_type", std::any::type_name::<T>());
    tracing::Span::current().record("payment_id", &router_data.payment_id);

    // Capture request data for logging
    let connector_name = router_data.connector.clone();
    let payment_id = router_data.payment_id.clone();
    let merchant_id = router_data.merchant_id.clone();
    let refund_id = router_data.refund_id.clone();
    let dispute_id = router_data.dispute_id.clone();
    let grpc_header = grpc_header_builder.build();
    // Log the actual gRPC request with masking
    let grpc_request_body = masking::masked_serialize(&grpc_request)
        .unwrap_or_else(|_| serde_json::json!({"error": "failed_to_serialize_grpc_request"}));

    // Update connector call count metrics for UCS operations
    crate::routes::metrics::CONNECTOR_CALL_COUNT.add(
        1,
        router_env::metric_attributes!(
            ("connector", connector_name.clone()),
            (
                "flow",
                std::any::type_name::<T>()
                    .split("::")
                    .last()
                    .unwrap_or_default()
            ),
        ),
    );

    // Execute UCS function and measure timing
    let start_time = Instant::now();
    let result = handler(router_data, grpc_request, grpc_header).await;
    let external_latency = start_time.elapsed().as_millis();

    // Create and emit connector event after UCS call
    let (status_code, response_body, router_result) = match result {
        Ok((updated_router_data, grpc_response)) => {
            let status = updated_router_data
                .connector_http_status_code
                .unwrap_or(200);

            // Log the actual gRPC response
            let grpc_response_body = serde_json::to_value(&grpc_response).unwrap_or_else(
                |_| serde_json::json!({"error": "failed_to_serialize_grpc_response"}),
            );

            (status, Some(grpc_response_body), Ok(updated_router_data))
        }
        Err(error) => {
            // Update error metrics for UCS calls
            crate::routes::metrics::CONNECTOR_ERROR_RESPONSE_COUNT.add(
                1,
                router_env::metric_attributes!(("connector", connector_name.clone(),)),
            );

            let error_body = serde_json::json!({
                "error": error.to_string(),
                "error_type": "ucs_call_failed"
            });
            (500, Some(error_body), Err(error))
        }
    };

    let mut connector_event = ConnectorEvent::new(
        state.tenant.tenant_id.clone(),
        connector_name,
        std::any::type_name::<T>(),
        grpc_request_body,
        "grpc://unified-connector-service".to_string(),
        Method::Post,
        payment_id,
        merchant_id,
        state.request_id.as_ref(),
        external_latency,
        refund_id,
        dispute_id,
        status_code,
    );

    // Set response body based on status code
    if let Some(body) = response_body {
        match status_code {
            400..=599 => {
                connector_event.set_error_response_body(&body);
            }
            _ => {
                connector_event.set_response_body(&body);
            }
        }
    }

    // Emit event
    state.event_handler.log_event(&connector_event);

    // Set external latency on router data
    router_result.map(|mut router_data| {
        router_data.external_latency =
            Some(router_data.external_latency.unwrap_or(0) + external_latency);
        router_data
    })
}

#[derive(serde::Serialize)]
pub struct ComparisonData {
    pub hyperswitch_data: Secret<serde_json::Value>,
    pub unified_connector_service_data: Secret<serde_json::Value>,
}

/// Generic function to serialize router data and send comparison to external service
/// Works for both payments and refunds
#[cfg(feature = "v1")]
pub async fn serialize_router_data_and_send_to_comparison_service<F, RouterDReq, RouterDResp>(
    state: &SessionState,
    hyperswitch_router_data: RouterData<F, RouterDReq, RouterDResp>,
    unified_connector_service_router_data: RouterData<F, RouterDReq, RouterDResp>,
) -> RouterResult<()>
where
    F: Send + Clone + Sync + 'static,
    RouterDReq: Send + Sync + Clone + 'static + serde::Serialize,
    RouterDResp: Send + Sync + Clone + 'static + serde::Serialize,
{
    logger::info!("Simulating UCS call for shadow mode comparison");

    let [hyperswitch_data, unified_connector_service_data] = [
        (hyperswitch_router_data, "hyperswitch"),
        (unified_connector_service_router_data, "ucs"),
    ]
    .map(|(data, source)| {
        serde_json::to_value(data)
            .map(Secret::new)
            .unwrap_or_else(|e| {
                Secret::new(serde_json::json!({
                    "error": e.to_string(),
                    "source": source
                }))
            })
    });

    let comparison_data = ComparisonData {
        hyperswitch_data,
        unified_connector_service_data,
    };
    let _ = send_comparison_data(state, comparison_data)
        .await
        .map_err(|e| {
            logger::debug!("Failed to send comparison data: {:?}", e);
        });
    Ok(())
}

/// Sends router data comparison to external service
pub async fn send_comparison_data(
    state: &SessionState,
    comparison_data: ComparisonData,
) -> RouterResult<()> {
    // Check if comparison service is enabled
    let comparison_config = match state.conf.comparison_service.as_ref() {
        Some(comparison_config) => comparison_config,
        None => {
            tracing::warn!(
                "Comparison service configuration missing, skipping comparison data send"
            );
            return Ok(());
        }
    };

    let mut request = RequestBuilder::new()
        .method(Method::Post)
        .url(comparison_config.url.get_string_repr())
        .header(CONTENT_TYPE, "application/json")
        .header(X_FLOW_NAME, "router-data")
        .set_body(RequestContent::Json(Box::new(comparison_data)))
        .build();
    if let Some(req_id) = &state.request_id {
        request.add_header(X_REQUEST_ID, masking::Maskable::Normal(req_id.to_string()));
    }

    let _ = http_client::send_request(&state.conf.proxy, request, comparison_config.timeout_secs)
        .await
        .map_err(|e| {
            tracing::debug!("Error sending comparison data: {:?}", e);
        });

    Ok(())
}

// ============================================================================
// REFUND UCS FUNCTIONS
// ============================================================================

/// Execute UCS refund request using PaymentService.Refund gRPC method
#[instrument(skip_all)]
pub async fn call_unified_connector_service_for_refund_execute(
    state: &SessionState,
    platform: &Platform,
    router_data: RouterData<refunds::Execute, RefundsData, RefundsResponseData>,
    execution_mode: ExecutionMode,
    #[cfg(feature = "v1")] merchant_connector_account: MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: MerchantConnectorAccountTypeDetails,
) -> RouterResult<RouterData<refunds::Execute, RefundsData, RefundsResponseData>> {
    // Get UCS client
    let ucs_client = get_ucs_client(state)?;

    // Build auth metadata using standard UCS function
    let connector_auth_metadata =
        build_unified_connector_service_auth_metadata(merchant_connector_account, platform)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to build UCS auth metadata for refund execute")?;

    // Transform router data to UCS refund request
    let ucs_refund_request =
        payments_grpc::PaymentServiceRefundRequest::foreign_try_from(&router_data)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to transform router data to UCS refund request")?;

    // Build gRPC headers
    // Use merchant_id as profile_id fallback since RouterData doesn't have profile_id field
    let merchant_id = platform.get_processor().get_account().get_id().clone();
    let profile_id = id_type::ProfileId::from_str(merchant_id.get_string_repr())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert merchant_id to profile_id for UCS refund")?;
    let lineage_ids = LineageIds::new(merchant_id, profile_id);
    let grpc_header_builder = state
        .get_grpc_headers_ucs(execution_mode)
        .lineage_ids(lineage_ids)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None);

    // Make UCS refund call with logging wrapper
    Box::pin(ucs_logging_wrapper(
        router_data,
        state,
        ucs_refund_request,
        grpc_header_builder,
        |router_data, grpc_request, grpc_headers| async move {
            // Call UCS payment_refund method
            let response = ucs_client
                .payment_refund(grpc_request, connector_auth_metadata, grpc_headers)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("UCS refund execution failed")?;

            let grpc_response = response.into_inner();

            // Transform UCS response back to RouterData
            let (refund_response_data, status_code) =
                handle_unified_connector_service_response_for_refund_execute(grpc_response.clone())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to transform UCS refund response")?;

            let mut updated_router_data = router_data;
            updated_router_data.response = refund_response_data;
            updated_router_data.connector_http_status_code = Some(status_code);

            Ok((updated_router_data, (), grpc_response))
        },
    ))
    .await
    .map(|(router_data, _flow_response)| router_data)
}

/// Execute UCS refund sync request using RefundService.Get gRPC method
#[instrument(skip_all)]
pub async fn call_unified_connector_service_for_refund_sync(
    state: &SessionState,
    platform: &Platform,
    router_data: RouterData<refunds::RSync, RefundsData, RefundsResponseData>,
    execution_mode: ExecutionMode,
    #[cfg(feature = "v1")] merchant_connector_account: MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: MerchantConnectorAccountTypeDetails,
) -> RouterResult<RouterData<refunds::RSync, RefundsData, RefundsResponseData>> {
    // Get UCS client
    let ucs_client = get_ucs_client(state)?;

    // Build auth metadata using standard UCS function
    let connector_auth_metadata =
        build_unified_connector_service_auth_metadata(merchant_connector_account, platform)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to build UCS auth metadata for refund sync")?;

    // Transform router data to UCS refund sync request
    let ucs_refund_sync_request =
        payments_grpc::RefundServiceGetRequest::foreign_try_from(&router_data)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to transform router data to UCS refund sync request")?;

    // Build gRPC headers
    // Use merchant_id as profile_id fallback since RouterData doesn't have profile_id field
    let merchant_id = platform.get_processor().get_account().get_id().clone();
    let profile_id = id_type::ProfileId::from_str(merchant_id.get_string_repr())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert merchant_id to profile_id for UCS refund")?;
    let lineage_ids = LineageIds::new(merchant_id, profile_id);

    let grpc_header_builder = state
        .get_grpc_headers_ucs(execution_mode)
        .lineage_ids(lineage_ids)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None);

    // Make UCS refund sync call with logging wrapper
    Box::pin(ucs_logging_wrapper(
        router_data,
        state,
        ucs_refund_sync_request,
        grpc_header_builder,
        |router_data, grpc_request, grpc_headers| async move {
            // Call UCS refund_sync method
            let response = ucs_client
                .refund_sync(grpc_request, connector_auth_metadata, grpc_headers)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("UCS refund sync execution failed")?;

            let grpc_response = response.into_inner();

            // Transform UCS response back to RouterData
            let (refund_response_data, status_code) =
                handle_unified_connector_service_response_for_refund_sync(grpc_response.clone())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to transform UCS refund sync response")?;

            let mut updated_router_data = router_data;
            updated_router_data.response = refund_response_data;
            updated_router_data.connector_http_status_code = Some(status_code);

            Ok((updated_router_data, (), grpc_response))
        },
    ))
    .await
    .map(|(router_data, _flow_response)| router_data)
}
