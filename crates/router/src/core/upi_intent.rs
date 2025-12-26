//! UPI Intent Payment Flow Orchestrator
//!
//! This module implements the UPI In-App payment flow based on the Haskell specification.
//! It handles generating SDK parameters for UPI Intent payments, including support for
//! mandates (One-Time and Recurring).

use api_models::payments::{NextActionData, WaitScreenInstructions};
use common_enums::enums;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{GatewayAuthParams, UpiData, UpiIntentData, UpiPaymentSource},
    router_data::{RouterData, SessionTokenizingRouterData},
    types,
};
use hyperswitch_interfaces::api::ConnectorSessionTokenInfo;
use masking::Secret;
use router_env::{instrument, logger, tracing};
use url::Url;

use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payments::{helpers, CallConnectorAction},
    },
    db::StorageInterface,
    routes::{metrics, SessionState},
    services,
    types::{
        self,
        api::{ConnectorData, GetToken},
        domain,
        storage::{
            self,
            enums::{MerchantStorageScheme, TxnStatus},
            MandateUpdate,
        },
    },
    utils::OptionExt,
};

/// Generate SDK parameters for UPI Intent payments
/// Equivalent to Haskell's `getSdkParams` function
#[instrument(skip(state))]
pub async fn get_sdk_params<T: MandateBehaviour>(
    state: &SessionState,
    connector_data: &ConnectorData,
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    storage_scheme: MerchantStorageScheme,
) -> RouterResult<types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>> {
    logger::debug!("Generating UPI Intent SDK parameters for connector: {}", connector_data.connector_name);

    let payment_method_data = router_data.request.get_payment_method_data();
    let payment_method = router_data.request.get_payment_method();
    let payment_method_type = router_data.request.get_payment_method_type();

    // Check if this is a UPI Intent payment
    if !matches!(payment_method, Some(enums::PaymentMethod::Upi))
        || !matches!(payment_method_type, Some(enums::PaymentMethodType::UpiIntent))
    {
        return Err(errors::ApiErrorResponse::PaymentMethodNotSupported.into());
    }

    // Extract UPI-specific details
    let upi_intent_data = match payment_method_data {
        hyperswitch_domain_models::payments::PaymentMethodData::Upi(UpiData::UpiIntent(intent_data)) => intent_data,
        _ => return Err(errors::ApiErrorResponse::InvalidPaymentMethodData.into()),
    };

    // Check if this is a mandate registration flow
    let is_mandate_registration = router_data.request.get_setup_mandate_details().is_some()
        || router_data.request.get_mandate_id().is_some();

    if is_mandate_registration {
        // Handle UPI Intent mandate registration
        let mandate_response = handle_mandate_registration(
            state,
            connector_data,
            router_data,
            upi_intent_data,
            storage_scheme,
        )
        .await?;

        // Process mandate-specific response
        process_mandate_response(mandate_response, is_otm(mandate_details)).await
    } else {
        handle_direct_upi_intent(state, connector_data, router_data, upi_intent_data, storage_scheme).await
    }
}

/// Handle UPI Intent payments for mandate registration
#[instrument(skip(state))]
async fn handle_mandate_registration<T: MandateBehaviour>(
    state: &SessionState,
    connector_data: &ConnectorData,
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: UpiIntentData,
    storage_scheme: MerchantStorageScheme,
) -> RouterResult<types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>> {
    logger::debug!("Handling UPI Intent mandate registration");

    let mandate_details = router_data
        .request
        .get_setup_mandate_details()
        .or_else(|| router_data.request.get_mandate_details());

    if mandate_details.is_none() {
        return Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "mandate_details".to_string(),
        }
        .into());
    }

    // Generate unique mandate registration reference ID
    let mandate_reg_ref_id = generate_mandate_reg_ref_id(router_data);

    // Build gateway-specific request payload
    let gateway_request = build_mandate_intent_request(
        connector_data,
        router_data,
        &upi_intent_data,
        &mandate_reg_ref_id,
    )?;

    // Save authentication parameters in SecondFactor equivalent
    save_auth_params_in_second_factor(
        state,
        router_data,
        &upi_intent_data,
        &mandate_reg_ref_id,
        storage_scheme,
    )
    .await?;

    // Make connector API call
    let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        types::api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > = connector_data.connector.get_connector_integration();

    let connector_response = services::execute_connector_processing_step(
        state,
        connector_integration,
        router_data,
        CallConnectorAction::Trigger,
        Some(gateway_request),
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    // Process connector response and generate SDK params
    process_mandate_registration_response(connector_response)
}

