use crate::core::errors;
use crate::types::domain;
use error_stack::ResultExt;

/// Load Account Updater error codes for a specific connector from the configs table
pub async fn load_account_updater_error_codes(
    state: &crate::SessionState,
    connector: &str,
) -> crate::core::errors::RouterResult<Vec<String>> {
    let config_key = format!("account_updater_error_codes_{}", connector);
    
    let config = state
        .store
        .find_config_by_key(&config_key)
        .await?;
    
    match config {
        Some(config) => {
            let error_codes: Vec<String> = serde_json::from_value(config.value)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse Account Updater error codes")?;
            Ok(error_codes)
        }
        None => {
            // Return default error codes for Vantiv
            Ok(vec![
                "110".to_string(), // Insufficient Funds
                "111".to_string(), // Invalid Card Number
                "112".to_string(), // Expired Card
                "113".to_string(), // Invalid CVV
                "114".to_string(), // Card Declined
                "115".to_string(), // Processing Error
            ])
        }
    }
}

/// Check if Account Updater is enabled for a business profile
pub fn is_account_updater_enabled(business_profile: &domain::Profile) -> bool {
    business_profile.is_account_updater_enabled.unwrap_or(false)
}

/// Check if a payment method is card-based
pub fn is_card_based_payment_method(payment_method_type: &str) -> bool {
    payment_method_type == "card"
}

/// Check if a payment is eligible for Account Updater
pub fn is_eligible_for_account_updater(
    payment_method_type: &str,
    payment_method_sub_type: &str,
) -> bool {
    // Only card-based payments are eligible for Account Updater
    is_card_based_payment_method(payment_method_type) && 
    (payment_method_sub_type == "credit" || payment_method_sub_type == "debit")
}

/// Get error codes from a token's error history
pub fn get_token_error_codes(error_code: &Option<String>) -> Option<Vec<String>> {
    error_code.as_ref().map(|code| vec![code.clone()])
}
