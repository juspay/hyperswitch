use std::marker::PhantomData;

use api_models::merchant_connector_webhook_management::{
    ConnectorWebhookRegisterRequest as ApiConnectorWebhookRegisterRequest,
    ConnectorWebhookResponse, ConnectorWebhookScope, RegisterConnectorWebhookResponse, Scope,
    ScopeIdentifier, ScopeType, WebhookRegistrationResult,
};
use common_utils::ext_traits::{Encode, ValueExt};
use error_stack::{ensure, Report, ResultExt};
use hyperswitch_domain_models::{
    connector_endpoints::Connectors,
    router_request_types::merchant_connector_webhook_management::ConnectorWebhookRegisterRequest,
};
use hyperswitch_interfaces::api::ConnectorSpecifications;
use hyperswitch_masking::{ExposeInterface, Secret};
use router_env::tracing::{self, instrument};

use crate::{
    consts,
    core::errors::{ConnectorErrorExt, RouterResult},
    errors, types,
    types::{
        api::ConnectorData, domain,
        ConnectorWebhookGenerateSecretRequest as ConnectorWebhookGenerateSecretData,
        ConnectorWebhookGenerateSecretResponse, ConnectorWebhookGenerateSecretRouterData,
        ConnectorWebhookRegisterRouterData, ErrorResponse,
    },
    SessionState,
};

fn is_webhook_auto_config_supported(
    connector_name: types::Connector,
) -> RouterResult<Option<bool>> {
    let connector_config =
        connector_configs::connector::ConnectorConfig::get_connector_config(connector_name)
            .map_err(|err| {
                Report::from(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(format!("Failed to retrieve connector configuration: {err}"))
            })?;

    Ok(connector_config
        .and_then(|toml| toml.connector_webhook_register_details)
        .map(|details| details.webhook_auto_configuration_supported))
}

#[cfg(feature = "v2")]
pub async fn construct_webhook_register_router_data(
    _state: &SessionState,
    _merchant_connector_account: domain::MerchantConnectorAccount,
    _webhook_register_request: ConnectorWebhookRegisterRequest,
) -> RouterResult<types::ConnectorWebhookRegisterRouterData> {
    Err(errors::ApiErrorResponse::NotImplemented {
        message: errors::NotImplementedMessage::Reason(
            "Webhook registration not yet implemented for v2".to_string(),
        ),
    }
    .into())
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_webhook_register_router_data<'a>(
    state: &'a SessionState,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    webhook_register_request: ConnectorWebhookRegisterRequest,
) -> RouterResult<ConnectorWebhookRegisterRouterData> {
    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let (payment_method, payment_method_type) = match &webhook_register_request.scope {
        ScopeIdentifier::PaymentMethodType(pmt) => {
            let pm = common_enums::PaymentMethod::from(*pmt);
            (pm, Some(*pmt))
        }
        _ => (common_enums::PaymentMethod::default(), None),
    };

    Ok(types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_connector_account.merchant_id.clone(),
        customer_id: None,
        connector_customer: None,
        connector: merchant_connector_account.connector_name.clone(),
        payment_id: consts::IRRELEVANT_PAYMENT_INTENT_ID.to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: consts::IRRELEVANT_PAYMENT_ATTEMPT_ID.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method,
        payment_method_type,
        connector_auth_type: auth_type,
        description: None,
        address: types::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata().clone(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request: webhook_register_request,
        response: Err(ErrorResponse::default()),
        connector_request_reference_id: consts::IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID
            .to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: None,
        refund_id: None,
        payment_method_status: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
        payout_id: None,
        customer_document_details: None,
        feature_data: None,
        sender_payment_instrument_id: None,
    })
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_generate_secret_router_data<'a>(
    state: &'a SessionState,
    merchant_connector_account: &domain::MerchantConnectorAccount,
    connector_webhook_id: String,
) -> RouterResult<ConnectorWebhookGenerateSecretRouterData> {
    let request = ConnectorWebhookGenerateSecretData {
        connector_webhook_id,
    };

    let auth_type = merchant_connector_account
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get connector auth details for HMAC generation")?;

    Ok(types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_connector_account.merchant_id.clone(),
        customer_id: None,
        connector_customer: None,
        connector: merchant_connector_account.connector_name.clone(),
        payment_id: consts::IRRELEVANT_PAYMENT_INTENT_ID.to_owned(),
        tenant_id: state.tenant.tenant_id.clone(),
        attempt_id: consts::IRRELEVANT_PAYMENT_ATTEMPT_ID.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method: common_enums::PaymentMethod::default(),
        payment_method_type: None,
        connector_auth_type: auth_type,
        description: None,
        address: types::PaymentAddress::default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata().clone(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request,
        response: Err(ErrorResponse::default()),
        connector_request_reference_id: consts::IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID
            .to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: None,
        refund_id: None,
        payment_method_status: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        l2_l3_data: None,
        minor_amount_capturable: None,
        authorized_amount: None,
        payout_id: None,
        customer_document_details: None,
        feature_data: None,
        sender_payment_instrument_id: None,
    })
}

/// Recursively merges `patch` into `base`.
/// Nested objects are merged key-by-key; non-object values are overwritten.
fn deep_merge_json_values(base: &mut serde_json::Value, patch: serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                match base_map.get_mut(&key) {
                    Some(base_value) => deep_merge_json_values(base_value, patch_value),
                    None => {
                        base_map.insert(key, patch_value);
                    }
                }
            }
        }
        (base, patch) => *base = patch,
    }
}

