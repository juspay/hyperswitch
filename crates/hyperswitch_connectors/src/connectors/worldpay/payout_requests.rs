use masking::Secret;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayPayoutRequest {
    pub transaction_reference: String,
    pub merchant: Merchant,
    pub instruction: PayoutInstruction,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutInstruction {
    pub payout_instrument: PayoutInstrument,
    pub narrative: InstructionNarrative,
    pub value: PayoutValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayoutValue {
    pub amount: i64,
    pub currency: api_models::enums::Currency,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    pub entity: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionNarrative {
    pub line1: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PayoutInstrument {
    ApplePayDecrypt(ApplePayDecrypt),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayDecrypt {
    #[serde(rename = "type")]
    pub payout_type: PayoutType,
    pub dpan: cards::CardNumber,
    pub card_expiry_date: PayoutExpiryDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_holder_name: Option<Secret<String>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PayoutExpiryDate {
    pub month: Secret<i8>,
    pub year: Secret<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PayoutType {
    #[serde(rename = "card/networkToken+applepay")]
    ApplePayDecrypt,
}
