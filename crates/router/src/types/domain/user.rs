use std::{
    collections::HashSet,
    ops::{self, Not},
    str::FromStr,
};

use api_models::{
    admin as admin_api, organization as api_org, user as user_api, user_role as user_role_api,
};
use common_enums::TokenPurpose;
use common_utils::{crypto::Encryptable, errors::CustomResult, pii};
use diesel_models::{
    enums::{TotpStatus, UserStatus},
    organization as diesel_org,
    organization::Organization,
    user as storage_user,
    user_role::{UserRole, UserRoleNew},
};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use once_cell::sync::Lazy;
use rand::distributions::{Alphanumeric, DistString};
use router_env::env;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    consts,
    core::{
        admin,
        errors::{self, UserErrors, UserResult},
    },
    db::StorageInterface,
    routes::AppState,
    services::{self, authentication as auth, authentication::UserFromToken, authorization::info},
    types::transformers::ForeignFrom,
    utils::{self, user::password},
};

pub mod dashboard_metadata;
pub mod decision_manager;
pub use decision_manager::*;

use super::{types as domain_types, UserKeyStore};

#[derive(Clone)]
pub struct UserName(Secret<String>);

impl UserName {
    pub fn new(name: Secret<String>) -> UserResult<Self> {
        let name = name.expose();
        let is_empty_or_whitespace = name.trim().is_empty();
        let is_too_long = name.graphemes(true).count() > consts::user::MAX_NAME_LENGTH;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = name.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(UserErrors::NameParsingError.into())
        } else {
            Ok(Self(name.into()))
        }
    }

    pub fn get_secret(self) -> Secret<String> {
        self.0
    }
}

impl TryFrom<pii::Email> for UserName {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: pii::Email) -> UserResult<Self> {
        Self::new(Secret::new(
            value
                .peek()
                .split_once('@')
                .ok_or(UserErrors::InvalidEmailError)?
                .0
                .to_string(),
        ))
    }
}

#[derive(Clone, Debug)]
pub struct UserEmail(pii::Email);

