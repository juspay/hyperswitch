use std::{collections::HashSet, ops, str::FromStr};

use api_models::{admin as admin_api, organization as api_org, user as user_api};
use common_utils::pii;
use diesel_models::{
    enums::UserStatus,
    organization as diesel_org,
    organization::Organization,
    user as storage_user,
    user_role::{UserRole, UserRoleNew},
};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use once_cell::sync::Lazy;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    consts::user as consts,
    core::{
        admin,
        errors::{UserErrors, UserResult},
    },
    db::StorageInterface,
    routes::AppState,
    services::authentication::AuthToken,
    types::transformers::ForeignFrom,
    utils::user::password,
};

#[derive(Clone)]
pub struct UserName(Secret<String>);

impl UserName {
    pub fn new(name: Secret<String>) -> UserResult<Self> {
        let name = name.expose();
        let is_empty_or_whitespace = name.trim().is_empty();
        let is_too_long = name.graphemes(true).count() > consts::MAX_NAME_LENGTH;

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
        let email_string = email.expose();
        let email =
            pii::Email::from_str(&email_string).change_context(UserErrors::EmailParsingError)?;

        if validator::validate_email(&email_string) {
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
        let email_string = email.peek();
        if validator::validate_email(email_string) {
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
        if password.is_empty() {
            Err(UserErrors::PasswordParsingError.into())
        } else {
            Ok(Self(password.into()))
        }
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
        let is_too_long = company_name.graphemes(true).count() > consts::MAX_COMPANY_NAME_LENGTH;

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

impl From<user_api::ConnectAccountRequest> for NewUserOrganization {
    fn from(_value: user_api::ConnectAccountRequest) -> Self {
        let new_organization = api_org::OrganizationNew::new(None);
        let db_organization = ForeignFrom::foreign_from(new_organization);
        Self(db_organization)
    }
}

#[derive(Clone)]
pub struct NewUserMerchant {
    merchant_id: String,
    company_name: Option<UserCompanyName>,
    new_organization: NewUserOrganization,
}

impl NewUserMerchant {
    pub fn get_company_name(&self) -> Option<String> {
        self.company_name.clone().map(UserCompanyName::get_secret)
    }

    pub fn get_merchant_id(&self) -> String {
        self.merchant_id.clone()
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
            )))
            .into_report();
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
                payment_link_config: None,
                sub_merchants_enabled: None,
                frm_routing_algorithm: None,
                intent_fulfillment_time: None,
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

impl TryFrom<user_api::ConnectAccountRequest> for NewUserMerchant {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::ConnectAccountRequest) -> UserResult<Self> {
        let merchant_id = format!("merchant_{}", common_utils::date_time::now_unix_timestamp());
        let new_organization = NewUserOrganization::from(value);

        Ok(Self {
            company_name: None,
            merchant_id,
            new_organization,
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

    pub async fn insert_user_in_db(
        &self,
        db: &dyn StorageInterface,
    ) -> UserResult<UserFromStorage> {
        match db.insert_user(self.clone().try_into()?).await {
            Ok(user) => Ok(user.into()),
            Err(e) => {
                if e.current_context().is_db_unique_violation() {
                    return Err(e.change_context(UserErrors::UserExists));
                } else {
                    return Err(e.change_context(UserErrors::InternalServerError));
                }
            }
        }
        .attach_printable("Error while inserting user")
    }

    pub async fn insert_user_and_merchant_in_db(
        &self,
        state: AppState,
    ) -> UserResult<UserFromStorage> {
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
                last_modified_at: now,
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
        Ok(Self {
            user_id: value.get_user_id(),
            name: value.get_name(),
            email: value.get_email().into_inner(),
            password: hashed_password,
            ..Default::default()
        })
    }
}

impl TryFrom<user_api::ConnectAccountRequest> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::ConnectAccountRequest) -> UserResult<Self> {
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
        })
    }
}

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

    pub fn compare_password(&self, candidate: Secret<String>) -> UserResult<()> {
        match password::is_correct_password(candidate, self.0.password.clone()) {
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

    pub async fn get_jwt_auth_token(&self, state: AppState, org_id: String) -> UserResult<String> {
        let role_id = self.get_role_from_db(state.clone()).await?.role_id;
        let merchant_id = state
            .store
            .find_user_role_by_user_id(self.get_user_id())
            .await
            .change_context(UserErrors::InternalServerError)?
            .merchant_id;
        AuthToken::new_token(
            self.0.user_id.clone(),
            merchant_id,
            role_id,
            &state.conf,
            org_id,
        )
        .await
    }

    pub async fn get_role_from_db(&self, state: AppState) -> UserResult<UserRole> {
        state
            .store
            .find_user_role_by_user_id(self.get_user_id())
            .await
            .change_context(UserErrors::InternalServerError)
    }
}
