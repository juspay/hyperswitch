use std::collections::HashMap;

use api_models::revenue_recovery_data_backfill::{
    BackfillError, ComprehensiveCardData, GetRedisDataQuery, RedisDataResponse, RedisKeyType,
    RevenueRecoveryBackfillRequest, RevenueRecoveryDataBackfillResponse, ScheduledAtUpdate,
    UnlockStatusResponse, UpdateTokenStatusRequest, UpdateTokenStatusResponse,
};
use common_enums::{CardNetwork, PaymentMethodType};
use common_utils::id_type;
use error_stack::ResultExt;
use hyperswitch_domain_models::api::ApplicationResponse;
use masking::ExposeInterface;
use router_env::{instrument, logger};
use time::{macros::format_description, Date};

use crate::{
    connection,
    core::errors::{self, RouterResult},
    routes::SessionState,
    types::{domain, storage},
};

pub async fn revenue_recovery_data_backfill(
    state: SessionState,
    records: Vec<RevenueRecoveryBackfillRequest>,
    cutoff_datetime: Option<time::PrimitiveDateTime>,
) -> RouterResult<ApplicationResponse<RevenueRecoveryDataBackfillResponse>> {
    let mut processed_records = 0;
    let mut failed_records = 0;

    // Process each record
    for record in records {
        match process_payment_method_record(&state, &record, cutoff_datetime).await {
            Ok(_) => {
                processed_records += 1;
                logger::info!(
                    "Successfully processed record with connector customer id: {}",
                    record.customer_id_resp
                );
            }
            Err(e) => {
                failed_records += 1;
                logger::error!(
                    "Payment method backfill failed: customer_id={}, error={}",
                    record.customer_id_resp,
                    e
                );
            }
        }
    }

    let response = RevenueRecoveryDataBackfillResponse {
        processed_records,
        failed_records,
    };

    logger::info!(
        "Revenue recovery data backfill completed - Processed: {}, Failed: {}",
        processed_records,
        failed_records
    );

    Ok(ApplicationResponse::Json(response))
}

pub async fn unlock_connector_customer_status_handler(
    state: SessionState,
    connector_customer_id: String,
    payment_id: id_type::GlobalPaymentId,
) -> RouterResult<ApplicationResponse<UnlockStatusResponse>> {
    let unlocked = storage::revenue_recovery_redis_operation::
        RedisTokenManager::unlock_connector_customer_status(&state, &connector_customer_id, &payment_id)
        .await
        .map_err(|e| {
            logger::error!(
                "Failed to unlock connector customer status for {}: {:?}",
                connector_customer_id,
                e
            );
            match e.current_context() {
                errors::StorageError::RedisError(redis_error) => {
                    match redis_error.current_context() {
                        storage_impl::errors::RedisError::DeleteFailed => {
                            // This indicates the payment_id doesn't own the lock
                            errors::ApiErrorResponse::InvalidPaymentIdProvided {
                                resource: format!("The Status token for connector costumer id:- {} is locked by different PaymentIntent ID", connector_customer_id)
                            }
                        }
                        _ => {
                            // Other Redis errors - infrastructure issue
                            errors::ApiErrorResponse::InternalServerError
                        }
                    }
                }
                errors::StorageError::ValueNotFound(_) => {
                    // Lock doesn't exist
                    errors::ApiErrorResponse::GenericNotFoundError {
                        message: format!("Lock not found for connector customer id: {}", connector_customer_id)
                    }
                }
                _ => {
                    // Fallback for other storage errors
                    errors::ApiErrorResponse::InternalServerError
                }
            }
        })?;

    let response = UnlockStatusResponse { unlocked };

    logger::info!(
        "Unlock operation completed for connector customer {}: {}",
        connector_customer_id,
        unlocked
    );

    Ok(ApplicationResponse::Json(response))
}
pub async fn get_redis_data(
    state: SessionState,
    connector_customer_id: &str,
    key_type: &RedisKeyType,
) -> RouterResult<ApplicationResponse<RedisDataResponse>> {
    match storage::revenue_recovery_redis_operation::RedisTokenManager::get_redis_key_data_raw(
        &state,
        connector_customer_id,
        key_type,
    )
    .await
    {
        Ok((exists, ttl_seconds, data)) => {
            let response = RedisDataResponse {
                exists,
                ttl_seconds,
                data,
            };

            logger::info!(
                "Retrieved Redis data for connector customer {}, exists={}, ttl={}",
                connector_customer_id,
                exists,
                ttl_seconds
            );

            Ok(ApplicationResponse::Json(response))
        }
        Err(error) => Err(
            error.change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: format!(
                    "Redis data not found for connector customer id:- '{}'",
                    connector_customer_id
                ),
            }),
        ),
    }
}

