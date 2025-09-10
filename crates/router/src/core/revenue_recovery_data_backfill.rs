use std::collections::HashMap;

use api_models::revenue_recovery_data_backfill::{
    BackfillError, RevenueRecoveryBackfillRequest, RevenueRecoveryDataBackfillResponse,
};
use common_enums::CardNetwork;
use hyperswitch_domain_models::api::ApplicationResponse;
use router_env::{instrument, logger};

use crate::{
    connection,
    core::errors::{self, RouterResult},
    routes::SessionState,
    types::{
        domain,
        storage::revenue_recovery_redis_operation::{RedisTokenManager, RedisTokenUpdateParams},
    },
};

pub async fn revenue_recovery_data_backfill(
    state: SessionState,
    records: Vec<RevenueRecoveryBackfillRequest>,
) -> RouterResult<ApplicationResponse<RevenueRecoveryDataBackfillResponse>> {
    let mut processed_records = 0;
    let mut failed_records = 0;

    // Process each record
    for record in records {
        match process_payment_method_record(&state, &record).await {
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

async fn process_payment_method_record(
    state: &SessionState,
    record: &RevenueRecoveryBackfillRequest,
) -> Result<(), BackfillError> {
    // Build comprehensive card data from CSV record
    let card_data = match build_comprehensive_card_data(record) {
        Ok(data) => data,
        Err(e) => {
            logger::warn!(
                "Failed to build card data for connector customer id: {}, error: {}. Using minimal data.",
                record.customer_id_resp,
                e
            );
            serde_json::json!({})
        }
    };
    logger::info!(
        "Built comprehensive card data: {}",
        serde_json::to_string_pretty(&card_data).unwrap_or_default()
    );

    // Update Redis if token exists and is valid
    match record.token.as_deref() {
        Some(token) if !token.is_empty() && token != "nan" => {
            logger::info!(
                "Updating Redis for customer: {}, token: {}",
                record.customer_id_resp,
                token
            );
            RedisTokenManager::update_redis_token_comprehensive_data(
                state,
                &record.customer_id_resp,
                token,
                &card_data,
            )
            .await
            .map_err(|e| {
                logger::error!("Redis update failed for token {}: {}", token, e);
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

fn map_card_type(raw_type: &str) -> Result<String, BackfillError> {
    match raw_type {
        "Debit" => Ok("debit".to_string()),
        "Credit" => Ok("credit".to_string()),
        _ if raw_type.is_empty() || raw_type == "nan" => Err(BackfillError::InvalidCardType(
            "Missing card type".to_string(),
        )),
        _ => Err(BackfillError::InvalidCardType(raw_type.to_string())),
    }
}

/// Parse daily retry history JSON from CSV
fn parse_daily_retry_history(json_str: Option<&str>) -> Option<HashMap<String, i32>> {
    match json_str {
        Some(json) if !json.is_empty() && json != "nan" => {
            match serde_json::from_str::<HashMap<String, i32>>(json) {
                Ok(retry_history) => {
                    logger::debug!(
                        "Successfully parsed daily_retry_history with {} entries",
                        retry_history.len()
                    );
                    Some(retry_history)
                }
                Err(e) => {
                    logger::warn!("Failed to parse daily_retry_history JSON '{}': {}", json, e);
                    None
                }
            }
        }
        _ => {
            logger::debug!("Daily retry history not present or invalid");
            None
        }
    }
}

/// Build comprehensive card data from CSV record
fn build_comprehensive_card_data(
    record: &RevenueRecoveryBackfillRequest,
) -> Result<serde_json::Value, BackfillError> {
    let mut card_data = serde_json::Map::new();

    // Extract card type from request, if not present then update it with 'card'
    let card_type = determine_card_type(&record.type_field);
    card_data.insert(
        "card_type".to_string(),
        serde_json::Value::String(card_type),
    );

    // Parse expiration date
    let (exp_month, exp_year) = parse_expiration_date(record.exp_date.as_deref())?;

    [(exp_month, "card_exp_month"), (exp_year, "card_exp_year")]
        .into_iter()
        .filter_map(|(value, key)| value.map(|v| (key, serde_json::Value::String(v))))
        .for_each(|(key, value)| {
            card_data.insert(key.to_string(), value);
        });

    // Add card network
    record
        .card_network
        .clone()
        .and_then(|network| serde_json::to_value(network).ok())
        .map(|network_value| card_data.insert("card_network".to_string(), network_value));

    [
        (&record.clean_bank_name, "card_issuer"),
        (&record.country_name, "card_issuing_country"),
    ]
    .into_iter()
    .filter_map(|(field, key)| {
        field
            .as_ref()
            .filter(|value| !value.is_empty() && *value != "nan")
            .map(|value| (key, serde_json::Value::String(value.clone())))
    })
    .for_each(|(key, value)| {
        card_data.insert(key.to_string(), value);
    });

    // Add daily retry history
    parse_daily_retry_history(record.daily_retry_history.as_deref())
        .and_then(|retry_history| serde_json::to_value(retry_history).ok())
        .map(|retry_history_value| {
            card_data.insert("daily_retry_history".to_string(), retry_history_value)
        });

    Ok(serde_json::Value::Object(card_data))
}

/// Determine card type with fallback logic: type_field if not present -> "Card"
fn determine_card_type(type_field: &Option<String>) -> String {
    // First try the type_field
    if let Some(field) = type_field {
        if let Ok(mapped_type) = map_card_type(field) {
            logger::debug!("Using type_field '{}' -> '{}'", field, mapped_type);
            return mapped_type;
        }
    }

    // Finally, default to "Card"
    logger::info!("In CSV type_field not present or invalid, defaulting to 'Card'");
    "card".to_string()
}

/// Parse expiration date
fn parse_expiration_date(
    exp_date: Option<&str>,
) -> Result<(Option<String>, Option<String>), BackfillError> {
    match exp_date {
        Some(date) if !date.is_empty() && date != "nan" => {
            logger::debug!("Parsing expiration date: '{}'", date);
        }
        _ => {
            logger::debug!("Empty expiration date, returning None");
            return Ok((None, None));
        }
    }

    let exp_date = match exp_date {
        Some(date) => date,
        None => {
            logger::error!("Unexpected None value for exp_date after validation");
            return Err(BackfillError::CsvParsingError(
                "Internal error: exp_date became None after validation".to_string(),
            ));
        }
    };

    if let Some((month_part, year_part)) = exp_date.split_once('/') {
        let month = month_part.trim();
        let year = year_part.trim();

        logger::debug!(
            "Split expiration date - month: '{}', year: '{}'",
            month,
            year
        );

        // Validate and parse month using functional programming patterns
        let month_num = month.parse::<u8>().map_err(|parse_err| {
            logger::warn!(
                "Failed to parse month '{}' in expiration date '{}': {}",
                month,
                exp_date,
                parse_err
            );
            BackfillError::CsvParsingError(format!(
                "Invalid month format in expiration date '{}': {} (parse error: {})",
                exp_date, month, parse_err
            ))
        })?;

        if !(1..=12).contains(&month_num) {
            logger::warn!(
                "Invalid month value in expiration date '{}' - month: {} (not in range 1-12)",
                exp_date,
                month_num
            );
            return Err(BackfillError::CsvParsingError(format!(
                "Invalid month value in expiration date '{}': {}",
                exp_date, month_num
            )));
        }

        // Handle year conversion
        let final_year = match year.len() {
            4 => &year[2..4], // Convert 4-digit to 2-digit
            2 => year,        // Already 2-digit
            _ => {
                logger::warn!(
                    "Invalid year length in expiration date '{}' - year: '{}'",
                    exp_date,
                    year
                );
                return Err(BackfillError::CsvParsingError(format!(
                    "Invalid year format in expiration date '{}': {}",
                    exp_date, year
                )));
            }
        };

        logger::debug!(
            "Successfully parsed expiration date '{}' - month: {}, year: {}",
            exp_date,
            month,
            final_year
        );
        Ok((Some(month.to_string()), Some(final_year.to_string())))
    } else {
        logger::warn!("Unrecognized expiration date format: '{}'", exp_date);
        Err(BackfillError::CsvParsingError(format!(
            "Invalid expiration date format: {}",
            exp_date
        )))
    }
}
