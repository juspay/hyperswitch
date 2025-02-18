use std::collections::HashMap;

#[cfg(feature = "payouts")]
use api_models::enums::PayoutConnectors;
use api_models::{
    enums::{AuthenticationConnectors, Connector, PmAuthConnectors, TaxConnectors},
    payments,
};
use serde::Deserialize;
use toml;

use crate::common_config::{CardProvider, InputData, Provider, ZenApplePay};

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
    CertificateAuth {
        certificate: String,
        private_key: String,
    },
    #[default]
    NoKey,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(untagged)]
pub enum ApplePayTomlConfig {
    Standard(Box<payments::ApplePayMetadata>),
    Zen(ZenApplePay),
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub enum KlarnaEndpoint {
    Europe,
    NorthAmerica,
    Oceania,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub struct ConfigMerchantAdditionalDetails {
    pub open_banking_recipient_data: Option<InputData>,
    pub account_data: Option<InputData>,
    pub iban: Option<Vec<InputData>>,
    pub bacs: Option<Vec<InputData>>,
    pub connector_recipient_id: Option<InputData>,
    pub wallet_id: Option<InputData>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub struct ConfigMetadata {
    pub merchant_config_currency: Option<InputData>,
    pub merchant_account_id: Option<InputData>,
    pub account_name: Option<InputData>,
    pub terminal_id: Option<InputData>,
    pub google_pay: Option<Vec<InputData>>,
    pub apple_pay: Option<Vec<InputData>>,
    pub merchant_id: Option<InputData>,
    pub endpoint_prefix: Option<InputData>,
    pub mcc: Option<InputData>,
    pub merchant_country_code: Option<InputData>,
    pub merchant_name: Option<InputData>,
    pub acquirer_bin: Option<InputData>,
    pub acquirer_merchant_id: Option<InputData>,
    pub acquirer_country_code: Option<InputData>,
    pub three_ds_requestor_name: Option<InputData>,
    pub three_ds_requestor_id: Option<InputData>,
    pub pull_mechanism_for_external_3ds_enabled: Option<InputData>,
    pub klarna_region: Option<InputData>,
    pub source_balance_account: Option<InputData>,
    pub brand_id: Option<InputData>,
    pub destination_account_number: Option<InputData>,
    pub dpa_id: Option<InputData>,
    pub dpa_name: Option<InputData>,
    pub locale: Option<InputData>,
    pub card_brands: Option<InputData>,
    pub merchant_category_code: Option<InputData>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub struct ConnectorWalletDetailsConfig {
    pub samsung_pay: Option<Vec<InputData>>,
    pub paze: Option<Vec<InputData>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub struct ConnectorTomlConfig {
    pub connector_auth: Option<ConnectorAuthType>,
    pub connector_webhook_details: Option<api_models::admin::MerchantConnectorWebhookDetails>,
    pub metadata: Option<Box<ConfigMetadata>>,
    pub connector_wallets_details: Option<Box<ConnectorWalletDetailsConfig>>,
    pub additional_merchant_data: Option<Box<ConfigMerchantAdditionalDetails>>,
    pub credit: Option<Vec<CardProvider>>,
    pub debit: Option<Vec<CardProvider>>,
    pub bank_transfer: Option<Vec<Provider>>,
    pub bank_redirect: Option<Vec<Provider>>,
    pub bank_debit: Option<Vec<Provider>>,
    pub open_banking: Option<Vec<Provider>>,
    pub pay_later: Option<Vec<Provider>>,
    pub wallet: Option<Vec<Provider>>,
    pub crypto: Option<Vec<Provider>>,
    pub reward: Option<Vec<Provider>>,
    pub upi: Option<Vec<Provider>>,
    pub voucher: Option<Vec<Provider>>,
    pub gift_card: Option<Vec<Provider>>,
    pub card_redirect: Option<Vec<Provider>>,
    pub is_verifiable: Option<bool>,
    pub real_time_payment: Option<Vec<Provider>>,
}
#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub struct ConnectorConfig {
    pub aci: Option<ConnectorTomlConfig>,
    pub adyen: Option<ConnectorTomlConfig>,
    #[cfg(feature = "payouts")]
    pub adyen_payout: Option<ConnectorTomlConfig>,
    #[cfg(feature = "payouts")]
    pub adyenplatform_payout: Option<ConnectorTomlConfig>,
    pub airwallex: Option<ConnectorTomlConfig>,
    pub authorizedotnet: Option<ConnectorTomlConfig>,
    pub bamboraapac: Option<ConnectorTomlConfig>,
    pub bankofamerica: Option<ConnectorTomlConfig>,
    pub billwerk: Option<ConnectorTomlConfig>,
    pub bitpay: Option<ConnectorTomlConfig>,
    pub bluesnap: Option<ConnectorTomlConfig>,
    pub boku: Option<ConnectorTomlConfig>,
    pub braintree: Option<ConnectorTomlConfig>,
    pub cashtocode: Option<ConnectorTomlConfig>,
    pub checkout: Option<ConnectorTomlConfig>,
    pub coinbase: Option<ConnectorTomlConfig>,
    pub coingate: Option<ConnectorTomlConfig>,
    pub cryptopay: Option<ConnectorTomlConfig>,
    pub cybersource: Option<ConnectorTomlConfig>,
    #[cfg(feature = "payouts")]
    pub cybersource_payout: Option<ConnectorTomlConfig>,
    pub iatapay: Option<ConnectorTomlConfig>,
    pub itaubank: Option<ConnectorTomlConfig>,
    pub opennode: Option<ConnectorTomlConfig>,
    pub bambora: Option<ConnectorTomlConfig>,
    pub datatrans: Option<ConnectorTomlConfig>,
    pub deutschebank: Option<ConnectorTomlConfig>,
    pub digitalvirgo: Option<ConnectorTomlConfig>,
    pub dlocal: Option<ConnectorTomlConfig>,
    pub ebanx_payout: Option<ConnectorTomlConfig>,
    pub elavon: Option<ConnectorTomlConfig>,
    pub fiserv: Option<ConnectorTomlConfig>,
    pub fiservemea: Option<ConnectorTomlConfig>,
    pub fiuu: Option<ConnectorTomlConfig>,
    pub forte: Option<ConnectorTomlConfig>,
    // pub getnet: Option<ConnectorTomlConfig>,
    pub globalpay: Option<ConnectorTomlConfig>,
    pub globepay: Option<ConnectorTomlConfig>,
    pub gocardless: Option<ConnectorTomlConfig>,
    pub gpayments: Option<ConnectorTomlConfig>,
    pub helcim: Option<ConnectorTomlConfig>,
    pub inespay: Option<ConnectorTomlConfig>,
    pub jpmorgan: Option<ConnectorTomlConfig>,
    pub klarna: Option<ConnectorTomlConfig>,
    pub mifinity: Option<ConnectorTomlConfig>,
    pub mollie: Option<ConnectorTomlConfig>,
    pub moneris: Option<ConnectorTomlConfig>,
    pub multisafepay: Option<ConnectorTomlConfig>,
    pub nexinets: Option<ConnectorTomlConfig>,
    pub nexixpay: Option<ConnectorTomlConfig>,
    pub nmi: Option<ConnectorTomlConfig>,
    pub noon: Option<ConnectorTomlConfig>,
    pub novalnet: Option<ConnectorTomlConfig>,
    pub nuvei: Option<ConnectorTomlConfig>,
    pub paybox: Option<ConnectorTomlConfig>,
    pub payme: Option<ConnectorTomlConfig>,
    #[cfg(feature = "payouts")]
    pub payone_payout: Option<ConnectorTomlConfig>,
    pub paypal: Option<ConnectorTomlConfig>,
    #[cfg(feature = "payouts")]
    pub paypal_payout: Option<ConnectorTomlConfig>,
    pub payu: Option<ConnectorTomlConfig>,
    pub placetopay: Option<ConnectorTomlConfig>,
    pub plaid: Option<ConnectorTomlConfig>,
    pub powertranz: Option<ConnectorTomlConfig>,
    pub prophetpay: Option<ConnectorTomlConfig>,
    pub razorpay: Option<ConnectorTomlConfig>,
    pub riskified: Option<ConnectorTomlConfig>,
    pub rapyd: Option<ConnectorTomlConfig>,
    pub shift4: Option<ConnectorTomlConfig>,
    pub stripe: Option<ConnectorTomlConfig>,
    #[cfg(feature = "payouts")]
    pub stripe_payout: Option<ConnectorTomlConfig>,
    pub signifyd: Option<ConnectorTomlConfig>,
    pub trustpay: Option<ConnectorTomlConfig>,
    pub threedsecureio: Option<ConnectorTomlConfig>,
    pub netcetera: Option<ConnectorTomlConfig>,
    pub tsys: Option<ConnectorTomlConfig>,
    pub volt: Option<ConnectorTomlConfig>,
    pub wellsfargo: Option<ConnectorTomlConfig>,
    #[cfg(feature = "payouts")]
    pub wise_payout: Option<ConnectorTomlConfig>,
    pub worldline: Option<ConnectorTomlConfig>,
    pub worldpay: Option<ConnectorTomlConfig>,
    pub xendit: Option<ConnectorTomlConfig>,
    pub square: Option<ConnectorTomlConfig>,
    pub stax: Option<ConnectorTomlConfig>,
    pub dummy_connector: Option<ConnectorTomlConfig>,
    pub stripe_test: Option<ConnectorTomlConfig>,
    pub paypal_test: Option<ConnectorTomlConfig>,
    pub zen: Option<ConnectorTomlConfig>,
    pub zsl: Option<ConnectorTomlConfig>,
    pub taxjar: Option<ConnectorTomlConfig>,
    pub ctp_mastercard: Option<ConnectorTomlConfig>,
    pub unified_authentication_service: Option<ConnectorTomlConfig>,
}

impl ConnectorConfig {
    fn new() -> Result<Self, String> {
        let config_str = if cfg!(feature = "production") {
            include_str!("../toml/production.toml")
        } else if cfg!(feature = "sandbox") {
            include_str!("../toml/sandbox.toml")
        } else {
            include_str!("../toml/development.toml")
        };
        let config = toml::from_str::<Self>(config_str);
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
            PayoutConnectors::Adyenplatform => Ok(connector_data.adyenplatform_payout),
            PayoutConnectors::Cybersource => Ok(connector_data.cybersource_payout),
            PayoutConnectors::Ebanx => Ok(connector_data.ebanx_payout),
            PayoutConnectors::Payone => Ok(connector_data.payone_payout),
            PayoutConnectors::Paypal => Ok(connector_data.paypal_payout),
            PayoutConnectors::Stripe => Ok(connector_data.stripe_payout),
            PayoutConnectors::Wise => Ok(connector_data.wise_payout),
        }
    }

    pub fn get_authentication_connector_config(
        connector: AuthenticationConnectors,
    ) -> Result<Option<ConnectorTomlConfig>, String> {
        let connector_data = Self::new()?;
        match connector {
            AuthenticationConnectors::Threedsecureio => Ok(connector_data.threedsecureio),
            AuthenticationConnectors::Netcetera => Ok(connector_data.netcetera),
            AuthenticationConnectors::Gpayments => Ok(connector_data.gpayments),
            AuthenticationConnectors::CtpMastercard => Ok(connector_data.ctp_mastercard),
            AuthenticationConnectors::UnifiedAuthenticationService => {
                Ok(connector_data.unified_authentication_service)
            }
        }
    }

    pub fn get_tax_processor_config(
        connector: TaxConnectors,
    ) -> Result<Option<ConnectorTomlConfig>, String> {
        let connector_data = Self::new()?;
        match connector {
            TaxConnectors::Taxjar => Ok(connector_data.taxjar),
        }
    }

    pub fn get_pm_authentication_processor_config(
        connector: PmAuthConnectors,
    ) -> Result<Option<ConnectorTomlConfig>, String> {
        let connector_data = Self::new()?;
        match connector {
            PmAuthConnectors::Plaid => Ok(connector_data.plaid),
        }
    }

    pub fn get_connector_config(
        connector: Connector,
    ) -> Result<Option<ConnectorTomlConfig>, String> {
        let connector_data = Self::new()?;
        match connector {
            Connector::Aci => Ok(connector_data.aci),
            Connector::Adyen => Ok(connector_data.adyen),
            Connector::Adyenplatform => Err("Use get_payout_connector_config".to_string()),
            Connector::Airwallex => Ok(connector_data.airwallex),
            Connector::Authorizedotnet => Ok(connector_data.authorizedotnet),
            Connector::Bamboraapac => Ok(connector_data.bamboraapac),
            Connector::Bankofamerica => Ok(connector_data.bankofamerica),
            Connector::Billwerk => Ok(connector_data.billwerk),
            Connector::Bitpay => Ok(connector_data.bitpay),
            Connector::Bluesnap => Ok(connector_data.bluesnap),
            Connector::Boku => Ok(connector_data.boku),
            Connector::Braintree => Ok(connector_data.braintree),
            Connector::Cashtocode => Ok(connector_data.cashtocode),
            Connector::Checkout => Ok(connector_data.checkout),
            Connector::Coinbase => Ok(connector_data.coinbase),
            Connector::Coingate => Ok(connector_data.coingate),
            Connector::Cryptopay => Ok(connector_data.cryptopay),
            Connector::Cybersource => Ok(connector_data.cybersource),
            Connector::Iatapay => Ok(connector_data.iatapay),
            Connector::Itaubank => Ok(connector_data.itaubank),
            Connector::Opennode => Ok(connector_data.opennode),
            Connector::Bambora => Ok(connector_data.bambora),
            Connector::Datatrans => Ok(connector_data.datatrans),
            Connector::Deutschebank => Ok(connector_data.deutschebank),
            Connector::Digitalvirgo => Ok(connector_data.digitalvirgo),
            Connector::Dlocal => Ok(connector_data.dlocal),
            Connector::Ebanx => Ok(connector_data.ebanx_payout),
            Connector::Elavon => Ok(connector_data.elavon),
            Connector::Fiserv => Ok(connector_data.fiserv),
            Connector::Fiservemea => Ok(connector_data.fiservemea),
            Connector::Fiuu => Ok(connector_data.fiuu),
            Connector::Forte => Ok(connector_data.forte),
            // Connector::Getnet => Ok(connector_data.getnet),
            Connector::Globalpay => Ok(connector_data.globalpay),
            Connector::Globepay => Ok(connector_data.globepay),
            Connector::Gocardless => Ok(connector_data.gocardless),
            Connector::Gpayments => Ok(connector_data.gpayments),
            Connector::Helcim => Ok(connector_data.helcim),
            Connector::Inespay => Ok(connector_data.inespay),
            Connector::Jpmorgan => Ok(connector_data.jpmorgan),
            Connector::Klarna => Ok(connector_data.klarna),
            Connector::Mifinity => Ok(connector_data.mifinity),
            Connector::Mollie => Ok(connector_data.mollie),
            Connector::Moneris => Ok(connector_data.moneris),
            Connector::Multisafepay => Ok(connector_data.multisafepay),
            Connector::Nexinets => Ok(connector_data.nexinets),
            Connector::Nexixpay => Ok(connector_data.nexixpay),
            Connector::Prophetpay => Ok(connector_data.prophetpay),
            Connector::Nmi => Ok(connector_data.nmi),
            Connector::Novalnet => Ok(connector_data.novalnet),
            Connector::Noon => Ok(connector_data.noon),
            Connector::Nuvei => Ok(connector_data.nuvei),
            Connector::Paybox => Ok(connector_data.paybox),
            Connector::Payme => Ok(connector_data.payme),
            Connector::Payone => Err("Use get_payout_connector_config".to_string()),
            Connector::Paypal => Ok(connector_data.paypal),
            Connector::Payu => Ok(connector_data.payu),
            Connector::Placetopay => Ok(connector_data.placetopay),
            Connector::Plaid => Ok(connector_data.plaid),
            Connector::Powertranz => Ok(connector_data.powertranz),
            Connector::Razorpay => Ok(connector_data.razorpay),
            Connector::Rapyd => Ok(connector_data.rapyd),
            Connector::Riskified => Ok(connector_data.riskified),
            Connector::Shift4 => Ok(connector_data.shift4),
            Connector::Signifyd => Ok(connector_data.signifyd),
            Connector::Square => Ok(connector_data.square),
            Connector::Stax => Ok(connector_data.stax),
            Connector::Stripe => Ok(connector_data.stripe),
            Connector::Trustpay => Ok(connector_data.trustpay),
            Connector::Threedsecureio => Ok(connector_data.threedsecureio),
            Connector::Taxjar => Ok(connector_data.taxjar),
            Connector::Tsys => Ok(connector_data.tsys),
            Connector::Volt => Ok(connector_data.volt),
            Connector::Wellsfargo => Ok(connector_data.wellsfargo),
            Connector::Wise => Err("Use get_payout_connector_config".to_string()),
            Connector::Worldline => Ok(connector_data.worldline),
            Connector::Worldpay => Ok(connector_data.worldpay),
            Connector::Zen => Ok(connector_data.zen),
            Connector::Zsl => Ok(connector_data.zsl),
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
            Connector::Netcetera => Ok(connector_data.netcetera),
            Connector::CtpMastercard => Ok(connector_data.ctp_mastercard),
            Connector::Xendit => Ok(connector_data.xendit),
        }
    }
}
