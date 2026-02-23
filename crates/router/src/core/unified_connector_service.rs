use std::{borrow::Cow, str::FromStr, time::Instant};

use api_models::admin;
#[cfg(feature = "v2")]
use base64::Engine;
use common_enums::{
    connector_enums::Connector, AttemptStatus, CallConnectorAction, ConnectorIntegrationType,
    ExecutionMode, ExecutionPath, GatewaySystem, PaymentMethodType, UcsAvailability,
};
#[cfg(feature = "v2")]
use common_utils::consts::BASE64_ENGINE;
use common_utils::{
    consts::{X_CONNECTOR_NAME, X_FLOW_NAME, X_SUB_FLOW_NAME},
    errors::CustomResult,
    ext_traits::ValueExt,
    id_type,
    request::{Method, RequestBuilder, RequestContent},
    ucs_types,
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
    platform::Processor,
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
    CryptoCurrency, EVoucher, OpenBanking, PaymentServiceAuthorizeResponse,
};

#[cfg(feature = "v2")]
use crate::types::api::enums as api_enums;
use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payments::{
            helpers::{
                is_ucs_enabled, should_execute_based_on_rollout, MerchantConnectorAccountType,
                ProxyOverride,
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
    processor: &Processor,
    connector_name: &str,
    merchant_connector_id: Option<&id_type::MerchantConnectorAccountId>,
    creds_identifier: Option<String>,
    ucs_state: Option<&unified_connector_service_client::payments::ConnectorState>,
) -> CustomResult<Option<AccessToken>, UnifiedConnectorServiceError> {
    let ucs_access_token = ucs_state
        .and_then(|state| state.access_token.as_ref())
        .map(AccessToken::foreign_try_from)
        .transpose()?;

    let merchant_id = processor.get_account().get_id();

    let merchant_connector_id_or_connector_name = merchant_connector_id
        .map(|mca_id| mca_id.get_string_repr().to_string())
        .or(creds_identifier.map(|id| id.to_string()))
        .unwrap_or(connector_name.to_string());

    let key = common_utils::access_token::get_default_access_token_key(
        merchant_id,
        merchant_connector_id_or_connector_name,
    );

    if let Ok(Some(cached_token)) = session_state.store.get_access_token(key).await {
        if let Some(ref ucs_token) = ucs_access_token {
            if cached_token.token.peek() == ucs_token.token.peek() {
                return Ok(None);
            }
        }
    }

    Ok(ucs_access_token)
}

pub async fn set_access_token_for_ucs(
    state: &SessionState,
    processor: &Processor,
    connector_name: &str,
    access_token: AccessToken,
    merchant_connector_id: Option<&id_type::MerchantConnectorAccountId>,
    creds_identifier: Option<String>,
) -> Result<(), errors::StorageError> {
    let merchant_id = processor.get_account().get_id();

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

type UnifiedConnectorServiceCreateOrderResult = CustomResult<
    (
        Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
        u16,
    ),
    UnifiedConnectorServiceError,
>;

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
) -> RouterResult<ConnectorIntegrationType> {
    match state.conf.grpc_client.unified_connector_service.as_ref() {
        Some(ucs_config) => {
            let is_ucs_only = ucs_config.ucs_only_connectors.contains(&connector);

            if is_ucs_only {
                router_env::logger::debug!(
                    connector = ?connector,
                    ucs_only_list = is_ucs_only,
                    "Using UcsConnector"
                );
                Ok(ConnectorIntegrationType::UcsConnector)
            } else {
                router_env::logger::debug!(
                    connector = ?connector,
                    "Using DirectandUCSConnector - not in ucs_only_list"
                );
                Ok(ConnectorIntegrationType::DirectandUCSConnector)
            }
        }
        None => {
            router_env::logger::debug!(
                connector = ?connector,
                "UCS config not present, using DirectandUCSConnector"
            );
            Ok(ConnectorIntegrationType::DirectandUCSConnector)
        }
    }
}

pub async fn should_call_unified_connector_service<F: Clone, T, R>(
    state: &SessionState,
    processor: &Processor,
    router_data: &RouterData<F, T, R>,
    previous_gateway: Option<GatewaySystem>,
    call_connector_action: CallConnectorAction,
    shadow_ucs_call_connector_action: Option<CallConnectorAction>,
) -> RouterResult<(ExecutionPath, SessionState)>
where
    R: Send + Sync + Clone,
{
    // Extract context information
    let merchant_id = processor.get_account().get_id().get_string_repr();

    let connector_name = &router_data.connector;
    let connector_enum = Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
        .attach_printable_lazy(|| format!("Failed to parse connector name: {connector_name}"))?;

    let flow_name = get_flow_name::<F>()?;

    // Check UCS availability using idiomatic helper
    let ucs_availability = check_ucs_availability(state).await;

    let rollout_key = build_rollout_keys(
        merchant_id,
        connector_name,
        &flow_name,
        router_data.payment_method,
        router_data.payment_method_type,
    );

    // Determine connector integration type
    let connector_integration_type =
        determine_connector_integration_type(state, connector_enum).await?;

    // Check rollout key availability and shadow key presence (optimized to reduce DB calls)
    let rollout_result = should_execute_based_on_rollout(state, &rollout_key).await?;

    // Single decision point using pattern matching
    let (gateway_system, mut execution_path) = if ucs_availability == UcsAvailability::Disabled {
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
                    rollout_result.execution_mode,
                )?
            }
        }
    };

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
                        "No proxy override available for Shadow UCS, Using the Original State and Sending Request Directly"
                    );
                    execution_path = ExecutionPath::Direct;
                    state.clone()
                }
            }
        }
        ExecutionPath::Direct | ExecutionPath::UnifiedConnectorService => {
            // For Direct and UCS flows, use original state
            state.clone()
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
    execution_mode: ExecutionMode,
) -> RouterResult<(GatewaySystem, ExecutionPath)> {
    match connector_type {
        // UCS-only connectors always use UCS
        ConnectorIntegrationType::UcsConnector => Ok((
            GatewaySystem::UnifiedConnectorService,
            ExecutionPath::UnifiedConnectorService,
        )),
        ConnectorIntegrationType::DirectandUCSConnector => {
            match (previous_gateway, execution_mode) {
                (Some(GatewaySystem::Direct), ExecutionMode::NotApplicable) => {
                    // Previous gateway was Direct, continue using Direct
                    Ok((GatewaySystem::Direct, ExecutionPath::Direct))
                }
                (Some(GatewaySystem::Direct), ExecutionMode::Primary) => {
                    // Previous gateway was Direct, continue using Direct
                    Ok((GatewaySystem::Direct, ExecutionPath::Direct))
                }
                (Some(GatewaySystem::Direct), ExecutionMode::Shadow) => {
                    // Previous gateway was Direct, but now UCS is in shadow mode for comparison
                    Ok((
                        GatewaySystem::Direct,
                        ExecutionPath::ShadowUnifiedConnectorService,
                    ))
                }
                (Some(GatewaySystem::UnifiedConnectorService), ExecutionMode::NotApplicable) => {
                    // Previous gateway was UCS, continue using Direct as the config key has notapplicable execution mode
                    Ok((GatewaySystem::Direct, ExecutionPath::Direct))
                }
                (Some(GatewaySystem::UnifiedConnectorService), ExecutionMode::Primary) => {
                    // previous gateway was UCS, and config key has execution mode primary - continue using UCS
                    Ok((
                        GatewaySystem::UnifiedConnectorService,
                        ExecutionPath::UnifiedConnectorService,
                    ))
                }
                (Some(GatewaySystem::UnifiedConnectorService), ExecutionMode::Shadow) => {
                    // previous gateway was UCS, but now UCS is in shadow mode for comparison
                    Ok((
                        GatewaySystem::Direct,
                        ExecutionPath::ShadowUnifiedConnectorService,
                    ))
                }
                (None, ExecutionMode::Primary) => {
                    // Fresh payment for a UCS-enabled connector - use UCS as primary
                    Ok((
                        GatewaySystem::UnifiedConnectorService,
                        ExecutionPath::UnifiedConnectorService,
                    ))
                }
                (None, ExecutionMode::Shadow) => {
                    // Fresh payment for UCS-enabled connector with shadow mode - use shadow UCS
                    Ok((
                        GatewaySystem::Direct,
                        ExecutionPath::ShadowUnifiedConnectorService,
                    ))
                }
                (None, ExecutionMode::NotApplicable) => {
                    // Fresh payment request for direct connector - use direct gateway
                    Ok((GatewaySystem::Direct, ExecutionPath::Direct))
                }
            }
        }
    }
}