/// Handle direct UPI Intent payments (non-mandate)
#[instrument(skip(state))]
async fn handle_direct_upi_intent<T: MandateBehaviour>(
    state: &SessionState,
    connector_data: &ConnectorData,
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: UpiIntentData,
    storage_scheme: MerchantStorageScheme,
) -> RouterResult<types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>> {
    logger::debug!("Handling direct UPI Intent payment");

    // Determine payment source structure
    let payment_source = determine_payment_source(&connector_data.connector_name, &upi_intent_data)?;

    // Check for S2S disabled configuration
    let is_s2s_disabled = check_s2s_disabled(state, connector_data, router_data).await?;

    // Build gateway-specific request
    let gateway_request = if is_s2s_disabled {
        build_ajax_request(connector_data, router_data, &upi_intent_data)?
    } else {
        build_s2s_request(connector_data, router_data, &upi_intent_data)?
    };

    // Make connector API call
    let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        types::api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > = connector_data.connector.get_connector_integration();

    let connector_response = services::execute_connector_processing_step(
        state,
        connector_integration,
        router_data,
        CallConnectorAction::Trigger,
        Some(gateway_request),
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    // Generate SDK params from gateway response
    process_direct_upi_response(connector_response)
}

/// Generate unique mandate registration reference ID
fn generate_mandate_reg_ref_id<T>(router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>) -> String {
    let payment_id = router_data.get_payment_id();
    format!("mdtreg_{}", payment_id)
}

/// Build gateway-specific mandate intent request
fn build_mandate_intent_request<T: MandateBehaviour>(
    connector_data: &ConnectorData,
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: &UpiIntentData,
    mandate_reg_ref_id: &str,
) -> RouterResult<services::Request> {
    match connector_data.connector_name {
        #[cfg(feature = "connector_razorpay")]
        types::Connector::Razorpay => build_razorpay_mandate_request(router_data, upi_intent_data, mandate_reg_ref_id),
        #[cfg(feature = "connector_icici")]
        types::Connector::IciciUpi => build_icici_mandate_request(router_data, upi_intent_data, mandate_reg_ref_id),
        #[cfg(feature = "connector_easebuzz")]
        types::Connector::EaseBuzz => build_easebuzz_mandate_request(router_data, upi_intent_data, mandate_reg_ref_id),
        _ => Err(errors::ApiErrorResponse::UnsupportedPaymentMethod.into()),
    }
}

/// Determine payment source based on gateway
fn determine_payment_source(
    gateway_name: &str,
    upi_intent_data: &UpiIntentData,
) -> RouterResult<UpiPaymentSource> {
    // Gateways requiring JSON payment source (per Haskell documentation)
    const PAYMENT_SOURCE_AS_JSON: &[&str] = &["ICICI", "EASEBUZZ"];

    if PAYMENT_SOURCE_AS_JSON.contains(&gateway_name.to_uppercase().as_str()) {
        Ok(UpiPaymentSource {
            upi_identifier: "UPI_PAY".to_string(),
            upi_app: upi_intent_data.upi_app.clone(),
            payer_vpa: upi_intent_data.vpa_id.clone(),
        })
    } else {
        Ok(UpiPaymentSource {
            upi_identifier: upi_intent_data
                .vpa_id
                .as_ref()
                .map(|vpa| vpa.expose_masked().clone())
                .unwrap_or_default(),
            upi_app: upi_intent_data.upi_app.clone(),
            payer_vpa: upi_intent_data.vpa_id.clone(),
        })
    }
}

/// Check if S2S is disabled for this merchant/gateway
async fn check_s2s_disabled<T: MandateBehaviour>(
    state: &SessionState,
    connector_data: &ConnectorData,
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
) -> RouterResult<bool> {
    // Check merchant-level feature flags
    let merchant_id = router_data.get_merchant_id();
    let profile_id = router_data.get_profile_id().unwrap_or_default();

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(merchant_id, &profile_id)
        .await
        .change_context(errors::ApiErrorResponse::MerchantNotFound)?;

    Ok(false) // TODO: Implement actual feature flag check
}

/// Save authentication parameters in SecondFactor equivalent
async fn save_auth_params_in_second_factor<T: MandateBehaviour>(
    state: &SessionState,
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: &UpiIntentData,
    mandate_reg_ref_id: &str,
    storage_scheme: MerchantStorageScheme,
) -> RouterResult<()> {
    let auth_params = GatewayAuthParams {
        version: Some("v2".to_string()),
        tr: Some(mandate_reg_ref_id.to_string()),
        collect_by_date: Some(calculate_collect_by_date()),
        additional_params: None,
    };

    // TODO: Save to SecondFactor equivalent storage
    // This would save gateway-specific authentication parameters

    Ok(())
}

/// Calculate collection by date
fn calculate_collect_by_date() -> String {
    // Calculate date 7 days from now (typical UPI mandate collection window)
    use chrono::{Duration, Utc};
    let future_date = Utc::now() + Duration::days(7);
    future_date.format("%Y%m%d").to_string()
}

/// Process mandate registration response and generate SDK params
fn process_mandate_registration_response<T>(
    connector_response: types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
) -> RouterResult<types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>> {
    // TODO: Implement response processing based on Haskell documentation
    // This should handle:
    // 1. Parsing gateway response
    // 2. Extracting deep link/signed QR data
    // 3. Generating SdkParams with WaitScreenInstructions
    // 4. Handling error cases

    Err(errors::ApiErrorResponse::NotImplemented.into())
}

/// Process direct UPI Intent response and generate SDK params
fn process_direct_upi_response<T>(
    connector_response: types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
) -> RouterResult<types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>> {
    // TODO: Implement direct UPI response processing
    // This should handle generating SDK params for direct UPI payments

    Err(errors::ApiErrorResponse::NotImplemented.into())
}

/// Placeholder functions for gateway-specific request builders
#[cfg(feature = "connector_razorpay")]
fn build_razorpay_mandate_request<T: MandateBehaviour>(
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: &UpiIntentData,
    mandate_reg_ref_id: &str,
) -> RouterResult<services::Request> {
    // TODO: Build Razorpay mandate request
    Err(errors::ApiErrorResponse::NotImplemented.into())
}

#[cfg(feature = "connector_icici")]
fn build_icici_mandate_request<T: MandateBehaviour>(
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: &UpiIntentData,
    mandate_reg_ref_id: &str,
) -> RouterResult<services::Request> {
    // TODO: Build ICICI mandate request with encryption
    Err(errors::ApiErrorResponse::NotImplemented.into())
}

#[cfg(feature = "connector_easebuzz")]
fn build_easebuzz_mandate_request<T: MandateBehaviour>(
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: &UpiIntentData,
    mandate_reg_ref_id: &str,
) -> RouterResult<services::Request> {
    // TODO: Build EaseBuzz mandate request
    Err(errors::ApiErrorResponse::NotImplemented.into())
}

fn build_ajax_request<T: MandateBehaviour>(
    connector_data: &ConnectorData,
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: &UpiIntentData,
) -> RouterResult<services::Request> {
    // TODO: Build AJAX request for S2S disabled flow
    Err(errors::ApiErrorResponse::NotImplemented.into())
}

fn build_s2s_request<T: MandateBehaviour>(
    connector_data: &ConnectorData,
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: &UpiIntentData,
) -> RouterResult<services::Request> {
    // TODO: Build S2S request
    Err(errors::ApiErrorResponse::NotImplemented.into())
}

/// Check if transaction status indicates pending
/// Gateway-specific implementation (per Haskell documentation)
pub fn is_pending_status(
    gateway_response: &types::PaymentsResponseData,
    is_mandate: bool,
    is_otm: bool,
) -> bool {
    // TODO: Implement gateway-specific pending status checking
    // For OTM (One-Time Mandates), special handling:
    // - CREATE-INITIATED is AUTHORIZED, not PENDING
    // Other logic based on gateway response codes
    false
}

/// Check if transaction is successful
pub fn is_transaction_successful(gateway_response: &types::PaymentsResponseData) -> bool {
    // TODO: Implement gateway-specific success checking
    false
}

/// Check if transaction was not found
pub fn is_transaction_not_found(gateway_response: &types::PaymentsResponseData) -> bool {
    // TODO: Implement gateway-specific not found checking
    false
}

/// Extract VPA from payment source
pub fn fetch_vpa_from_payment_source(
    payment_source: &api_models::payments::UpiPaymentSource,
) -> Option<Secret<String, common_utils::pii::UpiVpaMaskingStrategy>> {
    payment_source.payer_vpa.clone()
}

/// Validate SDK params for One-Time Mandate (OTM)
pub fn validate_sdk_params_for_otm(
    sdk_params: &NextActionData,
    mandate_type: &common_enums::MandateType,
) -> RouterResult<()> {
    if !matches!(mandate_type, common_enums::MandateType::SingleUse) {
        return Ok(());
    }

    // TODO: Validate OTM-specific parameters
    // Per Haskell documentation, one-time mandates require:
    // - purpose field
    // - block flag
    // - revokable flag

    Ok(())
}