use masking::Secret;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayPaymentsRequest {
    pub transaction_reference: String,
    pub merchant: Merchant,
    pub instruction: Instruction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<Customer>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Merchant {
    pub entity: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_facilitator: Option<PaymentFacilitator>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement: Option<AutoSettlement>,
    pub method: PaymentMethod,
    pub payment_instrument: PaymentInstrument,
    pub narrative: InstructionNarrative,
    pub value: PaymentValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_repayment: Option<bool>,
    #[serde(rename = "threeDS", skip_serializing_if = "Option::is_none")]
    pub three_ds: Option<ThreeDSRequest>,
    /// For setting up mandates
    pub token_creation: Option<TokenCreation>,
    /// For specifying CIT vs MIT
    pub customer_agreement: Option<CustomerAgreement>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TokenCreation {
    #[serde(rename = "type")]
    pub token_type: TokenCreationType,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenCreationType {
    Worldpay,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerAgreement {
    #[serde(rename = "type")]
    pub agreement_type: CustomerAgreementType,
    pub stored_card_usage: Option<StoredCardUsageType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme_reference: Option<Secret<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomerAgreementType {
    Subscription,
    Unscheduled,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StoredCardUsageType {
    First,
    Subsequent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PaymentInstrument {
    Card(CardPayment),
    CardToken(CardToken),
    RawCardForNTI(RawCardDetails),
    Googlepay(WalletPayment),
    Applepay(WalletPayment),
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPayment {
    #[serde(flatten)]
    pub raw_card_details: RawCardDetails,
    pub cvc: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_holder_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<BillingAddress>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCardDetails {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub card_number: cards::CardNumber,
    pub expiry_date: ExpiryDate,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardToken {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvc: Option<Secret<String>>,
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

#[derive(
    Clone, Copy, Debug, Eq, Default, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum PaymentType {
    #[default]
    Plain,
    Token,
    Encrypted,
    Checkout,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ExpiryDate {
    pub month: Secret<i8>,
    pub year: Secret<i32>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    pub address1: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address2: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address3: Option<Secret<String>>,
    pub city: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Secret<String>>,
    pub postal_code: Secret<String>,
    pub country_code: common_enums::CountryAlpha2,
}

#[derive(
    Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    #[default]
    Ecom,
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
pub struct AutoSettlement {
    pub auto: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSRequest {
    #[serde(rename = "type")]
    pub three_ds_type: String,
    pub mode: String,
    pub device_data: ThreeDSRequestDeviceData,
    pub challenge: ThreeDSRequestChallenge,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSRequestDeviceData {
    pub accept_header: String,
    pub user_agent_header: String,
    pub browser_language: Option<String>,
    pub browser_screen_width: Option<u32>,
    pub browser_screen_height: Option<u32>,
    pub browser_color_depth: Option<String>,
    pub time_zone: Option<String>,
    pub browser_java_enabled: Option<bool>,
    pub browser_javascript_enabled: Option<bool>,
    pub channel: Option<ThreeDSRequestChannel>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreeDSRequestChannel {
    Browser,
    Native,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSRequestChallenge {
    pub return_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preference: Option<ThreeDsPreference>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThreeDsPreference {
    ChallengeMandated,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethod {
    #[default]
    Card,
    ApplePay,
    GooglePay,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionNarrative {
    pub line1: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PaymentValue {
    pub amount: i64,
    pub currency: api_models::enums::Currency,
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
pub struct WorldpayPartialRequest {
    pub value: PaymentValue,
    pub reference: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayCompleteAuthorizationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_reference: Option<String>,
}

pub(super) const THREE_DS_MODE: &str = "always";
pub(super) const THREE_DS_TYPE: &str = "integrated";
