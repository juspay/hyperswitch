use std::{collections::HashSet, ops, str::FromStr};

use api_models::{
    admin as admin_api, organization as api_org, user as user_api, user_role as user_role_api,
};
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
use router_env::env;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    consts,
    core::{
        admin,
        errors::{UserErrors, UserResult},
    },
    db::StorageInterface,
    routes::AppState,
    services::{
        authentication::UserFromToken,
        authorization::{info, predefined_permissions},
    },
    types::transformers::ForeignFrom,
    utils::user::password,
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
                sub_merchants_enabled: None,
                frm_routing_algorithm: None,
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
                    return Err(e.change_context(UserErrors::UserExists));
                } else {
                    return Err(e.change_context(UserErrors::InternalServerError));
                }
            }
        }
        .attach_printable("Error while inserting user")
    }

    pub async fn check_if_already_exists_in_db(&self, state: AppState) -> UserResult<()> {
        if state
            .store
            .find_user_by_email(self.get_email().into_inner().expose().expose().as_str())
            .await
            .is_ok()
        {
            return Err(UserErrors::UserExists).into_report();
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
            .find_user_role_by_user_id(self.get_user_id())
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
            info::PermissionModule::Forex => Self::Forex,
            info::PermissionModule::Connectors => Self::Connectors,
            info::PermissionModule::Routing => Self::Routing,
            info::PermissionModule::Analytics => Self::Analytics,
            info::PermissionModule::Mandates => Self::Mandates,
            info::PermissionModule::Customer => Self::Customer,
            info::PermissionModule::Disputes => Self::Disputes,
            info::PermissionModule::Files => Self::Files,
            info::PermissionModule::ThreeDsDecisionManager => Self::ThreeDsDecisionManager,
            info::PermissionModule::SurchargeDecisionManager => Self::SurchargeDecisionManager,
            info::PermissionModule::AccountCreate => Self::AccountCreate,
        }
    }
}

impl From<info::PermissionInfo> for user_role_api::PermissionInfo {
    fn from(value: info::PermissionInfo) -> Self {
        Self {
            enum_name: value.enum_name.into(),
            description: value.description,
        }
    }
}

pub struct UserAndRoleJoined(pub storage_user::User, pub UserRole);

impl TryFrom<UserAndRoleJoined> for user_api::UserDetails {
    type Error = ();
    fn try_from(user_and_role: UserAndRoleJoined) -> Result<Self, Self::Error> {
        let status = match user_and_role.1.status {
            UserStatus::Active => user_role_api::UserStatus::Active,
            UserStatus::InvitationSent => user_role_api::UserStatus::InvitationSent,
        };

        let role_id = user_and_role.1.role_id;
        let role_name = predefined_permissions::get_role_name_from_id(role_id.as_str())
            .ok_or(())?
            .to_string();

        Ok(Self {
            user_id: user_and_role.0.user_id,
            email: user_and_role.0.email,
            name: user_and_role.0.name,
            role_id,
            status,
            role_name,
            last_modified_at: user_and_role.0.last_modified_at,
        })
    }
}