static BLOCKED_EMAIL: Lazy<HashSet<String>> = Lazy::new(|| {
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

        let email_string = email.expose();
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
        use validator::ValidateEmail;

        let email_string = email.peek();
        if email_string.validate_email() {
            let (_username, domain) = match email_string.split_once('@') {
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

    pub fn into_inner(self) -> pii::Email {
        self.0
    }

    pub fn get_secret(self) -> Secret<String, pii::EmailStrategy> {
        (*self.0).clone()
    }
}

impl TryFrom<pii::Email> for UserEmail {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: pii::Email) -> Result<Self, Self::Error> {
        Self::from_pii_email(value)
    }
}

impl ops::Deref for UserEmail {
    type Target = Secret<String, pii::EmailStrategy>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct UserPassword(Secret<String>);

impl UserPassword {
    pub fn new(password: Secret<String>) -> UserResult<Self> {
        let password = password.expose();

        let mut has_upper_case = false;
        let mut has_lower_case = false;
        let mut has_numeric_value = false;
        let mut has_special_character = false;
        let mut has_whitespace = false;

        for c in password.chars() {
            has_upper_case = has_upper_case || c.is_uppercase();
            has_lower_case = has_lower_case || c.is_lowercase();
            has_numeric_value = has_numeric_value || c.is_numeric();
            has_special_character = has_special_character || !c.is_alphanumeric();
            has_whitespace = has_whitespace || c.is_whitespace();
        }

        let is_password_format_valid = has_upper_case
            && has_lower_case
            && has_numeric_value
            && has_special_character
            && !has_whitespace;

        let is_too_long = password.graphemes(true).count() > consts::user::MAX_PASSWORD_LENGTH;
        let is_too_short = password.graphemes(true).count() < consts::user::MIN_PASSWORD_LENGTH;

        if is_too_short || is_too_long || !is_password_format_valid {
            return Err(UserErrors::PasswordParsingError.into());
        }
        Ok(Self(password.into()))
    }

    pub fn new_password_without_validation(password: Secret<String>) -> UserResult<Self> {
        let password = password.expose();
        if password.is_empty() {
            return Err(UserErrors::PasswordParsingError.into());
        }
        Ok(Self(password.into()))
    }

    pub fn get_secret(&self) -> Secret<String> {
        self.0.clone()
    }
}

#[derive(Clone)]
pub struct UserCompanyName(String);

impl UserCompanyName {
    pub fn new(company_name: String) -> UserResult<Self> {
        let company_name = company_name.trim();
        let is_empty_or_whitespace = company_name.is_empty();
        let is_too_long =
            company_name.graphemes(true).count() > consts::user::MAX_COMPANY_NAME_LENGTH;

        let is_all_valid_characters = company_name
            .chars()
            .all(|x| x.is_alphanumeric() || x.is_ascii_whitespace() || x == '_');
        if is_empty_or_whitespace || is_too_long || !is_all_valid_characters {
            Err(UserErrors::CompanyNameParsingError.into())
        } else {
            Ok(Self(company_name.to_string()))
        }
    }

    pub fn get_secret(self) -> String {
        self.0
    }
}

#[derive(Clone)]
pub struct NewUserOrganization(diesel_org::OrganizationNew);

impl NewUserOrganization {
    pub async fn insert_org_in_db(self, state: AppState) -> UserResult<Organization> {
        state
            .store
            .insert_organization(self.0)
            .await
            .map_err(|e| {
                if e.current_context().is_db_unique_violation() {
                    e.change_context(UserErrors::DuplicateOrganizationId)
                } else {
                    e.change_context(UserErrors::InternalServerError)
                }
            })
            .attach_printable("Error while inserting organization")
    }

    pub fn get_organization_id(&self) -> String {
        self.0.org_id.clone()
    }
}

impl TryFrom<user_api::SignUpWithMerchantIdRequest> for NewUserOrganization {
    type Error = error_stack::Report<UserErrors>;
    fn try_from(value: user_api::SignUpWithMerchantIdRequest) -> UserResult<Self> {
        let new_organization = api_org::OrganizationNew::new(Some(
            UserCompanyName::new(value.company_name)?.get_secret(),
        ));
        let db_organization = ForeignFrom::foreign_from(new_organization);
        Ok(Self(db_organization))
    }
}

impl From<user_api::SignUpRequest> for NewUserOrganization {
    fn from(_value: user_api::SignUpRequest) -> Self {
        let new_organization = api_org::OrganizationNew::new(None);
        let db_organization = ForeignFrom::foreign_from(new_organization);
        Self(db_organization)
    }
}

impl From<user_api::ConnectAccountRequest> for NewUserOrganization {
    fn from(_value: user_api::ConnectAccountRequest) -> Self {
        let new_organization = api_org::OrganizationNew::new(None);
        let db_organization = ForeignFrom::foreign_from(new_organization);
        Self(db_organization)
    }
}

impl From<user_api::CreateInternalUserRequest> for NewUserOrganization {
    fn from(_value: user_api::CreateInternalUserRequest) -> Self {
        let new_organization = api_org::OrganizationNew::new(None);
        let db_organization = ForeignFrom::foreign_from(new_organization);
        Self(db_organization)
    }
}

impl From<UserMerchantCreateRequestWithToken> for NewUserOrganization {
    fn from(value: UserMerchantCreateRequestWithToken) -> Self {
        Self(diesel_org::OrganizationNew {
            org_id: value.2.org_id,
            org_name: Some(value.1.company_name),
        })
    }
}

type InviteeUserRequestWithInvitedUserToken = (user_api::InviteUserRequest, UserFromToken);
impl From<InviteeUserRequestWithInvitedUserToken> for NewUserOrganization {
    fn from(_value: InviteeUserRequestWithInvitedUserToken) -> Self {
        let new_organization = api_org::OrganizationNew::new(None);
        let db_organization = ForeignFrom::foreign_from(new_organization);
        Self(db_organization)
    }
}

#[derive(Clone)]
pub struct MerchantId(String);

impl MerchantId {
    pub fn new(merchant_id: String) -> UserResult<Self> {
        let merchant_id = merchant_id.trim().to_lowercase().replace(' ', "_");
        let is_empty_or_whitespace = merchant_id.is_empty();

        let is_all_valid_characters = merchant_id.chars().all(|x| x.is_alphanumeric() || x == '_');
        if is_empty_or_whitespace || !is_all_valid_characters {
            Err(UserErrors::MerchantIdParsingError.into())
        } else {
            Ok(Self(merchant_id.to_string()))
        }
    }

    pub fn get_secret(&self) -> String {
        self.0.clone()
    }
}

#[derive(Clone)]
pub struct NewUserMerchant {
    merchant_id: MerchantId,
    company_name: Option<UserCompanyName>,
    new_organization: NewUserOrganization,
}

impl NewUserMerchant {
    pub fn get_company_name(&self) -> Option<String> {
        self.company_name.clone().map(UserCompanyName::get_secret)
    }

    pub fn get_merchant_id(&self) -> String {
        self.merchant_id.get_secret()
    }

    pub fn get_new_organization(&self) -> NewUserOrganization {
        self.new_organization.clone()
    }

    pub async fn check_if_already_exists_in_db(&self, state: AppState) -> UserResult<()> {
        if state
            .store
            .get_merchant_key_store_by_merchant_id(
                self.get_merchant_id().as_str(),
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .is_ok()
        {
            return Err(UserErrors::MerchantAccountCreationError(format!(
                "Merchant with {} already exists",
                self.get_merchant_id()
            ))
            .into());
        }
        Ok(())
    }

    pub async fn create_new_merchant_and_insert_in_db(&self, state: AppState) -> UserResult<()> {
        self.check_if_already_exists_in_db(state.clone()).await?;
        Box::pin(admin::create_merchant_account(
            state.clone(),
            admin_api::MerchantAccountCreate {
                merchant_id: self.get_merchant_id(),
                metadata: None,
                locker_id: None,
                return_url: None,
                merchant_name: self.get_company_name().map(Secret::new),
                webhook_details: None,
                publishable_key: None,
                organization_id: Some(self.new_organization.get_organization_id()),
                merchant_details: None,
                routing_algorithm: None,
                parent_merchant_id: None,
                sub_merchants_enabled: None,
                frm_routing_algorithm: None,
                #[cfg(feature = "payouts")]
                payout_routing_algorithm: None,
                primary_business_details: None,
                payment_response_hash_key: None,
                enable_payment_response_hash: None,
                redirect_to_merchant_with_http_post: None,
            },
        ))
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error while creating a merchant")?;
        Ok(())
    }
}

impl TryFrom<user_api::SignUpRequest> for NewUserMerchant {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::SignUpRequest) -> UserResult<Self> {
        let merchant_id = MerchantId::new(format!(
            "merchant_{}",
            common_utils::date_time::now_unix_timestamp()
        ))?;
        let new_organization = NewUserOrganization::from(value);

        Ok(Self {
            company_name: None,
            merchant_id,
            new_organization,
        })
    }
}

impl TryFrom<user_api::ConnectAccountRequest> for NewUserMerchant {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::ConnectAccountRequest) -> UserResult<Self> {
        let merchant_id = MerchantId::new(format!(
            "merchant_{}",
            common_utils::date_time::now_unix_timestamp()
        ))?;
        let new_organization = NewUserOrganization::from(value);

        Ok(Self {
            company_name: None,
            merchant_id,
            new_organization,
        })
    }
}

impl TryFrom<user_api::SignUpWithMerchantIdRequest> for NewUserMerchant {
    type Error = error_stack::Report<UserErrors>;
    fn try_from(value: user_api::SignUpWithMerchantIdRequest) -> UserResult<Self> {
        let company_name = Some(UserCompanyName::new(value.company_name.clone())?);
        let merchant_id = MerchantId::new(value.company_name.clone())?;
        let new_organization = NewUserOrganization::try_from(value)?;

        Ok(Self {
            company_name,
            merchant_id,
            new_organization,
        })
    }
}

impl TryFrom<user_api::CreateInternalUserRequest> for NewUserMerchant {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::CreateInternalUserRequest) -> UserResult<Self> {
        let merchant_id =
            MerchantId::new(consts::user_role::INTERNAL_USER_MERCHANT_ID.to_string())?;
        let new_organization = NewUserOrganization::from(value);

        Ok(Self {
            company_name: None,
            merchant_id,
            new_organization,
        })
    }
}

impl TryFrom<InviteeUserRequestWithInvitedUserToken> for NewUserMerchant {
    type Error = error_stack::Report<UserErrors>;
    fn try_from(value: InviteeUserRequestWithInvitedUserToken) -> UserResult<Self> {
        let merchant_id = MerchantId::new(value.clone().1.merchant_id)?;
        let new_organization = NewUserOrganization::from(value);
        Ok(Self {
            company_name: None,
            merchant_id,
            new_organization,
        })
    }
}

type UserMerchantCreateRequestWithToken =
    (UserFromStorage, user_api::UserMerchantCreate, UserFromToken);

impl TryFrom<UserMerchantCreateRequestWithToken> for NewUserMerchant {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: UserMerchantCreateRequestWithToken) -> UserResult<Self> {
        let merchant_id = if matches!(env::which(), env::Env::Production) {
            MerchantId::new(value.1.company_name.clone())?
        } else {
            MerchantId::new(format!(
                "merchant_{}",
                common_utils::date_time::now_unix_timestamp()
            ))?
        };
        Ok(Self {
            merchant_id,
            company_name: Some(UserCompanyName::new(value.1.company_name.clone())?),
            new_organization: NewUserOrganization::from(value),
        })
    }
}

