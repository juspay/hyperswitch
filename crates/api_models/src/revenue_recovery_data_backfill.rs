use std::{fs::File, io::BufReader};

use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{HttpResponse, ResponseError};
use common_utils::events::ApiEventMetric;
use csv::Reader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RevenueRecoveryBackfillRequest {
    #[serde(rename = "Binnumber")]
    pub bin_number: String,
    #[serde(rename = "Cardtype")]
    pub card_type: String,
    #[serde(rename = "CustomerID_resp")]
    pub customer_id_resp: String,
    #[serde(rename = "cnpTxnId")]
    pub cnp_txn_id: String,
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "ExpiryDate")]
    pub exp_date: String,
    #[serde(rename = "CreditCardType.x")]
    pub credit_card_type_x: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "product_name")]
    pub product_name: String,
    #[serde(rename = "clean_bank_name")]
    pub clean_bank_name: String,
    #[serde(rename = "country_name")]
    pub country_name: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentMethodDataBackfillResponse {
    pub processed_records: usize,
    pub failed_records: usize,
}

impl ApiEventMetric for PaymentMethodDataBackfillResponse {
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
    pub fn validate_and_get_records(
        &self,
    ) -> Result<Vec<RevenueRecoveryBackfillRequest>, BackfillError> {
        // Step 1: Open the file
        let file = File::open(self.file.file.path())
            .map_err(|e| BackfillError::FileProcessingError(e.to_string()))?;

        let mut csv_reader = Reader::from_reader(BufReader::new(file));

        // Step 2: Parse CSV into typed records
        let mut records = Vec::new();
        for record in csv_reader
            .deserialize::<RevenueRecoveryBackfillRequest>()
            .flatten()
        {
            records.push(record);
        }

        Ok(records)
    }
}
