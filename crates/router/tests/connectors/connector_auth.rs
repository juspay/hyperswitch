use std::env;

use common_enums::ConnectorAuthType;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ConnectorAuthentication {
    pub aci: Option<Aci>,
    pub adyen: Option<Adyen>,
    pub airwallex: Option<Airwallex>,
    pub authorizedotnet: Option<Authorizedotnet>,
    pub bambora: Option<Bambora>,
    pub bluesnap: Option<Bluesnap>,
    pub checkout: Option<Checkout>,
    pub coinbase: Option<Coinbase>,
    pub cybersource: Option<Cybersource>,
    pub dlocal: Option<Dlocal>,
    pub fiserv: Option<Fiserv>,
    pub forte: Option<Forte>,
    pub globalpay: Option<Globalpay>,
    pub mollie: Option<Mollie>,
    pub multisafepay: Option<Multisafepay>,
    pub nexinets: Option<Nexinets>,
    pub nuvei: Option<Nuvei>,
    pub opennode: Option<Opennode>,
    pub payeezy: Option<Payeezy>,
    pub paypal: Option<Paypal>,
    pub payu: Option<Payu>,
    pub rapyd: Option<Rapyd>,
    pub shift4: Option<Shift4>,
    pub stripe: Option<Stripe>,
    pub worldpay: Option<Worldpay>,
    pub worldline: Option<Worldline>,
    pub trustpay: Option<TrustPay>,
}

impl ConnectorAuthentication {
    #[allow(clippy::expect_used)]
    pub(crate) fn new() -> Self {
        let path = env::var("CONNECTOR_AUTH_FILE_PATH")
            .expect("connector authentication file path not set");
        toml::from_str(
            &std::fs::read_to_string(path).expect("connector authentication config file not found"),
        )
        .expect("Failed to read connector authentication config file")
    }
}


//-------
impl From<Aci> for ConnectorAuthType {
    fn from(key: Aci) -> Self {
        common_enums::AciAuthType {
            api_key: key.api_key,
            entity_id: key.entity_id,
        }
    }
}
impl From<Adyen> for ConnectorAuthType {
    fn from(key: Adyen) -> Self {
        common_enums::AdyenAuthType {
            adyen_api_key: key.adyen_api_key,
            adyen_account_id: key.adyen_account_id,
        }
    }
}
impl From<Airwallex> for ConnectorAuthType {
    fn from(key: Airwallex) -> Self {
        common_enums::AirwallexAuthType {
            app_id: key.app_id,
            key1: key.key1
        }
    }
}
impl From<Authorizedotnet> for ConnectorAuthType {
    fn from(key: Authorizedotnet) -> Self {
        common_enums::AuthorizedotnetAuthType {
            api_login_id: key.api_login_id,
            transaction_key: key.transaction_key,
        }
    }
}
impl From<Bambora> for ConnectorAuthType {
    fn from(key: Bambora) -> Self {
        common_enums::BamboraAuthType {
            passcode: key.passcode,
            merchant_id: key.merchant_id,
        }
    }
}
impl From<Bluesnap> for ConnectorAuthType {
    fn from(key: Bluesnap) -> Self {
        common_enums::BluesnapAuthType {
            username: key.username,
            password: key.password,
        }
    }
}
impl From<Braintree> for ConnectorAuthType {
    fn from(key: Braintree) -> Self {
        common_enums::BraintreeAuthType {
            public_key: key.public_key,
            merchant_id: key.merchant_id,
            private_key: key.private_key,
        }
    }
}
impl From<Checkout> for ConnectorAuthType {
    fn from(key: Checkout) -> Self {
        common_enums::CheckoutAuthType {
            checkout_api_key: key.checkout_api_key,
            processing_channel_id: key.processing_channel_id,
        }
    }
}
impl From<Coinbase> for ConnectorAuthType {
    fn from(key: Coinbase) -> Self {
        common_enums::CoinbaseAuthType {
            api_key: key.api_key,
        }
    }
}
impl From<Cybersource> for ConnectorAuthType {
    fn from(key: Cybersource) -> Self {
        common_enums::CybersourceAuthType {
            api_key: key.api_key,
            merchant_account: key.merchant_account,
            api_secret: key.api_secret,
        }
    }
}