#[derive(Clone)]
pub struct NewUser {
    user_id: String,
    name: UserName,
    email: UserEmail,
    password: UserPassword,
    new_merchant: NewUserMerchant,
    is_temporary_password: bool,
}

impl NewUser {
    pub fn get_user_id(&self) -> String {
        self.user_id.clone()
    }

    pub fn get_email(&self) -> UserEmail {
        self.email.clone()
    }

    pub fn get_name(&self) -> Secret<String> {
        self.name.clone().get_secret()
    }

    pub fn get_new_merchant(&self) -> NewUserMerchant {
        self.new_merchant.clone()
    }

    pub fn get_password(&self) -> UserPassword {
        self.password.clone()
    }

    pub async fn insert_user_in_db(
        &self,
        db: &dyn StorageInterface,
    ) -> UserResult<UserFromStorage> {
        match db.insert_user(self.clone().try_into()?).await {
            Ok(user) => Ok(user.into()),
            Err(e) => {
                if e.current_context().is_db_unique_violation() {
                    Err(e.change_context(UserErrors::UserExists))
                } else {
                    Err(e.change_context(UserErrors::InternalServerError))
                }
            }
        }
        .attach_printable("Error while inserting user")
    }

    pub async fn check_if_already_exists_in_db(&self, state: AppState) -> UserResult<()> {
        if state
            .store
            .find_user_by_email(&self.get_email().into_inner())
            .await
            .is_ok()
        {
            return Err(report!(UserErrors::UserExists));
        }
        Ok(())
    }

