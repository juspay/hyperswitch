use api_models::payment_method_backfill::{
    BackfillError, PaymentMethodDataBackfillResponse, RevenueRecoveryBackfillRequest,
};
use hyperswitch_domain_models::api::ApplicationResponse;
use redis_interface::{RedisConnectionPool, types::RedisKey};
use router_env::{instrument, tracing, logger};

use crate::{
    core::errors::{self, RouterResult},
    routes::SessionState,
    types::domain,
    connection,
};

use diesel_models::payment_attempt::PaymentAttemptUpdateInternal;
use hyperswitch_domain_models::{payments::payment_attempt::{PaymentAttemptInterface, PaymentAttemptUpdate}, behaviour::Conversion};
use common_utils::{pii, id_type};
use masking::PeekInterface;

#[instrument(skip_all)]
pub async fn payment_method_data_backfill(
    state: SessionState,
    records: Vec<RevenueRecoveryBackfillRequest>,
    merchant_context: domain::MerchantContext,
    profile: domain::Profile,
) -> RouterResult<ApplicationResponse<PaymentMethodDataBackfillResponse>> {
    let mut processed_records = 0;
    let mut failed_records = 0;

    // Process each record
    for record in records {
        match process_payment_method_record(&state, &record, &merchant_context, &profile).await {
            Ok(_) => {
                processed_records += 1;
                logger::info!("Successfully processed record with cnpTxnId: {}", record.cnp_txn_id);
            }
            Err(e) => {
                failed_records += 1;
                logger::error!(
                    "Payment method backfill failed: cnp_txn_id={}, customer_id={}, card_type={}, error={}",
                    record.cnp_txn_id,
                    record.customer_id_resp, 
                    record.type_field,
                    e
                );
            }
        }
    }

    let response = PaymentMethodDataBackfillResponse {
        processed_records,
        failed_records,
    };

    logger::info!(
        "Payment method backfill completed - Processed: {}, Failed: {}",
        processed_records,
        failed_records
    );

    Ok(ApplicationResponse::Json(response))
}

async fn process_payment_method_record(
    state: &SessionState,
    record: &RevenueRecoveryBackfillRequest,
    merchant_context: &domain::MerchantContext,
    profile: &domain::Profile,
) -> Result<(), BackfillError> {
    logger::info!(
        "Processing backfill record - cnp_txn_id: {}, type_field: {}, card_type: {}",
        record.cnp_txn_id,
        record.type_field,
        record.card_type,
    );

    // Map card type from CSV
    let card_type = map_card_type(&record.type_field)?;
    logger::info!("Mapped card type from '{}' to '{}'", record.type_field, card_type);

    // Update database
    update_payment_attempt_card_type(state, &record.cnp_txn_id, &card_type, merchant_context, profile).await?;

    // Update Redis if token exists and is valid
    if !record.token.is_empty() && record.token != "nan" {
        logger::info!("Updating Redis for customer: {}, token: {}", record.customer_id_resp, record.token);
        let redis_conn = state.store.get_redis_conn()
            .map_err(|e| BackfillError::RedisError(format!("Failed to get Redis connection: {}", e)))?;
        update_redis_token_card_type(
            &redis_conn,
            &record.customer_id_resp,
            &record.token,
            &card_type,
        )
        .await?;
    } else {
        logger::info!("Skipping Redis update - token is empty or 'nan': {}", record.token);
    }

    logger::info!("Successfully completed processing for cnp_txn_id: {}", record.cnp_txn_id);
    Ok(())
}

fn map_card_type(raw_type: &str) -> Result<String, BackfillError> {
    match raw_type {
        "Debit" => Ok("debit".to_string()),
        "Credit" => Ok("credit".to_string()),
        _ if raw_type.is_empty() || raw_type == "nan" => {
            Err(BackfillError::InvalidCardType(
                "Missing card type".to_string(),
            ))
        }
        _ => Err(BackfillError::InvalidCardType(raw_type.to_string())),
    }
}

