use std::{collections::HashMap, fs::File, io::BufReader};

use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{HttpResponse, ResponseError};
use common_enums::{CardNetwork, PaymentMethodType};
use common_utils::{events::ApiEventMetric, id_type, pii::PhoneNumberStrategy};
use csv::Reader;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::{Date, PrimitiveDateTime};

use crate::payments;

#[derive(Debug, Deserialize, Serialize)]
pub struct RevenueRecoveryBackfillRequest {
    pub bin_number: Option<Secret<String>>,
    pub customer_id_resp: String,
    pub connector_payment_id: Option<String>,
    pub token: Option<Secret<String>>,
    pub exp_date: Option<Secret<String>>,
    pub card_network: Option<CardNetwork>,
    pub payment_method_sub_type: Option<PaymentMethodType>,
    pub clean_bank_name: Option<String>,
    pub country_name: Option<String>,
    pub daily_retry_history: Option<String>,
    pub is_active: Option<bool>,
    #[serde(
        default,
        deserialize_with = "RevenueRecoveryBackfillRequest::deserialize_history_vec_opt"
    )]
    pub account_update_history: Option<Vec<AccountUpdateHistoryRecord>>,
}

impl RevenueRecoveryBackfillRequest {
    pub fn deserialize_history_vec_opt<'de, D>(
        deserializer: D,
    ) -> Result<Option<Vec<AccountUpdateHistoryRecord>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;
        let val = Option::<String>::deserialize(deserializer)?;
        match val.as_deref().map(str::trim) {
            None | Some("") => Ok(None),
            Some(s) => serde_json::from_str::<Vec<AccountUpdateHistoryRecord>>(s)
                .map(Some)
                .map_err(serde::de::Error::custom),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UnlockStatusResponse {
    pub unlocked: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UnlockStatusRequest {
    pub connector_customer_id: String,
    pub payment_intent_id: id_type::GlobalPaymentId,
}

#[derive(Debug, Serialize)]
pub struct RevenueRecoveryDataBackfillResponse {
    pub processed_records: usize,
    pub failed_records: usize,
}

#[derive(Debug, Serialize)]
pub struct CsvParsingResult {
    pub records: Vec<RevenueRecoveryBackfillRequest>,
    pub failed_records: Vec<CsvParsingError>,
}

#[derive(Debug, Serialize)]
pub struct CsvParsingError {
    pub row_number: usize,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdateHistoryRecord {
    pub old_token: String,
    pub new_token: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub updated_at: PrimitiveDateTime,
    pub old_token_info: Option<payments::AdditionalCardInfo>,
    pub new_token_info: Option<payments::AdditionalCardInfo>,
}

/// Comprehensive card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveCardData {
    pub card_type: Option<String>,
    pub card_exp_month: Option<Secret<String>>,
    pub card_exp_year: Option<Secret<String>>,
    pub card_network: Option<CardNetwork>,
    pub card_issuer: Option<String>,
    pub card_issuing_country: Option<String>,
    pub daily_retry_history: Option<HashMap<PrimitiveDateTime, i32>>,
    pub is_active: Option<bool>,
    pub account_update_history: Option<Vec<AccountUpdateHistoryRecord>>,
}

impl ApiEventMetric for RevenueRecoveryDataBackfillResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl ApiEventMetric for UnlockStatusResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl ApiEventMetric for UnlockStatusRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl ApiEventMetric for CsvParsingResult {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl ApiEventMetric for CsvParsingError {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl ApiEventMetric for RedisDataResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl ApiEventMetric for UpdateTokenStatusRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

impl ApiEventMetric for UpdateTokenStatusResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum BackfillError {
    InvalidCardType(String),
    DatabaseError(String),
    RedisError(String),
    CsvParsingError(String),
    FileProcessingError(String),
}

#[derive(serde::Deserialize)]
pub struct BackfillQuery {
    pub cutoff_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RedisKeyType {
    Status, // for customer:{id}:status
    Tokens, // for customer:{id}:tokens
}

#[derive(Debug, Deserialize)]
pub struct GetRedisDataQuery {
    pub key_type: RedisKeyType,
}

#[derive(Debug, Serialize)]
pub struct RedisDataResponse {
    pub exists: bool,
    pub ttl_seconds: i64,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub enum ScheduledAtUpdate {
    SetToNull,
    SetToDateTime(PrimitiveDateTime),
}

impl<'de> Deserialize<'de> for ScheduledAtUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        match value {
            serde_json::Value::String(s) => {
                if s.to_lowercase() == "null" {
                    Ok(Self::SetToNull)
                } else {
                    // Parse as datetime using iso8601 deserializer
                    common_utils::custom_serde::iso8601::deserialize(
                        &mut serde_json::Deserializer::from_str(&format!("\"{}\"", s)),
                    )
                    .map(Self::SetToDateTime)
                    .map_err(serde::de::Error::custom)
                }
            }
            _ => Err(serde::de::Error::custom(
                "Expected null variable or datetime iso8601 ",
            )),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateTokenStatusRequest {
    pub connector_customer_id: String,
    pub payment_processor_token: Secret<String, PhoneNumberStrategy>,
    pub scheduled_at: Option<ScheduledAtUpdate>,
    pub is_hard_decline: Option<bool>,
    pub error_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateTokenStatusResponse {
    pub updated: bool,
    pub message: String,
}

impl std::fmt::Display for BackfillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCardType(msg) => write!(f, "Invalid card type: {}", msg),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::RedisError(msg) => write!(f, "Redis error: {}", msg),
            Self::CsvParsingError(msg) => write!(f, "CSV parsing error: {}", msg),
            Self::FileProcessingError(msg) => write!(f, "File processing error: {}", msg),
        }
    }
}

impl std::error::Error for BackfillError {}

impl ResponseError for BackfillError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadRequest().json(serde_json::json!({
            "error": self.to_string()
        }))
    }
}

#[derive(Debug, MultipartForm)]
pub struct RevenueRecoveryDataBackfillForm {
    #[multipart(rename = "file")]
    pub file: TempFile,
}

impl RevenueRecoveryDataBackfillForm {
    pub fn validate_and_get_records_with_errors(&self) -> Result<CsvParsingResult, BackfillError> {
        // Step 1: Open the file
        let file = File::open(self.file.file.path())
            .map_err(|error| BackfillError::FileProcessingError(error.to_string()))?;

        let mut csv_reader = Reader::from_reader(BufReader::new(file));

        // Step 2: Parse CSV into typed records
        let mut records = Vec::new();
        let mut failed_records = Vec::new();

        for (row_index, record_result) in csv_reader
            .deserialize::<RevenueRecoveryBackfillRequest>()
            .enumerate()
        {
            match record_result {
                Ok(record) => {
                    records.push(record);
                }
                Err(err) => {
                    failed_records.push(CsvParsingError {
                        row_number: row_index + 2, // +2 because enumerate starts at 0 and CSV has header row
                        error: err.to_string(),
                    });
                }
            }
        }

        Ok(CsvParsingResult {
            records,
            failed_records,
        })
    }
}
