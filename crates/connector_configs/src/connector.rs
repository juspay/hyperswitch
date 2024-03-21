use std::collections::HashMap;

#[cfg(feature = "payouts")]
use api_models::enums::PayoutConnectors;
use api_models::{
    enums::{AuthenticationConnectors, Connector},
    payments,
};
use serde::Deserialize;
#[cfg(any(feature = "sandbox", feature = "development", feature = "production"))]
use toml;

use crate::common_config::{CardProvider, GooglePayData, Provider, ZenApplePay};

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Classic {
    pub password_classic: String,
    pub username_classic: String,
    pub merchant_id_classic: String,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Evoucher {
    pub password_evoucher: String,
    pub username_evoucher: String,
    pub merchant_id_evoucher: String,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CurrencyAuthKeyType {
    pub classic: Classic,
    pub evoucher: Evoucher,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConnectorAuthType {
    HeaderKey {
        api_key: String,
    },
    BodyKey {
        api_key: String,
        key1: String,
    },
    SignatureKey {
        api_key: String,
        key1: String,
        api_secret: String,
    },
    MultiAuthKey {
        api_key: String,
        key1: String,
        api_secret: String,
        key2: String,
    },
    CurrencyAuthKey {
        auth_key_map: HashMap<String, CurrencyAuthKeyType>,
    },
    #[default]
    NoKey,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum ApplePayTomlConfig {
    Standard(payments::ApplePayMetadata),
    Zen(ZenApplePay),
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub struct ConfigMetadata {
    pub merchant_config_currency: Option<String>,
    pub merchant_account_id: Option<String>,
    pub account_name: Option<String>,
    pub terminal_id: Option<String>,
    pub google_pay: Option<GooglePayData>,
    pub apple_pay: Option<ApplePayTomlConfig>,
    pub merchant_id: Option<String>,
    pub endpoint_prefix: Option<String>,
    pub mcc: Option<String>,
    pub merchant_country_code: Option<String>,
    pub merchant_name: Option<String>,
    pub acquirer_bin: Option<String>,
    pub acquirer_merchant_id: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub struct ConnectorTomlConfig {
    pub connector_auth: Option<ConnectorAuthType>,
    pub connector_webhook_details: Option<api_models::admin::MerchantConnectorWebhookDetails>,
    pub metadata: Option<ConfigMetadata>,
    pub credit: Option<Vec<CardProvider>>,
    pub debit: Option<Vec<CardProvider>>,
    pub bank_transfer: Option<Vec<Provider>>,
    pub bank_redirect: Option<Vec<Provider>>,
    pub bank_debit: Option<Vec<Provider>>,
    pub pay_later: Option<Vec<Provider>>,
    pub wallet: Option<Vec<Provider>>,
    pub crypto: Option<Vec<Provider>>,
    pub reward: Option<Vec<Provider>>,
    pub upi: Option<Vec<Provider>>,
    pub voucher: Option<Vec<Provider>>,
    pub gift_card: Option<Vec<Provider>>,
    pub card_redirect: Option<Vec<Provider>>,
    pub is_verifiable: Option<bool>,
}
#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub struct ConnectorConfig {
    pub aci: Option<ConnectorTomlConfig>,
    pub adyen: Option<ConnectorTomlConfig>,
    #[cfg(feature = "payouts")]
    pub adyen_payout: Option<ConnectorTomlConfig>,
    pub airwallex: Option<ConnectorTomlConfig>,
    pub authorizedotnet: Option<ConnectorTomlConfig>,
    pub bankofamerica: Option<ConnectorTomlConfig>,
    pub bitpay: Option<ConnectorTomlConfig>,
    pub bluesnap: Option<ConnectorTomlConfig>,
    pub boku: Option<ConnectorTomlConfig>,
    pub braintree: Option<ConnectorTomlConfig>,
    pub cashtocode: Option<ConnectorTomlConfig>,
    pub checkout: Option<ConnectorTomlConfig>,
    pub coinbase: Option<ConnectorTomlConfig>,
    pub cryptopay: Option<ConnectorTomlConfig>,
    pub cybersource: Option<ConnectorTomlConfig>,
    pub iatapay: Option<ConnectorTomlConfig>,
    pub opennode: Option<ConnectorTomlConfig>,
    pub bambora: Option<ConnectorTomlConfig>,
    pub dlocal: Option<ConnectorTomlConfig>,
    pub fiserv: Option<ConnectorTomlConfig>,
    pub forte: Option<ConnectorTomlConfig>,
    pub globalpay: Option<ConnectorTomlConfig>,
    pub globepay: Option<ConnectorTomlConfig>,
    pub gocardless: Option<ConnectorTomlConfig>,
    pub helcim: Option<ConnectorTomlConfig>,
    pub klarna: Option<ConnectorTomlConfig>,
    pub mollie: Option<ConnectorTomlConfig>,
    pub multisafepay: Option<ConnectorTomlConfig>,
    pub nexinets: Option<ConnectorTomlConfig>,
    pub nmi: Option<ConnectorTomlConfig>,
    pub noon: Option<ConnectorTomlConfig>,
    pub nuvei: Option<ConnectorTomlConfig>,
    pub payme: Option<ConnectorTomlConfig>,
    pub paypal: Option<ConnectorTomlConfig>,
    pub payu: Option<ConnectorTomlConfig>,
    pub placetopay: Option<ConnectorTomlConfig>,
    pub plaid: Option<ConnectorTomlConfig>,
    pub powertranz: Option<ConnectorTomlConfig>,
    pub prophetpay: Option<ConnectorTomlConfig>,
    pub riskified: Option<ConnectorTomlConfig>,
    pub rapyd: Option<ConnectorTomlConfig>,
    pub shift4: Option<ConnectorTomlConfig>,
    pub stripe: Option<ConnectorTomlConfig>,
    pub signifyd: Option<ConnectorTomlConfig>,
    pub trustpay: Option<ConnectorTomlConfig>,
    pub threedsecureio: Option<ConnectorTomlConfig>,
    pub tsys: Option<ConnectorTomlConfig>,
    pub volt: Option<ConnectorTomlConfig>,
    #[cfg(feature = "payouts")]
    pub wise_payout: Option<ConnectorTomlConfig>,
    pub worldline: Option<ConnectorTomlConfig>,
    pub worldpay: Option<ConnectorTomlConfig>,
    pub zen: Option<ConnectorTomlConfig>,
    pub square: Option<ConnectorTomlConfig>,
    pub stax: Option<ConnectorTomlConfig>,
    pub dummy_connector: Option<ConnectorTomlConfig>,
    pub stripe_test: Option<ConnectorTomlConfig>,
    pub paypal_test: Option<ConnectorTomlConfig>,
}

impl ConnectorConfig {
    fn new() -> Result<Self, String> {
        #[cfg(all(
            feature = "production",
            not(any(feature = "sandbox", feature = "development"))
        ))]
        let config = toml::from_str::<Self>(include_str!("../toml/production.toml"));
        #[cfg(all(
            feature = "sandbox",
            not(any(feature = "production", feature = "development"))
        ))]
        let config = toml::from_str::<Self>(include_str!("../toml/sandbox.toml"));
        #[cfg(feature = "development")]
        let config = toml::from_str::<Self>(include_str!("../toml/development.toml"));

        #[cfg(not(any(feature = "sandbox", feature = "development", feature = "production")))]
        return Err(String::from(
            "Atleast one features has to be enabled for connectorconfig",
        ));

        #[cfg(any(feature = "sandbox", feature = "development", feature = "production"))]
        match config {
            Ok(data) => Ok(data),
            Err(err) => Err(err.to_string()),
        }
    }

    #[cfg(feature = "payouts")]
    pub fn get_payout_connector_config(
        connector: PayoutConnectors,
    ) -> Result<Option<ConnectorTomlConfig>, String> {
        let connector_data = Self::new()?;
        match connector {
            PayoutConnectors::Adyen => Ok(connector_data.adyen_payout),
            PayoutConnectors::Wise => Ok(connector_data.wise_payout),
        }
    }

    pub fn get_authentication_connector_config(
        connector: AuthenticationConnectors,
    ) -> Result<Option<ConnectorTomlConfig>, String> {
        let connector_data = Self::new()?;
        match connector {
            AuthenticationConnectors::Threedsecureio => Ok(connector_data.threedsecureio),
        }
    }

    pub fn get_connector_config(
        connector: Connector,
    ) -> Result<Option<ConnectorTomlConfig>, String> {
        let connector_data = Self::new()?;
        match connector {
            Connector::Aci => Ok(connector_data.aci),
            Connector::Adyen => Ok(connector_data.adyen),
            Connector::Airwallex => Ok(connector_data.airwallex),
            Connector::Authorizedotnet => Ok(connector_data.authorizedotnet),
            Connector::Bankofamerica => Ok(connector_data.bankofamerica),
            Connector::Bitpay => Ok(connector_data.bitpay),
            Connector::Bluesnap => Ok(connector_data.bluesnap),
            Connector::Boku => Ok(connector_data.boku),
            Connector::Braintree => Ok(connector_data.braintree),
            Connector::Cashtocode => Ok(connector_data.cashtocode),
            Connector::Checkout => Ok(connector_data.checkout),
            Connector::Coinbase => Ok(connector_data.coinbase),
            Connector::Cryptopay => Ok(connector_data.cryptopay),
            Connector::Cybersource => Ok(connector_data.cybersource),
            Connector::Iatapay => Ok(connector_data.iatapay),
            Connector::Opennode => Ok(connector_data.opennode),
            Connector::Bambora => Ok(connector_data.bambora),
            Connector::Dlocal => Ok(connector_data.dlocal),
            Connector::Fiserv => Ok(connector_data.fiserv),
            Connector::Forte => Ok(connector_data.forte),
            Connector::Globalpay => Ok(connector_data.globalpay),
            Connector::Globepay => Ok(connector_data.globepay),
            Connector::Gocardless => Ok(connector_data.gocardless),
            Connector::Helcim => Ok(connector_data.helcim),
            Connector::Klarna => Ok(connector_data.klarna),
            Connector::Mollie => Ok(connector_data.mollie),
            Connector::Multisafepay => Ok(connector_data.multisafepay),
            Connector::Nexinets => Ok(connector_data.nexinets),
            Connector::Prophetpay => Ok(connector_data.prophetpay),
            Connector::Nmi => Ok(connector_data.nmi),
            Connector::Noon => Ok(connector_data.noon),
            Connector::Nuvei => Ok(connector_data.nuvei),
            Connector::Payme => Ok(connector_data.payme),
            Connector::Paypal => Ok(connector_data.paypal),
            Connector::Payu => Ok(connector_data.payu),
            Connector::Placetopay => Ok(connector_data.placetopay),
            Connector::Plaid => Ok(connector_data.plaid),
            Connector::Powertranz => Ok(connector_data.powertranz),
            Connector::Rapyd => Ok(connector_data.rapyd),
            Connector::Riskified => Ok(connector_data.riskified),
            Connector::Shift4 => Ok(connector_data.shift4),
            Connector::Signifyd => Ok(connector_data.signifyd),
            Connector::Square => Ok(connector_data.square),
            Connector::Stax => Ok(connector_data.stax),
            Connector::Stripe => Ok(connector_data.stripe),
            Connector::Trustpay => Ok(connector_data.trustpay),
            Connector::Threedsecureio => Ok(connector_data.threedsecureio),
            Connector::Tsys => Ok(connector_data.tsys),
            Connector::Volt => Ok(connector_data.volt),
            Connector::Wise => Err("Use get_payout_connector_config".to_string()),
            Connector::Worldline => Ok(connector_data.worldline),
            Connector::Worldpay => Ok(connector_data.worldpay),
            Connector::Zen => Ok(connector_data.zen),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector1 => Ok(connector_data.dummy_connector),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector2 => Ok(connector_data.dummy_connector),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector3 => Ok(connector_data.dummy_connector),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector4 => Ok(connector_data.stripe_test),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector5 => Ok(connector_data.dummy_connector),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector6 => Ok(connector_data.dummy_connector),
            #[cfg(feature = "dummy_connector")]
            Connector::DummyConnector7 => Ok(connector_data.paypal_test),
        }
    }
}