    pub async fn insert_user_and_merchant_in_db(
        &self,
        state: AppState,
    ) -> UserResult<UserFromStorage> {
        self.check_if_already_exists_in_db(state.clone()).await?;
        let db = state.store.as_ref();
        let merchant_id = self.get_new_merchant().get_merchant_id();
        self.new_merchant
            .create_new_merchant_and_insert_in_db(state.clone())
            .await?;
        let created_user = self.insert_user_in_db(db).await;
        if created_user.is_err() {
            let _ = admin::merchant_account_delete(state, merchant_id).await;
        };
        created_user
    }

    pub async fn insert_user_role_in_db(
        self,
        state: AppState,
        role_id: String,
        user_status: UserStatus,
    ) -> UserResult<UserRole> {
        let now = common_utils::date_time::now();
        let user_id = self.get_user_id();

        state
            .store
            .insert_user_role(UserRoleNew {
                merchant_id: self.get_new_merchant().get_merchant_id(),
                status: user_status,
                created_by: user_id.clone(),
                last_modified_by: user_id.clone(),
                user_id,
                role_id,
                created_at: now,
                last_modified: now,
                org_id: self
                    .get_new_merchant()
                    .get_new_organization()
                    .get_organization_id(),
            })
            .await
            .change_context(UserErrors::InternalServerError)
    }
}