async fn update_payment_attempt_card_type(
    state: &SessionState,
    cnp_txn_id: &str,
    card_type: &str,
    merchant_context: &domain::MerchantContext,
    profile: &domain::Profile,
) -> Result<(), BackfillError> {
    let key_manager_state = &state.into();

    logger::info!(
        "Starting payment attempt update - profile_id: {}, connector_payment_id: {}, target_card_type: {}",
        profile.get_id().get_string_repr(),
        cnp_txn_id,
        card_type
    );

    // Find the payment attempt by connector payment ID
    let payment_attempt = state.store
        .find_payment_attempt_by_profile_id_connector_payment_id(
            key_manager_state,
            merchant_context.get_merchant_key_store(),
            profile.get_id(),
            cnp_txn_id,
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .map_err(|e| BackfillError::DatabaseError(format!("Failed to find payment attempt: {}", e)))?;

    logger::info!(
        "Found payment attempt - id: {}, payment_id: {}, merchant_id: {}, payment_method_subtype: {:?}, existing_payment_method_data: {:?}",
        payment_attempt.id.get_string_repr(),
        payment_attempt.payment_id.get_string_repr(),
        payment_attempt.merchant_id.get_string_repr(),
        payment_attempt.payment_method_subtype,
        payment_attempt.payment_method_data.as_ref().map(|d| d.peek())
    );

    // Parse existing payment_method_data or create new structure
    let existing_data = payment_attempt
        .payment_method_data
        .as_ref()
        .and_then(|data| serde_json::from_value::<serde_json::Value>(data.peek().clone()).ok());
    
    logger::info!("Existing payment_method_data parsed: {:?}", existing_data);

    let mut payment_method_data = existing_data.unwrap_or_else(|| serde_json::json!({}));

    // Log the current state before modification
    logger::info!("Current payment_method_data before update: {}", serde_json::to_string_pretty(&payment_method_data).unwrap_or_default());

    // Upsert the card object with the new card_type
    if let Some(card_obj) = payment_method_data.get_mut("card") {
        logger::info!("Found existing card object: {}", serde_json::to_string(card_obj).unwrap_or_default());
        if let Some(card_map) = card_obj.as_object_mut() {
            let old_card_type = card_map.get("card_type").and_then(|v| v.as_str());
            logger::info!("Updating card_type from {:?} to {}", old_card_type, card_type);
            card_map.insert("card_type".to_string(), serde_json::Value::String(card_type.to_string()));
        }
    } else {
        logger::info!("No existing card object found, creating new one with card_type: {}", card_type);
        // Create new card object if it doesn't exist
        if let Some(data_obj) = payment_method_data.as_object_mut() {
            data_obj.insert("card".to_string(), serde_json::json!({
                "card_type": card_type
            }));
        } else {
            // If payment_method_data is not an object, create a new object structure
            payment_method_data = serde_json::json!({
                "card": {
                    "card_type": card_type
                }
            });
        }
    }

    logger::info!("Updated payment_method_data: {}", serde_json::to_string_pretty(&payment_method_data).unwrap_or_default());


        let payment_attempt_update = PaymentAttemptUpdate::PaymentMethodDataUpdate {
            payment_method_data: Some(pii::SecretSerdeValue::new(payment_method_data.clone())),
            updated_by: "payment_method_backfill".to_string(),
        };

        logger::info!("Attempting to update payment attempt with PaymentMethodDataUpdate");

        let updated_payment_attempt = state.store
            .update_payment_attempt(
                key_manager_state,
                merchant_context.get_merchant_key_store(),
                payment_attempt,
                payment_attempt_update,
                merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .map_err(|e| BackfillError::DatabaseError(format!("Failed to update payment attempt: {}", e)))?;

        logger::info!(
            "Successfully updated payment attempt - id: {}, updated_payment_method_data: {:?}",
            updated_payment_attempt.id.get_string_repr(),
            updated_payment_attempt.payment_method_data.as_ref().map(|d| d.peek())
        );

    logger::info!(
        "Successfully completed payment attempt update for connector_payment_id: {} with card_type: {}",
        cnp_txn_id,
        card_type
    );

    Ok(())
}

async fn update_redis_token_card_type(
    redis_conn: &RedisConnectionPool,
    customer_id: &str,
    token: &str,
    card_type: &str,
) -> Result<(), BackfillError> {
    let redis_key: RedisKey = format!("customer:{}:tokens", customer_id).into();

    // Get existing token data
    let existing_data: Option<String> = redis_conn
        .get_hash_field(&redis_key, token)
        .await
        .map_err(|e| BackfillError::RedisError(format!("Failed to get token data: {}", e)))?;

    if let Some(data) = existing_data {
        // Parse existing JSON
        let mut token_data: serde_json::Value = serde_json::from_str(&data)
            .map_err(|e| BackfillError::RedisError(format!("Failed to parse token data: {}", e)))?;

        // Update card_type in payment_processor_token_details
        if let Some(processor_details) = token_data
            .get_mut("payment_processor_token_details")
            .and_then(|v| v.as_object_mut())
        {
            processor_details.insert("card_type".to_string(), serde_json::Value::String(card_type.to_string()));

            // Save updated data back to Redis
            let updated_data = serde_json::to_string(&token_data)
                .map_err(|e| BackfillError::RedisError(format!("Failed to serialize token data: {}", e)))?;

            let hash_map = std::collections::HashMap::from([(token.to_string(), updated_data)]);
            redis_conn
                .set_hash_fields(&redis_key, hash_map, None)
                .await
                .map_err(|e| BackfillError::RedisError(format!("Failed to update token data: {}", e)))?;

            logger::info!("Updated Redis token data for customer: {}, token: {}", customer_id, token);
        } else {
            logger::warn!("Token data structure invalid for customer: {}, token: {}", customer_id, token);
        }
    } else {
        logger::warn!("Token not found in Redis for customer: {}, token: {}", customer_id, token);
    }

    Ok(())
}
