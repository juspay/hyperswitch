use hyperswitch_masking::Secret;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OfferEngineCredentialSource {
    None,
    Application,
}

#[derive(Debug, Clone)]
pub struct ResolvedOfferEngineConfig {
    pub base_url: url::Url,
    pub api_key: Secret<String>,
    pub merchant_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OfferEngineError {
    #[error("Offer Engine application config is missing or invalid: {0}")]
    MissingApplicationConfig(String),
    #[error("Offer Engine request failed")]
    RequestFailed,
    #[error("Failed to parse Offer Engine response")]
    ResponseParseFailed,
}
