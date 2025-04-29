//! Configs interface
use common_enums::{connector_enums,ApplicationError};
use common_utils::errors::CustomResult;
use masking::Secret;
use router_derive;
use serde::Deserialize;

use crate::errors::api_error_response;
// struct Connectors
#[allow(missing_docs, missing_debug_implementations)]
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct Connectors {
    pub aci: ConnectorParams,
    pub adyen: AdyenParamsWithThreeBaseUrls,
    pub adyenplatform: ConnectorParams,
    pub airwallex: ConnectorParams,
    pub amazonpay: ConnectorParams,
    pub applepay: ConnectorParams,
    pub authorizedotnet: ConnectorParams,
    pub bambora: ConnectorParams,
    pub bamboraapac: ConnectorParams,
    pub bankofamerica: ConnectorParams,
    pub billwerk: ConnectorParams,
    pub bitpay: ConnectorParams,
    pub bluesnap: ConnectorParamsWithSecondaryBaseUrl,
    pub boku: ConnectorParams,
    pub braintree: ConnectorParams,
    pub cashtocode: ConnectorParams,
    pub chargebee: ConnectorParams,
    pub checkout: ConnectorParams,
    pub coinbase: ConnectorParams,
    pub coingate: ConnectorParams,
    pub cryptopay: ConnectorParams,
    pub ctp_mastercard: NoParams,
    pub ctp_visa: NoParams,
    pub cybersource: ConnectorParams,
    pub datatrans: ConnectorParamsWithSecondaryBaseUrl,
    pub deutschebank: ConnectorParams,
    pub digitalvirgo: ConnectorParams,
    pub dlocal: ConnectorParams,
    #[cfg(feature = "dummy_connector")]
    pub dummyconnector: ConnectorParams,
    pub ebanx: ConnectorParams,
    pub elavon: ConnectorParams,
    pub facilitapay: ConnectorParams,
    pub fiserv: ConnectorParams,
    pub fiservemea: ConnectorParams,
    pub fiuu: ConnectorParamsWithThreeUrls,
    pub forte: ConnectorParams,
    pub getnet: ConnectorParams,
    pub globalpay: ConnectorParams,
    pub globepay: ConnectorParams,
    pub gocardless: ConnectorParams,
    pub gpayments: ConnectorParams,
    pub helcim: ConnectorParams,
    pub hipay: ConnectorParamsWithThreeUrls,
    pub iatapay: ConnectorParams,
    pub inespay: ConnectorParams,
    pub itaubank: ConnectorParams,
    pub jpmorgan: ConnectorParams,
    pub juspaythreedsserver: ConnectorParams,
    pub klarna: ConnectorParams,
    pub mifinity: ConnectorParams,
    pub mollie: ConnectorParams,
    pub moneris: ConnectorParams,
    pub multisafepay: ConnectorParams,
    pub netcetera: ConnectorParams,
    pub nexinets: ConnectorParams,
    pub nexixpay: ConnectorParams,
    pub nmi: ConnectorParams,
    pub nomupay: ConnectorParams,
    pub noon: ConnectorParamsWithModeType,
    pub novalnet: ConnectorParams,
    pub nuvei: ConnectorParams,
    pub opayo: ConnectorParams,
    pub opennode: ConnectorParams,
    pub paybox: ConnectorParamsWithSecondaryBaseUrl,
    pub payeezy: ConnectorParams,
    pub payme: ConnectorParams,
    pub payone: ConnectorParams,
    pub paypal: ConnectorParams,
    pub paystack: ConnectorParams,
    pub payu: ConnectorParams,
    pub placetopay: ConnectorParams,
    pub plaid: ConnectorParams,
    pub powertranz: ConnectorParams,
    pub prophetpay: ConnectorParams,
    pub rapyd: ConnectorParams,
    pub razorpay: ConnectorParamsWithKeys,
    pub recurly: ConnectorParams,
    pub redsys: ConnectorParams,
    pub riskified: ConnectorParams,
    pub shift4: ConnectorParams,
    pub signifyd: ConnectorParams,
    pub square: ConnectorParams,
    pub stax: ConnectorParams,
    pub stripe: ConnectorParamsWithFileUploadUrl,
    pub stripebilling: ConnectorParams,
    pub taxjar: ConnectorParams,
    pub threedsecureio: ConnectorParams,
    pub thunes: ConnectorParams,
    pub trustpay: ConnectorParamsWithMoreUrls,
    pub tsys: ConnectorParams,
    pub unified_authentication_service: ConnectorParams,
    pub volt: ConnectorParams,
    pub wellsfargo: ConnectorParams,
    pub wellsfargopayout: ConnectorParams,
    pub wise: ConnectorParams,
    pub worldline: ConnectorParams,
    pub worldpay: ConnectorParams,
    pub xendit: ConnectorParams,
    pub zen: ConnectorParams,
    pub zsl: ConnectorParams,
}

