use std::collections::HashMap;

use api_models::revenue_recovery_data_backfill::{
    BackfillError, ComprehensiveCardData, RevenueRecoveryBackfillRequest,
    RevenueRecoveryDataBackfillResponse,
};
use common_enums::{CardNetwork, PaymentMethodType};
use hyperswitch_domain_models::api::ApplicationResponse;
use masking::ExposeInterface;
use router_env::{instrument, logger};
use time::{format_description, Date};

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
            }
        }
    };
    logger::info!(
        "Built comprehensive card data: {}",
        serde_json::to_string_pretty(&card_data).unwrap_or_default()
    );

    // Update Redis if token exists and is valid
    match record.token.as_ref().map(|token| token.clone().expose()) {
        Some(token) if !token.is_empty() => {
            logger::info!(
                "Updating Redis for customer: {}, token: {}",
                record.customer_id_resp,
                token
            );

            // Use efficient direct method without JSON conversion
            RedisTokenManager::update_redis_token_with_comprehensive_card_data(
                state,
                &record.customer_id_resp,
                &token,
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

/// Parse daily retry history JSON from CSV
fn parse_daily_retry_history(json_str: Option<&str>) -> Option<HashMap<Date, i32>> {
    match json_str {
        Some(json) if !json.is_empty() => {
            match serde_json::from_str::<HashMap<String, i32>>(json) {
                Ok(string_retry_history) => {
                    // Convert string dates to Date objects
                    let format = format_description::parse("[year]-[month]-[day]")
                        .map_err(|e| {
                            BackfillError::CsvParsingError(format!(
                                "Invalid date format configuration: {}",
                                e
                            ))
                        })
                        .ok()?;

                    let mut date_retry_history = HashMap::new();

                    for (date_str, count) in string_retry_history {
                        match Date::parse(&date_str, &format) {
                            Ok(date) => {
                                date_retry_history.insert(date, count);
                            }
                            Err(e) => {
                                logger::warn!(
                                    "Failed to parse date '{}' in daily_retry_history: {}",
                                    date_str,
                                    e
                                );
                            }
                        }
                    }

                    logger::debug!(
                        "Successfully parsed daily_retry_history with {} entries",
                        date_retry_history.len()
                    );
                    Some(date_retry_history)
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
    let card_type = Some(determine_card_type(record.payment_method_sub_type));

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
        .filter(|value| !value.is_empty() && *value != "nan")
        .cloned();

    let card_issuing_country = record
        .country_name
        .as_ref()
        .filter(|value| !value.is_empty() && *value != "nan")
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
    })
}

/// Determine card type with fallback logic: payment_method_sub_type if not present -> "Card"
fn determine_card_type(payment_method_sub_type: Option<PaymentMethodType>) -> String {
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
            mapped_type
        }
        None => {
            logger::info!("In CSV payment_method_sub_type not present, defaulting to 'card'");
            "card".to_string()
        }
    }
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

        // Validate and parse month
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
