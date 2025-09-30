use crate::connectors::finix::transformers::finix_common::*;
use common_enums::Currency;
use common_utils::types::MinorUnit;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a generic Authorization response object.
/// This structure is used for create, get, capture, and void responses.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FinixPaymentsResponse {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub application: String,
    pub amount: MinorUnit,
    pub captured_amount: MinorUnit,
    pub currency: Currency,
    pub is_void: bool,
    pub source: String,
    pub state: FinixState,
    pub tags: FinixTags,
    pub trace_id: String,
    pub three_d_secure: Option<FinixThreeDSecure>,
    // Add other fields from the API response as needed.
}

/// Represents the response object for an Identity.
/// API Reference: https://docs.finix.com/api/identities/createidentity
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FinixIdentityResponse {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub application: String,
    pub entity: HashMap<String, serde_json::Value>,
    pub tags: FinixTags,
}

/// Represents the response object for a payment instrument.
/// API Reference: https://docs.finix.com/api/payment-instruments/createpaymentinstrument
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FinixInstrumentResponse {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub application: String,
    pub identity: Option<String>,
    #[serde(rename = "type")]
    pub instrument_type: FinixPaymentInstrumentType,
    pub tags: Option<FinixTags>,
    pub expiration_month: Option<i32>,
    pub expiration_year: Option<i32>,
    pub last_four: Option<String>,
    pub bin: Option<String>,
    pub card_type: Option<FinixCardType>,
    pub card_brand: Option<FinixCardBrand>,
    pub fingerprint: Option<String>,
    pub address: Option<FinixAddress>,
    pub name: Option<String>,
    pub currency: Option<Currency>,
    pub enabled: bool,
}
