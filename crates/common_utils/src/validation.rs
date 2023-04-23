//! Custom validations for some shared types.

use error_stack::report;
use once_cell::sync::Lazy;
use regex::Regex;
#[cfg(feature = "logs")]
use router_env::logger;

use crate::errors::{CustomResult, ValidationError};

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

#[cfg(test)]
mod tests {
    use fake::{faker::internet::en::SafeEmail, Fake};
    use proptest::{
        prop_assert,
        strategy::{Just, NewTree, Strategy},
        test_runner::TestRunner,
    };

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
