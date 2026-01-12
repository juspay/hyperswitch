//! UPI Mandate Handling
//!
//! This module implements the mandate handling for UPI In-App payments, including
//! One-Time Mandates (OTM) and recurring mandates.

use api_models::payments::NextActionData;
use common_enums::enums;
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{GatewayAuthParams, UpiData, UpiIntentData},
    router_data::{SessionTokenizingRouterData, SessionTokenizingSessionData},
    types,
};
use router_env::{instrument, logger, tracing};

use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments::{helpers, CallConnectorAction},
    },
    db::StorageInterface,
    routes::SessionState,
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

/// Handle UPI mandate registration
#[instrument(skip(state))]
pub async fn handle_mandate_registration<T: MandateBehaviour>(
    state: &SessionState,
    connector_data: &ConnectorData,
    router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    upi_intent_data: UpiIntentData,
    storage_scheme: MerchantStorageScheme,
) -> RouterResult<types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>> {
    logger::debug!("Handling UPI mandate registration");

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

    // Save authentication parameters
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

    // Process connector response
    process_mandate_registration_response(connector_response, is_otm(mandate_details))
}

/// Determine if this is a One-Time Mandate
fn is_otm(mandate_details: Option<&api_models::payments::MandateDetails>) -> bool {
    mandate_details.map_or(false, |details| {
        matches!(details.mandate_type, Some(api_models::payments::MandateType::SingleUse))
    })
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

/// Save authentication parameters
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
    Ok(())
}

/// Calculate collection by date
fn calculate_collect_by_date() -> String {
    // Calculate date 7 days from now (typical UPI mandate collection window)
    use chrono::{Duration, Utc};
    let future_date = Utc::now() + Duration::days(7);
    future_date.format("%Y%m%d").to_string()
}

/// Process mandate registration response
fn process_mandate_registration_response<T>(
    connector_response: types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    is_otm: bool,
) -> RouterResult<types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>> {
    let response = match connector_response.response {
        Ok(ref response) => response,
        Err(_) => return Ok(connector_response),
    };

    match response {
        types::PaymentsResponseData::SessionResponse {
            session_token: api_models::payments::SessionToken::Razorpay(razorpay_response),
            ..
        } => {
            if let Some(next_action) = razorpay_response.next_action_data.clone() {
                handle_next_action(connector_response, next_action, is_otm)
            } else {
                Ok(connector_response)
            }
        }
        _ => Ok(connector_response),
    }
}

/// Handle the next action from the SDK response
fn handle_next_action<T>(
    router_data: types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    next_action: api_models::payments::NextActionData,
    is_otm: bool,
) -> RouterResult<types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>> {
    match next_action {
        api_models::payments::NextActionData::InvokeUpiIntentSdk { sdk_uri, .. } => {
            logger::info!("Handling UPI Intent SDK invocation for mandate: {}", sdk_uri);

            let response = types::PaymentsResponseData::SessionResponse {
                session_token: api_models::payments::SessionToken::UpiIntentSdk(Box::new(
                    api_models::payments::UpiIntentSdkResponse {
                        sdk_uri,
                        next_action: api_models::payments::SdkNextAction {
                            next_action: api_models::payments::NextActionCall::Confirm,
                        },
                    },
                )),
                connector_transaction_id: router_data.response.clone().map(|r| r.connector_transaction_id()),
            };

            types::RouterData::try_from(types::ResponseRouterData {
                response: Ok(response),
                data: router_data.clone(),
                http_code: router_data.http_code,
            })
            .change_context(errors::ApiErrorResponse::InternalServerError)
        }
        _ => Ok(router_data),
    }
}

