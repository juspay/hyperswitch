//! UPI Intent Payment Flow Integration
//!
//! This module integrates the UPI Intent payment flow with the existing Hyperswitch payment flow.
//! It connects the UPI Intent orchestrator with the core payment processing system.

use api_models::payments::NextActionData;
use common_enums::enums;
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::payment_method_data::UpiData;
use router_env::{instrument, logger, tracing};

use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments::helpers,
    },
    routes::SessionState,
    services,
    types::{
        self,
        api::{self, ConnectorData, GetToken},
        domain,
        storage::{enums, MandateUpdate, MerchantStorageScheme},
    },
    utils::OptionExt,
};

use super::upi_intent::get_sdk_params;

/// Implement the UPI Intent payment flow
#[instrument(skip(state))]
pub async fn process_upi_intent_payment(
    state: &SessionState,
    connector_data: &ConnectorData,
    router_data: types::PaymentsSessionRouterData,
    storage_scheme: MerchantStorageScheme,
) -> RouterResult<types::PaymentsSessionRouterData> {
    logger::info!("Processing UPI Intent payment");

    // Check if this is a UPI Intent payment
    let payment_method_data = router_data.request.get_payment_method_data();
    if !matches!(payment_method_data, domain::payments::PaymentMethodData::Upi(UpiData::UpiIntent(_))) {
        return Ok(router_data);
    }

    // Generate SDK parameters for UPI Intent
    let router_data = get_sdk_params(
        state,
        connector_data,
        &router_data,
        storage_scheme,
    )
    .await?;

    // Process response and handle next actions
    process_sdk_response(state, &router_data)
}

/// Process the SDK response and determine next action
fn process_sdk_response(
    state: &SessionState,
    router_data: &types::PaymentsSessionRouterData,
) -> RouterResult<types::PaymentsSessionRouterData> {
    let response = match router_data.response {
        Ok(ref response) => response,
        Err(_) => return Ok(router_data.clone()),
    };

    match response {
        types::PaymentsResponseData::SessionResponse {
            session_token: types::api::SessionToken::Razorpay(razorpay_response),
            ..
        } => {
            if let Some(next_action) = razorpay_response.next_action_data {
                handle_next_action(state, router_data, next_action)
            } else {
                Ok(router_data.clone())
            }
        }
        _ => Ok(router_data.clone()),
    }
}

/// Handle the next action from the SDK response
fn handle_next_action(
    _state: &SessionState,
    router_data: &types::PaymentsSessionRouterData,
    next_action: api_models::payments::NextActionData,
) -> RouterResult<types::PaymentsSessionRouterData> {
    match next_action {
        api_models::payments::NextActionData::InvokeUpiIntentSdk { sdk_uri, .. } => {
            logger::info!("Handling UPI Intent SDK invocation: {}", sdk_uri);

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
        _ => Ok(router_data.clone()),
    }
}

/// Check if a payment method is supported by UPI Intent
pub fn is_upi_intent_payment(
    payment_method: Option<enums::PaymentMethod>,
    payment_method_type: Option<enums::PaymentMethodType>,
) -> bool {
    matches!(payment_method, Some(enums::PaymentMethod::Upi))
        && matches!(payment_method_type, Some(enums::PaymentMethodType::UpiIntent))
}

/// Update mandate status when needed
#[instrument(skip(state))]
pub async fn handle_mandate_status_update(
    state: &SessionState,
    payment_id: &str,
    mandate_id: Option<&str>,
    status: enums::MandateStatus,
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

/// Update payment attempt status when needed
#[instrument(skip(state))]
pub async fn handle_payment_status_update(
    state: &SessionState,
    payment_id: &str,
    status: enums::AttemptStatus,
    storage_scheme: MerchantStorageScheme,
) -> RouterResult<()> {
    let payment_attempt = state
        .store
        .find_payment_attempt_by_payment_id(payment_id, "", storage_scheme)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let update = storage::PaymentAttemptUpdate::StatusUpdate {
        status,
        updated_at: common_utils::date_time::now(),
    };

    state
        .store
        .update_payment_attempt(payment_attempt, update)
        .await
        .change_context(errors::ApiErrorResponse::PaymentUpdateFailed)?;

    Ok(())
}

/// Convert UPI Intent payment data to router data
pub fn convert_upi_intent_payment_data(
    data: &types::PaymentsSessionData,
) -> RouterResult<types::PaymentsAuthorizeRouterData> {
    let mut authorize_data = types::PaymentsAuthorizeRouterData::from(data.clone());

    // Add UPI Intent-specific data
    if let types::api::PaymentMethodData::Upi(UpiData::UpiIntent(upi_intent_data)) = &data.payment_method_data {
        authorize_data.request.vpa_id = upi_intent_data.vpa_id.clone();
        authorize_data.request.upi_app = upi_intent_data.upi_app.clone();
        authorize_data.request.mandate_reg_ref_id = upi_intent_data.mandate_reg_ref_id.clone();
    }

    Ok(authorize_data)
}