pub async fn redis_update_additional_details_for_revenue_recovery(
    state: SessionState,
    request: UpdateTokenStatusRequest,
) -> RouterResult<ApplicationResponse<UpdateTokenStatusResponse>> {
    // Get existing token
    let existing_token = storage::revenue_recovery_redis_operation::
        RedisTokenManager::get_payment_processor_token_using_token_id(
            &state,
            &request.connector_customer_id,
            &request.payment_processor_token.clone().expose(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve existing token data")?;

    // Check if token exists
    let mut token_status = existing_token.ok_or_else(|| {
        error_stack::Report::new(errors::ApiErrorResponse::GenericNotFoundError {
            message: format!(
                "Token '{:?}' not found for connector customer id:- '{}'",
                request.payment_processor_token, request.connector_customer_id
            ),
        })
    })?;

    let mut updated_fields = Vec::new();

    // Handle scheduled_at update
    match request.scheduled_at {
        Some(ScheduledAtUpdate::SetToDateTime(dt)) => {
            // Field provided with datetime - update schedule_at field with datetime
            token_status.scheduled_at = Some(dt);
            updated_fields.push(format!("scheduled_at: {}", dt));
            logger::info!(
                "Set scheduled_at to '{}' for token '{:?}'",
                dt,
                request.payment_processor_token
            );
        }
        Some(ScheduledAtUpdate::SetToNull) => {
            // Field provided with "null" variable - set schedule_at field to null
            token_status.scheduled_at = None;
            updated_fields.push("scheduled_at: set to null".to_string());
            logger::info!(
                "Set scheduled_at to null for token '{:?}'",
                request.payment_processor_token
            );
        }
        None => {
            // Field not provided - we don't update schedule_at field
            logger::debug!("scheduled_at not provided in request - leaving unchanged");
        }
    }

    // Update is_hard_decline field
    request.is_hard_decline.map(|is_hard_decline| {
        token_status.is_hard_decline = Some(is_hard_decline);
        updated_fields.push(format!("is_hard_decline: {}", is_hard_decline));
    });

    // Update error_code field
    request.error_code.as_ref().map(|error_code| {
        token_status.error_code = Some(error_code.clone());
        updated_fields.push(format!("error_code: {}", error_code));
    });

    // Update Redis with modified token
    let mut tokens_map = HashMap::new();
    tokens_map.insert(
        request.payment_processor_token.clone().expose(),
        token_status,
    );

    storage::revenue_recovery_redis_operation::
        RedisTokenManager::update_or_add_connector_customer_payment_processor_tokens(
            &state,
            &request.connector_customer_id,
            tokens_map,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update token status in Redis")?;

    let updated_fields_str = if updated_fields.is_empty() {
        "no fields were updated".to_string()
    } else {
        updated_fields.join(", ")
    };

    let response = UpdateTokenStatusResponse {
        updated: true,
        message: format!(
            "Successfully updated token '{:?}' for connector customer '{}'. Updated fields: {}",
            request.payment_processor_token, request.connector_customer_id, updated_fields_str
        ),
    };

    logger::info!(
        "Updated token status for connector customer {}, token: {:?}",
        request.connector_customer_id,
        request.payment_processor_token
    );

    Ok(ApplicationResponse::Json(response))
}

async fn process_payment_method_record(
    state: &SessionState,
    record: &RevenueRecoveryBackfillRequest,
    cutoff_datetime: Option<time::PrimitiveDateTime>,
) -> Result<(), BackfillError> {
    // Build comprehensive card data from CSV record
    let card_data = match build_comprehensive_card_data(record) {
        Ok(data) => data,
        Err(e) => {
            logger::warn!(
                "Failed to build card data for connector customer id: {}, error: {}.",
                record.customer_id_resp,
                e
            );
            ComprehensiveCardData {
                card_type: Some("card".to_string()),
                card_exp_month: None,
                card_exp_year: None,
                card_network: None,
                card_issuer: None,
                card_issuing_country: None,
                daily_retry_history: None,
                is_active: None,
                account_update_history: None,
            }
        }
    };
    logger::info!(
        "Built comprehensive card data - card_type: {:?}, exp_month: {}, exp_year: {}, network: {:?}, issuer: {:?}, country: {:?}, daily_retry_history: {:?}",
        card_data.card_type,
        card_data.card_exp_month.as_ref().map(|_| "**").unwrap_or("None"),
        card_data.card_exp_year.as_ref().map(|_| "**").unwrap_or("None"),
        card_data.card_network,
        card_data.card_issuer,
        card_data.card_issuing_country,
        card_data.daily_retry_history
    );

    // Update Redis if token exists and is valid
    match record.token.as_ref().map(|token| token.clone().expose()) {
        Some(token) if !token.is_empty() => {
            logger::info!("Updating Redis for customer: {}", record.customer_id_resp,);

            storage::revenue_recovery_redis_operation::
            RedisTokenManager::update_redis_token_with_comprehensive_card_data(
                state,
                &record.customer_id_resp,
                &token,
                &card_data,
                cutoff_datetime,
            )
            .await
            .map_err(|e| {
                logger::error!("Redis update failed: {}", e);
                BackfillError::RedisError(format!("Token not found in Redis: {}", e))
            })?;
        }
        _ => {
            logger::info!(
                "Skipping Redis update - token is missing, empty or 'nan': {:?}",
                record.token
            );
        }
    }

    logger::info!(
        "Successfully completed processing for connector customer id: {}",
        record.customer_id_resp
    );
    Ok(())
}

/// Parse daily retry history from CSV
fn parse_daily_retry_history(
    json_str: Option<&str>,
) -> Option<HashMap<time::PrimitiveDateTime, i32>> {
    match json_str {
        Some(json) if !json.is_empty() => {
            match serde_json::from_str::<HashMap<String, i32>>(json) {
                Ok(string_retry_history) => {
                    let date_format = format_description!("[year]-[month]-[day]");
                    let datetime_format = format_description!(
                        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"
                    );

                    let mut hourly_retry_history = HashMap::new();

                    for (key, count) in string_retry_history {
                        // Try parsing full datetime first
                        let parsed_dt = time::PrimitiveDateTime::parse(&key, &datetime_format)
                            .or_else(|_| {
                                // Fallback to date only
                                Date::parse(&key, &date_format).map(|date| {
                                    time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT)
                                })
                            });

                        match parsed_dt {
                            Ok(dt) => {
                                hourly_retry_history.insert(dt, count);
                            }
                            Err(_) => {
                                logger::error!("Error: failed to parse retry history key '{}'", key)
                            }
                        }
                    }

                    logger::debug!(
                        "Successfully parsed daily_retry_history with {} entries",
                        hourly_retry_history.len()
                    );

                    Some(hourly_retry_history)
                }
                Err(e) => {
                    logger::warn!("Failed to parse daily_retry_history JSON '{}': {}", json, e);
                    None
                }
            }
        }
        _ => {
            logger::debug!("Daily retry history not present or invalid, preserving existing data");
            None
        }
    }
}

/// Build comprehensive card data from CSV record
fn build_comprehensive_card_data(
    record: &RevenueRecoveryBackfillRequest,
) -> Result<ComprehensiveCardData, BackfillError> {
    // Extract card type from request, if not present then update it with 'card'
    let card_type = determine_card_type(record.payment_method_sub_type);

    // Parse expiration date
    let (exp_month, exp_year) = parse_expiration_date(
        record
            .exp_date
            .as_ref()
            .map(|date| date.clone().expose())
            .as_deref(),
    )?;

    let card_exp_month = exp_month.map(masking::Secret::new);
    let card_exp_year = exp_year.map(masking::Secret::new);

    // Extract card network
    let card_network = record.card_network.clone();

    // Extract card issuer and issuing country
    let card_issuer = record
        .clean_bank_name
        .as_ref()
        .filter(|value| !value.is_empty())
        .cloned();

    let card_issuing_country = record
        .country_name
        .as_ref()
        .filter(|value| !value.is_empty())
        .cloned();

    // Parse daily retry history
    let daily_retry_history = parse_daily_retry_history(record.daily_retry_history.as_deref());

    Ok(ComprehensiveCardData {
        card_type,
        card_exp_month,
        card_exp_year,
        card_network,
        card_issuer,
        card_issuing_country,
        daily_retry_history,
        is_active: record.is_active,
        account_update_history: record.account_update_history.clone(),
    })
}

/// Determine card type with fallback logic: payment_method_sub_type if not present -> "Card"
fn determine_card_type(payment_method_sub_type: Option<PaymentMethodType>) -> Option<String> {
    match payment_method_sub_type {
        Some(card_type_enum) => {
            let mapped_type = match card_type_enum {
                PaymentMethodType::Credit => "credit".to_string(),
                PaymentMethodType::Debit => "debit".to_string(),
                PaymentMethodType::Card => "card".to_string(),
                // For all other payment method types, default to "card"
                _ => "card".to_string(),
            };
            logger::debug!(
                "Using payment_method_sub_type enum '{:?}' -> '{}'",
                card_type_enum,
                mapped_type
            );
            Some(mapped_type)
        }
        None => {
            logger::info!("In CSV payment_method_sub_type not present...");
            None
        }
    }
}

/// Parse expiration date
fn parse_expiration_date(
    exp_date: Option<&str>,
) -> Result<(Option<String>, Option<String>), BackfillError> {
    exp_date
        .filter(|date| !date.is_empty())
        .map(|date| {
            date.split_once('/')
                .ok_or_else(|| {
                    logger::warn!("Unrecognized expiration date format (MM/YY expected)");
                    BackfillError::CsvParsingError(
                        "Invalid expiration date format: expected MM/YY".to_string(),
                    )
                })
                .and_then(|(month_part, year_part)| {
                    let month = month_part.trim();
                    let year = year_part.trim();

                    logger::debug!("Split expiration date - parsing month and year");

                    // Validate and parse month
                    let month_num = month.parse::<u8>().map_err(|_| {
                        logger::warn!("Failed to parse month component in expiration date");
                        BackfillError::CsvParsingError(
                            "Invalid month format in expiration date".to_string(),
                        )
                    })?;

                    if !(1..=12).contains(&month_num) {
                        logger::warn!("Invalid month value in expiration date (not in range 1-12)");
                        return Err(BackfillError::CsvParsingError(
                            "Invalid month value in expiration date".to_string(),
                        ));
                    }

                    // Handle year conversion
                    let final_year = match year.len() {
                        4 => &year[2..4], // Convert 4-digit to 2-digit
                        2 => year,        // Already 2-digit
                        _ => {
                            logger::warn!(
                                "Invalid year length in expiration date (expected 2 or 4 digits)"
                            );
                            return Err(BackfillError::CsvParsingError(
                                "Invalid year format in expiration date".to_string(),
                            ));
                        }
                    };

                    logger::debug!("Successfully parsed expiration date... ",);
                    Ok((Some(month.to_string()), Some(final_year.to_string())))
                })
        })
        .unwrap_or_else(|| {
            logger::debug!("Empty expiration date, returning None");
            Ok((None, None))
        })
}
