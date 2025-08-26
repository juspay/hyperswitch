use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{HttpResponse, ResponseError};
use common_utils::events::ApiEventMetric;
use serde::{Deserialize, Serialize};
use std::io::Read;

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
    #[serde(rename = "Amount")]
    pub amount: String,
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "expDate")]
    pub exp_date: String,
    #[serde(rename = "CreditCardType.x")]
    pub credit_card_type_x: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "funding_source")]
    pub funding_source: String,
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
pub struct PaymentMethodDataBackfillForm {
    #[multipart(rename = "file")]
    pub file: TempFile,
}

impl PaymentMethodDataBackfillForm {
    pub fn validate_and_get_records(&self) -> Result<Vec<RevenueRecoveryBackfillRequest>, BackfillError> {
        
        let mut file_content = String::new();
        let mut file = std::fs::File::open(self.file.file.path())
            .map_err(|e| BackfillError::FileProcessingError(e.to_string()))?;
        
        file.read_to_string(&mut file_content)
            .map_err(|e| BackfillError::FileProcessingError(e.to_string()))?;
        
        let mut csv_reader = csv::Reader::from_reader(file_content.as_bytes());
        let mut records = Vec::new();
        
        for result in csv_reader.deserialize() {
            let record = result
                .map_err(|e| BackfillError::CsvParsingError(e.to_string()))?;
            records.push(record);
        }
        
        Ok(records)
    }
}
