use crate::core::errors;
use crate::types::domain;
use crate::types::storage::revenue_recovery_redis_operation::PaymentProcessorTokenStatus;
use error_stack::ResultExt;

/// Check if Account Updater should be triggered for a payment
pub async fn should_trigger_account_updater(
    state: &crate::SessionState,
    payment_method_type: &str,
    payment_method_sub_type: &str,
    business_profile: &domain::Profile,
    connector: &str,
    token_error_code: &Option<String>,
) -> crate::core::errors::RouterResult<bool> {
    // Check if Account Updater is enabled for this business profile
    if !super::config::is_account_updater_enabled(business_profile) {
        return Ok(false);
    }

    // Check if payment method is eligible for Account Updater
    if !super::config::is_eligible_for_account_updater(payment_method_type, payment_method_sub_type) {
        return Ok(false);
    }

    // Check if connector supports Account Updater
    if connector != "worldpayvantiv" {
        return Ok(false);
    }

    // Check if token has an error code that triggers Account Updater
    if let Some(error_code) = token_error_code {
        let au_error_codes = super::config::load_account_updater_error_codes(state, connector).await?;
        if au_error_codes.contains(error_code) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if a payment method is card-based
pub fn is_card_based_payment_method(payment_method_type: &str) -> bool {
    payment_method_type == "card"
}

/// Check if a payment is eligible for Account Updater based on transaction type
pub fn is_eligible_for_account_updater(
    payment_method_type: &str,
    payment_method_sub_type: &str,
    is_mit: bool,
    customer_agreement_stored_card_usage: Option<&str>,
) -> bool {
    // Only card-based payments are eligible
    if !is_card_based_payment_method(payment_method_type) {
        return false;
    }

    // MIT (Merchant Initiated Transaction) - always eligible
    if is_mit {
        return true;
    }

    // CIT (Customer Initiated Transaction) - only if stored card usage is "subsequent"
    if let Some(usage) = customer_agreement_stored_card_usage {
        return usage == "subsequent";
    }

    false
}

/// Get error codes from a token
pub fn get_token_error_codes(token: &PaymentProcessorTokenStatus) -> Option<Vec<String>> {
    token.error_code.as_ref().map(|code| vec![code.clone()])
}

/// Process Account Updater response and update tokens
pub async fn process_account_updater_response(
    state: &crate::SessionState,
    updated_instrument: &crate::connectors::worldpayvantiv::transformers::UpdatedPaymentInstrument,
    old_token_id: &str,
    customer_id: &str,
    attempt_id: &str,
) -> crate::core::errors::RouterResult<Option<PaymentProcessorTokenStatus>> {
    // Check if there's an updated payment instrument
    if updated_instrument.token.is_none() && 
       updated_instrument.card_number.is_none() && 
       updated_instrument.exp_date.is_none() {
        // No update available
        return Ok(None);
    }

    // Create new token from Account Updater response
    let new_token = super::token_management::create_new_token_from_au_response(
        state,
        updated_instrument,
        old_token_id,
        customer_id,
        attempt_id,
    ).await?;

    // Deactivate the old token
    let new_token_id = new_token.payment_processor_token_details.payment_processor_token.clone();
    super::token_management::deactivate_old_token(
        state,
        old_token_id,
        &new_token_id,
        customer_id,
    ).await?;

    Ok(Some(new_token))
}

/// Check if Account Updater should be enabled based on business rules
pub fn should_enable_account_updater(
    business_profile: &domain::Profile,
    payment_method_type: &str,
    payment_method_sub_type: &str,
    is_mit: bool,
    customer_agreement_stored_card_usage: Option<&str>,
) -> bool {
    // Check if Account Updater is enabled for this business profile
    if !super::config::is_account_updater_enabled(business_profile) {
        return false;
    }

    // Check if payment is eligible for Account Updater
    is_eligible_for_account_updater(
        payment_method_type,
        payment_method_sub_type,
        is_mit,
        customer_agreement_stored_card_usage,
    )
}
