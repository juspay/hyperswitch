use hyperswitch_masking::Secret;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferEngineCredentialSource {
    Application,
}

impl OfferEngineCredentialSource {
    pub fn parse(raw: &str) -> Result<Option<Self>, error_stack::Report<OfferEngineError>> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "none" => Ok(None),
            "application" => Ok(Some(Self::Application)),
            other => Err(error_stack::report!(
                OfferEngineError::InvalidCredentialSource(other.to_string())
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedOfferEngineConfig {
    pub base_url: url::Url,
    pub api_key: Secret<String>,
    pub merchant_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OfferEngineError {
    #[error("Failed to fetch Offer Engine enablement flag from Superposition")]
    EnablementUnavailable,
    #[error("Failed to fetch Offer Engine credential source from Superposition")]
    CredentialSourceUnavailable,
    #[error("Unrecognised Offer Engine credential source: {0}")]
    InvalidCredentialSource(String),
    #[error("Offer Engine application config is missing or invalid: {0}")]
    MissingApplicationConfig(String),
    #[error("Offer Engine request failed")]
    RequestFailed,
    #[error("Failed to parse Offer Engine response")]
    ResponseParseFailed,
}
