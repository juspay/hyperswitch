use router::types::ConnectorAuthType;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ConnectorAuthentication {
    pub aci: Option<BodyKey>,
    pub authorizedotnet: Option<BodyKey>,
    pub checkout: Option<BodyKey>,
    pub shift4: Option<HeaderKey>,
}

impl ConnectorAuthentication {
    pub(crate) fn new() -> Self {
        #[allow(clippy::expect_used)]
        toml::de::from_slice(
            &std::fs::read("tests/connectors/auth.toml")
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