impl Connectors {
    pub fn get_connector_params(
        &self,
        connector: connector_enums::Connector,
    ) -> CustomResult<ConnectorParams, api_error_response::ApiErrorResponse> {
        match connector {
            connector_enums::Connector::Recurly=> Ok(self.recurly.clone()),
            connector_enums::Connector::Stripebilling=> Ok(self.stripebilling.clone()),
            connector_enums::Connector::Chargebee => Ok(self.chargebee.clone()),
            #[cfg(feature = "dummy_connector")] 
            connector_enums::Connector::DummyBillingConnector |
            connector_enums::Connector::DummyConnector1 |
            connector_enums::Connector::DummyConnector2 |
            connector_enums::Connector::DummyConnector3 |
            connector_enums::Connector::DummyConnector4 |
            connector_enums::Connector::DummyConnector5 |
            connector_enums::Connector::DummyConnector6 |
            connector_enums::Connector::DummyConnector7  => Err(api_error_response::ApiErrorResponse::IncorrectConnectorNameGiven.into()),
            connector_enums::Connector::Adyenplatform |
            connector_enums::Connector::Aci |
            connector_enums::Connector::Adyen |
            connector_enums::Connector::Airwallex |
            connector_enums::Connector::Authorizedotnet |
            connector_enums::Connector::Bambora |
            connector_enums::Connector::Bamboraapac |
            connector_enums::Connector::Bankofamerica |
            connector_enums::Connector::Billwerk |
            connector_enums::Connector::Bitpay |
            connector_enums::Connector::Bluesnap |
            connector_enums::Connector::Boku |
            connector_enums::Connector::Braintree |
            connector_enums::Connector::Cashtocode |
            connector_enums::Connector::Checkout |
            connector_enums::Connector::Coinbase |
            connector_enums::Connector::Coingate |
            connector_enums::Connector::Cryptopay |
            connector_enums::Connector::CtpMastercard |
            connector_enums::Connector::CtpVisa |
            connector_enums::Connector::Cybersource |
            connector_enums::Connector::Datatrans |
            connector_enums::Connector::Deutschebank |
            connector_enums::Connector::Digitalvirgo |
            connector_enums::Connector::Dlocal |
            connector_enums::Connector::Ebanx |
            connector_enums::Connector::Elavon |
            connector_enums::Connector::Facilitapay |
            connector_enums::Connector::Fiserv |
            connector_enums::Connector::Fiservemea |
            connector_enums::Connector::Fiuu |
            connector_enums::Connector::Forte |
            connector_enums::Connector::Getnet |
            connector_enums::Connector::Globalpay |
            connector_enums::Connector::Globepay |
            connector_enums::Connector::Gocardless |
            connector_enums::Connector::Gpayments |
            connector_enums::Connector::Hipay |
            connector_enums::Connector::Helcim |
            connector_enums::Connector::Inespay |
            connector_enums::Connector::Iatapay |
            connector_enums::Connector::Itaubank |
            connector_enums::Connector::Jpmorgan |
            connector_enums::Connector::Juspaythreedsserver |
            connector_enums::Connector::Klarna |
            connector_enums::Connector::Mifinity |
            connector_enums::Connector::Mollie |
            connector_enums::Connector::Moneris |
            connector_enums::Connector::Multisafepay |
            connector_enums::Connector::Netcetera |
            connector_enums::Connector::Nexinets |
            connector_enums::Connector::Nexixpay |
            connector_enums::Connector::Nmi |
            connector_enums::Connector::Nomupay |
            connector_enums::Connector::Noon |
            connector_enums::Connector::Novalnet |
            connector_enums::Connector::Nuvei |
            connector_enums::Connector::Opennode |
            connector_enums::Connector::Paybox |
            connector_enums::Connector::Payme |
            connector_enums::Connector::Payone |
            connector_enums::Connector::Paypal |
            connector_enums::Connector::Paystack |
            connector_enums::Connector::Payu |
            connector_enums::Connector::Placetopay |
            connector_enums::Connector::Powertranz |
            connector_enums::Connector::Prophetpay |
            connector_enums::Connector::Rapyd |
            connector_enums::Connector::Razorpay |
            connector_enums::Connector::Redsys |
            connector_enums::Connector::Shift4 |
            connector_enums::Connector::Square |
            connector_enums::Connector::Stax |
            connector_enums::Connector::Stripe |
            connector_enums::Connector::Taxjar |
            connector_enums::Connector::Threedsecureio |
            connector_enums::Connector::Trustpay |
            connector_enums::Connector::Tsys |
            connector_enums::Connector::Volt |
            connector_enums::Connector::Wellsfargo |
            connector_enums::Connector::Wise |
            connector_enums::Connector::Worldline |
            connector_enums::Connector::Worldpay |
            connector_enums::Connector::Signifyd |
            connector_enums::Connector::Plaid |
            connector_enums::Connector::Riskified |
            connector_enums::Connector::Xendit |
            connector_enums::Connector::Zen |
            connector_enums::Connector::Zsl
            => Err(api_error_response::ApiErrorResponse::IncorrectConnectorNameGiven.into()),
        }
    }
}

