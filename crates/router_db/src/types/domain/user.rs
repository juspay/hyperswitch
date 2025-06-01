use std::{
    collections::HashSet,
    ops::{Deref, Not},
    str::FromStr,
    sync::LazyLock,
};

use common_utils::pii;
use masking::Secret;

// use api_models::{
//     admin as admin_api, organization as api_org, user as user_api, user_role as user_role_api,
// };
// use common_enums::EntityType;
// use common_utils::{
//     crypto::Encryptable, id_type, new_type::MerchantName, pii, type_name,
//     types::keymanager::Identifier,
// };
// use diesel_models::{
//     enums::{TotpStatus, UserRoleVersion, UserStatus},
//     organization::{self as diesel_org, Organization, OrganizationBridge},
//     user as storage_user,
//     user_role::{UserRole, UserRoleNew},
// };
// use error_stack::{report, ResultExt};
// use hyperswitch_domain_models::api::ApplicationResponse;
// use masking::{ExposeInterface, PeekInterface, Secret};
// use rand::distributions::{Alphanumeric, DistString};
// use time::PrimitiveDateTime;
// use unicode_segmentation::UnicodeSegmentation;
// #[cfg(feature = "keymanager_create")]
// use {base64::Engine, common_utils::types::keymanager::EncryptionTransferRequest};

// use crate::{
//     consts,
//     core::{
//         admin,
//         errors::{UserErrors, UserResult},
//     },
//     db::GlobalStorageInterface,
//     routes::SessionState,
//     services::{
//         self,
//         authentication::{AuthenticationDataWithOrg, UserFromToken},
//     },
//     types::{domain, transformers::ForeignFrom},
//     utils::{self, user::password},
// };

// pub mod dashboard_metadata;
// pub mod decision_manager;
// pub use decision_manager::*;
// pub mod user_authentication_method;

// use super::{types as domain_types, UserKeyStore};

#[derive(Clone, Debug)]
pub struct UserEmail(pii::Email);

static BLOCKED_EMAIL: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let blocked_emails_content = include_str!("../../utils/user/blocker_emails.txt");
    let blocked_emails: HashSet<String> = blocked_emails_content
        .lines()
        .map(|s| s.trim().to_owned())
        .collect();
    blocked_emails
});

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
