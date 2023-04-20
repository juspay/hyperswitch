use serde::{Deserialize, Serialize};

//connector specific types
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AciAuthType {
    pub api_key: String,
    pub entity_id: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdyenAuthType {
    pub adyen_api_key: String,
    pub adyen_account_id: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AirwallexAuthType {
    pub app_id: String,
    pub key1: String
}
//applepay not sure it will work
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct AuthorizedotnetAuthType {
    pub api_login_id: String,
    pub transaction_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BamboraAuthType {
    pub passcode: String,
    pub merchant_id: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BluesnapAuthType {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BraintreeAuthType {
    pub public_key: String,
    pub merchant_id: String,
    pub private_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CheckoutAuthType {
    pub checkout_api_key: String,
    pub processing_channel_id: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CoinbaseAuthType {
    pub api_key: String,
}
//TODO:need to check  
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CybersourceAuthType{
    pub api_key: String,
    pub merchant_account: String,
    pub api_secret: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DlocalAuthType {
    pub x_login: String,
    pub x_trans_key: String,
    pub secret: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FiservAuthType {
    pub api_key: String,
    pub merchant_id: String,
    pub api_secret: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ForteAuthType {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GlobalpayAuthType {
    pub globalpay_app_id: String,
    pub globalpay_app_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KlarnaAuthType {
    pub klarna_api_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MollieAuthType {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MultisafepayAuthType {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NexinetsAuthType {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NuveiAuthType {
    pub merchant_id: String,
    pub merchant_site_id: String,
    pub merchant_secret: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpennodeAuthType {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PayeezyAuthType {
    pub api_key: String,
    pub api_secret: String,
    pub merchant_token: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PaypalAuthType {
    pub api_key: String,
    pub api_secret: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PayuAuthType{
    pub api_key: String,
    pub merchant_pos_id: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RapydAuthType {
    pub api_secret: String,
    pub secret_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Shift4AuthType {
    pub shift4_api_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StripeAuthType {
    pub stripe_api_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TrustpayAuthType {
    pub api_key: String,
    pub project_id: String,
    pub secret_key: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorldlineAuthType {
    pub api_key: String,
    pub api_secret: String,
    pub merchant_account_id: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorldpayAuthType {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Default, serde::Serialize, Clone, serde::Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "connector_name", content = "connector_account_details")]
pub enum ConnectorAuthType {
    #[strum(serialize = "aci")]
    Aci(AciAuthType),
    #[strum(serialize = "adyen")]
    Adyen(AdyenAuthType),
    #[strum(serialize = "airwallex")]
    Airwallex(AirwallexAuthType),
    //applepay not sure it will work
    #[strum(serialize = "authorizedotnet")]
    Authorizedotnet(AuthorizedotnetAuthType),
    #[strum(serialize = "bambora")]
    Bambora(BamboraAuthType),
    #[strum(serialize = "bluesnap")]
    Bluesnap(BluesnapAuthType),
    #[strum(serialize = "braintree")]
    Braintree(BraintreeAuthType),
    #[strum(serialize = "checkout")]
    Checkout(CheckoutAuthType),
    #[strum(serialize = "coinbase")]
    Coinbase(CoinbaseAuthType),
    //TODO:need to check  
    #[strum(serialize = "cybersource")]
    Cybersource(CybersourceAuthType),
    #[strum(serialize = "dlocal")]
    Dlocal(DlocalAuthType),
    #[strum(serialize = "fiserv")]
    Fiserv(FiservAuthType),
    #[strum(serialize = "forte")]
    Forte(ForteAuthType),
    #[strum(serialize = "globalpay")]
    Globalpay(GlobalpayAuthType),
    #[strum(serialize = "klarna")]
    Klarna(KlarnaAuthType),
    #[strum(serialize = "mollie")]
    Mollie(MollieAuthType),
    #[strum(serialize = "multisafepay")]
    Multisafepay(MultisafepayAuthType),
    #[strum(serialize = "nexinets")]
    Nexinets(NexinetsAuthType),
    #[strum(serialize = "nuvei")]
    Nuvei(NuveiAuthType),
    #[strum(serialize = "opennode")]
    Opennode(OpennodeAuthType),
    #[strum(serialize = "payeezy")]
    Payeezy(PayeezyAuthType),
    #[strum(serialize = "paypal")]
    Paypal(PaypalAuthType),
    #[strum(serialize = "payu")]
    Payu(PayuAuthType),
    #[strum(serialize = "rapyd")]
    Rapyd(RapydAuthType),
    #[strum(serialize = "shift4")]
    Shift4(Shift4AuthType),
    #[strum(serialize = "stripe")]
    Stripe(StripeAuthType),
    #[strum(serialize = "trustPay")]
    TrustPay(TrustpayAuthType),
    #[strum(serialize = "worldline")]
    Worldline(WorldlineAuthType),
    #[strum(serialize = "worldpay")]
    Worldpay(WorldpayAuthType),
    #[default]
    NoKey,
}