use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address2: Option<String>,
    pub postal_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address3: Option<String>,
    pub country_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address1: Option<String>,
}

impl BillingAddress {
    #[allow(dead_code)]
    pub fn new(postal_code: String, country_code: String) -> Self {
        Self {
            city: None,
            address2: None,
            postal_code,
            state: None,
            address3: None,
            country_code,
            address1: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<Channel>,
    pub instruction: Box<Instruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<Box<Customer>>,
    pub merchant: Box<Merchant>,
    pub transaction_reference: String,
}

impl PaymentsRequest {
    pub fn new(
        instruction: Instruction,
        merchant: Merchant,
        transaction_reference: String,
    ) -> Self {
        Self {
            channel: None,
            instruction: Box::new(instruction),
            customer: None,
            merchant: Box::new(merchant),
            transaction_reference,
        }
    }
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
    pub authentication: Option<Box<CustomerAuthentication>>,
}

impl Customer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            risk_profile: None,
            authentication: None,
        }
    }
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
    pub authentication_value: Option<String>,
    pub version: ThreeDSVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    pub eci: String,
    #[serde(rename = "type")]
    pub auth_type: CustomerAuthType,
}

impl ThreeDS {
    #[allow(dead_code)]
    pub fn new(version: ThreeDSVersion, eci: String, auth_type: CustomerAuthType) -> Self {
        Self {
            authentication_value: None,
            version,
            transaction_id: None,
            eci,
            auth_type,
        }
    }
}

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub enum ThreeDSVersion {
    #[serde(rename = "1")]
    #[default]
    Variant1,
    #[serde(rename = "2")]
    Variant2,
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
    pub authentication_value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eci: Option<String>,
}

impl NetworkToken {
    #[allow(dead_code)]
    pub fn new(auth_type: CustomerAuthType, authentication_value: String) -> Self {
        Self {
            auth_type,
            authentication_value,
            eci: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_repayment: Option<bool>,
    pub value: Box<PaymentValue>,
    pub narrative: Box<InstructionNarrative>,
    pub payment_instrument: Box<PaymentInstrument>,
}

impl Instruction {
    pub fn new(
        value: PaymentValue,
        narrative: InstructionNarrative,
        payment_instrument: PaymentInstrument,
    ) -> Self {
        Self {
            debt_repayment: None,
            value: Box::new(value),
            narrative: Box::new(narrative),
            payment_instrument: Box::new(payment_instrument),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionNarrative {
    pub line1: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,
}

impl InstructionNarrative {
    pub fn new(line1: String) -> Self {
        Self { line1, line2: None }
    }
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
    pub billing_address: Option<Box<BillingAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_holder_name: Option<String>,
    pub card_expiry_date: Box<CardExpiryDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvc: Option<String>,
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub card_number: String,
}

impl CardPayment {
    pub fn new(card_expiry_date: CardExpiryDate, card_number: String) -> Self {
        Self {
            billing_address: None,
            card_holder_name: None,
            card_expiry_date: Box::new(card_expiry_date),
            cvc: None,
            payment_type: PaymentType::Card,
            card_number,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardToken {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub href: String,
}

impl CardToken {
    #[allow(dead_code)]
    pub fn new(href: String) -> Self {
        Self {
            payment_type: PaymentType::CardToken,
            href,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPayment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub wallet_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<Box<BillingAddress>>,
}

impl WalletPayment {
    pub fn new(payment_type: PaymentType, wallet_token: String) -> Self {
        Self {
            payment_type,
            wallet_token,
            billing_address: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct CardExpiryDate {
    pub month: i32,
    pub year: i32,
}

impl CardExpiryDate {
    pub fn new(month: i32, year: i32) -> Self {
        Self { month, year }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentValue {
    pub amount: i64,
    pub currency: String,
}

impl PaymentValue {
    pub fn new(amount: i64, currency: String) -> Self {
        Self { amount, currency }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    pub entity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_facilitator: Option<Box<PaymentFacilitator>>,
}

impl Merchant {
    pub fn new(entity: String) -> Self {
        Self {
            entity,
            mcc: None,
            payment_facilitator: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentFacilitator {
    pub pf_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso_id: Option<String>,
    pub sub_merchant: Box<SubMerchant>,
}

impl PaymentFacilitator {
    #[allow(dead_code)]
    pub fn new(pf_id: String, sub_merchant: SubMerchant) -> Self {
        Self {
            pf_id,
            iso_id: None,
            sub_merchant: Box::new(sub_merchant),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubMerchant {
    pub city: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    pub postal_code: String,
    pub merchant_id: String,
    pub country_code: String,
    pub street: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<String>,
}

impl SubMerchant {
    #[allow(dead_code)]
    pub fn new(
        city: String,
        name: String,
        postal_code: String,
        merchant_id: String,
        country_code: String,
        street: String,
    ) -> Self {
        Self {
            city,
            name,
            state: None,
            postal_code,
            merchant_id,
            country_code,
            street,
            tax_id: None,
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct WorldpayRefundRequest {
    pub value: Box<PaymentValue>,
    pub reference: String,
}
