//! Custom validations for some shared types.

use std::collections::HashSet;

use error_stack::report;
use globset::Glob;
use once_cell::sync::Lazy;
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
    static EMAIL_REGEX: Lazy<Option<Regex>> = Lazy::new(|| {
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
                    "Invalid glob pattern for configured allowed_domain [{:?}]! - {:?}",
                    allowed_domain, err
                );
                #[cfg(feature = "logs")]
                logger::error!(err_msg);
                err_msg
            })
            .unwrap_or(false)
    })
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
}