impl TryFrom<NewUser> for storage_user::UserNew {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: NewUser) -> UserResult<Self> {
        let hashed_password = password::generate_password_hash(value.password.get_secret())?;
        let now = common_utils::date_time::now();
        Ok(Self {
            user_id: value.get_user_id(),
            name: value.get_name(),
            email: value.get_email().into_inner(),
            password: hashed_password,
            is_verified: false,
            created_at: Some(now),
            last_modified_at: Some(now),
            preferred_merchant_id: None,
            totp_status: TotpStatus::NotSet,
            totp_secret: None,
            totp_recovery_codes: None,
            last_password_modified_at: value.is_temporary_password.not().then_some(now),
        })
    }
}

impl TryFrom<user_api::SignUpWithMerchantIdRequest> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::SignUpWithMerchantIdRequest) -> UserResult<Self> {
        let email = value.email.clone().try_into()?;
        let name = UserName::new(value.name.clone())?;
        let password = UserPassword::new(value.password.clone())?;
        let user_id = uuid::Uuid::new_v4().to_string();
        let new_merchant = NewUserMerchant::try_from(value)?;

        Ok(Self {
            name,
            email,
            password,
            user_id,
            new_merchant,
            is_temporary_password: false,
        })
    }
}

impl TryFrom<user_api::SignUpRequest> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::SignUpRequest) -> UserResult<Self> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let email = value.email.clone().try_into()?;
        let name = UserName::try_from(value.email.clone())?;
        let password = UserPassword::new(value.password.clone())?;
        let new_merchant = NewUserMerchant::try_from(value)?;

        Ok(Self {
            user_id,
            name,
            email,
            password,
            new_merchant,
            is_temporary_password: false,
        })
    }
}

impl TryFrom<user_api::ConnectAccountRequest> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::ConnectAccountRequest) -> UserResult<Self> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let email = value.email.clone().try_into()?;
        let name = UserName::try_from(value.email.clone())?;
        let password = UserPassword::new(password::get_temp_password())?;
        let new_merchant = NewUserMerchant::try_from(value)?;

        Ok(Self {
            user_id,
            name,
            email,
            password,
            new_merchant,
            is_temporary_password: true,
        })
    }
}

impl TryFrom<user_api::CreateInternalUserRequest> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::CreateInternalUserRequest) -> UserResult<Self> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let email = value.email.clone().try_into()?;
        let name = UserName::new(value.name.clone())?;
        let password = UserPassword::new(value.password.clone())?;
        let new_merchant = NewUserMerchant::try_from(value)?;

        Ok(Self {
            user_id,
            name,
            email,
            password,
            new_merchant,
            is_temporary_password: false,
        })
    }
}

impl TryFrom<UserMerchantCreateRequestWithToken> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: UserMerchantCreateRequestWithToken) -> Result<Self, Self::Error> {
        let user = value.0.clone();
        let new_merchant = NewUserMerchant::try_from(value)?;

        Ok(Self {
            user_id: user.0.user_id,
            name: UserName::new(user.0.name)?,
            email: user.0.email.clone().try_into()?,
            password: UserPassword::new_password_without_validation(user.0.password)?,
            new_merchant,
            // This is true because we are not creating a user with this request. And if it is set
            // to false, last_password_modified_at will be overwritten if this user is inserted.
            is_temporary_password: true,
        })
    }
}

impl TryFrom<InviteeUserRequestWithInvitedUserToken> for NewUser {
    type Error = error_stack::Report<UserErrors>;
    fn try_from(value: InviteeUserRequestWithInvitedUserToken) -> UserResult<Self> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let email = value.0.email.clone().try_into()?;
        let name = UserName::new(value.0.name.clone())?;
        let password = UserPassword::new(password::get_temp_password())?;
        let new_merchant = NewUserMerchant::try_from(value)?;

        Ok(Self {
            user_id,
            name,
            email,
            password,
            new_merchant,
            is_temporary_password: true,
        })
    }
}

