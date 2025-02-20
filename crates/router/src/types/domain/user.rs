use std::{
    collections::HashSet,
    ops::{Deref, Not},
    str::FromStr,
};

use api_models::{
    admin as admin_api, organization as api_org, user as user_api, user_role as user_role_api,
};
use common_enums::EntityType;
use common_utils::{
    crypto::Encryptable, id_type, new_type::MerchantName, pii, type_name,
    types::keymanager::Identifier,
};
use diesel_models::{
    enums::{TotpStatus, UserRoleVersion, UserStatus},
    organization::{self as diesel_org, Organization, OrganizationBridge},
    user as storage_user,
    user_role::{UserRole, UserRoleNew},
};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use once_cell::sync::Lazy;
use rand::distributions::{Alphanumeric, DistString};
use router_env::env;
use time::PrimitiveDateTime;
use unicode_segmentation::UnicodeSegmentation;
#[cfg(feature = "keymanager_create")]
use {base64::Engine, common_utils::types::keymanager::EncryptionTransferRequest};

use crate::{
    consts,
    core::{
        admin,
        errors::{UserErrors, UserResult},
    },
    db::GlobalStorageInterface,
    routes::SessionState,
    services::{self, authentication::UserFromToken},
    types::transformers::ForeignFrom,
    utils::user::password,
};