/// Build rollout keys based on flow type - include payment method for payments, skip for refunds
fn build_rollout_keys(
    merchant_id: &str,
    connector_name: &str,
    flow_name: &str,
    payment_method: common_enums::PaymentMethod,
    payment_method_type: Option<PaymentMethodType>,
) -> String {
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
        match payment_method {
            common_enums::PaymentMethod::Wallet
            | common_enums::PaymentMethod::BankRedirect
            | common_enums::PaymentMethod::Voucher
            | common_enums::PaymentMethod::PayLater => {
                let payment_method_str = payment_method.to_string();
                let payment_method_type_str = payment_method_type
                    .map(|pmt| pmt.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                format!(
                    "{}_{}_{}_{}_{}_{}",
                    consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
                    merchant_id,
                    connector_name,
                    payment_method_str,
                    payment_method_type_str,
                    flow_name
                )
            }
            common_enums::PaymentMethod::Card
            | common_enums::PaymentMethod::CardRedirect
            | common_enums::PaymentMethod::Upi
            | common_enums::PaymentMethod::Crypto
            | common_enums::PaymentMethod::Reward
            | common_enums::PaymentMethod::BankDebit
            | common_enums::PaymentMethod::RealTimePayment
            | common_enums::PaymentMethod::BankTransfer
            | common_enums::PaymentMethod::GiftCard
            | common_enums::PaymentMethod::MobilePayment
            | common_enums::PaymentMethod::NetworkToken
            | common_enums::PaymentMethod::OpenBanking => {
                // For other payment methods, use a generic format without specific payment method type details
                let payment_method_str = payment_method.to_string();
                format!(
                    "{}_{}_{}_{}_{}",
                    consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
                    merchant_id,
                    connector_name,
                    payment_method_str,
                    flow_name
                )
            }
        }
    };

    rollout_key
}