#[derive(Clone)]
pub struct UserFromStorage(pub storage_user::User);

impl From<storage_user::User> for UserFromStorage {
    fn from(value: storage_user::User) -> Self {
        Self(value)
    }
}

impl UserFromStorage {
    pub fn get_user_id(&self) -> &str {
        self.0.user_id.as_str()
    }

    pub fn compare_password(&self, candidate: &Secret<String>) -> UserResult<()> {
        match password::is_correct_password(candidate, &self.0.password) {
            Ok(true) => Ok(()),
            Ok(false) => Err(UserErrors::InvalidCredentials.into()),
            Err(e) => Err(e),
        }
    }

    pub fn get_name(&self) -> Secret<String> {
        self.0.name.clone()
    }

    pub fn get_email(&self) -> pii::Email {
        self.0.email.clone()
    }

    pub async fn get_role_from_db(&self, state: AppState) -> UserResult<UserRole> {
        state
            .store
            .find_user_role_by_user_id(&self.0.user_id)
            .await
            .change_context(UserErrors::InternalServerError)
    }

    pub async fn get_roles_from_db(&self, state: &AppState) -> UserResult<Vec<UserRole>> {
        state
            .store
            .list_user_roles_by_user_id(&self.0.user_id)
            .await
            .change_context(UserErrors::InternalServerError)
    }

    #[cfg(feature = "email")]
    pub fn get_verification_days_left(&self, state: &AppState) -> UserResult<Option<i64>> {
        if self.0.is_verified {
            return Ok(None);
        }

        let allowed_unverified_duration =
            time::Duration::days(state.conf.email.allowed_unverified_days);

        let user_created = self.0.created_at.date();
        let last_date_for_verification = user_created
            .checked_add(allowed_unverified_duration)
            .ok_or(UserErrors::InternalServerError)?;

        let today = common_utils::date_time::now().date();
        if today >= last_date_for_verification {
            return Err(UserErrors::UnverifiedUser.into());
        }

        let days_left_for_verification = last_date_for_verification - today;
        Ok(Some(days_left_for_verification.whole_days()))
    }

    pub fn is_password_rotate_required(&self, state: &AppState) -> UserResult<bool> {
        let last_password_modified_at =
            if let Some(last_password_modified_at) = self.0.last_password_modified_at {
                last_password_modified_at.date()
            } else {
                return Ok(true);
            };

        let password_change_duration =
            time::Duration::days(state.conf.user.password_validity_in_days.into());
        let last_date_for_password_rotate = last_password_modified_at
            .checked_add(password_change_duration)
            .ok_or(UserErrors::InternalServerError)?;

        let today = common_utils::date_time::now().date();
        let days_left_for_password_rotate = last_date_for_password_rotate - today;

        Ok(days_left_for_password_rotate.whole_days() < 0)
    }

    pub fn get_preferred_merchant_id(&self) -> Option<String> {
        self.0.preferred_merchant_id.clone()
    }

    pub async fn get_role_from_db_by_merchant_id(
        &self,
        state: &AppState,
        merchant_id: &str,
    ) -> CustomResult<UserRole, errors::StorageError> {
        state
            .store
            .find_user_role_by_user_id_merchant_id(self.get_user_id(), merchant_id)
            .await
    }

    pub async fn get_preferred_or_active_user_role_from_db(
        &self,
        state: &AppState,
    ) -> CustomResult<UserRole, errors::StorageError> {
        if let Some(preferred_merchant_id) = self.get_preferred_merchant_id() {
            self.get_role_from_db_by_merchant_id(state, &preferred_merchant_id)
                .await
        } else {
            state
                .store
                .list_user_roles_by_user_id(&self.0.user_id)
                .await?
                .into_iter()
                .find(|role| role.status == UserStatus::Active)
                .ok_or(
                    errors::StorageError::ValueNotFound(
                        "No active role found for user".to_string(),
                    )
                    .into(),
                )
        }
    }

