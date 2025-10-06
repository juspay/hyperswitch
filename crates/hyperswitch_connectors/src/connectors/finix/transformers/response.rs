use std::collections::HashMap;

use common_enums::Currency;
use common_utils::types::MinorUnit;
use serde::{Deserialize, Serialize};

use super::*;
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FinixPaymentsResponse {
    pub id: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub application: Option<Secret<String>>,
    pub amount: MinorUnit,
    pub captured_amount: Option<MinorUnit>,
    pub currency: Currency,
    pub is_void: Option<bool>,
    pub source: Option<String>,
    pub state: FinixState,
    pub failure_code: Option<String>,
    pub messages: Option<Vec<String>>,
    pub failure_message: Option<String>,
    pub tags: FinixTags,
    #[serde(rename = "type")]
    pub payment_type: Option<FinixPaymentType>,
    // pub trace_id: String,
    pub three_d_secure: Option<FinixThreeDSecure>,
    // Add other fields from the API response as needed.
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FinixIdentityResponse {
    pub id: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub application: Option<String>,
    pub entity: Option<HashMap<String, serde_json::Value>>,
    pub tags: Option<FinixTags>,
}

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
    pub card_type: Option<FinixCardType>,
    pub card_brand: Option<String>,
    pub fingerprint: Option<String>,
    pub address: Option<FinixAddress>,
    pub name: Option<String>,
    pub currency: Option<Currency>,
    pub enabled: bool,
}
