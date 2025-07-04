use serde::{Deserialize, Serialize};

// PaymentsResponse
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PayloadPaymentStatus {
    Authorized,
    Declined,
    Processed,
    #[default]
    Processing,
    Rejected,
    Voided,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PayloadPaymentsResponse {
    PayloadCardsResponse(PayloadCardsResponseData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AvsResponse {
    Unknown,
    NoMatch,
    Zip,
    Street,
    StreetAndZip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayloadCardsResponseData {
    pub amount: f64,
    pub avs: Option<AvsResponse>,
    pub customer_id: Option<String>,
    #[serde(rename = "id")]
    pub transaction_id: String,
    pub payment_method_id: Option<String>,
    pub processing_id: Option<String>,
    pub processing_method_id: Option<String>,
    pub ref_number: Option<String>,
    pub status: PayloadPaymentStatus,
    pub status_code: Option<String>,
    pub status_message: Option<String>,
    #[serde(rename = "type")]
    pub response_type: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayloadCardResponse {
    pub card_brand: String,
    pub card_number: String, // Masked card number like "xxxxxxxxxxxx4242"
    pub card_type: String,
    pub expiry: String,
}

// Type definition for Refund Response
// Added based on assumptions since this is not provided in the documentation
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RefundStatus {
    Declined,
    Processed,
    #[default]
    Processing,
    Rejected,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundsLedger {
    pub amount: f64,
    #[serde(rename = "assoc_transaction_id")]
    pub associated_transaction_id: String, // Connector transaction id
    #[serde(rename = "id")]
    pub ledger_id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PayloadRefundResponse {
    pub amount: f64,
    #[serde(rename = "id")]
    pub transaction_id: String,
    pub ledger: Vec<RefundsLedger>,
    pub payment_method_id: Option<String>,
    pub processing_id: Option<String>,
    pub ref_number: Option<String>,
    pub status: RefundStatus,
    pub status_code: Option<String>,
    pub status_message: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayloadErrorResponse {
    pub error_type: String,
    pub error_description: String,
    pub object: String,
    /// Payload returns arbitrary details in JSON format
    pub details: Option<serde_json::Value>,
}