    pub async fn get_or_create_key_store(&self, state: &AppState) -> UserResult<UserKeyStore> {
        let master_key = state.store.get_master_key();
        let key_store_result = state
            .store
            .get_user_key_store_by_user_id(self.get_user_id(), &master_key.to_vec().into())
            .await;

        if let Ok(key_store) = key_store_result {
            Ok(key_store)
        } else if key_store_result
            .as_ref()
            .map_err(|e| e.current_context().is_db_not_found())
            .err()
            .unwrap_or(false)
        {
            let key = services::generate_aes256_key()
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Unable to generate aes 256 key")?;

            let key_store = UserKeyStore {
                user_id: self.get_user_id().to_string(),
                key: domain_types::encrypt(key.to_vec().into(), master_key)
                    .await
                    .change_context(UserErrors::InternalServerError)?,
                created_at: common_utils::date_time::now(),
            };
            state
                .store
                .insert_user_key_store(key_store, &master_key.to_vec().into())
                .await
                .change_context(UserErrors::InternalServerError)
        } else {
            Err(key_store_result
                .err()
                .map(|e| e.change_context(UserErrors::InternalServerError))
                .unwrap_or(UserErrors::InternalServerError.into()))
        }
    }

    pub fn get_totp_status(&self) -> TotpStatus {
        self.0.totp_status
    }

    pub fn get_recovery_codes(&self) -> Option<Vec<Secret<String>>> {
        self.0.totp_recovery_codes.clone()
    }

