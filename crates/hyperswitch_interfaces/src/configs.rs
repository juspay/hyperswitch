//! Configs interface
use common_enums::ApplicationError;
use masking::Secret;
use router_derive;
use serde::Deserialize;

// struct Connectors
#[allow(missing_docs, missing_debug_implementations)]
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct Connectors {
    pub aci: ConnectorParams,
    #[cfg(feature = "payouts")]
    pub adyen: ConnectorParamsWithSecondaryBaseUrl,
    pub adyenplatform: ConnectorParams,
    #[cfg(not(feature = "payouts"))]
    pub adyen: ConnectorParams,
    pub airwallex: ConnectorParams,
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
    pub checkout: ConnectorParams,
    pub coinbase: ConnectorParams,
    pub cryptopay: ConnectorParams,
    pub cybersource: ConnectorParams,
    pub datatrans: ConnectorParams,
    pub dlocal: ConnectorParams,
    #[cfg(feature = "dummy_connector")]
    pub dummyconnector: ConnectorParams,
    pub ebanx: ConnectorParams,
    pub fiserv: ConnectorParams,
    pub forte: ConnectorParams,
    pub globalpay: ConnectorParams,
    pub globepay: ConnectorParams,
    pub gocardless: ConnectorParams,
    pub gpayments: ConnectorParams,
    pub helcim: ConnectorParams,
    pub iatapay: ConnectorParams,
    pub klarna: ConnectorParams,
    pub mifinity: ConnectorParams,
    pub mollie: ConnectorParams,
    pub multisafepay: ConnectorParams,
    pub netcetera: ConnectorParams,
    pub nexinets: ConnectorParams,
    pub nmi: ConnectorParams,
    pub noon: ConnectorParamsWithModeType,
    pub nuvei: ConnectorParams,
    pub opayo: ConnectorParams,
    pub opennode: ConnectorParams,
    pub payeezy: ConnectorParams,
    pub payme: ConnectorParams,
    pub payone: ConnectorParams,
    pub paypal: ConnectorParams,
    pub payu: ConnectorParams,
    pub placetopay: ConnectorParams,
    pub powertranz: ConnectorParams,
    pub prophetpay: ConnectorParams,
    pub rapyd: ConnectorParams,
    pub razorpay: ConnectorParamsWithKeys,
    pub riskified: ConnectorParams,
    pub shift4: ConnectorParams,
    pub signifyd: ConnectorParams,
    pub square: ConnectorParams,
    pub stax: ConnectorParams,
    pub stripe: ConnectorParamsWithFileUploadUrl,
    pub threedsecureio: ConnectorParams,
    pub trustpay: ConnectorParamsWithMoreUrls,
    pub tsys: ConnectorParams,
    pub volt: ConnectorParams,
    pub wise: ConnectorParams,
    pub worldline: ConnectorParams,
    pub worldpay: ConnectorParams,
    pub zen: ConnectorParams,
    pub zsl: ConnectorParams,
    pub plaid: ConnectorParams,
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

/// struct ConnectorParamsWithSecondaryBaseUrl
#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithSecondaryBaseUrl {
    /// base url
    pub base_url: String,
    /// secondary base url
    pub secondary_base_url: String,
}
