use serde::Deserialize;

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NordeaPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}
//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NordeaPaymentsResponse {
    pub status: NordeaPaymentStatus,
    pub id: String,
}

// Enum for Nordea Payment Status from Swagger definition
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
// The enum variants in the Swagger are PascalCase. Serde handles this by default for Rust PascalCase enums.
pub enum NordeaSwaggerPaymentStatus {
    PendingConfirmation,
    PendingSecondConfirmation,
    PendingUserApproval,
    OnHold,
    Confirmed,
    Rejected,
    Paid,
    InsufficientFunds,
    LimitExceeded,
    UserApprovalFailed,
    UserApprovalTimeout,
    UserApprovalCancelled,
    #[serde(other)] // Catch-all for unknown variants not explicitly defined
    Unknown,
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct NordeaRefundRequest {
    pub amount: StringMinorUnit,
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub status: RefundStatus,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

// Corresponds to the 'Payment' object within the API response (Swagger definition)
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")] // Fields in the 'Payment' object are snake_case
pub struct NordeaPaymentDetails {
    #[serde(rename = "_id")] // Maps from "_id" in JSON to "payment_id" in struct
    pub payment_id: String,
    pub payment_status: NordeaSwaggerPaymentStatus,
    // Other fields from the 'Payment' object can be added here if needed by the application
    // For example:
    // pub amount: f64,
    // pub currency: String,
    // pub creditor: Option<SomeCreditorStruct>, // Define if needed
    // pub debtor: Option<SomeDebtorStruct>,     // Define if needed
    // pub requested_execution_date: Option<String>,
    // pub planned_execution_date: Option<String>,
}

// Top-level response structure for SEPA Credit Transfer (corresponds to 'PaymentResponse' in Swagger)
// This is what the connector's handle_response will parse the entire API JSON response into.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")] // Fields in 'PaymentResponse' are snake_case
pub struct NordeaSepaCreditTransferResponse {
    // The 'response' field in the API's JSON contains the actual payment details
    pub response: NordeaPaymentDetails,
    // pub group_header: Option<serde_json::Value>, // Represents 'ResponseHeader', can be added if needed
}