/// Checks whether a stored scope matches the given scope identifier.
fn scope_matches_identifier(identifier: &ScopeIdentifier, scope: &ConnectorWebhookScope) -> bool {
    match (identifier, scope) {
        (ScopeIdentifier::NotSpecific, ConnectorWebhookScope::NotSpecific) => true,
        (
            ScopeIdentifier::PaymentMethodType(pmt),
            ConnectorWebhookScope::PaymentMethodType { value },
        ) => pmt == value,
        (ScopeIdentifier::EventType(evt), ConnectorWebhookScope::EventType { value }) => {
            evt == value
        }
        _ => false,
    }
}

#[cfg(feature = "v1")]
/// Builds the DB update for connector webhook registration.
///
/// - `registration_entries`: `(connector_webhook_id, scope)` pairs that fully succeeded
///   and must be persisted.
/// - `scopes_to_remove`: scopes whose existing stored entries must be removed before
///   inserting the new ones. This covers partially-failed scopes (so they are not
///   persisted) and scopes that are being re-registered (so stale webhook IDs are
///   replaced rather than duplicated).
pub fn construct_connector_webhook_registration_details(
    merchant_connector_account: &domain::MerchantConnectorAccount,
    registration_entries: Vec<(String, ScopeIdentifier)>,
    generated_secret: Option<Secret<String>>,
    metadata_patches: Vec<common_utils::pii::SecretSerdeValue>,
    scopes_to_remove: &[ScopeIdentifier],
) -> RouterResult<domain::MerchantConnectorAccountUpdate> {
    // Only touch stored registration details if we have something to add or remove.
    let has_registration_changes = !registration_entries.is_empty() || !scopes_to_remove.is_empty();
    let performed_removals = !scopes_to_remove.is_empty();

    let connector_webhook_registration_details = if !has_registration_changes {
        None
    } else {
        // Start from existing registrations, or an empty object if none exist.
        let mut connector_webhook_registration_details = merchant_connector_account
            .get_connector_webhook_registration_details()
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

        let map = connector_webhook_registration_details
            .as_object_mut()
            .ok_or(errors::ApiErrorResponse::InternalServerError)?;

        // Step 1: Remove stored entries for scopes that failed or are being replaced.
        // Legacy (event_type based) and unparseable entries are kept unchanged.
        if performed_removals {
            let scopes_to_remove = scopes_to_remove.to_vec();
            map.retain(|_, entry_value| {
                let stored: Result<StoredConnectorWebhookEntry, _> =
                    serde_json::from_value(entry_value.clone());
                match stored {
                    Ok(StoredConnectorWebhookEntry::New(scope)) => !scopes_to_remove
                        .iter()
                        .any(|identifier| scope_matches_identifier(identifier, &scope)),
                    Ok(StoredConnectorWebhookEntry::Legacy(_)) => true,
                    Err(_) => true,
                }
            });
        }

        // Step 2: Persist the new webhook IDs for fully-successful scopes.
        for (connector_webhook_id, scope) in registration_entries {
            let entry_value = match &scope {
                ScopeIdentifier::NotSpecific => {
                    serde_json::to_value(ConnectorWebhookScope::NotSpecific)
                }
                ScopeIdentifier::PaymentMethodType(pmt) => {
                    serde_json::to_value(ConnectorWebhookScope::PaymentMethodType { value: *pmt })
                }
                ScopeIdentifier::EventType(evt) => {
                    serde_json::to_value(ConnectorWebhookScope::EventType { value: *evt })
                }
            }
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

            map.insert(connector_webhook_id, entry_value);
        }

        if map.is_empty() {
            // If we intentionally removed every entry, return an empty object so the
            // DB update is still written. Returning None here would skip the update
            // and leave stale registrations behind.
            performed_removals.then(|| serde_json::Value::Object(Default::default()))
        } else {
            Some(connector_webhook_registration_details)
        }
    };

    // Merge connector-returned metadata patches into the existing metadata,
    // creating a fresh metadata object if none exists yet.
    let metadata = if metadata_patches.is_empty() {
        None
    } else {
        let mut merged_metadata = merchant_connector_account
            .get_metadata()
            .map(|secret| secret.expose().clone())
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

        for patch in metadata_patches {
            deep_merge_json_values(&mut merged_metadata, patch.expose().clone());
        }

        Some(common_utils::pii::SecretSerdeValue::new(merged_metadata))
    };

    // If the connector requires a generated webhook secret, persist it while preserving
    // any existing additional_secret already stored for this merchant connector.
    let connector_webhook_details = generated_secret
        .map(|secret| {
            build_connector_webhook_details_with_secret(merchant_connector_account, secret)
        })
        .transpose()?;

    Ok(
        domain::MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
            connector_webhook_registration_details,
            connector_webhook_details,
            metadata,
        },
    )
}

