use common_utils::types::StringMinorUnit; // Will be used by transformer, request struct uses String for amount
use masking::Secret;
use serde::Serialize;

pub struct NordeaRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct NordeaPaymentsRequest {
    pub amount: StringMinorUnit,
    pub card: NordeaCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct NordeaCard {
    pub number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvc: Secret<String>,
    pub complete: bool,
}

// Helper struct for deserializing Nordea specific metadata from connector_details
#[derive(Debug, Deserialize, Serialize)]
struct NordeaConnectorMetadata {
    #[serde(rename = "merchant_creditor_name")]
    pub creditor_name: Option<String>,
    #[serde(rename = "merchant_iban")]
    pub creditor_iban: Option<Secret<String>>,
}

// Represents an Account Number (IBAN, BBAN, etc.)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct NordeaAccountNumber {
    pub value: Secret<String>, // The actual account number
    #[serde(rename = "_type")]
    pub account_type: String, // Type of account number, e.g., "IBAN"
    pub currency: Option<String>, // Currency of the account, e.g., "EUR"
}

// Represents the Debtor (Payer)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct NordeaPaymentDebtor {
    pub account: NordeaAccountNumber,
    pub message: Option<String>, // Optional message for the debtor's statement
}

// Represents a Creditor Reference
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct NordeaReference {
    #[serde(rename = "_type")]
    pub reference_type: String, // Type of reference, e.g., "RF"
    pub value: String, // The actual reference value
}

// Represents the Creditor (Beneficiary)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct NordeaPaymentCreditor {
    pub account: NordeaAccountNumber,
    pub name: String,
    pub message: Option<String>, // Message for the creditor, maps to remittance_information_unstructured
    pub reference: Option<NordeaReference>,
}

// Request structure for Nordea SEPA Credit Transfer (CreatePaymentRequest)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct NordeaSepaCreditTransferRequest {
    // Payment details
    pub amount: String, // Amount as a string (e.g., "123.45"), converted by transformer
    pub currency: String, // e.g., "EUR"

    // Debtor (Payer) information
    pub debtor: NordeaPaymentDebtor,

    // Creditor (Beneficiary) information
    pub creditor: NordeaPaymentCreditor,

    // Identifiers and references
    pub end_to_end_identification: String, // Unique identifier for the transaction
    pub external_id: Option<String>,       // Optional partner-assigned unique ID for the payment

    // Optional SEPA specific fields
    pub requested_execution_date: Option<String>, // Format YYYY-MM-DD
    pub urgency: Option<String>,                  // e.g., "standard", "express"
                                                  // recurring and tpp_messages are skipped for now due to complexity
}
