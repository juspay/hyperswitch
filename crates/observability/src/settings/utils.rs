//! Helpers shared by the configuration sections.

use std::collections::HashMap;

use crate::errors;

/// Ids are addressed by callers and set from the environment, so they must survive both. `config`
/// lowercases environment keys and splits on `__`; an id that would come back different is
/// rejected at boot rather than silently failing to match at lookup time.
pub(crate) fn validate_config_ids<T>(
    entries: &HashMap<String, T>,
    section: &str,
) -> Result<(), errors::ConfigurationError> {
    for id in entries.keys() {
        if id.is_empty() || id.contains("__") || id != &id.to_lowercase() {
            Err(errors::ConfigurationError::ConfigParsingError(format!(
                "{section} id `{id}` must be lowercase, non-empty and free of `__`, so that it \
                 can be set from the environment"
            )))?
        }
    }
    Ok(())
}
