//! Utilities for converting between foreign types

use common_enums::ApiClientError;
use error_stack::ResultExt;

/// Trait for converting from one foreign type to another
pub trait ForeignFrom<F> {
    /// Convert from a foreign type to the current type
    fn foreign_from(from: F) -> Self;
}

/// Trait for converting from one foreign type to another
pub trait ForeignTryFrom<F>: Sized {
    /// Custom error for conversion failure
    type Error;
    /// Convert from a foreign type to the current type and return an error if the conversion fails
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

// Reqwest uses http crate 1.x, while the server and connector response models use http 0.2.
// TODO: Remove this once actix supports http crte 1.x
impl ForeignTryFrom<&reqwest::header::HeaderMap> for http::HeaderMap {
    type Error = error_stack::Report<ApiClientError>;

    fn foreign_try_from(headers: &reqwest::header::HeaderMap) -> Result<Self, Self::Error> {
        headers
            .iter()
            .map(|(name, value)| {
                let name = http::header::HeaderName::try_from(name.as_str())
                    .change_context(ApiClientError::HeaderMapConstructionFailed)?;
                let mut header_value = http::HeaderValue::try_from(value.as_bytes())
                    .change_context(ApiClientError::HeaderMapConstructionFailed)?;
                header_value.set_sensitive(value.is_sensitive());
                Ok((name, header_value))
            })
            .collect()
    }
}
