//! Helpers shared by the configuration sections.

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    hash::Hash,
    str::FromStr,
};

use serde::{Deserialize, Deserializer};

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

/// A comma-separated value, as a set.
///
/// `router` carries the same helper for its own configuration. Copied rather than shared, so a
/// change made for one of its settings cannot quietly change how alarms are read here.
pub(crate) fn deserialize_hashset<'de, D, T>(deserializer: D) -> Result<HashSet<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Eq + Hash + FromStr,
    <T as FromStr>::Err: Display,
{
    use serde::de::Error;

    deserialize_hashset_inner(<String>::deserialize(deserializer)?).map_err(D::Error::custom)
}

pub(crate) fn deserialize_hashset_inner<T>(value: impl AsRef<str>) -> Result<HashSet<T>, String>
where
    T: Eq + Hash + FromStr,
    <T as FromStr>::Err: Display,
{
    let (values, errors) = value
        .as_ref()
        .trim()
        .split(',')
        .map(|element| {
            T::from_str(element.trim()).map_err(|error| {
                format!(
                    "Unable to deserialize `{}` as `{}`: {error}",
                    element.trim(),
                    std::any::type_name::<T>()
                )
            })
        })
        .fold(
            (HashSet::new(), Vec::new()),
            |(mut values, mut errors), result| {
                match result {
                    Ok(value) => {
                        values.insert(value);
                    }
                    Err(error) => errors.push(error),
                }
                (values, errors)
            },
        );

    if errors.is_empty() {
        Ok(values)
    } else {
        Err(format!("Some errors occurred:\n{}", errors.join("\n")))
    }
}
