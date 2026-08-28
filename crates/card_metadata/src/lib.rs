use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CardMetadataError {
    #[error("failed to parse card metadata configuration")]
    ConfigParsingFailed,
}

pub type CardMetadataResult<T> = error_stack::Result<T, CardMetadataError>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardMetadataConfig {
    pub card_subtypes: Vec<String>,
}

impl CardMetadataConfig {
    pub fn load() -> CardMetadataResult<Self> {
        toml::from_str::<Self>(include_str!("../toml/card_subtypes.toml"))
            .change_context(CardMetadataError::ConfigParsingFailed)
            .attach_printable("Unable to deserialize the embedded card metadata TOML")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::CardMetadataConfig;

    #[test]
    fn card_subtypes_are_unique_canonical_values() {
        let config = CardMetadataConfig::load().expect("card metadata config must be valid");
        let unique_subtypes = config.card_subtypes.iter().collect::<HashSet<_>>();

        assert!(!config.card_subtypes.is_empty());
        assert_eq!(unique_subtypes.len(), config.card_subtypes.len());
        assert!(config.card_subtypes.iter().all(|subtype| {
            !subtype.is_empty()
                && subtype.trim() == subtype
                && subtype.chars().all(|character| !character.is_lowercase())
        }));
        assert!(config
            .card_subtypes
            .iter()
            .any(|subtype| subtype == "SMALLCORPORATE"));
        assert!(config
            .card_subtypes
            .iter()
            .any(|subtype| subtype == "VISA TRADITIONAL"));
    }
}
