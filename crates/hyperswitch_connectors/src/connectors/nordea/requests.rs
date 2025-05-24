use common_utils::types::StringMajorUnit;
use masking::Secret;
use serde::{Deserialize, Serialize};

pub struct NordeaRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GrantType {
    AuthorizationCode,
    RefreshToken,
}

// To be passed in query parameters for OAuth scopes
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccessScope {
    AccountsBasic,
    AccountsBalances,
    AccountsDetails,
    AccountsTransactions,
    PaymentsMultiple,
    PaymentsSingleSca,
    CardsInformation,
    CardsTransactions,
}

#[derive(Debug, Serialize)]
pub struct NordeaOAuthTokenExchangeRequest {
    pub grant_type: GrantType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<Secret<String>>, // For refresh_token flow
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    /// International bank account number
    Iban,
    /// National bank account number of Sweden
    BbanSe,
    /// National bank account number of Denmark
    BbanDk,
    /// National bank account number of Norway
    BbanNo,
    /// Bankgiro number
    Bgnr,
    /// Plusgiro number
    Pgnr,
    /// Creditor number (Giro) Denmark
    GiroDk,
    /// Any bank account number without any check-digit validations
    BbanOther,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountNumber {
    /// Type of account number
    #[serde(rename = "_type")]
    pub account_type: AccountType,
    /// Currency of the account (Mandatory for debtor, Optional for creditor)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<api_models::enums::Currency>,
    /// Actual account number
    pub value: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CreditorAccountReference {
    /// RF or Invoice for FI Sepa payments, OCR for NO Kid payments and 01, 04, 15, 71, 73 or 75 for Danish Transfer Form payments.
    #[serde(rename = "_type")]
    pub creditor_reference_type: String,
    /// Actual reference number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CreditorAccount {
    /// Account number
    pub account: AccountNumber,
    pub country: Option<api_models::enums::CountryAlpha2>,
    /// Message for the creditor to appear on their transaction.
    /// Max length: FI SEPA:140; SE:12; PGNR:25; BGNR:150; DK: 40 (Instant/Express: 140); NO: 140
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Name of the creditor.
    /// Max length: FI SEPA: 30; SE: 35; DK: Not use (Mandatory for Instant/Express payments: 70);
    /// NO: 30 (mandatory for Straksbetaling/Express payments).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Creditor reference number
    pub reference: CreditorAccountReference,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DebitorAccount {
    /// Account number
    pub account: AccountNumber,
    /// Own message to be on the debtor's transaction.
    /// Max length 20. NB: This field is not supported for SEPA and Norwegian payments and will be ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct InstructedAmount {
    /// Monetary amount of the payment. Max (digits+decimals): FI SEPA: (9+2); SE:(11+2); DK:(7+2); NO:(7+2)
    amount: StringMajorUnit,
    /// Currency code according to ISO 4217.
    /// NB: Possible value depends on the type of the payment.
    /// For domestic payment it should be same as debtor local currency,
    /// for SEPA it must be EUR,
    /// for cross border it can be Currency code according to ISO 4217.
    currency: api_models::enums::Currency,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecurrenceType {
    Daily,
    Weekly,
    Biweekly,
    MonthlySameDay,
    MonthlyEom,
    QuartelySameDay,
    QuartelyEom,
    SemiAnnualySameDay,
    SemiAnnualyEom,
    TriAnnuallySameDay,
    YearlySameDay,
    YearlyEom,
    EveryMinuteSandboxOnly,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FundsAvailabilityRequest {
    True,
    False,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentsUrgency {
    Standard,
    Express,
    #[serde(rename = "sameday (Deprecated)")]
    Sameday,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RecurringInformation {
    /// Number of occurrences. Not applicable for NO (use end_date instead). Format: int32.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    /// Date on which the recurrence will end. Format: YYYY-MM-DD. Applicable only for Norway. Format: date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// Repeats every interval
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_type: Option<RecurrenceType>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TppCategory {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TppCode {
    Ds0a,
    Narr,
    Am21,
    Am04,
    Tm01,
    Am12,
    Rc06,
    Rc07,
    Rc04,
    Ag06,
    Bg06,
    Be22,
    Be20,
    Ff06,
    Be19,
    Am03,
    Am11,
    Ch04,
    Dt01,
    Ch03,
    Ff08,
    Ac10,
    Ac02,
    Ag08,
    Rr09,
    Rc11,
    Ff10,
    Rr10,
    Ff05,
    Ch15,
    Ff04,
    Ac11,
    Ac03,
    Ac13,
    Ac14,
    Ac05,
    Ac06,
    Rr07,
    Dt03,
    Am13,
    Ds24,
    Fr01,
    Am02,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ThirdPartyMessages {
    /// Category of the TPP message, INFO is further information, WARNING is something can be fixed, ERROR possibly non fixable issue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<TppCategory>,
    /// Additional code that is combined with the text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<TppCode>,
    /// Additional explaining text to the TPP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct NordeaPaymentsRequest {
    /// Creditor of the payment
    #[serde(rename = "creditor")]
    pub creditor_account: CreditorAccount,
    /// Debtor of the payment
    #[serde(rename = "debtor")]
    pub debitor_account: DebitorAccount,
    /// Free text reference that can be provided by the PSU.
    /// This identification is passed on throughout the entire end-to-end chain.
    /// Only in scope for Nordea Business DK.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_to_end_identification: Option<String>,
    /// Unique identification as assigned by a partner to identify the payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// Monetary amount
    pub instructed_amount: InstructedAmount,
    /// Recurring information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring: Option<RecurringInformation>,
    /// Use as an indicator that the supplied payment (amount, currency and debtor account)
    /// should be used to check whether the funds are available for further processing - at this moment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_availability_of_funds: Option<FundsAvailabilityRequest>,
    /// Choose a preferred execution date (or leave blank for today's date).
    /// This should be a valid bank day, and depending on the country the date will either be
    /// pushed to the next valid bank day, or return an error if a non-banking day date was
    /// supplied (all dates accepted in sandbox). SEPA: max +5 years from yesterday,
    /// Domestic: max. +1 year from yesterday. NB: Not supported for Own transfer Non-Recurring Norway.
    /// Format:date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_execution_date: Option<String>,
    /// Additional messages for third parties
    #[serde(rename = "tpp_messages")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tpp_messages: Option<Vec<ThirdPartyMessages>>,
    /// Urgency of the payment. NB: This field is supported for
    /// DK Domestic ('standard' and 'express')
    /// NO Domestic bank transfer payments ('standard'). Use 'express' for Straksbetaling (Instant payment).
    /// FI Sepa ('standard' and 'express') All other payment types ignore this input.
    /// For further details on urgencies and cut-offs, refer to the Nordea website. Value 'sameday' is marked as deprecated and will be removed in the future.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgency: Option<PaymentsUrgency>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NordeaConnectorMetadata {
    #[serde(rename = "value")]
    pub creditor_account_value: Secret<String>,
    #[serde(rename = "_type")]
    pub creditor_account_type: String,
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct NordeaRefundRequest {
    pub amount: StringMajorUnit,
}