/// Extracts the gateway system from the payment intent's feature metadata
/// Returns None if metadata is missing, corrupted, or doesn't contain gateway_system
pub fn extract_gateway_system_from_payment_intent<F: Clone, D>(
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
    processor: &Processor,
    connector_name: &str,
) -> RouterResult<ExecutionPath> {
    // Extract context information
    let merchant_id = processor.get_account().get_id().get_string_repr();

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

    // Determine connector integration type
    let connector_integration_type =
        determine_connector_integration_type(state, connector_enum).await?;

    // For webhooks, there is no previous gateway system to consider (webhooks are stateless)
    let previous_gateway = None;

    // Check both rollout keys to determine priority based on shadow percentage
    let rollout_result = should_execute_based_on_rollout(state, &rollout_key).await?;

    // Use the same decision logic as payments, with no call_connector_action to consider
    let (gateway_system, execution_path) = if ucs_availability == UcsAvailability::Disabled {
        router_env::logger::debug!("UCS is disabled for webhooks, using Direct gateway");
        (GatewaySystem::Direct, ExecutionPath::Direct)
    } else {
        // UCS is enabled, use decide function with no previous gateway for webhooks
        decide_execution_path(
            connector_integration_type,
            previous_gateway,
            rollout_result.execution_mode,
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
                .map(payments_grpc::CardNetwork::foreign_from);

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
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardRedirect(
            card_redirect_data,
        ) => {
            let card_redirect_type = match card_redirect_data {
                hyperswitch_domain_models::payment_method_data::CardRedirectData::Knet {} => {
                    payments_grpc::card_redirect::CardRedirectType::Knet
                }
                hyperswitch_domain_models::payment_method_data::CardRedirectData::Benefit {} => {
                    payments_grpc::card_redirect::CardRedirectType::Benefit
                }
                hyperswitch_domain_models::payment_method_data::CardRedirectData::MomoAtm {} => {
                    payments_grpc::card_redirect::CardRedirectType::MomoAtm
                }
                hyperswitch_domain_models::payment_method_data::CardRedirectData::CardRedirect {} => {
                    payments_grpc::card_redirect::CardRedirectType::CardRedirect
                }
            };

            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::CardRedirect(payments_grpc::CardRedirect {
                    r#type: card_redirect_type.into(),
                })),
            })
        }
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Upi(upi_data) => {
            let upi_type = match upi_data {
                hyperswitch_domain_models::payment_method_data::UpiData::UpiCollect(
                    upi_collect_data,
                ) => {
                    let upi_details = payments_grpc::UpiCollect {
                        vpa_id: upi_collect_data.vpa_id.map(|vpa| vpa.expose().into()),
                        upi_source: upi_collect_data
                            .upi_source
                            .map(payments_grpc::UpiSource::foreign_try_from)
                            .transpose()?
                            .map(|upi_source| upi_source.into()),
                    };
                    PaymentMethod::UpiCollect(upi_details)
                }
                hyperswitch_domain_models::payment_method_data::UpiData::UpiIntent(upi_intent_data) => {
                    let upi_details = payments_grpc::UpiIntent {
                        app_name: upi_intent_data.app_name,
                        upi_source: upi_intent_data
                            .upi_source
                            .map(payments_grpc::UpiSource::foreign_try_from)
                            .transpose()?
                            .map(|upi_source| upi_source.into()),
                    };
                    PaymentMethod::UpiIntent(upi_details)
                }
                hyperswitch_domain_models::payment_method_data::UpiData::UpiQr(upi_qr_data) => {
                    let upi_details = payments_grpc::UpiQr {
                        upi_source: upi_qr_data
                            .upi_source
                            .map(payments_grpc::UpiSource::foreign_try_from)
                            .transpose()?
                            .map(|upi_source| upi_source.into()),
                    };
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
            hyperswitch_domain_models::payment_method_data::BankRedirectData::OpenBanking {} =>
                Ok(payments_grpc::PaymentMethod {
                        payment_method: Some(PaymentMethod::OpenBanking(OpenBanking {})),
                    }),
            hyperswitch_domain_models::payment_method_data::BankRedirectData::Ideal { bank_name } => {
                let ideal = payments_grpc::Ideal {
                    bank_name: bank_name.map(payments_grpc::BankNames::foreign_try_from)
                    .transpose()?
                    .map(|b| b.into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Ideal(ideal)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankRedirectData::Giropay {
                bank_account_bic,
                bank_account_iban,
                country,
            } => {
                let giropay = payments_grpc::Giropay {
                    bank_account_bic: bank_account_bic.map(|v| v.expose().into()),
                    bank_account_iban: bank_account_iban.map(|v| v.expose().into()),
                    country: country.and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                    .map(|country| country.into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Giropay(giropay)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankRedirectData::Eps { bank_name, country } => {
                let eps = payments_grpc::Eps {
                    bank_name: bank_name.map(payments_grpc::BankNames::foreign_try_from)
                    .transpose()?
                    .map(|b| b.into()),
                    country: country.and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                    .map(|country| country.into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Eps(eps)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankRedirectData::Sofort { country, preferred_language } => {
                let sofort = payments_grpc::Sofort {
                    country: country.and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                    .map(|country| country.into()),
                    preferred_language,
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Sofort(sofort)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankRedirectData::Przelewy24 { bank_name } => {
                let p24 = payments_grpc::Przelewy24 {
                    bank_name: bank_name.map(payments_grpc::BankNames::foreign_try_from)
                    .transpose()?
                    .map(|b| b.into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Przelewy24(p24)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankRedirectData::Blik { blik_code}  => {
                let blik = payments_grpc::Blik {
                    blik_code,
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Blik(blik)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankRedirectData::Trustly { country } => {
                let trustly = payments_grpc::Trustly {
                    country: country.and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string())).map(|c| c.into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Trustly(trustly)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankRedirectData::Interac {
                country,
                email,
            } => {
                let interac = payments_grpc::Interac {
                    country: country
                        .and_then(|c| payments_grpc::CountryAlpha2::from_str_name(&c.to_string()))
                        .map(|c| c.into()),
                    email: email.map(|e| e.expose().expose().into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Interac(interac)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankRedirectData::BancontactCard {
                card_number,
                card_exp_month,
                card_exp_year,
                card_holder_name,
            } => {
                let bancontact = payments_grpc::BancontactCard {
                    card_number: card_number.map(|v| CardNumber::from_str(&v.get_card_no())).transpose()
                        .change_context(
                        UnifiedConnectorServiceError::RequestEncodingFailedWithReason(
                            "Failed to parse card number".to_string(),
                        ),
                    )?,
                    card_exp_month: card_exp_month.map(|v| v.expose().into()),
                    card_exp_year: card_exp_year.map(|v| v.expose().into()),
                    card_holder_name: card_holder_name.map(|v| v.expose().into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::BancontactCard(bancontact)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankRedirectData::OnlineBankingFpx { issuer }  => {
                let online_banking_fpx = payments_grpc::OnlineBankingFpx {
                    issuer: payments_grpc::BankNames::foreign_try_from(issuer)?.into(),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::OnlineBankingFpx(online_banking_fpx)),
                })
            }
            _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                "Unimplemented payment method subtype: {payment_method_type:?}"
            ))
            .into()),
        },
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::RealTimePayment(
            bank_transfer_data,
        ) => match *bank_transfer_data {
            hyperswitch_domain_models::payment_method_data::RealTimePaymentData::DuitNow {  } => Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::DuitNow(
                    payments_grpc::DuitNow {  }
                )),
            }),
            _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                "Unimplemented payment method subtype: {payment_method_type:?}"
            ))
            .into()),
        }
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankTransfer(
            bank_transfer_data,
        ) => match *bank_transfer_data {
            hyperswitch_domain_models::payment_method_data::BankTransferData::AchBankTransfer {  } => Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::AchBankTransfer(
                    payments_grpc::AchBankTransfer {  }
                )),
            }),
            hyperswitch_domain_models::payment_method_data::BankTransferData::BacsBankTransfer {  } => Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::BacsBankTransfer(
                    payments_grpc::BacsBankTransfer {  }
                )),
            }),
            hyperswitch_domain_models::payment_method_data::BankTransferData::SepaBankTransfer {  } => Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::SepaBankTransfer(
                    payments_grpc::SepaBankTransfer {  }
                )),
            }),
            hyperswitch_domain_models::payment_method_data::BankTransferData::MultibancoBankTransfer {  } => Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::MultibancoBankTransfer(
                    payments_grpc::MultibancoBankTransfer {  }
                )),
            }),
            hyperswitch_domain_models::payment_method_data::BankTransferData::Pix {
                pix_key,
                cpf,
                cnpj,
                source_bank_account_id,
                destination_bank_account_id,
                expiry_date,
            } => Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::Pix(payments_grpc::PixPayment {
                    pix_key: pix_key.map(|v| v.expose().into()),
                    cpf: cpf.map(|v| v.expose().into()),
                    cnpj: cnpj.map(|v| v.expose().into()),
                    source_bank_account_id: source_bank_account_id.map(|v| v.expose_inner()),
                    destination_bank_account_id: destination_bank_account_id.map(|v| v.expose_inner()),
                    expiry_date: expiry_date.map(|dt| {
                        dt.format(&time::format_description::well_known::Iso8601::DEFAULT)
                            .unwrap_or_default()
                    }),
                })),
            }),
            hyperswitch_domain_models::payment_method_data::BankTransferData::PermataBankTransfer {} => {
                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::PermataBankTransfer(
                        payments_grpc::PermataBankTransfer {},
                    )),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankTransferData::BcaBankTransfer {} => {
                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::BcaBankTransfer(
                        payments_grpc::BcaBankTransfer {},
                    )),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankTransferData::BniVaBankTransfer {} => {
                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::BniVaBankTransfer(
                        payments_grpc::BniVaBankTransfer {},
                    )),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankTransferData::BriVaBankTransfer {} => {
                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::BriVaBankTransfer(
                        payments_grpc::BriVaBankTransfer {},
                    )),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankTransferData::CimbVaBankTransfer {} => {
                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::CimbVaBankTransfer(
                        payments_grpc::CimbVaBankTransfer {},
                    )),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankTransferData::DanamonVaBankTransfer {} => {
                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::DanamonVaBankTransfer(
                        payments_grpc::DanamonVaBankTransfer {},
                    )),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankTransferData::MandiriVaBankTransfer {} => {
                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::MandiriVaBankTransfer(
                        payments_grpc::MandiriVaBankTransfer {},
                    )),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankTransferData::InstantBankTransfer {} =>
                Ok(payments_grpc::PaymentMethod {
                        payment_method: Some(PaymentMethod::InstantBankTransfer(payments_grpc::InstantBankTransfer {})),
                    }),
            hyperswitch_domain_models::payment_method_data::BankTransferData::InstantBankTransferFinland {} =>
                Ok(payments_grpc::PaymentMethod {
                        payment_method: Some(PaymentMethod::InstantBankTransferFinland(payments_grpc::InstantBankTransferFinland {})),
                    }),
            hyperswitch_domain_models::payment_method_data::BankTransferData::InstantBankTransferPoland {} =>
                Ok(payments_grpc::PaymentMethod {
                        payment_method: Some(PaymentMethod::InstantBankTransferPoland(payments_grpc::InstantBankTransferPoland {})),
                    }),
            _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                "Unimplemented payment method subtype: {payment_method_type:?}"
            ))
            .into()),
        },
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::PayLater(
            pay_later_data,
        ) => match pay_later_data {
            hyperswitch_domain_models::payment_method_data::PayLaterData::KlarnaRedirect {  } => Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::Klarna(
                    payments_grpc::Klarna {  }
                )),
            }),
            hyperswitch_domain_models::payment_method_data::PayLaterData::AffirmRedirect {  } => Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::Affirm(
                    payments_grpc::Affirm {  }
                )),
            }),
            hyperswitch_domain_models::payment_method_data::PayLaterData::AfterpayClearpayRedirect {  } => Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::AfterpayClearpay(
                    payments_grpc::AfterpayClearpay {  }
                )),
            }),
            _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                "Unimplemented payment method subtype: {payment_method_type:?}"
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
                hyperswitch_domain_models::payment_method_data::WalletData::GooglePayThirdPartySdk(
                    google_pay_sdk_data,
                ) => {
                    Ok(payments_grpc::PaymentMethod {
                        payment_method: Some(PaymentMethod::GooglePayThirdPartySdk(
                            payments_grpc::GooglePayThirdPartySdkWallet {
                                token: google_pay_sdk_data.token.map(|t| t.expose().into()),
                            }
                        )),
                    })
                },
                hyperswitch_domain_models::payment_method_data::WalletData::ApplePayThirdPartySdk(
                    apple_pay_sdk_data,
                ) => {
                    Ok(payments_grpc::PaymentMethod {
                        payment_method: Some(PaymentMethod::ApplePayThirdPartySdk(
                            payments_grpc::ApplePayThirdPartySdkWallet {
                                token: apple_pay_sdk_data.token.map(|t| t.expose().into()),
                            }
                        )),
                    })
                },
                hyperswitch_domain_models::payment_method_data::WalletData::PaypalSdk(
                    paypal_sdk_data,
                ) => {
                    Ok(payments_grpc::PaymentMethod {
                        payment_method: Some(PaymentMethod::PaypalSdk(
                            payments_grpc::PaypalSdkWallet {
                                token: Some(paypal_sdk_data.token.into()),
                            }
                        )),
                    })
                },
                hyperswitch_domain_models::payment_method_data::WalletData::AmazonPayRedirect(
                    _,
                ) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::AmazonPayRedirect(
                        payments_grpc::AmazonPayRedirectWallet {  }
                    )),
                }),
                hyperswitch_domain_models::payment_method_data::WalletData::CashappQr(
                    _,
                ) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::CashappQr(
                        payments_grpc::CashappQrWallet {  }
                    )),
                }),
                hyperswitch_domain_models::payment_method_data::WalletData::WeChatPayQr(
                    _,
                ) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::WeChatPayQr(
                        payments_grpc::WeChatPayQrWallet {  }
                    )),
                }),
                hyperswitch_domain_models::payment_method_data::WalletData::AliPayRedirect(
                    _,
                ) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::AliPayRedirect(
                        payments_grpc::AliPayRedirectWallet {  }
                    )),
                }),
                hyperswitch_domain_models::payment_method_data::WalletData::RevolutPay(
                    _,
                ) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::RevolutPay(
                        payments_grpc::RevolutPayWallet {  }
                    )),
                }),
                hyperswitch_domain_models::payment_method_data::WalletData::BluecodeRedirect {} => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Bluecode(
                        payments_grpc::Bluecode {  }
                    )),
                }),
                hyperswitch_domain_models::payment_method_data::WalletData::PaypalRedirect(
                    paypal_redirection,
                ) => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::PaypalRedirect(
                        payments_grpc::PaypalRedirectWallet {
                            email: paypal_redirection.email.map(|e| e.expose().expose().into()),
                        }
                    )),
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
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardDetailsForNetworkTransactionId(card_nti_data) => {
            let card_details_for_nti = payments_grpc::CardDetailsForNetworkTransactionId::foreign_try_from(card_nti_data)?;

            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::CardDetailsForNetworkTransactionId(card_details_for_nti)),
            })
        }
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::NetworkToken(network_token_data) => {
            let network_token = payments_grpc::NetworkTokenData::foreign_try_from(network_token_data)?;

            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::NetworkToken(network_token)),
            })
        }
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankDebit(
            bank_debit_data,
        ) => match bank_debit_data {
            hyperswitch_domain_models::payment_method_data::BankDebitData::AchBankDebit {
                account_number,
                routing_number,
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            } => {
                let bank_name = bank_name
                    .map(payments_grpc::BankNames::foreign_try_from)
                    .transpose()?;
                let bank_type = bank_type
                    .map(payments_grpc::BankType::foreign_try_from)
                    .transpose()?;
                let bank_holder_type = bank_holder_type
                    .map(payments_grpc::BankHolderType::foreign_try_from)
                    .transpose()?;

                let ach = payments_grpc::Ach {
                    account_number: Some(account_number.expose().into()),
                    routing_number: Some(routing_number.expose().into()),
                    card_holder_name: card_holder_name.map(|name| name.expose().into()),
                    bank_account_holder_name: bank_account_holder_name
                        .map(|name| name.expose().into()),
                    bank_name: bank_name.map(Into::into).unwrap_or_default(),
                    bank_type: bank_type.map(Into::into).unwrap_or_default(),
                    bank_holder_type: bank_holder_type.map(Into::into).unwrap_or_default(),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Ach(ach)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankDebitData::SepaBankDebit {
                iban,
                bank_account_holder_name,
            } => {
                let sepa = payments_grpc::Sepa {
                    iban: Some(iban.expose().into()),
                    bank_account_holder_name: bank_account_holder_name
                        .map(|name| name.expose().into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Sepa(sepa)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankDebitData::BecsBankDebit {
                account_number,
                bsb_number,
                bank_account_holder_name,
            } => {
                let becs = payments_grpc::Becs {
                    account_number: Some(account_number.expose().into()),
                    bsb_number: Some(bsb_number.expose().into()),
                    bank_account_holder_name: bank_account_holder_name
                        .map(|name| name.expose().into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Becs(becs)),
                })
            }
            hyperswitch_domain_models::payment_method_data::BankDebitData::BacsBankDebit {
                account_number,
                sort_code,
                bank_account_holder_name,
            } => {
                let bacs = payments_grpc::Bacs {
                    account_number: Some(account_number.expose().into()),
                    sort_code: Some(sort_code.expose().into()),
                    bank_account_holder_name: bank_account_holder_name
                        .map(|name| name.expose().into()),
                };

                Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Bacs(bacs)),
                })
            }
            _ => Err(UnifiedConnectorServiceError::NotImplemented(
                "Unimplemented bank debit variant".to_string(),
            )
            .into()),
        },
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
                .map(payments_grpc::CardNetwork::foreign_from);
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
    processor: &Processor,
    connector_name: String,
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

    let merchant_id = processor.get_account().get_id().get_string_repr();

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

pub fn parse_merchant_reference_id(id: &str) -> Option<id_type::PaymentReferenceId> {
    id_type::PaymentReferenceId::from_str(id)
        .inspect_err(|err| {
            logger::warn!(
                error = ?err,
                "Invalid Merchant ReferenceId found"
            )
        })
        .ok()
}

#[cfg(feature = "v2")]
pub fn build_unified_connector_service_external_vault_proxy_metadata(
    external_vault_merchant_connector_account: MerchantConnectorAccountTypeDetails,
) -> CustomResult<String, UnifiedConnectorServiceError> {
    let external_vault_metadata = external_vault_merchant_connector_account
        .get_metadata()
        .ok_or(UnifiedConnectorServiceError::ParsingFailed)
        .attach_printable("Failed to obtain ConnectorMetadata")?;

    let connector = external_vault_merchant_connector_account.get_connector_name();

    let external_vault_connector =
        api_enums::VaultConnectors::try_from(connector).map_err(|err| {
            error_stack::report!(UnifiedConnectorServiceError::InvalidConnectorName)
                .attach_printable(format!("Failed to parse Vault connector: {err}"))
        })?;

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
    prev_status: AttemptStatus,
) -> CustomResult<UcsAuthorizeResponseData, UnifiedConnectorServiceError> {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response.clone(),
            prev_status,
        ))?;

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
    prev_status: AttemptStatus,
) -> UnifiedConnectorServiceCreateOrderResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response,
            prev_status,
        ))?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_post_authenticate(
    response: payments_grpc::PaymentServicePostAuthenticateResponse,
    prev_status: AttemptStatus,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response,
            prev_status,
        ))?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_method_token_create(
    response: payments_grpc::PaymentServiceCreatePaymentMethodTokenResponse,
) -> CustomResult<(Result<PaymentsResponseData, ErrorResponse>, u16), UnifiedConnectorServiceError>
{
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_sdk_session_token(
    response: payments_grpc::PaymentServiceSdkSessionTokenResponse,
) -> CustomResult<(Result<PaymentsResponseData, ErrorResponse>, u16), UnifiedConnectorServiceError>
{
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_incremental_authorization(
    response: payments_grpc::PaymentServiceIncrementalAuthorizationResponse,
) -> CustomResult<(Result<PaymentsResponseData, ErrorResponse>, u16), UnifiedConnectorServiceError>
{
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_authenticate(
    response: payments_grpc::PaymentServiceAuthenticateResponse,
    prev_status: AttemptStatus,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response,
            prev_status,
        ))?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_pre_authenticate(
    response: payments_grpc::PaymentServicePreAuthenticateResponse,
    prev_status: AttemptStatus,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response,
            prev_status,
        ))?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_capture(
    response: payments_grpc::PaymentServiceCaptureResponse,
    prev_status: AttemptStatus,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response,
            prev_status,
        ))?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_register(
    response: payments_grpc::PaymentServiceRegisterResponse,
    prev_status: AttemptStatus,
) -> CustomResult<UcsSetupMandateResponseData, UnifiedConnectorServiceError> {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response.clone(),
            prev_status,
        ))?;

    let connector_customer_id =
        extract_connector_customer_id_from_ucs_state(response.state.as_ref());
    let connector_response =
        extract_connector_response_from_ucs(response.connector_response.as_ref());

    Ok(UcsSetupMandateResponseData {
        router_data_response,
        status_code,
        connector_customer_id,
        connector_response,
        amount_captured: response.captured_amount,
        minor_amount_captured: response
            .minor_captured_amount
            .map(common_utils::types::MinorUnit::new),
    })
}

