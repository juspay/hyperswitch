use common_utils::types::StringMajorUnit;
use masking::Secret;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum PayloadPaymentsRequest {
    PayloadCardsRequest(PayloadCardsRequestData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionTypes {
    Payment,
    Deposit,
    Reversal,
    Refund,
    Credit,
    Chargeback,
    ChargebackReversal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BillingAddress {
    #[serde(rename = "payment_method[billing_address][city]")]
    pub city: String,
    #[serde(rename = "payment_method[billing_address][country_code]")]
    pub country: common_enums::CountryAlpha2,
    #[serde(rename = "payment_method[billing_address][postal_code]")]
    pub postal_code: Secret<String>,
    #[serde(rename = "payment_method[billing_address][state_province]")]
    pub state_province: Secret<String>,
    #[serde(rename = "payment_method[billing_address][street_address]")]
    pub street_address: Secret<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PayloadCardsRequestData {
    pub amount: StringMajorUnit,
    #[serde(flatten)]
    pub card: PayloadCard,
    #[serde(rename = "type")]
    pub transaction_types: TransactionTypes,
    #[serde(rename = "payment_method[type]")]
    pub payment_method_type: String,
    // Billing address fields are for AVS validation
    #[serde(flatten)]
    pub billing_address: BillingAddress,
}

#[derive(Default, Clone, Debug, Serialize, Eq, PartialEq)]
pub struct PayloadCard {
    #[serde(rename = "payment_method[card][card_number]")]
    pub number: cards::CardNumber,
    #[serde(rename = "payment_method[card][expiry]")]
    pub expiry: Secret<String>,
    #[serde(rename = "payment_method[card][card_code]")]
    pub cvc: Secret<String>,
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct PayloadRefundRequest {
    pub amount: StringMajorUnit,
}
