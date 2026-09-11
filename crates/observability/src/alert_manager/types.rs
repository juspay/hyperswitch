pub mod config;
pub mod dictionary;
pub mod notifications;

use actix_web::http::header::HeaderMap;
use error_stack::report;
use serde::Serialize;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

pub const X_USER_NAME: &str = "X-User-Name";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserName(String);

impl UserName {
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

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_option(&self) -> Option<String> {
        Some(self.0.clone()).filter(|name| !name.is_empty())
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadStatus {
    Found,
    Absent,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WriteStatus {
    Saved,
    Retired,
    Absent,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_user_header_is_the_empty_name() {
        let user = UserName::from_headers(&HeaderMap::new()).unwrap();

        assert_eq!(user.as_str(), "");
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
