//! Custom validations for some shared types.

#![deny(clippy::invalid_regex)]

use std::{collections::HashSet, sync::LazyLock};

use error_stack::report;
use globset::Glob;
use regex::Regex;
#[cfg(feature = "logs")]
use router_env::logger;

use crate::errors::{CustomResult, ValidationError};

/// Validates a given phone number using the [phonenumber] crate
///
/// It returns a [ValidationError::InvalidValue] in case it could not parse the phone number
pub fn validate_phone_number(phone_number: &str) -> Result<(), ValidationError> {
    let _ = phonenumber::parse(None, phone_number).map_err(|e| ValidationError::InvalidValue {
        message: format!("Could not parse phone number: {phone_number}, because: {e:?}"),
    })?;

    Ok(())
}

/// Performs a simple validation against a provided email address.
pub fn validate_email(email: &str) -> CustomResult<(), ValidationError> {
    #[deny(clippy::invalid_regex)]
    static EMAIL_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| {
        #[allow(unknown_lints)]
        #[allow(clippy::manual_ok_err)]
        match Regex::new(
            r"^(?i)[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)+$",
        ) {
            Ok(regex) => Some(regex),
            Err(_error) => {
                #[cfg(feature = "logs")]
                logger::error!(?_error);
                None
            }
        }
    });
    let email_regex = match EMAIL_REGEX.as_ref() {
        Some(regex) => Ok(regex),
        None => Err(report!(ValidationError::InvalidValue {
            message: "Invalid regex expression".into()
        })),
    }?;

    const EMAIL_MAX_LENGTH: usize = 319;
    if email.is_empty() || email.chars().count() > EMAIL_MAX_LENGTH {
        return Err(report!(ValidationError::InvalidValue {
            message: "Email address is either empty or exceeds maximum allowed length".into()
        }));
    }

    if !email_regex.is_match(email) {
        return Err(report!(ValidationError::InvalidValue {
            message: "Invalid email address format".into()
        }));
    }

    Ok(())
}

/// Checks whether a given domain matches against a list of valid domain glob patterns
pub fn validate_domain_against_allowed_domains(
    domain: &str,
    allowed_domains: HashSet<String>,
) -> bool {
    allowed_domains.iter().any(|allowed_domain| {
        Glob::new(allowed_domain)
            .map(|glob| glob.compile_matcher().is_match(domain))
            .map_err(|err| {
                let err_msg = format!(
                    "Invalid glob pattern for configured allowed_domain [{allowed_domain:?}]! - {err:?}",

                );
                #[cfg(feature = "logs")]
                logger::error!(err_msg);
                err_msg
            })
            .unwrap_or(false)
    })
}

/// checks whether the input string contains potential XSS or SQL injection attack vectors
pub fn contains_potential_xss_or_sqli(input: &str) -> bool {
    let decoded = urlencoding::decode(input).unwrap_or_else(|_| input.into());

    // Check for suspicious percent-encoded patterns
    static PERCENT_ENCODED: LazyLock<Option<Regex>> = LazyLock::new(|| {
        Regex::new(r"%[0-9A-Fa-f]{2}")
            .map_err(|_err| {
                #[cfg(feature = "logs")]
                logger::error!(?_err);
            })
            .ok()
    });

    if decoded.contains('%') {
        match PERCENT_ENCODED.as_ref() {
            Some(regex) => {
                if regex.is_match(&decoded) {
                    return true;
                }
            }
            None => return true,
        }
    }

    if ammonia::is_html(&decoded) {
        return true;
    }

    static XSS: LazyLock<Option<Regex>> = LazyLock::new(|| {
        Regex::new(
            r"(?is)\bon[a-z]+\s*=|\bjavascript\s*:|\bdata\s*:\s*text/html|\b(alert|prompt|confirm|eval)\s*\(",
        )
        .map_err(|_err| {
            #[cfg(feature = "logs")]
             logger::error!(?_err);
        })
        .ok()
    });

    static SQLI: LazyLock<Option<Regex>> = LazyLock::new(|| {
        Regex::new(
            r"(?is)(?:')\s*or\s*'?\d+'?=?\d*|union\s+select|insert\s+into|drop\s+table|information_schema|sleep\s*\(|--|;",
        )
        .map_err(|_err| {
            #[cfg(feature = "logs")]
             logger::error!(?_err);
        })
        .ok()
    });

    match XSS.as_ref() {
        Some(regex) => {
            if regex.is_match(&decoded) {
                return true;
            }
        }
        None => return true,
    }

    match SQLI.as_ref() {
        Some(regex) => {
            if regex.is_match(&decoded) {
                return true;
            }
        }
        None => return true,
    }

    false
}

