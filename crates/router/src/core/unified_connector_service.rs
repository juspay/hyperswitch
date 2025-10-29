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
    merchant_context::MerchantContext,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_response_types::PaymentsResponseData,
};
use masking::{ExposeInterface, PeekInterface, Secret};
use router_env::{instrument, logger, tracing};
use unified_connector_service_cards::CardNumber;
use unified_connector_service_client::payments::{
    self as payments_grpc, payment_method::PaymentMethod, CardDetails, CardPaymentMethodType,
    PaymentServiceAuthorizeResponse, RewardPaymentMethodType,
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
    types::transformers::ForeignTryFrom,
};

pub mod transformers;

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

/// Gets the rollout percentage for a given config key
async fn get_rollout_percentage(state: &SessionState, config_key: &str) -> Option<f64> {
    let db = state.store.as_ref();

    match db.find_config_by_key(config_key).await {
        Ok(rollout_config) => {
            // Try to parse as JSON first (new format), fallback to float (legacy format)
            match serde_json::from_str::<helpers::RolloutConfig>(&rollout_config.config) {
                Ok(config) => Some(config.rollout_percent),
                Err(_) => {
                    // Fallback to legacy format (simple float)
                    rollout_config.config.parse::<f64>().ok()
                }
            }
        }
        Err(_) => None,
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

pub async fn should_call_unified_connector_service<F: Clone, T, D>(
    state: &SessionState,
    merchant_context: &MerchantContext,
    router_data: &RouterData<F, T, PaymentsResponseData>,
    payment_data: Option<&D>,
    call_connector_action: CallConnectorAction,
) -> RouterResult<(ExecutionPath, SessionState)>
where
    D: OperationSessionGetters<F>,
{
    // Extract context information
    let merchant_id = merchant_context
        .get_merchant_account()
        .get_id()
        .get_string_repr();

    let connector_name = &router_data.connector;
    let connector_enum = Connector::from_str(connector_name)
        .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
        .attach_printable_lazy(|| format!("Failed to parse connector name: {}", connector_name))?;

    let payment_method = router_data.payment_method.to_string();
    let flow_name = get_flow_name::<F>()?;

    // Check UCS availability using idiomatic helper
    let ucs_availability = check_ucs_availability(state).await;

    // Build rollout keys
    let rollout_key = format!(
        "{}_{}_{}_{}_{}",
        consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
        merchant_id,
        connector_name,
        payment_method,
        flow_name
    );

    // Determine connector integration type
    let connector_integration_type =
        determine_connector_integration_type(state, connector_enum, &rollout_key).await?;

    // Extract previous gateway from payment data
    let previous_gateway = payment_data.and_then(extract_gateway_system_from_payment_intent);
    let shadow_rollout_key = format!("{}_shadow", rollout_key);

    // Check both rollout keys to determine priority based on shadow percentage
    let rollout_result = should_execute_based_on_rollout(state, &rollout_key).await?;
    let shadow_rollout_result = should_execute_based_on_rollout(state, &shadow_rollout_key).await?;

    // Get shadow percentage to determine priority
    let shadow_percentage = get_rollout_percentage(state, &shadow_rollout_key)
        .await
        .unwrap_or(0.0);

    let shadow_rollout_availability =
        if shadow_rollout_result.should_execute && shadow_percentage != 0.0 {
            // Shadow is present and percentage is non-zero, use shadow
            router_env::logger::debug!(
                shadow_percentage = shadow_percentage,
                "Shadow rollout is present with non-zero percentage, using shadow"
            );
            ShadowRolloutAvailability::IsAvailable
        } else if rollout_result.should_execute {
            // Either shadow is 0.0 or not present, use rollout if available
            router_env::logger::debug!(
                shadow_percentage = shadow_percentage,
                "Shadow rollout is 0.0 or not present, using rollout"
            );
            ShadowRolloutAvailability::IsAvailable
        } else {
            ShadowRolloutAvailability::NotAvailable
        };

    // Single decision point using pattern matching
    let (_gateway_system, execution_path) = if ucs_availability == UcsAvailability::Disabled {
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
                (GatewaySystem::Direct, ExecutionPath::Direct)
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

    router_env::logger::info!( "Payment gateway decision: execution_path={:?} - merchant_id={}, connector={}, payment_method={}, flow={}", execution_path, merchant_id, connector_name, payment_method, flow_name );

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
                GatewaySystem::UnifiedConnectorService,
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
    merchant_context: &MerchantContext,
    connector_name: &str,
) -> RouterResult<bool> {
    if state.grpc_client.unified_connector_service_client.is_none() {
        logger::debug!(
            connector = connector_name.to_string(),
            "Unified Connector Service client is not available for webhooks"
        );
        return Ok(false);
    }

    let ucs_config_key = consts::UCS_ENABLED;

    if !is_ucs_enabled(state, ucs_config_key).await {
        return Ok(false);
    }

    let merchant_id = merchant_context
        .get_merchant_account()
        .get_id()
        .get_string_repr();

    let config_key = format!(
        "{}_{}_{}_Webhooks",
        consts::UCS_ROLLOUT_PERCENT_CONFIG_PREFIX,
        merchant_id,
        connector_name
    );

    let rollout_result = should_execute_based_on_rollout(state, &config_key).await?;

    Ok(rollout_result.should_execute)
}

pub fn build_unified_connector_service_payment_method(
    payment_method_data: hyperswitch_domain_models::payment_method_data::PaymentMethodData,
    payment_method_type: PaymentMethodType,
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
                card_exp_year: Some(card.get_expiry_year_4_digit().expose().into()),
                card_cvc: Some(card.card_cvc.expose().into()),
                card_holder_name: card.card_holder_name.map(|name| name.expose().into()),
                card_issuer: card.card_issuer.clone(),
                card_network: card_network.map(|card_network| card_network.into()),
                card_type: card.card_type.clone(),
                bank_code: card.bank_code.clone(),
                nick_name: card.nick_name.map(|n| n.expose()),
                card_issuing_country_alpha2: card.card_issuing_country.clone(),
            };

            let grpc_card_type = match payment_method_type {
                PaymentMethodType::Credit => {
                    payments_grpc::card_payment_method_type::CardType::Credit(card_details)
                }
                PaymentMethodType::Debit => {
                    payments_grpc::card_payment_method_type::CardType::Debit(card_details)
                }
                _ => {
                    return Err(UnifiedConnectorServiceError::NotImplemented(format!(
                        "Unimplemented payment method subtype: {payment_method_type:?}"
                    ))
                    .into());
                }
            };

            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::Card(CardPaymentMethodType {
                    card_type: Some(grpc_card_type),
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
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::Reward => {
            match payment_method_type {
                PaymentMethodType::ClassicReward => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Reward(RewardPaymentMethodType {
                        reward_type: 1,
                    })),
                }),
                PaymentMethodType::Evoucher => Ok(payments_grpc::PaymentMethod {
                    payment_method: Some(PaymentMethod::Reward(RewardPaymentMethodType {
                        reward_type: 2,
                    })),
                }),
                _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
                    "Unimplemented payment method subtype: {payment_method_type:?}"
                ))
                .into()),
            }
        }
        _ => Err(UnifiedConnectorServiceError::NotImplemented(format!(
            "Unimplemented payment method: {payment_method_data:?}"
        ))
        .into()),
    }
}

