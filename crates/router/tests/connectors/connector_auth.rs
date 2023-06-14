use std::env;

use router::types::ConnectorAuthType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectorAuthentication {
    pub aci: Option<BodyKey>,
    pub adyen: Option<BodyKey>,
    pub adyen_uk: Option<BodyKey>,
    pub airwallex: Option<BodyKey>,
    pub authorizedotnet: Option<BodyKey>,
    pub bambora: Option<BodyKey>,
    pub bitpay: Option<HeaderKey>,
    pub bluesnap: Option<BodyKey>,
    pub checkout: Option<SignatureKey>,
    pub coinbase: Option<HeaderKey>,
    pub cybersource: Option<SignatureKey>,
    pub dlocal: Option<SignatureKey>,
    #[cfg(feature = "dummy_connector")]
    pub dummyconnector: Option<HeaderKey>,
    pub fiserv: Option<SignatureKey>,
    pub forte: Option<MultiAuthKey>,
    pub globalpay: Option<HeaderKey>,
    pub iatapay: Option<SignatureKey>,
    pub mollie: Option<HeaderKey>,
    pub multisafepay: Option<HeaderKey>,
    pub nexinets: Option<BodyKey>,
    pub noon: Option<SignatureKey>,
    pub nmi: Option<HeaderKey>,
    pub nuvei: Option<SignatureKey>,
    pub opennode: Option<HeaderKey>,
    pub payeezy: Option<SignatureKey>,
    pub paypal: Option<BodyKey>,
    pub payu: Option<BodyKey>,
    pub rapyd: Option<BodyKey>,
    pub shift4: Option<HeaderKey>,
    pub stripe: Option<HeaderKey>,
    pub stripe_au: Option<HeaderKey>,
    pub stripe_uk: Option<HeaderKey>,
    pub trustpay: Option<SignatureKey>,
    pub worldpay: Option<BodyKey>,
    pub worldline: Option<SignatureKey>,
    pub zen: Option<HeaderKey>,
    pub automation_configs: Option<AutomationConfigs>,
}

impl ConnectorAuthentication {
    #[allow(clippy::expect_used)]
    pub(crate) fn new() -> Self {
        // Do `export CONNECTOR_AUTH_FILE_PATH="/hyperswitch/crates/router/tests/connectors/sample_auth.toml"`
        // before running tests
        let path = env::var("CONNECTOR_AUTH_FILE_PATH")
            .expect("connector authentication file path not set");
        toml::from_str(
            &std::fs::read_to_string(path).expect("connector authentication config file not found"),
        )
        .expect("Failed to read connector authentication config file")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeaderKey {
    pub api_key: String,
}

impl From<HeaderKey> for ConnectorAuthType {
    fn from(key: HeaderKey) -> Self {
        Self::HeaderKey {
            api_key: key.api_key,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BodyKey {
    pub api_key: String,
    pub key1: String,
}

impl From<BodyKey> for ConnectorAuthType {
    fn from(key: BodyKey) -> Self {
        Self::BodyKey {
            api_key: key.api_key,
            key1: key.key1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignatureKey {
    pub api_key: String,
    pub key1: String,
    pub api_secret: String,
}

impl From<SignatureKey> for ConnectorAuthType {
    fn from(key: SignatureKey) -> Self {
        Self::SignatureKey {
            api_key: key.api_key,
            key1: key.key1,
            api_secret: key.api_secret,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiAuthKey {
    pub api_key: String,
    pub key1: String,
    pub api_secret: String,
    pub key2: String,
}

impl From<MultiAuthKey> for ConnectorAuthType {
    fn from(key: MultiAuthKey) -> Self {
        Self::MultiAuthKey {
            api_key: key.api_key,
            key1: key.key1,
            api_secret: key.api_secret,
            key2: key.key2,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutomationConfigs {
    pub hs_base_url: Option<String>,
    pub hs_api_key: Option<String>,
    pub hs_test_browser: Option<String>,
    pub chrome_profile_path: Option<String>,
    pub firefox_profile_path: Option<String>,
    pub pypl_email: Option<String>,
    pub pypl_pass: Option<String>,
    pub gmail_email: Option<String>,
    pub gmail_pass: Option<String>,
    pub configs_url: Option<String>,
    pub stripe_pub_key: Option<String>,
    pub testcases_path: Option<String>,
    pub run_minimum_steps: Option<bool>,
}
