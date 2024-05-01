use std::{collections::HashSet, ops, str::FromStr};

use api_models::{
    admin as admin_api, organization as api_org, user as user_api, user_role as user_role_api,
};
use common_enums::enums::TokenPurpose;
use common_utils::{errors::CustomResult, pii};
use diesel_models::{
    enums::UserStatus,
    organization as diesel_org,
    organization::Organization,
    user as storage_user,
    user_role::{UserRole, UserRoleNew},
};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use once_cell::sync::Lazy;
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
    services::{authentication as auth, authentication::UserFromToken, authorization::info},
    types::transformers::ForeignFrom,
    utils::{self, user::password},
};

pub mod dashboard_metadata;

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
        Ok(Self {
            user_id: value.get_user_id(),
            name: value.get_name(),
            email: value.get_email().into_inner(),
            password: hashed_password,
            ..Default::default()
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
        })
    }
}

impl TryFrom<user_api::ConnectAccountRequest> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::ConnectAccountRequest) -> UserResult<Self> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let email = value.email.clone().try_into()?;
        let name = UserName::try_from(value.email.clone())?;
        let password = UserPassword::new(uuid::Uuid::new_v4().to_string().into())?;
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
            password: UserPassword::new(user.0.password)?,
            new_merchant,
        })
    }
}