    pub async fn decrypt_and_get_totp_secret(
        &self,
        state: &AppState,
    ) -> UserResult<Option<Secret<String>>> {
        if self.0.totp_secret.is_none() {
            return Ok(None);
        }

        let user_key_store = state
            .store
            .get_user_key_store_by_user_id(
                self.get_user_id(),
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .change_context(UserErrors::InternalServerError)?;

        Ok(domain_types::decrypt::<String, masking::WithType>(
            self.0.totp_secret.clone(),
            user_key_store.key.peek(),
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .map(Encryptable::into_inner))
    }
}

impl From<info::ModuleInfo> for user_role_api::ModuleInfo {
    fn from(value: info::ModuleInfo) -> Self {
        Self {
            module: value.module.into(),
            description: value.description,
            permissions: value.permissions.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<info::PermissionModule> for user_role_api::PermissionModule {
    fn from(value: info::PermissionModule) -> Self {
        match value {
            info::PermissionModule::Payments => Self::Payments,
            info::PermissionModule::Refunds => Self::Refunds,
            info::PermissionModule::MerchantAccount => Self::MerchantAccount,
            info::PermissionModule::Connectors => Self::Connectors,
            info::PermissionModule::Routing => Self::Routing,
            info::PermissionModule::Analytics => Self::Analytics,
            info::PermissionModule::Mandates => Self::Mandates,
            info::PermissionModule::Customer => Self::Customer,
            info::PermissionModule::Disputes => Self::Disputes,
            info::PermissionModule::ThreeDsDecisionManager => Self::ThreeDsDecisionManager,
            info::PermissionModule::SurchargeDecisionManager => Self::SurchargeDecisionManager,
            info::PermissionModule::AccountCreate => Self::AccountCreate,
            info::PermissionModule::Payouts => Self::Payouts,
        }
    }
}

pub enum SignInWithRoleStrategyType {
    SingleRole(SignInWithSingleRoleStrategy),
    MultipleRoles(SignInWithMultipleRolesStrategy),
}

impl SignInWithRoleStrategyType {
    pub async fn decide_signin_strategy_by_user_roles(
        user: UserFromStorage,
        user_roles: Vec<UserRole>,
    ) -> UserResult<Self> {
        if user_roles.is_empty() {
            return Err(UserErrors::InternalServerError.into());
        }

        if let Some(user_role) = user_roles
            .iter()
            .find(|role| role.status == UserStatus::Active)
        {
            Ok(Self::SingleRole(SignInWithSingleRoleStrategy {
                user,
                user_role: user_role.clone(),
            }))
        } else {
            Ok(Self::MultipleRoles(SignInWithMultipleRolesStrategy {
                user,
                user_roles,
            }))
        }
    }

    pub async fn get_signin_response(
        self,
        state: &AppState,
    ) -> UserResult<user_api::SignInResponse> {
        match self {
            Self::SingleRole(strategy) => strategy.get_signin_response(state).await,
            Self::MultipleRoles(strategy) => strategy.get_signin_response(state).await,
        }
    }
}

pub struct SignInWithSingleRoleStrategy {
    pub user: UserFromStorage,
    pub user_role: UserRole,
}

impl SignInWithSingleRoleStrategy {
    async fn get_signin_response(self, state: &AppState) -> UserResult<user_api::SignInResponse> {
        let token =
            utils::user::generate_jwt_auth_token(state, &self.user, &self.user_role).await?;
        utils::user_role::set_role_permissions_in_cache_by_user_role(state, &self.user_role).await;

        let dashboard_entry_response =
            utils::user::get_dashboard_entry_response(state, self.user, self.user_role, token)?;

        Ok(user_api::SignInResponse::DashboardEntry(
            dashboard_entry_response,
        ))
    }
}

pub struct SignInWithMultipleRolesStrategy {
    pub user: UserFromStorage,
    pub user_roles: Vec<UserRole>,
}

impl SignInWithMultipleRolesStrategy {
    async fn get_signin_response(self, state: &AppState) -> UserResult<user_api::SignInResponse> {
        let merchant_accounts = state
            .store
            .list_multiple_merchant_accounts(
                self.user_roles
                    .iter()
                    .map(|role| role.merchant_id.clone())
                    .collect(),
            )
            .await
            .change_context(UserErrors::InternalServerError)?;

        let roles =
            utils::user_role::get_multiple_role_info_for_user_roles(state, &self.user_roles)
                .await?;

        let merchant_details = utils::user::get_multiple_merchant_details_with_status(
            self.user_roles,
            merchant_accounts,
            roles,
        )?;

        Ok(user_api::SignInResponse::MerchantSelect(
            user_api::MerchantSelectResponse {
                name: self.user.get_name(),
                email: self.user.get_email(),
                token: auth::SinglePurposeToken::new_token(
                    self.user.get_user_id().to_string(),
                    TokenPurpose::AcceptInvite,
                    Origin::SignIn,
                    &state.conf,
                )
                .await?
                .into(),
                merchants: merchant_details,
                verification_days_left: utils::user::get_verification_days_left(state, &self.user)?,
            },
        ))
    }
}

impl ForeignFrom<UserStatus> for user_role_api::UserStatus {
    fn foreign_from(value: UserStatus) -> Self {
        match value {
            UserStatus::Active => Self::Active,
            UserStatus::InvitationSent => Self::InvitationSent,
        }
    }
}

#[derive(Clone)]
pub struct RoleName(String);

impl RoleName {
    pub fn new(name: String) -> UserResult<Self> {
        let is_empty_or_whitespace = name.trim().is_empty();
        let is_too_long = name.graphemes(true).count() > consts::user_role::MAX_ROLE_NAME_LENGTH;

        if is_empty_or_whitespace || is_too_long || name.contains(' ') {
            Err(UserErrors::RoleNameParsingError.into())
        } else {
            Ok(Self(name.to_lowercase()))
        }
    }

    pub fn get_role_name(self) -> String {
        self.0
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RecoveryCodes(pub Vec<Secret<String>>);

impl RecoveryCodes {
    pub fn generate_new() -> Self {
        let mut rand = rand::thread_rng();
        let recovery_codes = (0..consts::user::RECOVERY_CODES_COUNT)
            .map(|_| {
                let code_part_1 =
                    Alphanumeric.sample_string(&mut rand, consts::user::RECOVERY_CODE_LENGTH / 2);
                let code_part_2 =
                    Alphanumeric.sample_string(&mut rand, consts::user::RECOVERY_CODE_LENGTH / 2);

                Secret::new(format!("{}-{}", code_part_1, code_part_2))
            })
            .collect::<Vec<_>>();

        Self(recovery_codes)
    }

    pub fn get_hashed(&self) -> UserResult<Vec<Secret<String>>> {
        self.0
            .iter()
            .cloned()
            .map(password::generate_password_hash)
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn into_inner(self) -> Vec<Secret<String>> {
        self.0
    }
}