#[cfg(test)]
mod tests {
    use fake::{faker::internet::en::SafeEmail, Fake};
    use proptest::{
        prop_assert,
        strategy::{Just, NewTree, Strategy},
        test_runner::TestRunner,
    };
    use test_case::test_case;

    use super::*;

    #[derive(Debug)]
    struct ValidEmail;

    impl Strategy for ValidEmail {
        type Tree = Just<String>;
        type Value = String;

        fn new_tree(&self, _runner: &mut TestRunner) -> NewTree<Self> {
            Ok(Just(SafeEmail().fake()))
        }
    }

    #[test]
    fn test_validate_email() {
        let result = validate_email("abc@example.com");
        assert!(result.is_ok());

        let result = validate_email("abc+123@example.com");
        assert!(result.is_ok());

        let result = validate_email("");
        assert!(result.is_err());
    }

    #[test_case("+40745323456" ; "Romanian valid phone number")]
    #[test_case("+34912345678" ; "Spanish valid phone number")]
    #[test_case("+41 79 123 45 67" ; "Swiss valid phone number")]
    #[test_case("+66 81 234 5678" ; "Thailand valid phone number")]
    fn test_validate_phone_number(phone_number: &str) {
        assert!(validate_phone_number(phone_number).is_ok());
    }

    #[test_case("9123456789" ; "Romanian invalid phone number")]
    fn test_invalid_phone_number(phone_number: &str) {
        let res = validate_phone_number(phone_number);
        assert!(res.is_err());
    }

    proptest::proptest! {
        /// Example of unit test
        #[test]
        fn proptest_valid_fake_email(email in ValidEmail) {
            prop_assert!(validate_email(&email).is_ok());
        }

        /// Example of unit test
        #[test]
        fn proptest_invalid_data_email(email in "\\PC*") {
            prop_assert!(validate_email(&email).is_err());
        }

        #[test]
        fn proptest_invalid_email(email in "[.+]@(.+)") {
            prop_assert!(validate_email(&email).is_err());
        }
    }

    #[test]
    fn detects_basic_script_tags() {
        assert!(contains_potential_xss_or_sqli(
            "<script>alert('xss')</script>"
        ));
    }

    #[test]
    fn detects_event_handlers() {
        assert!(contains_potential_xss_or_sqli(
            "onload=alert('xss') onclick=alert('xss') onmouseover=alert('xss')",
        ));
    }

    #[test]
    fn detects_data_url_payload() {
        assert!(contains_potential_xss_or_sqli(
            "data:text/html,<script>alert('xss')</script>",
        ));
    }

    #[test]
    fn detects_iframe_javascript_src() {
        assert!(contains_potential_xss_or_sqli(
            "<iframe src=javascript:alert('xss')></iframe>",
        ));
    }

    #[test]
    fn detects_svg_with_script() {
        assert!(contains_potential_xss_or_sqli(
            "<svg><script>alert('xss')</script></svg>",
        ));
    }

    #[test]
    fn detects_object_with_js() {
        assert!(contains_potential_xss_or_sqli(
            "<object data=javascript:alert('xss')></object>",
        ));
    }

    #[test]
    fn detects_mixed_case_tags() {
        assert!(contains_potential_xss_or_sqli(
            "<ScRiPt>alert('xss')</ScRiPt>"
        ));
    }

    #[test]
    fn detects_embedded_script_in_text() {
        assert!(contains_potential_xss_or_sqli(
            "Test<script>alert('xss')</script>Company",
        ));
    }

    #[test]
    fn detects_math_with_script() {
        assert!(contains_potential_xss_or_sqli(
            "<math><script>alert('xss')</script></math>",
        ));
    }

    #[test]
    fn detects_basic_sql_tautology() {
        assert!(contains_potential_xss_or_sqli("' OR '1'='1"));
    }

    #[test]
    fn detects_time_based_sqli() {
        assert!(contains_potential_xss_or_sqli("' OR SLEEP(5) --"));
    }

    #[test]
    fn detects_percent_encoded_sqli() {
        // %27 OR %271%27=%271 is a typical encoded variant
        assert!(contains_potential_xss_or_sqli("%27%20OR%20%271%27%3D%271"));
    }

    #[test]
    fn detects_benign_html_as_suspicious() {
        assert!(contains_potential_xss_or_sqli("<b>Hello</b>"));
    }

    #[test]
    fn allows_legitimate_plain_text() {
        assert!(!contains_potential_xss_or_sqli("My Test Company Ltd."));
    }

    #[test]
    fn allows_normal_url() {
        assert!(!contains_potential_xss_or_sqli("https://example.com"));
    }

    #[test]
    fn allows_percent_char_without_encoding() {
        assert!(!contains_potential_xss_or_sqli("Get 50% off today"));
    }
}
