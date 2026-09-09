pub mod config;
pub mod dictionary;
pub mod notifications;

use actix_web::http::header::HeaderMap;
use error_stack::report;
use serde::Serialize;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

/// The header naming the person a request is made on behalf of.
pub const X_USER_NAME: &str = "X-User-Name";

/// Who a request is made on behalf of.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserName(String);

impl UserName {
    /// Read the caller's asserted identity from a request's headers.
    pub fn from_headers(headers: &HeaderMap) -> ObservabilityApiResult<Self> {
        headers.get(X_USER_NAME).map_or_else(
            || Ok(Self::default()),
            |value| {
                std::str::from_utf8(value.as_bytes())
                    .map(|name| Self(name.trim().to_owned()))
                    .map_err(|_| {
                        report!(ObservabilityError::InvalidRequest)
                            .attach_printable("The user name header is not valid UTF-8")
                    })
            },
        )
    }

    /// The asserted name, empty when the caller named nobody.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The asserted name, or `None` when the caller named nobody.
    pub fn to_option(&self) -> Option<String> {
        Some(self.0.clone()).filter(|name| !name.is_empty())
    }
}

/// Whether a read found anything.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadStatus {
    /// The store answered, and had something.
    Found,
    /// The store answered, and had nothing.
    Absent,
}

/// What a write left behind.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WriteStatus {
    /// The live row now holds what the request sent.
    Saved,
    /// The live row was retired.
    Retired,
    /// There was nothing to retire.
    Absent,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Every caller sends this today, because authentication is disabled in both environments.
    #[test]
    fn an_absent_user_header_is_the_empty_name() {
        let user = UserName::from_headers(&HeaderMap::new()).unwrap();

        assert_eq!(user.as_str(), "");
        // `None`, not `Some("")`, so the column's default names the team rather than nobody.
        assert!(user.to_option().is_none());
    }

    #[test]
    fn a_user_header_names_the_person_the_write_is_attributed_to() {
        let mut headers = HeaderMap::new();
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-user-name"),
            actix_web::http::header::HeaderValue::from_static("  ops@example.com  "),
        );

        let user = UserName::from_headers(&headers).unwrap();

        assert_eq!(user.as_str(), "ops@example.com");
    }

    /// Falling back to the empty name would file one person's watermark under the row every unauthenticated caller shares.
    #[test]
    fn a_user_header_that_is_not_utf8_is_rejected_rather_than_ignored() {
        let mut headers = HeaderMap::new();
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-user-name"),
            actix_web::http::header::HeaderValue::from_bytes(&[0xff, 0xfe]).unwrap(),
        );

        assert!(UserName::from_headers(&headers).is_err());
    }
}