/// struct ConnectorParams
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParams {
    /// base url
    pub base_url: String,
    /// secondary base url
    pub secondary_base_url: Option<String>,
}

///struct No Param for connectors with no params
#[derive(Debug, Deserialize, Clone, Default)]
pub struct NoParams;

impl NoParams {
    /// function to satisfy connector param validation macro
    pub fn validate(&self, _parent_field: &str) -> Result<(), ApplicationError> {
        Ok(())
    }
}

/// struct ConnectorParamsWithKeys
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithKeys {
    /// base url
    pub base_url: String,
    /// api key
    pub api_key: Secret<String>,
    /// merchant ID
    pub merchant_id: Secret<String>,
}

/// struct ConnectorParamsWithModeType
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithModeType {
    /// base url
    pub base_url: String,
    /// secondary base url
    pub secondary_base_url: Option<String>,
    /// Can take values like Test or Live for Noon
    pub key_mode: String,
}

/// struct ConnectorParamsWithMoreUrls
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithMoreUrls {
    /// base url
    pub base_url: String,
    /// base url for bank redirects
    pub base_url_bank_redirects: String,
}

/// struct ConnectorParamsWithFileUploadUrl
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithFileUploadUrl {
    /// base url
    pub base_url: String,
    /// base url for file upload
    pub base_url_file_upload: String,
}

/// struct ConnectorParamsWithThreeBaseUrls
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct AdyenParamsWithThreeBaseUrls {
    /// base url
    pub base_url: String,
    /// secondary base url
    #[cfg(feature = "payouts")]
    pub payout_base_url: String,
    /// third base url
    pub dispute_base_url: String,
}
/// struct ConnectorParamsWithSecondaryBaseUrl
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithSecondaryBaseUrl {
    /// base url
    pub base_url: String,
    /// secondary base url
    pub secondary_base_url: String,
}
/// struct ConnectorParamsWithThreeUrls
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithThreeUrls {
    /// base url
    pub base_url: String,
    /// secondary base url
    pub secondary_base_url: String,
    /// third base url
    pub third_base_url: String,
}
