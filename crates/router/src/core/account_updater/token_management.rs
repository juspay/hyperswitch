use crate::core::errors;
use crate::types::domain;
use crate::types::storage::revenue_recovery_redis_operation::{
    AccountUpdaterChange, AccountUpdaterChangeType, PaymentProcessorTokenStatus,
};
use error_stack::ResultExt;
use time::PrimitiveDateTime;

/// Filter active tokens from a list of tokens
pub fn filter_active_tokens(tokens: Vec<PaymentProcessorTokenStatus>) -> Vec<PaymentProcessorTokenStatus> {
    tokens.into_iter()
        .filter(|token| token.is_active)
        .collect()
}

/// Deactivate an old token and mark it as replaced
pub async fn deactivate_old_token(
    state: &crate::SessionState,
    old_token_id: &str,
    new_token_id: &str,
    customer_id: &str,
) -> crate::core::errors::RouterResult<()> {
    let mut token_map = crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::get_connector_customer_payment_processor_tokens(
        state,
        customer_id,
    ).await?;

    if let Some(token) = token_map.get_mut(old_token_id) {
        token.is_active = false;
        token.replaced_by_token_id = Some(new_token_id.to_string());
        token.account_updater_changes.push(AccountUpdaterChange {
            change_type: AccountUpdaterChangeType::TokenDeactivated,
            old_value: Some("true".to_string()),
            new_value: Some("false".to_string()),
            changed_at: time::now(),
            change_reason: "Token replaced by Account Updater".to_string(),
        });
        token.account_updater_updated_at = Some(time::now());
    }

    crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::update_or_add_connector_customer_payment_processor_tokens(
        state,
        customer_id,
        token_map,
    ).await?;

    Ok(())
}

/// Create a new token from Account Updater response
pub async fn create_new_token_from_au_response(
    state: &crate::SessionState,
    updated_instrument: &crate::connectors::worldpayvantiv::transformers::UpdatedPaymentInstrument,
    old_token_id: &str,
    customer_id: &str,
    attempt_id: &str,
) -> crate::core::errors::RouterResult<PaymentProcessorTokenStatus> {
    let now = time::now();
    
    // Create new token with updated information
    let new_token = PaymentProcessorTokenStatus {
        payment_processor_token_details: crate::types::storage::revenue_recovery_redis_operation::PaymentProcessorTokenDetails {
            payment_processor_token: updated_instrument.token.as_ref().map(|t| t.peek().to_string()).unwrap_or_else(|| old_token_id.to_string()),
            expiry_month: updated_instrument.exp_date.as_ref().map(|d| {
                let exp = d.peek();
                if exp.len() >= 2 {
                    Secret::new(exp[..2].to_string())
                } else {
                    Secret::new(exp.to_string())
                }
            }),
            expiry_year: updated_instrument.exp_date.as_ref().map(|d| {
                let exp = d.peek();
                if exp.len() >= 4 {
                    Secret::new(exp[2..].to_string())
                } else {
                    Secret::new(exp.to_string())
                }
            }),
            card_issuer: None,
            last_four_digits: updated_instrument.card_number.as_ref().map(|c| {
                let card = c.peek();
                if card.len() >= 4 {
                    card[card.len()-4..].to_string()
                } else {
                    card.to_string()
                }
            }),
            card_network: None,
            card_type: None,
        },
        inserted_by_attempt_id: attempt_id.parse().change_context(errors::ApiErrorResponse::InternalServerError)?,
        error_code: None,
        daily_retry_history: std::collections::HashMap::new(),
        scheduled_at: None,
        is_hard_decline: None,
        is_active: true,
        replaced_by_token_id: None,
        account_updater_changes: vec![AccountUpdaterChange {
            change_type: AccountUpdaterChangeType::TokenActivated,
            old_value: None,
            new_value: Some("true".to_string()),
            changed_at: now,
            change_reason: "Token created from Account Updater response".to_string(),
        }],
        retry_count: 0,
        last_retry_at: None,
        max_retry_count: 3,
        account_updater_updated_at: Some(now),
    };

    // Store the new token
    let mut token_map = crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::get_connector_customer_payment_processor_tokens(
        state,
        customer_id,
    ).await?;

    let new_token_id = new_token.payment_processor_token_details.payment_processor_token.clone();
    token_map.insert(new_token_id.clone(), new_token.clone());

    crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::update_or_add_connector_customer_payment_processor_tokens(
        state,
        customer_id,
        token_map,
    ).await?;

    Ok(new_token)
}