impl TryFrom<InviteeUserRequestWithInvitedUserToken> for NewUser {
    type Error = error_stack::Report<UserErrors>;
    fn try_from(value: InviteeUserRequestWithInvitedUserToken) -> UserResult<Self> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let email = value.0.email.clone().try_into()?;
        let name = UserName::new(value.0.name.clone())?;
        let password = UserPassword::new(uuid::Uuid::new_v4().to_string().into())?;
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
                token: auth::UserAuthToken::new_token(
                    self.user.get_user_id().to_string(),
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

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Flows {
    SPTFlows(SPTFlow),
    JWTFlows(JWTFlow),
}

impl Flows {
    async fn is_required(&self, user: &UserFromStorage, state: &AppState) -> UserResult<bool> {
        match self {
            Flows::SPTFlows(flow) => flow.is_required(user, state).await,
            Flows::JWTFlows(flow) => flow.is_required(user, state).await,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum SPTFlow {
    TOTP,
    VerifyEmail,
    AcceptInvitationFromEmail,
    ForceSetPassword,
    MerchantSelect,
    ResetPassword,
}

impl SPTFlow {
    async fn is_required(&self, user: &UserFromStorage, state: &AppState) -> UserResult<bool> {
        match self {
            // TOTP
            Self::TOTP => Ok(true),
            // Main email APIs
            Self::AcceptInvitationFromEmail | Self::ResetPassword => Ok(true),
            Self::VerifyEmail => Ok(user.0.is_verified),
            // Final Checks
            // TODO: this should be based on last_password_modified_at as a placeholder using false
            Self::ForceSetPassword => Ok(false),
            Self::MerchantSelect => user.get_roles_from_db(&state).await.map(|roles| {
                roles
                    .iter()
                    .find(|role| role.status == UserStatus::Active)
                    .is_none()
            }),
        }
    }

    pub async fn generate_spt(
        self,
        state: &AppState,
        next_flow: &NextFlow,
    ) -> UserResult<Secret<String>> {
        auth::SinglePurposeToken::new_token(
            next_flow.user.get_user_id().to_string(),
            self.into(),
            next_flow.origin.clone(),
            &state.conf,
        )
        .await
        .map(|token| token.into())
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum JWTFlow {
    Home,
}

impl JWTFlow {
    async fn is_required(&self, _user: &UserFromStorage, _state: &AppState) -> UserResult<bool> {
        Ok(true)
    }

    pub async fn generate_jwt(
        self,
        state: &AppState,
        next_flow: &NextFlow,
        user_role: &UserRole,
    ) -> UserResult<Secret<String>> {
        auth::AuthToken::new_token(
            next_flow.user.get_user_id().to_string(),
            user_role.merchant_id.clone(),
            user_role.role_id.clone(),
            &state.conf,
            user_role.org_id.clone(),
        )
        .await
        .map(|token| token.into())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum Origin {
    SignIn,
    SignUp,
    MagicLink,
    VerifyEmail,
    AcceptInvitationFromEmail,
    ResetPassword,
}

const SIGNIN_FLOW: [Flows; 4] = [
    Flows::SPTFlows(SPTFlow::TOTP),
    Flows::SPTFlows(SPTFlow::ForceSetPassword),
    Flows::SPTFlows(SPTFlow::MerchantSelect),
    Flows::JWTFlows(JWTFlow::Home),
];

const SIGNUP_FLOW: [Flows; 4] = [
    Flows::SPTFlows(SPTFlow::TOTP),
    Flows::SPTFlows(SPTFlow::ForceSetPassword),
    Flows::SPTFlows(SPTFlow::MerchantSelect),
    Flows::JWTFlows(JWTFlow::Home),
];

const MAGIC_LINK_FLOW: [Flows; 5] = [
    Flows::SPTFlows(SPTFlow::TOTP),
    Flows::SPTFlows(SPTFlow::VerifyEmail),
    Flows::SPTFlows(SPTFlow::ForceSetPassword),
    Flows::SPTFlows(SPTFlow::MerchantSelect),
    Flows::JWTFlows(JWTFlow::Home),
];

const VERIFY_EMAIL_FLOW: [Flows; 5] = [
    Flows::SPTFlows(SPTFlow::TOTP),
    Flows::SPTFlows(SPTFlow::VerifyEmail),
    Flows::SPTFlows(SPTFlow::ForceSetPassword),
    Flows::SPTFlows(SPTFlow::MerchantSelect),
    Flows::JWTFlows(JWTFlow::Home),
];

const ACCEPT_INVITATION_FROM_EMAIL_FLOW: [Flows; 4] = [
    Flows::SPTFlows(SPTFlow::TOTP),
    Flows::SPTFlows(SPTFlow::AcceptInvitationFromEmail),
    Flows::SPTFlows(SPTFlow::ForceSetPassword),
    Flows::JWTFlows(JWTFlow::Home),
];

const RESET_PASSWORD_FLOW: [Flows; 2] = [
    Flows::SPTFlows(SPTFlow::TOTP),
    Flows::SPTFlows(SPTFlow::ResetPassword),
];

pub struct CurrentFlow {
    origin: Origin,
    current_flow: Flows,
}

impl CurrentFlow {
    pub fn new(origin: Origin, current_flow: Flows) -> UserResult<Self> {
        let flows = origin.get_flows();
        if !flows.contains(&current_flow) {
            return Err(UserErrors::InternalServerError.into());
        }

        Ok(Self {
            origin,
            current_flow,
        })
    }

    pub async fn next(&self, user: UserFromStorage, state: &AppState) -> UserResult<NextFlow> {
        let flows = self.origin.get_flows();
        let current_flow_index = flows
            .iter()
            .position(|flow| flow == &self.current_flow)
            .ok_or(UserErrors::InternalServerError)?;
        let remaining_flows = flows.iter().skip(current_flow_index + 1);
        for flow in remaining_flows {
            if flow.is_required(&user, state).await? {
                return Ok(NextFlow {
                    origin: self.origin.clone(),
                    next_flow: flow.clone(),
                    user,
                });
            }
        }
        return Err(UserErrors::InternalServerError.into());
    }
}

pub struct NextFlow {
    origin: Origin,
    next_flow: Flows,
    user: UserFromStorage,
}

impl NextFlow {
    pub async fn from_origin(
        origin: Origin,
        user: UserFromStorage,
        state: &AppState,
    ) -> UserResult<Self> {
        let flows = origin.get_flows();
        for flow in flows {
            if flow.is_required(&user, state).await? {
                return Ok(Self {
                    origin,
                    next_flow: flow.clone(),
                    user,
                });
            }
        }
        Err(UserErrors::InternalServerError.into())
    }

    pub fn get_flow(&self) -> Flows {
        self.next_flow.clone()
    }
}

impl Origin {
    fn get_flows(&self) -> &'static [Flows] {
        match self {
            Self::SignIn => &SIGNIN_FLOW,
            Self::SignUp => &SIGNUP_FLOW,
            Self::VerifyEmail => &VERIFY_EMAIL_FLOW,
            Self::MagicLink => &MAGIC_LINK_FLOW,
            Self::AcceptInvitationFromEmail => &ACCEPT_INVITATION_FROM_EMAIL_FLOW,
            Self::ResetPassword => &RESET_PASSWORD_FLOW,
        }
    }
}

impl Into<TokenPurpose> for Flows {
    fn into(self) -> TokenPurpose {
        match self {
            Flows::SPTFlows(flow) => flow.into(),
            Flows::JWTFlows(flow) => flow.into(),
        }
    }
}

impl Into<TokenPurpose> for SPTFlow {
    fn into(self) -> TokenPurpose {
        match self {
            SPTFlow::TOTP => TokenPurpose::TOTP,
            SPTFlow::VerifyEmail => TokenPurpose::VerifyEmail,
            SPTFlow::AcceptInvitationFromEmail => TokenPurpose::AcceptInvitationFromEmail,
            SPTFlow::MerchantSelect => TokenPurpose::AcceptInvite,
            SPTFlow::ResetPassword | SPTFlow::ForceSetPassword => TokenPurpose::ResetPassword,
        }
    }
}

impl Into<TokenPurpose> for JWTFlow {
    fn into(self) -> TokenPurpose {
        match self {
            JWTFlow::Home => TokenPurpose::Home,
        }
    }
}

// impl Flow for TerminalFlow {
//     async fn is_required(&self, _user: &UserFromStorage, _state: &AppState) -> UserResult<bool> {
//         Ok(true)
//     }
// }

// trait UserFlow {
//     async fn is_required(user: UserFromStorage, _state: AppState) -> UserResult<bool>;
// }
//
// struct SignInUserFlow;
// impl UserFlow for SignInUserFlow {
//     async fn is_required(_user: UserFromStorage, _state: AppState) -> UserResult<bool> {
//         Ok(false)
//     }
// }
//
// struct SignUpUserFlow;
// impl UserFlow for SignUpUserFlow {
//     async fn is_required(_user: UserFromStorage, _state: AppState) -> UserResult<bool> {
//         Ok(false)
//     }
// }
//
// struct FromEmailUserFlow;
// impl UserFlow for FromEmailUserFlow {
//     async fn is_required(_user: UserFromStorage, _state: AppState) -> UserResult<bool> {
//         Ok(false)
//     }
// }
//
// struct VerifyEmailUserFlow;
// impl UserFlow for VerifyEmailUserFlow {
//     async fn is_required(user: UserFromStorage, _state: AppState) -> UserResult<bool> {
//         Ok(user.0.is_verified)
//     }
// }
//
// struct AcceptInvitationFromEmailUserFlow;
// impl UserFlow for AcceptInvitationFromEmailUserFlow {
//     async fn is_required(user: UserFromStorage, _state: AppState) -> UserResult<bool> {
//         Ok(true)
//     }
// }
//
// struct ForceSetPasswordUserFlow;
// impl UserFlow for ForceSetPasswordUserFlow {
//     async fn is_required(user: UserFromStorage, _state: AppState) -> UserResult<bool> {
//         // TODO: this should be based on last_password_modified_at as a placeholder using false
//         Ok(false)
//     }
// }
// struct ResetPasswordUserFlow;
// impl UserFlow for ResetPasswordUserFlow {
//     async fn is_required(user: UserFromStorage, _state: AppState) -> UserResult<bool> {
//         Ok(true)
//     }
// }
//
// struct TOTPUserFlow;
// impl UserFlow for TOTPUserFlow {
//     async fn is_required(user: UserFromStorage, _state: AppState) -> UserResult<bool> {
//         Ok(true)
//     }
// }
//
// struct MerchantSelectUserFlow;
// impl UserFlow for MerchantSelectUserFlow {
//     async fn is_required(user: UserFromStorage, state: AppState) -> UserResult<bool> {
//         user.get_roles_from_db(&state)
//             .await
//             .find(|role| role.status == UserStatus::Active)
//             .is_none()
//     }
// }
//
// struct HomeUserFlow;
// impl UserFlow for HomeUserFlow {
//     async fn is_required(user: UserFromStorage, _state: AppState) -> UserResult<bool> {
//         Ok(true)
//     }
// }
