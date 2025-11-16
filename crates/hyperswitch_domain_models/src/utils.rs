//! Utility functions for hyperswitch domain models

use core::str::FromStr;

use common_utils::types::HasInvalidVariant;

/// Parses a string into an enum type that implements `impl_enum_str!` macro.
/// Logs a warning if the parsing results in an `Invalid` variant.
///
/// This function is designed to work with enums created using the `impl_enum_str!` macro,
/// which automatically generates an `Invalid` variant and implements `HasInvalidVariant` trait.
///
/// # Type Parameters
/// * `T` - The enum type to parse into. Must implement:
///   - `FromStr` with `Err = Infallible` (guaranteed by `impl_enum_str!`)
///   - `HasInvalidVariant` trait (automatically implemented by `impl_enum_str!`)
///
/// # Arguments
/// * `raw_value` - The string value to parse
///
/// # Returns
/// The parsed enum value (including `Invalid` variant if parsing fails)
///
/// # Panics
/// This function never panics. The `unwrap()` call is safe because enums created with
/// `impl_enum_str!` macro have `FromStr::Err = Infallible`, meaning parsing cannot fail.
/// Invalid input strings are converted to the `Invalid` variant instead of returning an error.
///
/// # Example
/// ```ignore
/// use common_utils::types::CreatedBy;
/// use hyperswitch_domain_models::utils::parse_enum_with_logging;
///
/// let created_by = storage_model.created_by.map(|s| parse_enum_with_logging::<CreatedBy>(&s));
/// ```
pub fn parse_enum_with_logging<T>(raw_value: &str) -> T
where
    T: FromStr<Err = core::convert::Infallible> + HasInvalidVariant,
{
    // Safe to unwrap: FromStr::Err = Infallible means parse cannot fail
    let parsed = raw_value.parse::<T>().unwrap();

    if parsed.is_invalid() {
        router_env::tracing::warn!(
            raw_value = %raw_value,
            type_name = core::any::type_name::<T>(),
            "Invalid enum value encountered while parsing"
        );
    }

    parsed
}
