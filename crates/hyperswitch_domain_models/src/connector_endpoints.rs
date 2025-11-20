//! Configs interface
use common_enums::{connector_enums, ApplicationError};
use common_utils::errors::CustomResult;
use masking::Secret;
use serde::Deserialize;

use crate::errors::api_error_response;
// struct Connectors
#[allow(missing_docs, missing_debug_implementations)]
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct Connectors {
    pub aci: ConnectorParams,
    pub authipay: ConnectorParams,
    pub adyen: AdyenParamsWithThreeBaseUrls,
    pub adyenplatform: ConnectorParams,
    pub affirm: ConnectorParams,
    pub airwallex: ConnectorParams,
    pub amazonpay: ConnectorParams,
    pub applepay: ConnectorParams,
    pub archipel: ConnectorParams,
    pub authorizedotnet: ConnectorParams,
    pub bambora: ConnectorParams,
    pub bamboraapac: ConnectorParams,
    pub bankofamerica: ConnectorParams,
    pub barclaycard: ConnectorParams,
    pub billwerk: ConnectorParams,
    pub bitpay: ConnectorParams,
    pub blackhawknetwork: ConnectorParams,
    pub calida: ConnectorParams,
    pub bluesnap: ConnectorParamsWithSecondaryBaseUrl,
    pub boku: ConnectorParams,
    pub braintree: ConnectorParams,
    pub breadpay: ConnectorParams,
    pub cashtocode: ConnectorParams,
    pub celero: ConnectorParams,
    pub chargebee: ConnectorParams,
    pub checkbook: ConnectorParams,
    pub checkout: ConnectorParams,
    pub coinbase: ConnectorParams,
    pub coingate: ConnectorParams,
    pub cryptopay: ConnectorParams,
    pub ctp_mastercard: NoParams,
    pub ctp_visa: NoParams,
    pub custombilling: NoParams,
    pub cybersource: ConnectorParams,
    pub datatrans: ConnectorParamsWithSecondaryBaseUrl,
    pub deutschebank: ConnectorParams,
    pub digitalvirgo: ConnectorParams,
    pub dlocal: ConnectorParams,
    #[cfg(feature = "dummy_connector")]
    pub dummyconnector: ConnectorParams,
    pub dwolla: ConnectorParams,
    pub ebanx: ConnectorParams,
    pub elavon: ConnectorParams,
    pub envoy: ConnectorParams,
    pub facilitapay: ConnectorParams,
    pub finix: ConnectorParams,
    pub fiserv: ConnectorParams,
    pub fiservemea: ConnectorParams,
    pub fiuu: ConnectorParamsWithThreeUrls,
    pub flexiti: ConnectorParams,
    pub forte: ConnectorParams,
    pub getnet: ConnectorParams,
    pub gigadat: ConnectorParams,
    pub globalpay: ConnectorParams,
    pub globepay: ConnectorParams,
    pub gocardless: ConnectorParams,
    pub gpayments: ConnectorParams,
    pub helcim: ConnectorParams,
    pub hipay: ConnectorParamsWithThreeUrls,
    pub hyperswitch_vault: ConnectorParams,
    pub hyperwallet: ConnectorParams,
    pub iatapay: ConnectorParams,
    pub inespay: ConnectorParams,
    pub itaubank: ConnectorParams,
    pub jpmorgan: ConnectorParams,
    pub juspaythreedsserver: ConnectorParams,
    pub cardinal: NoParams,
    pub katapult: ConnectorParams,
    pub klarna: ConnectorParams,
    pub loonio: ConnectorParams,
    pub mifinity: ConnectorParams,
    pub mollie: ConnectorParams,
    pub moneris: ConnectorParams,
    pub mpgs: ConnectorParams,
    pub multisafepay: ConnectorParams,
    pub netcetera: ConnectorParams,
    pub nexinets: ConnectorParams,
    pub nexixpay: ConnectorParams,
    pub nmi: ConnectorParams,
    pub nomupay: ConnectorParams,
    pub noon: ConnectorParamsWithModeType,
    pub nordea: ConnectorParams,
    pub novalnet: ConnectorParams,
    pub nuvei: ConnectorParams,
    pub opayo: ConnectorParams,
    pub opennode: ConnectorParams,
    pub paybox: ConnectorParamsWithSecondaryBaseUrl,
    pub payeezy: ConnectorParams,
    pub payjustnow: ConnectorParams,
    pub payload: ConnectorParams,
    pub payme: ConnectorParams,
    pub payone: ConnectorParams,
    pub paypal: ConnectorParams,
    pub paysafe: ConnectorParams,
    pub paystack: ConnectorParams,
    pub paytm: ConnectorParams,
    pub payu: ConnectorParams,
    pub peachpayments: ConnectorParams,
    pub phonepe: ConnectorParams,
    pub placetopay: ConnectorParams,
    pub plaid: ConnectorParams,
    pub powertranz: ConnectorParams,
    pub prophetpay: ConnectorParams,
    pub rapyd: ConnectorParams,
    pub razorpay: ConnectorParamsWithKeys,
    pub recurly: ConnectorParams,
    pub redsys: ConnectorParams,
    pub riskified: ConnectorParams,
    pub santander: ConnectorParams,
    pub shift4: ConnectorParams,
    pub sift: ConnectorParams,
    pub silverflow: ConnectorParams,
    pub signifyd: ConnectorParams,
    pub square: ConnectorParams,
    pub stax: ConnectorParams,
    pub stripe: ConnectorParamsWithFileUploadUrl,
    pub stripebilling: ConnectorParams,
    pub taxjar: ConnectorParams,
    pub tesouro: ConnectorParams,
    pub threedsecureio: ConnectorParams,
    pub thunes: ConnectorParams,
    pub tokenex: ConnectorParams,
    pub tokenio: ConnectorParams,
    pub trustpay: ConnectorParamsWithMoreUrls,
    pub trustpayments: ConnectorParams,
    pub tsys: ConnectorParams,
    pub unified_authentication_service: ConnectorParams,
    pub vgs: ConnectorParams,
    pub volt: ConnectorParams,
    pub wellsfargo: ConnectorParams,
    pub wellsfargopayout: ConnectorParams,
    pub wise: ConnectorParams,
    pub worldline: ConnectorParams,
    pub worldpay: ConnectorParams,
    pub worldpayvantiv: ConnectorParamsWithThreeUrls,
    pub worldpayxml: ConnectorParams,
    pub xendit: ConnectorParams,
    pub zift: ConnectorParams,
    pub zen: ConnectorParams,
    pub zsl: ConnectorParams,
}

impl Connectors {
    pub fn get_connector_params(
        &self,
        connector: connector_enums::Connector,
    ) -> CustomResult<ConnectorParams, api_error_response::ApiErrorResponse> {
        match connector {
            connector_enums::Connector::Recurly => Ok(self.recurly.clone()),
            connector_enums::Connector::Stripebilling => Ok(self.stripebilling.clone()),
            connector_enums::Connector::Chargebee => Ok(self.chargebee.clone()),
            _ => Err(api_error_response::ApiErrorResponse::IncorrectConnectorNameGiven.into()),
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