pub fn build_unified_connector_service_payment_method_for_external_proxy(
    payment_method_data: hyperswitch_domain_models::payment_method_data::ExternalVaultPaymentMethodData,
    payment_method_type: PaymentMethodType,
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
            let grpc_card_type = match payment_method_type {
                PaymentMethodType::Credit => {
                    payments_grpc::card_payment_method_type::CardType::CreditProxy(card_details)
                }
                PaymentMethodType::Debit => {
                    payments_grpc::card_payment_method_type::CardType::DebitProxy(card_details)
                }
                _ => {
                    return Err(UnifiedConnectorServiceError::NotImplemented(format!(
                        "Unimplemented payment method subtype: {payment_method_type:?}"
                    ))
                    .into());
                }
            };
            Ok(payments_grpc::PaymentMethod {
                payment_method: Some(PaymentMethod::Card(CardPaymentMethodType {
                    card_type: Some(grpc_card_type),
                })),
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
pub fn build_unified_connector_service_auth_metadata(
    #[cfg(feature = "v1")] merchant_connector_account: MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: MerchantConnectorAccountTypeDetails,
    merchant_context: &MerchantContext,
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

    let merchant_id = merchant_context
        .get_merchant_account()
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
            api_secret: Some(api_secret.clone()),
            auth_key_map: None,
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        ConnectorAuthType::BodyKey { api_key, key1 } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_BODY_KEY.to_string(),
            api_key: Some(api_key.clone()),
            key1: Some(key1.clone()),
            api_secret: None,
            auth_key_map: None,
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        ConnectorAuthType::HeaderKey { api_key } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_HEADER_KEY.to_string(),
            api_key: Some(api_key.clone()),
            key1: None,
            api_secret: None,
            auth_key_map: None,
            merchant_id: Secret::new(merchant_id.to_string()),
        }),
        ConnectorAuthType::CurrencyAuthKey { auth_key_map } => Ok(ConnectorAuthMetadata {
            connector_name,
            auth_type: consts::UCS_AUTH_CURRENCY_AUTH_KEY.to_string(),
            api_key: None,
            key1: None,
            api_secret: None,
            auth_key_map: Some(auth_key_map.clone()),
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
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

pub fn handle_unified_connector_service_response_for_payment_repeat(
    response: payments_grpc::PaymentServiceRepeatEverythingResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

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
    merchant_context: &MerchantContext,
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
        merchant_context
            .get_merchant_account()
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

            build_unified_connector_service_auth_metadata(mca_type, merchant_context)
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
            merchant_context.get_merchant_account().get_id().clone(),
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
pub async fn ucs_logging_wrapper<T, F, Fut, Req, Resp, GrpcReq, GrpcResp>(
    router_data: RouterData<T, Req, Resp>,
    state: &SessionState,
    grpc_request: GrpcReq,
    grpc_header_builder: external_services::grpc_client::GrpcHeadersUcsBuilderFinal,
    handler: F,
) -> RouterResult<RouterData<T, Req, Resp>>
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
    Fut: std::future::Future<Output = RouterResult<(RouterData<T, Req, Resp>, GrpcResp)>> + Send,
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

    router_result
}

#[derive(serde::Serialize)]
pub struct ComparisonData {
    pub hyperswitch_data: Secret<serde_json::Value>,
    pub unified_connector_service_data: Secret<serde_json::Value>,
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