/// Update mandate status
#[instrument(skip(state))]
pub async fn update_mandate_status(
    state: &SessionState,
    payment_id: &str,
    mandate_id: Option<&str>,
    status: storage_enums::MandateStatus,
    storage_scheme: MerchantStorageScheme,
) -> RouterResult<()> {
    if let Some(mandate_id) = mandate_id {
        let mandate = state
            .store
            .find_mandate_by_merchant_id_mandate_id(&common_utils::id_type::MerchantId::default(), mandate_id, storage_scheme)
            .await
            .ok();

        if let Some(mandate) = mandate {
            let update = MandateUpdate::StatusUpdate {
                mandate_status: status,
            };

            state
                .store
                .update_mandate_by_merchant_id_mandate_id(
                    &common_utils::id_type::MerchantId::default(),
                    mandate_id,
                    update,
                    mandate,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;
        }
    }

    Ok(())
}

/// Interpret mandate-specific transaction status
pub fn interpret_mandate_status(
    gateway_response: &types::PaymentsResponseData,
    is_mandate: bool,
    is_otm: bool,
) -> Option<storage_enums::MandateStatus> {
    if !is_mandate {
        return None;
    }

    match gateway_response {
        types::PaymentsResponseData::SessionResponse {
            session_token: api_models::payments::SessionToken::Razorpay(razorpay_response),
            ..
        } => {
            match razorpay_response.next_action_data.as_ref() {
                Some(api_models::payments::NextActionData::InvokeUpiIntentSdk { .. }) => {
                    Some(storage_enums::MandateStatus::Pending)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Validate mandate parameters
pub fn validate_mandate_parameters(
    sdk_params: &NextActionData,
    is_otm: bool,
) -> RouterResult<()> {
    if is_otm {
        if let api_models::payments::NextActionData::InvokeUpiIntentSdk { sdk_uri, .. } = sdk_params {
            // Validate UPI-specific parameters for OTM
            if !sdk_uri.contains("pa=") || !sdk_uri.contains("pn=") {
                return Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Missing required UPI parameters in SDK URI".to_string(),
                }
                .into());
            }
        }
    }

    Ok(())
}

/// Check if transaction status indicates pending
pub fn is_pending_status(
    gateway_response: &types::PaymentsResponseData,
    is_mandate: bool,
    is_otm: bool,
) -> bool {
    // Special handling for OTM (One-Time Mandates)
    if is_mandate && is_otm {
        if let types::PaymentsResponseData::SessionResponse {
            session_token: api_models::payments::SessionToken::Razorpay(razorpay_response),
            ..
        } = gateway_response
        {
            if let Some(api_models::payments::NextActionData::InvokeUpiIntentSdk { .. }) =
                razorpay_response.next_action_data
            {
                return true;
            }
        }
    }

    // General transaction status interpretation
    match gateway_response {
        types::PaymentsResponseData::SessionResponse {
            session_token: api_models::payments::SessionToken::Razorpay(razorpay_response),
            ..
        } => match razorpay_response.next_action_data.as_ref() {
            Some(api_models::payments::NextActionData::InvokeUpiIntentSdk { .. }) => true,
            _ => false,
        },
        _ => false,
    }
}

/// Check if transaction is successful
pub fn is_transaction_successful(gateway_response: &types::PaymentsResponseData) -> bool {
    match gateway_response {
        types::PaymentsResponseData::SessionResponse {
            session_token: api_models::payments::SessionToken::Razorpay(razorpay_response),
            ..
        } => match razorpay_response.next_action_data.as_ref() {
            None => true,
            Some(api_models::payments::NextActionData::InvokeUpiIntentSdk { .. }) => false,
            _ => false,
        },
        _ => false,
    }
}

/// Check if transaction was not found
pub fn is_transaction_not_found(gateway_response: &types::PaymentsResponseData) -> bool {
    match gateway_response {
        types::PaymentsResponseData::SessionResponse {
            session_token: api_models::payments::SessionToken::Razorpay(razorpay_response),
            ..
        } => match razorpay_response.next_action_data.as_ref() {
            Some(api_models::payments::NextActionData::InvokeUpiIntentSdk { .. }) => false,
            _ => false,
        },
        _ => false,
    }
}

/// Check if mandate is active
pub fn is_mandate_active(mandate_status: storage_enums::MandateStatus) -> bool {
    matches!(
        mandate_status,
        storage_enums::MandateStatus::Active
            | storage_enums::MandateStatus::Pending
            | storage_enums::MandateStatus::Processing
    )
}

/// Check if mandate is in a terminal state
pub fn is_mandate_terminal(mandate_status: storage_enums::MandateStatus) -> bool {
    matches!(
        mandate_status,
        storage_enums::MandateStatus::Revoked
            | storage_enums::MandateStatus::Failed
            | storage_enums::MandateStatus::Inactive
    )
}

/// Extract mandate reference ID
pub fn extract_mandate_ref_id(
    payment_method_data: &domain::payments::PaymentMethodData,
) -> Option<String> {
    match payment_method_data {
        domain::payments::PaymentMethodData::Upi(UpiData::UpiIntent(upi_intent_data)) => {
            upi_intent_data.mandate_reg_ref_id.clone()
        }
        _ => None,
    }
}

/// Build Razorpay mandate request
#[cfg(feature = "connector_razorpay")]
fn build_razorpay_mandate_request<T: MandateBehaviour>(
    _router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    _upi_intent_data: &UpiIntentData,
    _mandate_reg_ref_id: &str,
) -> RouterResult<services::Request> {
    // TODO: Implement Razorpay-specific mandate request
    Err(errors::ApiErrorResponse::NotImplemented.into())
}

/// Build ICICI mandate request
#[cfg(feature = "connector_icici")]
fn build_icici_mandate_request<T: MandateBehaviour>(
    _router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    _upi_intent_data: &UpiIntentData,
    _mandate_reg_ref_id: &str,
) -> RouterResult<services::Request> {
    // TODO: Implement ICICI-specific mandate request with encryption
    Err(errors::ApiErrorResponse::NotImplemented.into())
}

/// Build EaseBuzz mandate request
#[cfg(feature = "connector_easebuzz")]
fn build_easebuzz_mandate_request<T: MandateBehaviour>(
    _router_data: &types::RouterData<T, types::PaymentsSessionData, types::PaymentsResponseData>,
    _upi_intent_data: &UpiIntentData,
    _mandate_reg_ref_id: &str,
) -> RouterResult<services::Request> {
    // TODO: Implement EaseBuzz-specific mandate request
    Err(errors::ApiErrorResponse::NotImplemented.into())
}