impl From<Dlocal> for ConnectorAuthType {
    fn from(key: Dlocal) -> Self {
        common_enums::DlocalAuthType {
            x_login: key.x_login,
            x_trans_key: key.x_trans_key,
            secret: key.secret,
        }
    }
}
impl From<Fiserv> for ConnectorAuthType {
    fn from(key: Fiserv) -> Self {
        common_enums::FiservAuthType {
            api_key: key.api_key,
            merchant_id: key.merchant_id,
            api_secret: key.api_secret,
        }
    }
}
impl From<Forte> for ConnectorAuthType {
    fn from(key: Forte) -> Self {
        common_enums::ForteAuthType {
            api_key: key.api_key,
        }
    }
}
impl From<Globalpay> for ConnectorAuthType {
    fn from(key: Globalpay) -> Self {
        common_enums::GlobalpayAuthType {
            globalpay_app_id: key.globalpay_app_id,
            globalpay_app_key: key.globalpay_app_key,
        }
    }
}
impl From<Klarna> for ConnectorAuthType {
    fn from(key: Klarna) -> Self {
        common_enums::KlarnaAuthType {
            klarna_api_key: key.klarna_api_key,
        }
    }
}
impl From<Mollie> for ConnectorAuthType {
    fn from(key: Mollie) -> Self {
        common_enums::MollieAuthType {
            api_key: key.api_key,
        }
    }
}
impl From<Multisafepay> for ConnectorAuthType {
    fn from(key: Multisafepay) -> Self {
        common_enums::MultisafepayAuthType {
            api_key: key.api_key,
        }
    }
}
impl From<Nexinets> for ConnectorAuthType {
    fn from(key: Nexinets) -> Self {
        common_enums::NexinetsAuthType {
            api_key: key.api_key,
        }
    }
}
impl From<Nuvei> for ConnectorAuthType {
    fn from(key: Nuvei) -> Self {
        common_enums::NuveiAuthType {
            merchant_id: key.merchant_id,
            merchant_site_id: key.merchant_site_id,
            merchant_secret: key.merchant_secret,
        }
    }
}
impl From<Opennode> for ConnectorAuthType {
    fn from(key: Opennode) -> Self {
        common_enums::OpennodeAuthType {
            api_key: key.api_key,
        }
    }
}
impl From<Payeezy> for ConnectorAuthType {
    fn from(key: Payeezy) -> Self {
        common_enums::PayeezyAuthType {
            api_key: key.api_key,
            api_secret: key.api_secret,
            merchant_token: key.merchant_token,
        }
    }
}
impl From<Paypal> for ConnectorAuthType {
    fn from(key: Paypal) -> Self {
        common_enums::PaypalAuthType {
            api_key: key.api_key,
            api_secret: key.api_secret,
        }
    }
}
impl From<Payu> for ConnectorAuthType {
    fn from(key: Payu) -> Self {
        common_enums::PayuAuthType {
            api_key: key.api_key,
            merchant_pos_id: key.merchant_pos_id,
        }
    }
}
impl From<Rapyd> for ConnectorAuthType {
    fn from(key: Rapyd) -> Self {
        common_enums::RapydAuthType {
            api_secret: key.api_secret,
            secret_key: key.secret_key,
        }
    }
}
impl From<Shift4> for ConnectorAuthType {
    fn from(key: Shift4) -> Self {
        common_enums::Shift4AuthType {
            shift4_api_key: key.shift4_api_key,
        }
    }
}
impl From<Stripe> for ConnectorAuthType {
    fn from(key: Stripe) -> Self {
        common_enums::StripeAuthType {
            stripe_api_key: key.stripe_api_key,
        }
    }
}
impl From<TrustPay> for ConnectorAuthType {
    fn from(key: TrustPay) -> Self {
        common_enums::TrustpayAuthType {
            api_key: key.api_key,
            project_id: key.project_id,
            secret_key: key.secret_key,
        }
    }
}
impl From<Worldline> for ConnectorAuthType {
    fn from(key: Worldline) -> Self {
        common_enums::WorldlineAuthType {
            api_key: key.api_key,
            api_secret: key.api_secret,
            merchant_account_id: key.merchant_account_id,
        }
    }
}
impl From<Worldpay> for ConnectorAuthType {
    fn from(key: Worldpay) -> Self {
        common_enums::WorldpayAuthType {
            username: key.username,
            password: key.password,
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Aci {
    pub api_key: String,
    pub entity_id: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Adyen {
    pub adyen_api_key: String,
    pub adyen_account_id: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Airwallex {
    pub app_id: String,
    pub key1: String
}
//applepay not sure it will work
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Authorizedotnet {
    pub api_login_id: String,
    pub transaction_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Bambora {
    pub passcode: String,
    pub merchant_id: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Bluesnap {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Braintree {
    pub public_key: String,
    pub merchant_id: String,
    pub private_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Checkout {
    pub checkout_api_key: String,
    pub processing_channel_id: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Coinbase {
    pub api_key: String,
}
//TODO:need to check  
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Cybersource {
    pub api_key: String,
    pub merchant_account: String,
    pub api_secret: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Dlocal {
    pub x_login: String,
    pub x_trans_key: String,
    pub secret: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Fiserv {
    pub api_key: String,
    pub merchant_id: String,
    pub api_secret: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Forte {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Globalpay {
    pub globalpay_app_id: String,
    pub globalpay_app_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Klarna {
    pub klarna_api_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Mollie {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Multisafepay {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Nexinets {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Nuvei {
    pub merchant_id: String,
    pub merchant_site_id: String,
    pub merchant_secret: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Opennode {
    pub api_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Payeezy {
    pub api_key: String,
    pub api_secret: String,
    pub merchant_token: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Paypal {
    pub api_key: String,
    pub api_secret: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Payu {
    pub api_key: String,
    pub merchant_pos_id: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Rapyd {
    pub api_secret: String,
    pub secret_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Shift4 {
    pub shift4_api_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Stripe {
    pub stripe_api_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct TrustPay {
    pub api_key: String,
    pub project_id: String,
    pub secret_key: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Worldline {
    pub api_key: String,
    pub api_secret: String,
    pub merchant_account_id: String,
}
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Worldpay {
    pub username: String,
    pub password: String,
}
