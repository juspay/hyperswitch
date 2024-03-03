use masking::Secret;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address2: Option<Secret<String>>,
    pub postal_code: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address3: Option<Secret<String>>,
    pub country_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address1: Option<Secret<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayPaymentsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<Channel>,
    pub instruction: Instruction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<Customer>,
    pub merchant: Merchant,
    pub transaction_reference: String,
}

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    #[default]
    Moto,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<CustomerAuthentication>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CustomerAuthentication {
    ThreeDS(ThreeDS),
    Token(NetworkToken),
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDS {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication_value: Option<Secret<String>>,
    pub version: ThreeDSVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    pub eci: String,
    #[serde(rename = "type")]
    pub auth_type: CustomerAuthType,
}

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub enum ThreeDSVersion {
    #[default]
    #[serde(rename = "1")]
    One,
    #[serde(rename = "2")]
    Two,
}

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub enum CustomerAuthType {
    #[serde(rename = "3DS")]
    #[default]
    Variant3Ds,
    #[serde(rename = "card/networkToken")]
    NetworkToken,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkToken {
    #[serde(rename = "type")]
    pub auth_type: CustomerAuthType,
    pub authentication_value: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eci: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_repayment: Option<bool>,
    pub value: PaymentValue,
    pub narrative: InstructionNarrative,
    pub payment_instrument: PaymentInstrument,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionNarrative {
    pub line1: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PaymentInstrument {
    Card(CardPayment),
    CardToken(CardToken),
    Googlepay(WalletPayment),
    Applepay(WalletPayment),
}

#[derive(
    Clone, Copy, Debug, Eq, Default, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub enum PaymentType {
    #[default]
    #[serde(rename = "card/plain")]
    Card,
    #[serde(rename = "card/token")]
    CardToken,
    #[serde(rename = "card/wallet+googlepay")]
    Googlepay,
    #[serde(rename = "card/wallet+applepay")]
    Applepay,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPayment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<BillingAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_holder_name: Option<Secret<String>>,
    pub card_expiry_date: CardExpiryDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvc: Option<Secret<String>>,
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub card_number: cards::CardNumber,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardToken {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub href: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPayment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub wallet_token: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<BillingAddress>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct CardExpiryDate {
    pub month: Secret<i8>,
    pub year: Secret<i32>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentValue {
    pub amount: i64,
    pub currency: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    pub entity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_facilitator: Option<PaymentFacilitator>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentFacilitator {
    pub pf_id: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso_id: Option<Secret<String>>,
    pub sub_merchant: SubMerchant,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubMerchant {
    pub city: String,
    pub name: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    pub postal_code: Secret<String>,
    pub merchant_id: Secret<String>,
    pub country_code: String,
    pub street: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<String>,
}

#[derive(Default, Debug, Serialize)]
pub struct WorldpayRefundRequest {
    pub value: PaymentValue,
    pub reference: String,
}