pub mod dashboard_metadata;
pub mod decision_manager;
pub use decision_manager::*;
pub mod user_authentication_method;

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
    pub async fn insert_org_in_db(self, state: SessionState) -> UserResult<Organization> {
        state
            .accounts_store
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

    pub fn get_organization_id(&self) -> id_type::OrganizationId {
        self.0.get_organization_id()
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

impl From<(user_api::CreateInternalUserRequest, id_type::OrganizationId)> for NewUserOrganization {
    fn from(
        (_value, org_id): (user_api::CreateInternalUserRequest, id_type::OrganizationId),
    ) -> Self {
        let new_organization = api_org::OrganizationNew {
            org_id,
            org_name: None,
        };
        let db_organization = ForeignFrom::foreign_from(new_organization);
        Self(db_organization)
    }
}

impl From<UserMerchantCreateRequestWithToken> for NewUserOrganization {
    fn from(value: UserMerchantCreateRequestWithToken) -> Self {
        Self(diesel_org::OrganizationNew::new(
            value.2.org_id,
            Some(value.1.company_name),
        ))
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

impl From<(user_api::CreateTenantUserRequest, MerchantAccountIdentifier)> for NewUserOrganization {
    fn from(
        (_value, merchant_account_identifier): (
            user_api::CreateTenantUserRequest,
            MerchantAccountIdentifier,
        ),
    ) -> Self {
        let new_organization = api_org::OrganizationNew {
            org_id: merchant_account_identifier.org_id,
            org_name: None,
        };
        let db_organization = ForeignFrom::foreign_from(new_organization);
        Self(db_organization)
    }
}

impl ForeignFrom<api_models::user::UserOrgMerchantCreateRequest>
    for diesel_models::organization::OrganizationNew
{
    fn foreign_from(item: api_models::user::UserOrgMerchantCreateRequest) -> Self {
        let org_id = id_type::OrganizationId::default();
        let api_models::user::UserOrgMerchantCreateRequest {
            organization_name,
            organization_details,
            metadata,
            ..
        } = item;
        let mut org_new_db = Self::new(org_id, Some(organization_name.expose()));
        org_new_db.organization_details = organization_details;
        org_new_db.metadata = metadata;
        org_new_db
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

impl TryFrom<MerchantId> for id_type::MerchantId {
    type Error = error_stack::Report<UserErrors>;
    fn try_from(value: MerchantId) -> Result<Self, Self::Error> {
        Self::try_from(std::borrow::Cow::from(value.0))
            .change_context(UserErrors::MerchantIdParsingError)
            .attach_printable("Could not convert user merchant_id to merchant_id type")
    }
}

#[derive(Clone)]
pub struct NewUserMerchant {
    merchant_id: id_type::MerchantId,
    company_name: Option<UserCompanyName>,
    new_organization: NewUserOrganization,
}

impl TryFrom<UserCompanyName> for MerchantName {
    // We should ideally not get this error because all the validations are done for company name
    type Error = error_stack::Report<UserErrors>;

    fn try_from(company_name: UserCompanyName) -> Result<Self, Self::Error> {
        Self::try_new(company_name.get_secret()).change_context(UserErrors::CompanyNameParsingError)
    }
}

impl NewUserMerchant {
    pub fn get_company_name(&self) -> Option<String> {
        self.company_name.clone().map(UserCompanyName::get_secret)
    }

    pub fn get_merchant_id(&self) -> id_type::MerchantId {
        self.merchant_id.clone()
    }

    pub fn get_new_organization(&self) -> NewUserOrganization {
        self.new_organization.clone()
    }

    pub async fn check_if_already_exists_in_db(&self, state: SessionState) -> UserResult<()> {
        if state
            .store
            .get_merchant_key_store_by_merchant_id(
                &(&state).into(),
                &self.get_merchant_id(),
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .is_ok()
        {
            return Err(UserErrors::MerchantAccountCreationError(format!(
                "Merchant with {:?} already exists",
                self.get_merchant_id()
            ))
            .into());
        }
        Ok(())
    }

    #[cfg(feature = "v2")]
    fn create_merchant_account_request(&self) -> UserResult<admin_api::MerchantAccountCreate> {
        let merchant_name = if let Some(company_name) = self.company_name.clone() {
            MerchantName::try_from(company_name)
        } else {
            MerchantName::try_new("merchant".to_string())
                .change_context(UserErrors::InternalServerError)
                .attach_printable("merchant name validation failed")
        }
        .map(Secret::new)?;

        Ok(admin_api::MerchantAccountCreate {
            merchant_name,
            organization_id: self.new_organization.get_organization_id(),
            metadata: None,
            merchant_details: None,
        })
    }

    #[cfg(feature = "v1")]
    fn create_merchant_account_request(&self) -> UserResult<admin_api::MerchantAccountCreate> {
        Ok(admin_api::MerchantAccountCreate {
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
            pm_collect_link_config: None,
        })
    }

    pub async fn create_new_merchant_and_insert_in_db(
        &self,
        state: SessionState,
    ) -> UserResult<()> {
        self.check_if_already_exists_in_db(state.clone()).await?;

        let merchant_account_create_request = self
            .create_merchant_account_request()
            .attach_printable("unable to construct merchant account create request")?;

        Box::pin(admin::create_merchant_account(
            state.clone(),
            merchant_account_create_request,
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
        let merchant_id = id_type::MerchantId::new_from_unix_timestamp();

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
        let merchant_id = id_type::MerchantId::new_from_unix_timestamp();
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
            merchant_id: id_type::MerchantId::try_from(merchant_id)?,
            new_organization,
        })
    }
}

impl TryFrom<(user_api::CreateInternalUserRequest, id_type::OrganizationId)> for NewUserMerchant {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(
        value: (user_api::CreateInternalUserRequest, id_type::OrganizationId),
    ) -> UserResult<Self> {
        let merchant_id = id_type::MerchantId::get_internal_user_merchant_id(
            consts::user_role::INTERNAL_USER_MERCHANT_ID,
        );
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
        let merchant_id = value.clone().1.merchant_id;
        let new_organization = NewUserOrganization::from(value);
        Ok(Self {
            company_name: None,
            merchant_id,
            new_organization,
        })
    }
}

impl From<(user_api::CreateTenantUserRequest, MerchantAccountIdentifier)> for NewUserMerchant {
    fn from(value: (user_api::CreateTenantUserRequest, MerchantAccountIdentifier)) -> Self {
        let merchant_id = value.1.merchant_id.clone();
        let new_organization = NewUserOrganization::from(value);
        Self {
            company_name: None,
            merchant_id,
            new_organization,
        }
    }
}

type UserMerchantCreateRequestWithToken =
    (UserFromStorage, user_api::UserMerchantCreate, UserFromToken);

impl TryFrom<UserMerchantCreateRequestWithToken> for NewUserMerchant {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: UserMerchantCreateRequestWithToken) -> UserResult<Self> {
        let merchant_id = if matches!(env::which(), env::Env::Production) {
            id_type::MerchantId::try_from(MerchantId::new(value.1.company_name.clone())?)?
        } else {
            id_type::MerchantId::new_from_unix_timestamp()
        };
        Ok(Self {
            merchant_id,
            company_name: Some(UserCompanyName::new(value.1.company_name.clone())?),
            new_organization: NewUserOrganization::from(value),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MerchantAccountIdentifier {
    pub merchant_id: id_type::MerchantId,
    pub org_id: id_type::OrganizationId,
}

#[derive(Clone)]
pub struct NewUser {
    user_id: String,
    name: UserName,
    email: UserEmail,
    password: Option<NewUserPassword>,
    new_merchant: NewUserMerchant,
}

#[derive(Clone)]
pub struct NewUserPassword {
    password: UserPassword,
    is_temporary: bool,
}

impl Deref for NewUserPassword {
    type Target = UserPassword;

    fn deref(&self) -> &Self::Target {
        &self.password
    }
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

    pub fn get_password(&self) -> Option<UserPassword> {
        self.password
            .as_ref()
            .map(|password| password.deref().clone())
    }

    pub async fn insert_user_in_db(
        &self,
        db: &dyn GlobalStorageInterface,
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

    pub async fn check_if_already_exists_in_db(&self, state: SessionState) -> UserResult<()> {
        if state
            .global_store
            .find_user_by_email(&self.get_email())
            .await
            .is_ok()
        {
            return Err(report!(UserErrors::UserExists));
        }
        Ok(())
    }

    pub async fn insert_user_and_merchant_in_db(
        &self,
        state: SessionState,
    ) -> UserResult<UserFromStorage> {
        self.check_if_already_exists_in_db(state.clone()).await?;
        let db = state.global_store.as_ref();
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

    pub fn get_no_level_user_role(
        self,
        role_id: String,
        user_status: UserStatus,
    ) -> NewUserRole<NoLevel> {
        let now = common_utils::date_time::now();
        let user_id = self.get_user_id();

        NewUserRole {
            status: user_status,
            created_by: user_id.clone(),
            last_modified_by: user_id.clone(),
            user_id,
            role_id,
            created_at: now,
            last_modified: now,
            entity: NoLevel,
        }
    }

    pub async fn insert_org_level_user_role_in_db(
        self,
        state: SessionState,
        role_id: String,
        user_status: UserStatus,
    ) -> UserResult<UserRole> {
        let org_id = self
            .get_new_merchant()
            .get_new_organization()
            .get_organization_id();

        let org_user_role = self
            .get_no_level_user_role(role_id, user_status)
            .add_entity(OrganizationLevel {
                tenant_id: state.tenant.tenant_id.clone(),
                org_id,
            });

        org_user_role.insert_in_v2(&state).await
    }
}

impl TryFrom<NewUser> for storage_user::UserNew {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: NewUser) -> UserResult<Self> {
        let hashed_password = value
            .password
            .as_ref()
            .map(|password| password::generate_password_hash(password.get_secret()))
            .transpose()?;

        let now = common_utils::date_time::now();
        Ok(Self {
            user_id: value.get_user_id(),
            name: value.get_name(),
            email: value.get_email().into_inner(),
            password: hashed_password,
            is_verified: false,
            created_at: Some(now),
            last_modified_at: Some(now),
            totp_status: TotpStatus::NotSet,
            totp_secret: None,
            totp_recovery_codes: None,
            last_password_modified_at: value
                .password
                .and_then(|password_inner| password_inner.is_temporary.not().then_some(now)),
        })
    }
}

impl TryFrom<user_api::SignUpWithMerchantIdRequest> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: user_api::SignUpWithMerchantIdRequest) -> UserResult<Self> {
        let email = value.email.clone().try_into()?;
        let name = UserName::new(value.name.clone())?;
        let password = NewUserPassword {
            password: UserPassword::new(value.password.clone())?,
            is_temporary: false,
        };
        let user_id = uuid::Uuid::new_v4().to_string();
        let new_merchant = NewUserMerchant::try_from(value)?;

        Ok(Self {
            name,
            email,
            password: Some(password),
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
        let password = NewUserPassword {
            password: UserPassword::new(value.password.clone())?,
            is_temporary: false,
        };
        let new_merchant = NewUserMerchant::try_from(value)?;

        Ok(Self {
            user_id,
            name,
            email,
            password: Some(password),
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
        let new_merchant = NewUserMerchant::try_from(value)?;

        Ok(Self {
            user_id,
            name,
            email,
            password: None,
            new_merchant,
        })
    }
}

impl TryFrom<(user_api::CreateInternalUserRequest, id_type::OrganizationId)> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(
        (value, org_id): (user_api::CreateInternalUserRequest, id_type::OrganizationId),
    ) -> UserResult<Self> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let email = value.email.clone().try_into()?;
        let name = UserName::new(value.name.clone())?;
        let password = NewUserPassword {
            password: UserPassword::new(value.password.clone())?,
            is_temporary: false,
        };
        let new_merchant = NewUserMerchant::try_from((value, org_id))?;

        Ok(Self {
            user_id,
            name,
            email,
            password: Some(password),
            new_merchant,
        })
    }
}

impl TryFrom<UserMerchantCreateRequestWithToken> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(value: UserMerchantCreateRequestWithToken) -> Result<Self, Self::Error> {
        let user = value.0.clone();
        let new_merchant = NewUserMerchant::try_from(value)?;
        let password = user
            .0
            .password
            .map(UserPassword::new_password_without_validation)
            .transpose()?
            .map(|password| NewUserPassword {
                password,
                is_temporary: false,
            });

        Ok(Self {
            user_id: user.0.user_id,
            name: UserName::new(user.0.name)?,
            email: user.0.email.clone().try_into()?,
            password,
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
        let password = cfg!(not(feature = "email")).then_some(NewUserPassword {
            password: UserPassword::new(password::get_temp_password())?,
            is_temporary: true,
        });
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

impl TryFrom<(user_api::CreateTenantUserRequest, MerchantAccountIdentifier)> for NewUser {
    type Error = error_stack::Report<UserErrors>;

    fn try_from(
        (value, merchant_account_identifier): (
            user_api::CreateTenantUserRequest,
            MerchantAccountIdentifier,
        ),
    ) -> UserResult<Self> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let email = value.email.clone().try_into()?;
        let name = UserName::new(value.name.clone())?;
        let password = NewUserPassword {
            password: UserPassword::new(value.password.clone())?,
            is_temporary: false,
        };
        let new_merchant = NewUserMerchant::from((value, merchant_account_identifier));

        Ok(Self {
            user_id,
            name,
            email,
            password: Some(password),
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

    pub fn compare_password(&self, candidate: &Secret<String>) -> UserResult<()> {
        if let Some(password) = self.0.password.as_ref() {
            match password::is_correct_password(candidate, password) {
                Ok(true) => Ok(()),
                Ok(false) => Err(UserErrors::InvalidCredentials.into()),
                Err(e) => Err(e),
            }
        } else {
            Err(UserErrors::InvalidCredentials.into())
        }
    }

    pub fn get_name(&self) -> Secret<String> {
        self.0.name.clone()
    }

    pub fn get_email(&self) -> pii::Email {
        self.0.email.clone()
    }

    #[cfg(feature = "email")]
    pub fn get_verification_days_left(&self, state: &SessionState) -> UserResult<Option<i64>> {
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

    pub fn is_verified(&self) -> bool {
        self.0.is_verified
    }

    pub fn is_password_rotate_required(&self, state: &SessionState) -> UserResult<bool> {
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

    pub async fn get_or_create_key_store(&self, state: &SessionState) -> UserResult<UserKeyStore> {
        let master_key = state.store.get_master_key();
        let key_manager_state = &state.into();
        let key_store_result = state
            .global_store
            .get_user_key_store_by_user_id(
                key_manager_state,
                self.get_user_id(),
                &master_key.to_vec().into(),
            )
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

            #[cfg(feature = "keymanager_create")]
            {
                common_utils::keymanager::transfer_key_to_key_manager(
                    key_manager_state,
                    EncryptionTransferRequest {
                        identifier: Identifier::User(self.get_user_id().to_string()),
                        key: consts::BASE64_ENGINE.encode(key),
                    },
                )
                .await
                .change_context(UserErrors::InternalServerError)?;
            }

            let key_store = UserKeyStore {
                user_id: self.get_user_id().to_string(),
                key: domain_types::crypto_operation(
                    key_manager_state,
                    type_name!(UserKeyStore),
                    domain_types::CryptoOperation::Encrypt(key.to_vec().into()),
                    Identifier::User(self.get_user_id().to_string()),
                    master_key,
                )
                .await
                .and_then(|val| val.try_into_operation())
                .change_context(UserErrors::InternalServerError)?,
                created_at: common_utils::date_time::now(),
            };

            state
                .global_store
                .insert_user_key_store(key_manager_state, key_store, &master_key.to_vec().into())
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
        state: &SessionState,
    ) -> UserResult<Option<Secret<String>>> {
        if self.0.totp_secret.is_none() {
            return Ok(None);
        }
        let key_manager_state = &state.into();
        let user_key_store = state
            .global_store
            .get_user_key_store_by_user_id(
                key_manager_state,
                self.get_user_id(),
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .change_context(UserErrors::InternalServerError)?;

        Ok(domain_types::crypto_operation::<String, masking::WithType>(
            key_manager_state,
            type_name!(storage_user::User),
            domain_types::CryptoOperation::DecryptOptional(self.0.totp_secret.clone()),
            Identifier::User(user_key_store.user_id.clone()),
            user_key_store.key.peek(),
        )
        .await
        .and_then(|val| val.try_into_optionaloperation())
        .change_context(UserErrors::InternalServerError)?
        .map(Encryptable::into_inner))
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

// This is for easier construction
#[derive(Clone)]
pub struct NoLevel;

#[derive(Clone)]
pub struct TenantLevel {
    pub tenant_id: id_type::TenantId,
}

#[derive(Clone)]
pub struct OrganizationLevel {
    pub tenant_id: id_type::TenantId,
    pub org_id: id_type::OrganizationId,
}

#[derive(Clone)]
pub struct MerchantLevel {
    pub tenant_id: id_type::TenantId,
    pub org_id: id_type::OrganizationId,
    pub merchant_id: id_type::MerchantId,
}

#[derive(Clone)]
pub struct ProfileLevel {
    pub tenant_id: id_type::TenantId,
    pub org_id: id_type::OrganizationId,
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
}

#[derive(Clone)]
pub struct NewUserRole<E: Clone> {
    pub user_id: String,
    pub role_id: String,
    pub status: UserStatus,
    pub created_by: String,
    pub last_modified_by: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub entity: E,
}

impl NewUserRole<NoLevel> {
    pub fn add_entity<T>(self, entity: T) -> NewUserRole<T>
    where
        T: Clone,
    {
        NewUserRole {
            entity,
            user_id: self.user_id,
            role_id: self.role_id,
            status: self.status,
            created_by: self.created_by,
            last_modified_by: self.last_modified_by,
            created_at: self.created_at,
            last_modified: self.last_modified,
        }
    }
}

pub struct EntityInfo {
    tenant_id: id_type::TenantId,
    org_id: Option<id_type::OrganizationId>,
    merchant_id: Option<id_type::MerchantId>,
    profile_id: Option<id_type::ProfileId>,
    entity_id: String,
    entity_type: EntityType,
}

impl From<TenantLevel> for EntityInfo {
    fn from(value: TenantLevel) -> Self {
        Self {
            entity_id: value.tenant_id.get_string_repr().to_owned(),
            entity_type: EntityType::Tenant,
            tenant_id: value.tenant_id,
            org_id: None,
            merchant_id: None,
            profile_id: None,
        }
    }
}

impl From<OrganizationLevel> for EntityInfo {
    fn from(value: OrganizationLevel) -> Self {
        Self {
            entity_id: value.org_id.get_string_repr().to_owned(),
            entity_type: EntityType::Organization,
            tenant_id: value.tenant_id,
            org_id: Some(value.org_id),
            merchant_id: None,
            profile_id: None,
        }
    }
}

impl From<MerchantLevel> for EntityInfo {
    fn from(value: MerchantLevel) -> Self {
        Self {
            entity_id: value.merchant_id.get_string_repr().to_owned(),
            entity_type: EntityType::Merchant,
            tenant_id: value.tenant_id,
            org_id: Some(value.org_id),
            merchant_id: Some(value.merchant_id),
            profile_id: None,
        }
    }
}

impl From<ProfileLevel> for EntityInfo {
    fn from(value: ProfileLevel) -> Self {
        Self {
            entity_id: value.profile_id.get_string_repr().to_owned(),
            entity_type: EntityType::Profile,
            tenant_id: value.tenant_id,
            org_id: Some(value.org_id),
            merchant_id: Some(value.merchant_id),
            profile_id: Some(value.profile_id),
        }
    }
}

impl<E> NewUserRole<E>
where
    E: Clone + Into<EntityInfo>,
{
    fn convert_to_new_v2_role(self, entity: EntityInfo) -> UserRoleNew {
        UserRoleNew {
            user_id: self.user_id,
            role_id: self.role_id,
            status: self.status,
            created_by: self.created_by,
            last_modified_by: self.last_modified_by,
            created_at: self.created_at,
            last_modified: self.last_modified,
            org_id: entity.org_id,
            merchant_id: entity.merchant_id,
            profile_id: entity.profile_id,
            entity_id: Some(entity.entity_id),
            entity_type: Some(entity.entity_type),
            version: UserRoleVersion::V2,
            tenant_id: entity.tenant_id,
        }
    }

    pub async fn insert_in_v2(self, state: &SessionState) -> UserResult<UserRole> {
        let entity = self.entity.clone();

        let new_v2_role = self.convert_to_new_v2_role(entity.into());

        state
            .global_store
            .insert_user_role(new_v2_role)
            .await
            .change_context(UserErrors::InternalServerError)
    }
}
