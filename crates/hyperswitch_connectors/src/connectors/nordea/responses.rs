use common_enums::CountryAlpha2;
use common_utils::types::StringMajorUnit;
use masking::Secret;
use serde::{Deserialize, Serialize};

use super::requests::{
    CreditorAccount, DebitorAccount, InstructedAmount, PaymentsUrgency, RecurringInformation,
    ThirdPartyMessages,
};

// OAuth token response structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NordeaOAuthExchangeResponse {
    pub access_token: Option<Secret<String>>,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<Secret<String>>,
    pub token_type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum NordeaPaymentStatus {
    #[default]
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
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaGroupHeader {
    /// Response creation time. Format: date-time.
    pub creation_date_time: Option<String>,
    /// HTTP code for response. Format: int32.
    pub http_code: Option<i32>,
    /// Original request id for correlation purposes
    pub message_identification: Option<String>,
    /// Details of paginated response
    pub message_pagination: Option<MessagePagination>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaResponseLinks {
    /// Describes the nature of the link, e.g. 'details' for a link to the detailed information of a listed resource.
    pub rel: Option<String>,
    /// Relative path to the linked resource
    pub href: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeesType {
    Additional,
    Standard,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TransactionFee {
    /// Monetary amount
    pub amount: InstructedAmount,
    pub description: Option<String>,
    pub excluded_from_total_fee: Option<bool>,
    pub percentage: Option<bool>,
    #[serde(rename = "type")]
    pub fees_type: Option<FeesType>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BankFee {
    /// Example 'domestic_transaction' only for DK domestic payments
    #[serde(rename = "_type")]
    pub bank_fee_type: Option<String>,
    /// Country code according to ISO Alpha-2
    pub country_code: Option<CountryAlpha2>,
    /// Currency code according to ISO 4217
    pub currency_code: Option<api_models::enums::Currency>,
    /// Value of the fee.
    pub value: Option<StringMajorUnit>,
    pub fees: Option<Vec<TransactionFee>>,
    /// Monetary amount
    pub total_fee_amount: InstructedAmount,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChargeBearer {
    Shar,
    Debt,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ExchangeRate {
    pub base_currency: Option<api_models::enums::Currency>,
    pub exchange_currency: Option<api_models::enums::Currency>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MessagePagination {
    /// Resource listing may return a continuationKey if there's more results available.
    /// Request may be retried with the continuationKey, but otherwise same parameters, in order to get more results.
    pub continuation_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaPaymentsInitiateResponseData {
    /// Unique payment identifier assigned for new payment
    #[serde(rename = "_id")]
    pub payment_id: String,
    /// HATEOAS inspired links: 'rel' and 'href'. Context specific link (only GET supported)
    #[serde(rename = "_links")]
    pub links: Option<Vec<NordeaResponseLinks>>,
    /// Marked as required field in the docs, but connector does not send amount in payment_response.amount
    pub amount: Option<StringMajorUnit>,
    /// Bearer of charges. shar = The Debtor (sender of the payment) will pay all fees charged by the sending bank.
    /// The Creditor (recipient of the payment) will pay all fees charged by the receiving bank.
    /// debt = The Debtor (sender of the payment) will bear all of the payment transaction fees.
    /// The creditor (beneficiary) will receive the full amount of the payment.
    pub charge_bearer: Option<ChargeBearer>,
    /// Creditor of the payment
    #[serde(rename = "creditor")]
    pub creditor_account: CreditorAccount,
    pub currency: Option<api_models::enums::Currency>,
    /// Debtor of the payment
    #[serde(rename = "debtor")]
    pub debitor_account: Option<DebitorAccount>,
    /// Timestamp of payment creation. ISO 8601 format yyyy-mm-ddThh:mm:ss.fffZ. Format:date-time.
    pub entry_date_time: Option<String>,
    /// Unique identification as assigned by a partner to identify the payment.
    pub external_id: Option<String>,
    /// An amount the bank will charge for executing the payment
    pub fee: Option<BankFee>,
    pub indicative_exchange_rate: Option<ExchangeRate>,
    /// It is mentioned as `number`. It can be an integer or a decimal number.
    pub rate: Option<f32>,
    /// Monetary amount
    pub instructed_amount: Option<InstructedAmount>,
    /// Indication of cross border payment to own account
    pub is_own_account_transfer: Option<bool>,
    /// OTP Challenge
    pub otp_challenge: Option<String>,
    /// Status of the payment
    pub payment_status: NordeaPaymentStatus,
    /// Planned execution date will indicate the day the payment will be finalized. If the payment has been pushed due to cut-off, it will be indicated in planned execution date. Format:date.
    pub planned_execution_date: Option<String>,
    /// Recurring information
    pub recurring: Option<RecurringInformation>,
    /// Choose a preferred execution date (or leave blank for today's date).
    /// This should be a valid bank day, and depending on the country the date will either be pushed to the next valid bank day,
    /// or return an error if a non-banking day date was supplied (all dates accepted in sandbox).
    /// SEPA: max +5 years from yesterday, Domestic: max. +1 year from yesterday. NB: Not supported for Own transfer Non-Recurring Norway.
    /// Format:date.
    pub requested_execution_date: Option<String>,
    /// Timestamp of payment creation. ISO 8601 format yyyy-mm-ddThh:mm:ss.fffZ Format:date-time.
    pub timestamp: Option<String>,
    /// Additional messages for third parties
    pub tpp_messages: Option<Vec<ThirdPartyMessages>>,
    pub transaction_fee: Option<Vec<BankFee>>,
    /// Currency that the cross border payment will be transferred in.
    /// This field is only supported for cross border payments for DK.
    /// If this field is not supplied then the payment will use the currency specified for the currency field of instructed_amount.
    pub transfer_currency: Option<api_models::enums::Currency>,
    /// Urgency of the payment. NB: This field is supported for DK Domestic ('standard' and 'express') and NO Domestic bank transfer payments ('standard' and 'express').
    /// Use 'express' for Straksbetaling (Instant payment).
    /// All other payment types ignore this input.
    /// For further details on urgencies and cut-offs, refer to the Nordea website.
    /// Value 'sameday' is marked as deprecated and will be removed in the future.
    pub urgency: Option<PaymentsUrgency>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaPaymentsInitiateResponse {
    /// Payment information
    #[serde(rename = "response")]
    pub payments_response: Option<NordeaPaymentsInitiateResponseData>,
    /// External response header
    pub group_header: Option<NordeaGroupHeader>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaPaymentsConfirmErrorObject {
    /// Error message
    pub error: Option<String>,
    /// Description of the error
    pub error_description: Option<String>,
    /// Payment id of the payment, the error is associated with
    pub payment_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaPaymentsResponseWrapper {
    pub payments: Vec<NordeaPaymentsInitiateResponseData>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaPaymentsConfirmResponse {
    /// HATEOAS inspired links: 'rel' and 'href'
    #[serde(rename = "_links")]
    pub links: Option<Vec<NordeaResponseLinks>>,
    /// Error description
    pub errors: Option<Vec<NordeaPaymentsConfirmErrorObject>>,
    /// External response header
    pub group_header: Option<NordeaGroupHeader>,
    /// OTP Challenge
    pub otp_challenge: Option<String>,
    #[serde(rename = "response")]
    pub nordea_payments_response: Option<NordeaPaymentsResponseWrapper>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaOriginalRequest {
    /// Original request url
    #[serde(rename = "url")]
    pub nordea_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaFailures {
    /// Failure code
    pub code: Option<String>,
    /// Failure description
    pub description: Option<String>,
    /// JSON path of the failing element if applicable
    pub path: Option<String>,
    /// Type of the validation error, e.g. NotNull
    #[serde(rename = "type")]
    pub failure_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaErrorBody {
    // Serde JSON because connector returns an `(item)` object in failures array object
    /// More details on the occurred error: Validation error
    #[serde(rename = "failures")]
    pub nordea_failures: Option<Vec<NordeaFailures>>,
    /// Original request information
    #[serde(rename = "request")]
    pub nordea_request: Option<NordeaOriginalRequest>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NordeaErrorResponse {
    /// Error response body
    pub error: Option<NordeaErrorBody>,
    /// External response header
    pub group_header: Option<NordeaGroupHeader>,
    #[serde(rename = "httpCode")]
    pub http_code: Option<String>,
    #[serde(rename = "moreInformation")]
    pub more_information: Option<String>,
}

// Nordea does not support refunds in Private APIs. Only Corporate APIs support Refunds
