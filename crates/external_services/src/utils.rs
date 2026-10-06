//! Custom deserializers for external services configuration

use std::collections::HashSet;

use serde::Deserialize;

/// Parses a comma-separated string into a HashSet of typed values.
///
/// # Arguments
///
/// * `value` - String or string reference containing comma-separated values
///
/// # Returns
///
/// * `Ok(HashSet<T>)` - Successfully parsed HashSet
/// * `Err(String)` - Error message if any value parsing fails
///
/// # Type Parameters
///
/// * `T` - Target type that implements `FromStr`, `Eq`, and `Hash`
///
/// # Examples
///
/// ```
/// use std::collections::HashSet;
///
/// let result: Result<HashSet<i32>, String> =
///     deserialize_hashset_inner("1,2,3");
/// assert!(result.is_ok());
///
/// if let Ok(hashset) = result {
///     assert!(hashset.contains(&1));
///     assert!(hashset.contains(&2));
///     assert!(hashset.contains(&3));
/// }
/// ```
fn deserialize_hashset_inner<T>(value: impl AsRef<str>) -> Result<HashSet<T>, String>
where
    T: Eq + std::str::FromStr + std::hash::Hash,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    let (values, errors) = value
        .as_ref()
        .trim()
        .split(',')
        .map(|s| {
            T::from_str(s.trim()).map_err(|error| {
                format!(
                    "Unable to deserialize `{}` as `{}`: {error}",
                    s.trim(),
                    std::any::type_name::<T>()
                )
            })
        })
        .fold(
            (HashSet::new(), Vec::new()),
            |(mut values, mut errors), result| match result {
                Ok(t) => {
                    values.insert(t);
                    (values, errors)
                }
                Err(error) => {
                    errors.push(error);
                    (values, errors)
                }
            },
        );
    if !errors.is_empty() {
        Err(format!("Some errors occurred:\n{}", errors.join("\n")))
    } else {
        Ok(values)
    }
}

/// Serde deserializer function for converting comma-separated strings into typed HashSets.
///
/// This function is designed to be used with serde's `#[serde(deserialize_with = "deserialize_hashset")]`
/// attribute to customize deserialization of HashSet fields.
///
/// # Arguments
///
/// * `deserializer` - Serde deserializer instance
///
/// # Returns
///
/// * `Ok(HashSet<T>)` - Successfully deserialized HashSet
/// * `Err(D::Error)` - Serde deserialization error
///
/// # Type Parameters
///
/// * `D` - Serde deserializer type
/// * `T` - Target type that implements `FromStr`, `Eq`, and `Hash`
pub(crate) fn deserialize_hashset<'a, D, T>(deserializer: D) -> Result<HashSet<T>, D::Error>
where
    D: serde::Deserializer<'a>,
    T: Eq + std::str::FromStr + std::hash::Hash,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    deserialize_hashset_inner(<String>::deserialize(deserializer)?).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_deserialize_hashset_inner_success() {
        let result: Result<HashSet<i32>, String> = deserialize_hashset_inner("1,2,3");
        assert!(result.is_ok());

        if let Ok(hashset) = result {
            assert_eq!(hashset.len(), 3);
            assert!(hashset.contains(&1));
            assert!(hashset.contains(&2));
            assert!(hashset.contains(&3));
        }
    }

    #[test]
    fn test_deserialize_hashset_inner_with_whitespace() {
        let result: Result<HashSet<String>, String> = deserialize_hashset_inner(" a , b , c ");
        assert!(result.is_ok());

        if let Ok(hashset) = result {
            assert_eq!(hashset.len(), 3);
            assert!(hashset.contains("a"));
            assert!(hashset.contains("b"));
            assert!(hashset.contains("c"));
        }
    }

    #[test]
    fn test_deserialize_hashset_inner_empty_string() {
        let result: Result<HashSet<String>, String> = deserialize_hashset_inner("");
        assert!(result.is_ok());
        if let Ok(hashset) = result {
            assert_eq!(hashset.len(), 0);
        }
    }

    #[test]
    fn test_deserialize_hashset_inner_single_value() {
        let result: Result<HashSet<String>, String> = deserialize_hashset_inner("single");
        assert!(result.is_ok());

        if let Ok(hashset) = result {
            assert_eq!(hashset.len(), 1);
            assert!(hashset.contains("single"));
        }
    }

    #[test]
    fn test_deserialize_hashset_inner_invalid_int() {
        let result: Result<HashSet<i32>, String> = deserialize_hashset_inner("1,invalid,3");
        assert!(result.is_err());

        if let Err(error) = result {
            assert!(error.contains("Unable to deserialize `invalid` as `i32`"));
        }
    }

    #[test]
    fn test_deserialize_hashset_inner_duplicates() {
        let result: Result<HashSet<String>, String> = deserialize_hashset_inner("a,b,a,c,b");
        assert!(result.is_ok());

        if let Ok(hashset) = result {
            assert_eq!(hashset.len(), 3); // Duplicates should be removed
            assert!(hashset.contains("a"));
            assert!(hashset.contains("b"));
            assert!(hashset.contains("c"));
        }
    }
}
