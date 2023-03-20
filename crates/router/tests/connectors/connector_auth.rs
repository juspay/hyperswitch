use router::types::ConnectorAuthType;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ConnectorAuthentication {
	pub aci: Option<BodyKey>,
	pub forte: Option<HeaderKey>,
    pub adyen: Option<BodyKey>,
    pub airwallex: Option<BodyKey>,
    pub authorizedotnet: Option<BodyKey>,
    pub bambora: Option<BodyKey>,
    pub bluesnap: Option<BodyKey>,
    pub checkout: Option<BodyKey>,
    pub cybersource: Option<SignatureKey>,
    pub dlocal: Option<SignatureKey>,
    pub fiserv: Option<SignatureKey>,
    pub globalpay: Option<HeaderKey>,
    pub mollie: Option<HeaderKey>,
    pub multisafepay: Option<HeaderKey>,
    pub nuvei: Option<SignatureKey>,
    pub payu: Option<BodyKey>,
    pub rapyd: Option<BodyKey>,
    pub shift4: Option<HeaderKey>,
    pub stripe: Option<HeaderKey>,
    pub worldpay: Option<BodyKey>,
    pub worldline: Option<SignatureKey>,
    pub trustpay: Option<SignatureKey>,
}

impl ConnectorAuthentication {
    pub(crate) fn new() -> Self {
        #[allow(clippy::expect_used)]
        toml::from_str(
            &std::fs::read_to_string("tests/connectors/sample_auth.toml")
                .expect("connector authentication config file not found"),
        )
        .expect("Failed to read connector authentication config file")
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct HeaderKey {
    pub api_key: String,
}

impl From<HeaderKey> for ConnectorAuthType {
    fn from(key: HeaderKey) -> Self {
        Self::HeaderKey {
            api_key: key.api_key,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct BodyKey {
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

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct SignatureKey {
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
