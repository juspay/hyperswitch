use api_models::revenue_recovery_data_backfill::{
    BackfillError, PaymentMethodDataBackfillResponse, RevenueRecoveryBackfillRequest,
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
                logger::info!(
                    "Successfully processed record with cnpTxnId: {}",
                    record.cnp_txn_id
                );
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
        "Revenue recovery data backfill completed - Processed: {}, Failed: {}",
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
    // Build comprehensive card data from CSV record
    let card_data = match build_comprehensive_card_data(record) {
        Ok(data) => data,
        Err(e) => {
            logger::warn!(
                "Failed to build card data for cnp_txn_id: {}, error: {}. Using minimal data.",
                record.cnp_txn_id,
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
    match record.token.as_str() {
        "" | "nan" => {
            logger::info!(
                "Skipping Redis update - token is empty or 'nan': {}",
                record.token
            );
        }
        token => {
            logger::info!(
                "Updating Redis for customer: {}, token: {}",
                record.customer_id_resp,
                token
            );
            if let Err(e) = RedisTokenManager::update_redis_token_comprehensive_data(
                state,
                &record.customer_id_resp,
                token,
                &card_data,
            )
            .await
            {
                logger::error!("Redis update failed for token {}: {}", token, e);
                return Err(BackfillError::RedisError(format!(
                    "Token not found in Redis: {}",
                    e
                )));
            }
        }
    }

    logger::info!(
        "Successfully completed processing for cnp_txn_id: {}",
        record.cnp_txn_id
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

/// Determine card type with fallback logic: type_field -> card_type -> "Card"
fn determine_card_type(type_field: &str) -> String {
    // First try the type_field
    if let Ok(mapped_type) = map_card_type(type_field) {
        logger::debug!("Using type_field '{}' -> '{}'", type_field, mapped_type);
        return mapped_type;
    }
    
    // Finally, default to "Card"
    logger::info!("Both type_field '{}' invalid, defaulting to 'Card'", type_field);
    "card".to_string()
}

/// Build comprehensive card data from CSV record
fn build_comprehensive_card_data(
    record: &RevenueRecoveryBackfillRequest,
) -> Result<serde_json::Value, BackfillError> {
    // Extract card type from request, if not present then update it with 'card'
    let card_type = determine_card_type(&record.type_field);

    // Parse expiration date (format: MMYY or MM/YY)
    let (exp_month, exp_year) = parse_expiration_date(&record.exp_date)?;

    // Map card network from request
    let card_network = map_card_network(&record.credit_card_type_x)
        .and_then(|network| serde_json::to_value(network).ok());

    // Build comprehensive card object
    let card_data = serde_json::json!({
        "card_type": card_type,
        "card_exp_month": exp_month,
        "card_exp_year": exp_year,
        "card_issuer": if record.clean_bank_name.is_empty() || record.clean_bank_name == "nan" {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(record.clean_bank_name.clone())
        },
        "card_network": card_network,
        "card_issuing_country": if record.country_name.is_empty() || record.country_name == "nan" {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(record.country_name.clone())
        },
        "card_isin": serde_json::Value::Null,
        "last4": serde_json::Value::Null,
    });

    logger::info!(
        "Built card data from CSV: bin={}, exp_date={}, bank={}, country={}, product={}",
        record.bin_number,
        record.exp_date,
        record.clean_bank_name,
        record.country_name,
        record.product_name
    );

    Ok(card_data)
}

/// Parse expiration date
fn parse_expiration_date(
    exp_date: &str,
) -> Result<(Option<String>, Option<String>), BackfillError> {
    logger::debug!("Parsing expiration date: '{}'", exp_date);
    if exp_date.is_empty() || exp_date == "nan" {
        logger::debug!("Empty or 'nan' expiration date, returning None");
        return Ok((None, None));
    }

    if let Some((month_part, year_part)) = exp_date.split_once('/') {
        let month = month_part.trim();
        let year = year_part.trim();

        logger::debug!(
            "Split expiration date - month: '{}', year: '{}'",
            month,
            year
        );

        // Validate and parse month
        let month_num = match month.parse::<u8>().ok() {
            Some(num) if (1..=12).contains(&num) => num,
            _ => {
                logger::warn!("Invalid month in expiration date '{}', skipping", exp_date);
                return Ok((None, None));
            }
        };

        if !(1..=12).contains(&month_num) {
            logger::warn!(
                "Invalid month in expiration date '{}' - month: {} (not in range 1-12)",
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

/// Map card network from card network type
fn map_card_network(credit_card_type: &str) -> Option<CardNetwork> {
    if credit_card_type.is_empty() || credit_card_type == "nan" {
        logger::warn!("Card network type not present");
        return None;
    }

    match credit_card_type {
        "Visa" => Some(CardNetwork::Visa),
        "Master Card" => Some(CardNetwork::Mastercard),
        "American Express" => Some(CardNetwork::AmericanExpress),
        "Discover" => Some(CardNetwork::Discover),
        _ => {
            logger::warn!("Unknown card network type: {}", credit_card_type);
            None
        }
    }
}