#[cfg(feature = "v1")]
/// Encodes the generated webhook secret into `MerchantConnectorWebhookDetails`,
/// preserving any existing `additional_secret` instead of overwriting it.
fn build_connector_webhook_details_with_secret(
    merchant_connector_account: &domain::MerchantConnectorAccount,
    secret: Secret<String>,
) -> RouterResult<common_utils::pii::SecretSerdeValue> {
    let existing_additional_secret = merchant_connector_account
        .connector_webhook_details
        .as_ref()
        .and_then(|details| {
            details
                .clone()
                .expose()
                .parse_value::<api_models::admin::MerchantConnectorWebhookDetails>(
                    "MerchantConnectorWebhookDetails",
                )
                .inspect_err(|err| {
                    router_env::logger::warn!(
                        ?err,
                        "Failed to parse existing MerchantConnectorWebhookDetails; \
                         dropping additional_secret while persisting generated secret"
                    );
                })
                .ok()
                .and_then(|parsed| parsed.additional_secret)
        });

    let merged = api_models::admin::MerchantConnectorWebhookDetails {
        merchant_secret: secret,
        additional_secret: existing_additional_secret,
    };

    let serialized = merged
        .encode_to_value()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Failed to serialize MerchantConnectorWebhookDetails with generated secret",
        )?;

    Ok(Secret::new(serialized))
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn validate_webhook_registration_request(
    connector_data: &ConnectorData,
    webhook_register_request: ApiConnectorWebhookRegisterRequest,
    connectors: &Connectors,
) -> RouterResult<()> {
    let is_supported = is_webhook_auto_config_supported(connector_data.connector_name)?;

    ensure!(
        is_supported.unwrap_or(false),
        errors::ApiErrorResponse::FlowNotSupported {
            flow: "Webhook Registration".to_string(),
            connector: connector_data.connector_name.to_string(),
        }
    );

    let scope = webhook_register_request.scope.as_ref().ok_or_else(|| {
        Report::new(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("webhook registration scope is missing after request deserialization")
    })?;

    let plan = connector_data
        .connector
        .get_webhook_registration_plan(scope, connectors)
        .to_webhook_configuration_failed_response()?;

    if plan.is_empty() {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Webhook registration is not supported for the requested scope".to_string(),
        }
        .into());
    }

    Ok(())
}

pub fn determine_scope_type(scope: &Scope) -> ScopeType {
    match scope {
        Scope::NotSpecific => ScopeType::NotSpecific,
        Scope::PaymentMethodTypes(_) => ScopeType::PaymentMethodType,
        Scope::EventTypes(_) => ScopeType::EventType,
    }
}

pub fn extract_requested_identifiers(scope: &Scope) -> Vec<ScopeIdentifier> {
    match scope {
        Scope::NotSpecific => vec![ScopeIdentifier::NotSpecific],
        Scope::PaymentMethodTypes(pmts) => pmts
            .iter()
            .map(|pmt| ScopeIdentifier::PaymentMethodType(*pmt))
            .collect(),
        Scope::EventTypes(evts) => evts
            .iter()
            .map(|evt| ScopeIdentifier::EventType(*evt))
            .collect(),
    }
}

