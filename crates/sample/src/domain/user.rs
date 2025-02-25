use std::collections::HashSet;
use std::str::FromStr;
use std::ops::Deref;
use error_stack::ResultExt;

use once_cell::sync::Lazy;
use common_utils::pii;
use masking::{self, Secret, PeekInterface, ExposeInterface};


use crate::errors::UserResult;
use crate::errors::UserErrors;

static BLOCKED_EMAIL: Lazy<HashSet<String>> = Lazy::new(|| {
    let blocked_emails_content = include_str!("./blocker_emails.txt");
    let blocked_emails: HashSet<String> = blocked_emails_content
        .lines()
        .map(|s| s.trim().to_owned())
        .collect();
    blocked_emails
});

#[derive(Clone, Debug)]
pub struct UserEmail(pii::Email);

impl UserEmail {
    pub fn new(email: Secret<String, pii::EmailStrategy>) -> UserResult<Self> {
        use validator::ValidateEmail;

        let email_string = email.expose().to_lowercase();
        let email =
            pii::Email::from_str(&email_string).change_context(UserErrors::EmailParsingError)?;

        if email_string.validate_email() {
            let (_username, domain) = match email_string.as_str().split_once('@') {
                Some((u, d)) => (u, d),
                None => return Err(UserErrors::EmailParsingError.into()),
            };

            if BLOCKED_EMAIL.contains(domain) {
                return Err(UserErrors::InvalidEmailError.into());
            }
            Ok(Self(email))
        } else {
            Err(UserErrors::EmailParsingError.into())
        }
    }

    pub fn from_pii_email(email: pii::Email) -> UserResult<Self> {
        let email_string = email.expose().map(|inner| inner.to_lowercase());
        Self::new(email_string)
    }

    pub fn into_inner(self) -> pii::Email {
        self.0
    }

    pub fn get_inner(&self) -> &pii::Email {
        &self.0
    }

    pub fn get_secret(self) -> Secret<String, pii::EmailStrategy> {
        (*self.0).clone()
    }

    pub fn extract_domain(&self) -> UserResult<&str> {
        let (_username, domain) = self
            .peek()
            .split_once('@')
            .ok_or(UserErrors::InternalServerError)?;

        Ok(domain)
    }
}

impl TryFrom<pii::Email> for UserEmail {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: pii::Email) -> Result<Self, Self::Error> {
        Self::from_pii_email(value)
    }
}

impl Deref for UserEmail {
    type Target = Secret<String, pii::EmailStrategy>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}