pub fn handle_unified_connector_service_response_for_session_token_create(
    response: payments_grpc::PaymentServiceCreateSessionTokenResponse,
) -> CustomResult<(Result<PaymentsResponseData, ErrorResponse>, u16), UnifiedConnectorServiceError>
{
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<PaymentsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_repeat(
    response: payments_grpc::PaymentServiceRepeatEverythingResponse,
    prev_status: AttemptStatus,
) -> CustomResult<UcsRepeatPaymentResponseData, UnifiedConnectorServiceError> {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response.clone(),
            prev_status,
        ))?;

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
    prev_status: AttemptStatus,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from((
            response,
            prev_status,
        ))?;

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
    processor: &Processor,
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
        processor.get_account().get_id().get_string_repr(),
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

            build_unified_connector_service_auth_metadata(
                mca_type,
                processor,
                connector_name.to_string(),
            )
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
            processor.get_account().get_id().clone(),
            profile_id,
        ))
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(None)
        .resource_id(None)
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
    execution_mode: ExecutionMode,
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
    let payout_id = router_data.payout_id.clone();
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

    // Only emit connector event during primary mode
    if let ExecutionMode::Primary = execution_mode {
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
            payout_id,
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
    }

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
pub async fn ucs_logging_wrapper_granular<T, F, Fut, Req, Resp, GrpcReq, FlowOutput, GrpcResp>(
    router_data: RouterData<T, Req, Resp>,
    state: &SessionState,
    grpc_request: GrpcReq,
    grpc_header_builder: external_services::grpc_client::GrpcHeadersUcsBuilderFinal,
    execution_mode: ExecutionMode,
    handler: F,
) -> CustomResult<(RouterData<T, Req, Resp>, FlowOutput), UnifiedConnectorServiceError>
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
                (RouterData<T, Req, Resp>, FlowOutput, GrpcResp),
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
    let payout_id = router_data.payout_id.clone();
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

            // Log the actual gRPC response
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

    // Only emit connector event during primary mode
    if let ExecutionMode::Primary = execution_mode {
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
            payout_id,
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
    }

    // Set external latency on router data
    router_result.map(|mut router_data| {
        router_data.0.external_latency =
            Some(router_data.0.external_latency.unwrap_or(0) + external_latency);
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

    let connector_name = hyperswitch_router_data.connector.clone();
    let sub_flow_name = get_flow_name::<F>().ok();

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
    let _ = send_comparison_data(state, comparison_data, connector_name, sub_flow_name)
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
    connector_name: String,
    sub_flow_name: Option<String>,
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

    request.add_header(X_CONNECTOR_NAME, masking::Maskable::Normal(connector_name));

    if let Some(sub_flow_name) = sub_flow_name.filter(|name| !name.is_empty()) {
        request.add_header(X_SUB_FLOW_NAME, masking::Maskable::Normal(sub_flow_name));
    }

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
    processor: &Processor,
    router_data: RouterData<refunds::Execute, RefundsData, RefundsResponseData>,
    execution_mode: ExecutionMode,
    #[cfg(feature = "v1")] merchant_connector_account: MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: MerchantConnectorAccountTypeDetails,
) -> RouterResult<RouterData<refunds::Execute, RefundsData, RefundsResponseData>> {
    // Get UCS client
    let ucs_client = get_ucs_client(state)?;

    // Build auth metadata using standard UCS function
    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        processor,
        router_data.connector.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build UCS auth metadata for refund execute")?;

    // Transform router data to UCS refund request
    let ucs_refund_request =
        payments_grpc::PaymentServiceRefundRequest::foreign_try_from(&router_data)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to transform router data to UCS refund request")?;

    // Build gRPC headers
    // Use merchant_id as profile_id fallback since RouterData doesn't have profile_id field
    let merchant_id = processor.get_account().get_id().clone();
    let profile_id = id_type::ProfileId::from_str(merchant_id.get_string_repr())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert merchant_id to profile_id for UCS refund")?;
    let lineage_ids = LineageIds::new(merchant_id, profile_id);
    let merchant_reference_id =
        id_type::PaymentReferenceId::from_str(router_data.payment_id.as_str())
            .inspect_err(|err| logger::warn!(error=?err, "Invalid PaymentId for UCS reference id"))
            .ok()
            .map(ucs_types::UcsReferenceId::Payment);
    let resource_id = router_data
        .refund_id
        .as_ref()
        .and_then(|refund_id| {
            id_type::RefundReferenceId::try_from(Cow::Owned(refund_id.to_string()))
                .inspect_err(
                    |err| logger::warn!(error=?err, "Invalid RefundId for UCS resource id"),
                )
                .ok()
        })
        .map(ucs_types::UcsResourceId::Refund);
    let grpc_header_builder = state
        .get_grpc_headers_ucs(execution_mode)
        .lineage_ids(lineage_ids)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_reference_id)
        .resource_id(resource_id);

    // Make UCS refund call with logging wrapper
    Box::pin(ucs_logging_wrapper(
        router_data,
        state,
        ucs_refund_request,
        grpc_header_builder,
        execution_mode,
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
    processor: &Processor,
    router_data: RouterData<refunds::RSync, RefundsData, RefundsResponseData>,
    execution_mode: ExecutionMode,
    #[cfg(feature = "v1")] merchant_connector_account: MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: MerchantConnectorAccountTypeDetails,
) -> RouterResult<RouterData<refunds::RSync, RefundsData, RefundsResponseData>> {
    // Get UCS client
    let ucs_client = get_ucs_client(state)?;

    // Build auth metadata using standard UCS function
    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        processor,
        router_data.connector.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build UCS auth metadata for refund sync")?;

    // Transform router data to UCS refund sync request
    let ucs_refund_sync_request =
        payments_grpc::RefundServiceGetRequest::foreign_try_from(&router_data)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to transform router data to UCS refund sync request")?;

    // Build gRPC headers
    // Use merchant_id as profile_id fallback since RouterData doesn't have profile_id field
    let merchant_id = processor.get_account().get_id().clone();
    let profile_id = id_type::ProfileId::from_str(merchant_id.get_string_repr())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert merchant_id to profile_id for UCS refund")?;
    let lineage_ids = LineageIds::new(merchant_id, profile_id);
    let merchant_reference_id =
        id_type::PaymentReferenceId::from_str(router_data.payment_id.as_str())
            .inspect_err(|err| logger::warn!(error=?err, "Invalid PaymentId for UCS reference id"))
            .ok()
            .map(ucs_types::UcsReferenceId::Payment);
    let resource_id = router_data
        .refund_id
        .as_ref()
        .and_then(|refund_id| {
            id_type::RefundReferenceId::try_from(Cow::Owned(refund_id.to_string()))
                .inspect_err(
                    |err| logger::warn!(error=?err, "Invalid RefundId for UCS resource id"),
                )
                .ok()
        })
        .map(ucs_types::UcsResourceId::Refund);

    let grpc_header_builder = state
        .get_grpc_headers_ucs(execution_mode)
        .lineage_ids(lineage_ids)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_reference_id)
        .resource_id(resource_id);

    // Make UCS refund sync call with logging wrapper
    Box::pin(ucs_logging_wrapper(
        router_data,
        state,
        ucs_refund_sync_request,
        grpc_header_builder,
        execution_mode,
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