#[cfg(feature = "v1")]
#[allow(deprecated)]
/// Builds the response for the connector webhook register API.
///
/// Legacy requests receive a flattened single-status response. Non-legacy requests
/// receive the full list of requested scope identifiers and one result per connector call.
pub fn construct_connector_webhook_registration_response(
    results: Vec<WebhookRegistrationResult>,
    scope_type: ScopeType,
    requested: Vec<ScopeIdentifier>,
    generate_secret_response: Option<&ConnectorWebhookGenerateSecretResponse>,
    is_legacy_request: bool,
) -> RouterResult<RegisterConnectorWebhookResponse> {
    // Extract the optional webhook-secret generation status/error to include in the response.
    let (secret_generation_status, secret_error) = generate_secret_response
        .map(|resp| {
            let secret_error =
                (resp.error_code.is_some() || resp.error_message.is_some()).then(|| {
                    api_models::merchant_connector_webhook_management::WebhookSecretErrorDetails {
                        code: resp.error_code.clone(),
                        message: resp.error_message.clone(),
                    }
                });
            (Some(resp.status), secret_error)
        })
        .unwrap_or((None, None));

    // Legacy callers only see one top-level status/id/error.
    // Non-legacy callers receive the detailed per-call results below.
    let (connector_webhook_id, webhook_registration_status, error_code, error_message) =
        if is_legacy_request {
            if let Some(success) = results
                .iter()
                .find(|result| result.connector_webhook_id.is_some())
            {
                (
                    success.connector_webhook_id.clone(),
                    Some(success.status),
                    None,
                    None,
                )
            } else if let Some(failure) = results.iter().find(|result| result.error.is_some()) {
                (
                    None,
                    Some(failure.status),
                    failure.error.as_ref().map(|error| error.code.clone()),
                    failure.error.as_ref().map(|error| error.message.clone()),
                )
            } else {
                (
                    None,
                    results.first().map(|result| result.status),
                    None,
                    None,
                )
            }
        } else {
            (None, None, None, None)
        };

    let event_type = if is_legacy_request {
        match scope_type {
            ScopeType::NotSpecific => Some(common_enums::ConnectorWebhookEventType::AllEvents),
            ScopeType::EventType => requested.iter().find_map(|identifier| match identifier {
                ScopeIdentifier::EventType(event) => Some(
                    common_enums::ConnectorWebhookEventType::SpecificEvent(*event),
                ),
                _ => None,
            }),
            ScopeType::PaymentMethodType => None,
        }
    } else {
        None
    };

    let (scope_type, requested, results) = if is_legacy_request {
        (None, None, None)
    } else {
        (Some(scope_type), Some(requested), Some(results))
    };

    Ok(RegisterConnectorWebhookResponse {
        event_type,
        connector_webhook_id,
        webhook_registration_status,
        error_code,
        error_message,
        secret_generation_status,
        secret_error,
        scope_type,
        requested,
        results,
    })
}
/// Legacy shape stored in `connector_webhook_registration_details` before scope-based
/// registration was introduced.
#[derive(serde::Deserialize)]
struct LegacyConnectorWebhookData {
    event_type: common_enums::ConnectorWebhookEventType,
}

/// Stored connector webhook registration entry. Supports both the legacy event_type shape
/// and the new scope-based shape so that the list API works for records created before and
/// after this change. New registrations always write the `New` shape.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum StoredConnectorWebhookEntry {
    Legacy(LegacyConnectorWebhookData),
    New(ConnectorWebhookScope),
}

#[cfg(feature = "v1")]
#[allow(deprecated)]
/// Converts the persisted `connector_webhook_registration_details` blob into a list response.
///
/// Supports both the old event_type-based entries and the new scope-based entries so that
/// webhooks registered before this change continue to be listed correctly.
pub fn get_connector_webhook_list_response(
    register_webhook_response: &Option<serde_json::Value>,
) -> RouterResult<Vec<ConnectorWebhookResponse>> {
    use std::collections::HashMap;

    let webhook_map: HashMap<String, StoredConnectorWebhookEntry> = match register_webhook_response
    {
        Some(webhook_response) => serde_json::from_value(webhook_response.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)?,
        None => HashMap::new(),
    };

    let webhooks = webhook_map
        .into_iter()
        .map(|(connector_webhook_id, entry)| match entry {
            StoredConnectorWebhookEntry::Legacy(legacy) => {
                ConnectorWebhookResponse::legacy(connector_webhook_id, legacy.event_type)
            }
            StoredConnectorWebhookEntry::New(scope) => {
                ConnectorWebhookResponse::scope_based(connector_webhook_id, scope)
            }
        })
        .collect();

    Ok(webhooks)
}