/// Handle card number change from Account Updater
pub async fn handle_card_number_change(
    state: &crate::SessionState,
    token_id: &str,
    customer_id: &str,
    old_card_number: &str,
    new_card_number: &str,
) -> crate::core::errors::RouterResult<PaymentProcessorTokenStatus> {
    let mut token_map = crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::get_connector_customer_payment_processor_tokens(
        state,
        customer_id,
    ).await?;

    if let Some(token) = token_map.get_mut(token_id) {
        token.account_updater_changes.push(AccountUpdaterChange {
            change_type: AccountUpdaterChangeType::CardNumberChanged,
            old_value: Some(old_card_number.to_string()),
            new_value: Some(new_card_number.to_string()),
            changed_at: time::now(),
            change_reason: "Card number updated by Account Updater".to_string(),
        });
        token.account_updater_updated_at = Some(time::now());
    }

    crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::update_or_add_connector_customer_payment_processor_tokens(
        state,
        customer_id,
        token_map,
    ).await?;

    Ok(token_map.get(token_id).unwrap().clone())
}

/// Increment retry count for a token
pub async fn increment_retry_count(
    state: &crate::SessionState,
    token_id: &str,
    customer_id: &str,
) -> crate::core::errors::RouterResult<PaymentProcessorTokenStatus> {
    let mut token_map = crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::get_connector_customer_payment_processor_tokens(
        state,
        customer_id,
    ).await?;

    if let Some(token) = token_map.get_mut(token_id) {
        token.retry_count += 1;
        token.last_retry_at = Some(time::now());
        token.account_updater_changes.push(AccountUpdaterChange {
            change_type: AccountUpdaterChangeType::RetryCountIncremented,
            old_value: Some((token.retry_count - 1).to_string()),
            new_value: Some(token.retry_count.to_string()),
            changed_at: time::now(),
            change_reason: "Retry count incremented".to_string(),
        });
    }

    crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::update_or_add_connector_customer_payment_processor_tokens(
        state,
        customer_id,
        token_map,
    ).await?;

    Ok(token_map.get(token_id).unwrap().clone())
}

/// Get the token that replaced the given token
pub async fn get_replaced_by_token(
    state: &crate::SessionState,
    token_id: &str,
    customer_id: &str,
) -> crate::core::errors::RouterResult<Option<String>> {
    let token_map = crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::get_connector_customer_payment_processor_tokens(
        state,
        customer_id,
    ).await?;

    Ok(token_map.get(token_id)
        .and_then(|token| token.replaced_by_token_id.clone()))
}

/// Find the current active token for a customer
pub async fn find_current_active_token(
    state: &crate::SessionState,
    customer_id: &str,
) -> crate::core::errors::RouterResult<Option<PaymentProcessorTokenStatus>> {
    let token_map = crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::get_connector_customer_payment_processor_tokens(
        state,
        customer_id,
    ).await?;

    Ok(token_map.values()
        .find(|token| token.is_active)
        .cloned())
}

/// Resolve the correct token for payment processing (handles token replacement)
pub async fn resolve_token_for_payment(
    state: &crate::SessionState,
    original_token_id: &str,
    customer_id: &str,
) -> crate::core::errors::RouterResult<Option<PaymentProcessorTokenStatus>> {
    let token_map = crate::types::storage::revenue_recovery_redis_operation::RedisTokenManager::get_connector_customer_payment_processor_tokens(
        state,
        customer_id,
    ).await?;

    // Start with the original token
    let mut current_token_id = original_token_id.to_string();
    
    // Follow the replacement chain to find the current active token
    loop {
        if let Some(token) = token_map.get(&current_token_id) {
            if token.is_active {
                return Ok(Some(token.clone()));
            } else if let Some(replaced_by) = &token.replaced_by_token_id {
                current_token_id = replaced_by.clone();
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }
    }
}
