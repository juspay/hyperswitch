use common_enums::TokenPurpose;
use common_utils::{id_type, types::user::LineageContext};
use diesel_models::{
    enums::{UserRoleVersion, UserStatus},
    user_role::UserRole,
};
use error_stack::ResultExt;
use masking::Secret;
use router_env::logger;

use super::UserFromStorage;
use crate::{
    core::errors::{UserErrors, UserResult},
    db::user_role::ListUserRolesByUserIdPayload,
    routes::SessionState,
    services::authentication as auth,
    utils,
};

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum UserFlow {
    SPTFlow(SPTFlow),
    JWTFlow(JWTFlow),
}

impl UserFlow {
    async fn is_required(
        &self,
        user: &UserFromStorage,
        path: &[TokenPurpose],
        state: &SessionState,
        user_tenant_id: &id_type::TenantId,
    ) -> UserResult<bool> {
        match self {
            Self::SPTFlow(flow) => flow.is_required(user, path, state, user_tenant_id).await,
            Self::JWTFlow(flow) => flow.is_required(user, state).await,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum SPTFlow {
    AuthSelect,
    SSO,
    TOTP,
    VerifyEmail,
    AcceptInvitationFromEmail,
    ForceSetPassword,
    MerchantSelect,
    ResetPassword,
}

impl SPTFlow {
    async fn is_required(
        &self,
        user: &UserFromStorage,
        path: &[TokenPurpose],
        state: &SessionState,
        user_tenant_id: &id_type::TenantId,
    ) -> UserResult<bool> {
        match self {
            // Auth
            Self::AuthSelect => Ok(true),
            Self::SSO => Ok(true),
            // TOTP
            Self::TOTP => Ok(!path.contains(&TokenPurpose::SSO)),
            // Main email APIs
            Self::AcceptInvitationFromEmail | Self::ResetPassword => Ok(true),
            Self::VerifyEmail => Ok(true),
            // Final Checks
            Self::ForceSetPassword => user
                .is_password_rotate_required(state)
                .map(|rotate_required| rotate_required && !path.contains(&TokenPurpose::SSO)),
            Self::MerchantSelect => Ok(state
                .global_store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id: user.get_user_id(),
                    tenant_id: user_tenant_id,
                    org_id: None,
                    merchant_id: None,
                    profile_id: None,
                    entity_id: None,
                    version: None,
                    status: Some(UserStatus::Active),
                    limit: Some(1),
                })
                .await
                .change_context(UserErrors::InternalServerError)?
                .is_empty()),
        }
    }

    pub async fn generate_spt(
        self,
        state: &SessionState,
        next_flow: &NextFlow,
    ) -> UserResult<Secret<String>> {
        auth::SinglePurposeToken::new_token(
            next_flow.user.get_user_id().to_string(),
            self.into(),
            next_flow.origin.clone(),
            &state.conf,
            next_flow.path.to_vec(),
            Some(state.tenant.tenant_id.clone()),
        )
        .await
        .map(|token| token.into())
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum JWTFlow {
    UserInfo,
}

impl JWTFlow {
    async fn is_required(
        &self,
        _user: &UserFromStorage,
        _state: &SessionState,
    ) -> UserResult<bool> {
        Ok(true)
    }

    pub async fn generate_jwt(
        self,
        state: &SessionState,
        next_flow: &NextFlow,
        user_role: &UserRole,
    ) -> UserResult<Secret<String>> {
        let user_id = next_flow.user.get_user_id();
        // Fetch lineage context from DB
        let lineage_context_from_db = state
            .global_store
            .find_user_by_id(user_id)
            .await
            .inspect_err(|e| {
                logger::error!(
                    "Failed to fetch lineage context from DB for user {}: {:?}",
                    user_id,
                    e
                )
            })
            .ok()
            .and_then(|user| user.lineage_context);

        let new_lineage_context = match lineage_context_from_db {
            Some(ctx) => {
                let tenant_id = ctx.tenant_id.clone();
                let user_role_match_v2 = state
                    .global_store
                    .find_user_role_by_user_id_and_lineage(
                        &ctx.user_id,
                        &tenant_id,
                        &ctx.org_id,
                        &ctx.merchant_id,
                        &ctx.profile_id,
                        UserRoleVersion::V2,
                    )
                    .await
                    .inspect_err(|e| {
                        logger::error!("Failed to validate V2 role: {e:?}");
                    })
                    .map(|role| role.role_id == ctx.role_id)
                    .unwrap_or_default();

                if user_role_match_v2 {
                    ctx
                } else {
                    let user_role_match_v1 = state
                        .global_store
                        .find_user_role_by_user_id_and_lineage(
                            &ctx.user_id,
                            &tenant_id,
                            &ctx.org_id,
                            &ctx.merchant_id,
                            &ctx.profile_id,
                            UserRoleVersion::V1,
                        )
                        .await
                        .inspect_err(|e| {
                            logger::error!("Failed to validate V1 role: {e:?}");
                        })
                        .map(|role| role.role_id == ctx.role_id)
                        .unwrap_or_default();

                    if user_role_match_v1 {
                        ctx
                    } else {
                        // fallback to default lineage if cached context is invalid
                        Self::resolve_lineage_from_user_role(state, user_role, user_id).await?
                    }
                }
            }
            None =>
            // no cached context found
            {
                Self::resolve_lineage_from_user_role(state, user_role, user_id).await?
            }
        };

        utils::user::spawn_async_lineage_context_update_to_db(
            state,
            user_id,
            new_lineage_context.clone(),
        );

        auth::AuthToken::new_token(
            new_lineage_context.user_id,
            new_lineage_context.merchant_id,
            new_lineage_context.role_id,
            &state.conf,
            new_lineage_context.org_id,
            new_lineage_context.profile_id,
            Some(new_lineage_context.tenant_id),
        )
        .await
        .map(|token| token.into())
    }

    pub async fn resolve_lineage_from_user_role(
        state: &SessionState,
        user_role: &UserRole,
        user_id: &str,
    ) -> UserResult<LineageContext> {
        let org_id = utils::user_role::get_single_org_id(state, user_role).await?;
        let merchant_id =
            utils::user_role::get_single_merchant_id(state, user_role, &org_id).await?;
        let profile_id =
            utils::user_role::get_single_profile_id(state, user_role, &merchant_id).await?;

        Ok(LineageContext {
            user_id: user_id.to_string(),
            org_id,
            merchant_id,
            profile_id,
            role_id: user_role.role_id.clone(),
            tenant_id: user_role.tenant_id.clone(),
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    #[serde(rename = "sign_in_with_sso")]
    SignInWithSSO,
    SignIn,
    SignUp,
    MagicLink,
    VerifyEmail,
    AcceptInvitationFromEmail,
    ResetPassword,
}

impl Origin {
    fn get_flows(&self) -> &'static [UserFlow] {
        match self {
            Self::SignInWithSSO => &SIGNIN_WITH_SSO_FLOW,
            Self::SignIn => &SIGNIN_FLOW,
            Self::SignUp => &SIGNUP_FLOW,
            Self::VerifyEmail => &VERIFY_EMAIL_FLOW,
            Self::MagicLink => &MAGIC_LINK_FLOW,
            Self::AcceptInvitationFromEmail => &ACCEPT_INVITATION_FROM_EMAIL_FLOW,
            Self::ResetPassword => &RESET_PASSWORD_FLOW,
        }
    }
}

const SIGNIN_WITH_SSO_FLOW: [UserFlow; 2] = [
    UserFlow::SPTFlow(SPTFlow::MerchantSelect),
    UserFlow::JWTFlow(JWTFlow::UserInfo),
];

const SIGNIN_FLOW: [UserFlow; 4] = [
    UserFlow::SPTFlow(SPTFlow::TOTP),
    UserFlow::SPTFlow(SPTFlow::ForceSetPassword),
    UserFlow::SPTFlow(SPTFlow::MerchantSelect),
    UserFlow::JWTFlow(JWTFlow::UserInfo),
];

const SIGNUP_FLOW: [UserFlow; 4] = [
    UserFlow::SPTFlow(SPTFlow::TOTP),
    UserFlow::SPTFlow(SPTFlow::ForceSetPassword),
    UserFlow::SPTFlow(SPTFlow::MerchantSelect),
    UserFlow::JWTFlow(JWTFlow::UserInfo),
];

const MAGIC_LINK_FLOW: [UserFlow; 5] = [
    UserFlow::SPTFlow(SPTFlow::TOTP),
    UserFlow::SPTFlow(SPTFlow::VerifyEmail),
    UserFlow::SPTFlow(SPTFlow::ForceSetPassword),
    UserFlow::SPTFlow(SPTFlow::MerchantSelect),
    UserFlow::JWTFlow(JWTFlow::UserInfo),
];

const VERIFY_EMAIL_FLOW: [UserFlow; 5] = [
    UserFlow::SPTFlow(SPTFlow::TOTP),
    UserFlow::SPTFlow(SPTFlow::VerifyEmail),
    UserFlow::SPTFlow(SPTFlow::ForceSetPassword),
    UserFlow::SPTFlow(SPTFlow::MerchantSelect),
    UserFlow::JWTFlow(JWTFlow::UserInfo),
];

const ACCEPT_INVITATION_FROM_EMAIL_FLOW: [UserFlow; 6] = [
    UserFlow::SPTFlow(SPTFlow::AuthSelect),
    UserFlow::SPTFlow(SPTFlow::SSO),
    UserFlow::SPTFlow(SPTFlow::TOTP),
    UserFlow::SPTFlow(SPTFlow::AcceptInvitationFromEmail),
    UserFlow::SPTFlow(SPTFlow::ForceSetPassword),
    UserFlow::JWTFlow(JWTFlow::UserInfo),
];

const RESET_PASSWORD_FLOW: [UserFlow; 2] = [
    UserFlow::SPTFlow(SPTFlow::TOTP),
    UserFlow::SPTFlow(SPTFlow::ResetPassword),
];

pub struct CurrentFlow {
    origin: Origin,
    current_flow_index: usize,
    path: Vec<TokenPurpose>,
    tenant_id: Option<id_type::TenantId>,
}

impl CurrentFlow {
    pub fn new(
        token: auth::UserFromSinglePurposeToken,
        current_flow: UserFlow,
    ) -> UserResult<Self> {
        let flows = token.origin.get_flows();
        let index = flows
            .iter()
            .position(|flow| flow == &current_flow)
            .ok_or(UserErrors::InternalServerError)?;
        let mut path = token.path;
        path.push(current_flow.into());

        Ok(Self {
            origin: token.origin,
            current_flow_index: index,
            path,
            tenant_id: token.tenant_id,
        })
    }

    pub async fn next(self, user: UserFromStorage, state: &SessionState) -> UserResult<NextFlow> {
        let flows = self.origin.get_flows();
        let remaining_flows = flows.iter().skip(self.current_flow_index + 1);

        for flow in remaining_flows {
            if flow
                .is_required(
                    &user,
                    &self.path,
                    state,
                    self.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
                )
                .await?
            {
                return Ok(NextFlow {
                    origin: self.origin.clone(),
                    next_flow: *flow,
                    user,
                    path: self.path,
                    tenant_id: self.tenant_id,
                });
            }
        }
        Err(UserErrors::InternalServerError.into())
    }
}

pub struct NextFlow {
    origin: Origin,
    next_flow: UserFlow,
    user: UserFromStorage,
    path: Vec<TokenPurpose>,
    tenant_id: Option<id_type::TenantId>,
}

impl NextFlow {
    pub async fn from_origin(
        origin: Origin,
        user: UserFromStorage,
        state: &SessionState,
    ) -> UserResult<Self> {
        let flows = origin.get_flows();
        let path = vec![];
        for flow in flows {
            if flow
                .is_required(&user, &path, state, &state.tenant.tenant_id)
                .await?
            {
                return Ok(Self {
                    origin,
                    next_flow: *flow,
                    user,
                    path,
                    tenant_id: Some(state.tenant.tenant_id.clone()),
                });
            }
        }
        Err(UserErrors::InternalServerError.into())
    }

    pub fn get_flow(&self) -> UserFlow {
        self.next_flow
    }

    pub async fn get_token(&self, state: &SessionState) -> UserResult<Secret<String>> {
        match self.next_flow {
            UserFlow::SPTFlow(spt_flow) => spt_flow.generate_spt(state, self).await,
            UserFlow::JWTFlow(jwt_flow) => {
                #[cfg(feature = "email")]
                {
                    self.user.get_verification_days_left(state)?;
                }
                let user_role = state
                    .global_store
                    .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                        user_id: self.user.get_user_id(),
                        tenant_id: self.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
                        org_id: None,
                        merchant_id: None,
                        profile_id: None,
                        entity_id: None,
                        version: None,
                        status: Some(UserStatus::Active),
                        limit: Some(1),
                    })
                    .await
                    .change_context(UserErrors::InternalServerError)?
                    .pop()
                    .ok_or(UserErrors::InternalServerError)?;
                utils::user_role::set_role_info_in_cache_by_user_role(state, &user_role).await;

                jwt_flow.generate_jwt(state, self, &user_role).await
            }
        }
    }

    pub async fn get_token_with_user_role(
        &self,
        state: &SessionState,
        user_role: &UserRole,
    ) -> UserResult<Secret<String>> {
        match self.next_flow {
            UserFlow::SPTFlow(spt_flow) => spt_flow.generate_spt(state, self).await,
            UserFlow::JWTFlow(jwt_flow) => {
                #[cfg(feature = "email")]
                {
                    self.user.get_verification_days_left(state)?;
                }
                utils::user_role::set_role_info_in_cache_by_user_role(state, user_role).await;

                jwt_flow.generate_jwt(state, self, user_role).await
            }
        }
    }

    pub async fn skip(self, user: UserFromStorage, state: &SessionState) -> UserResult<Self> {
        let flows = self.origin.get_flows();
        let index = flows
            .iter()
            .position(|flow| flow == &self.get_flow())
            .ok_or(UserErrors::InternalServerError)?;
        let remaining_flows = flows.iter().skip(index + 1);
        for flow in remaining_flows {
            if flow
                .is_required(&user, &self.path, state, &state.tenant.tenant_id)
                .await?
            {
                return Ok(Self {
                    origin: self.origin.clone(),
                    next_flow: *flow,
                    user,
                    path: self.path,
                    tenant_id: Some(state.tenant.tenant_id.clone()),
                });
            }
        }
        Err(UserErrors::InternalServerError.into())
    }
}

impl From<UserFlow> for TokenPurpose {
    fn from(value: UserFlow) -> Self {
        match value {
            UserFlow::SPTFlow(flow) => flow.into(),
            UserFlow::JWTFlow(flow) => flow.into(),
        }
    }
}

impl From<SPTFlow> for TokenPurpose {
    fn from(value: SPTFlow) -> Self {
        match value {
            SPTFlow::AuthSelect => Self::AuthSelect,
            SPTFlow::SSO => Self::SSO,
            SPTFlow::TOTP => Self::TOTP,
            SPTFlow::VerifyEmail => Self::VerifyEmail,
            SPTFlow::AcceptInvitationFromEmail => Self::AcceptInvitationFromEmail,
            SPTFlow::MerchantSelect => Self::AcceptInvite,
            SPTFlow::ResetPassword => Self::ResetPassword,
            SPTFlow::ForceSetPassword => Self::ForceSetPassword,
        }
    }
}

impl From<JWTFlow> for TokenPurpose {
    fn from(value: JWTFlow) -> Self {
        match value {
            JWTFlow::UserInfo => Self::UserInfo,
        }
    }
}

impl From<SPTFlow> for UserFlow {
    fn from(value: SPTFlow) -> Self {
        Self::SPTFlow(value)
    }
}

impl From<JWTFlow> for UserFlow {
    fn from(value: JWTFlow) -> Self {
        Self::JWTFlow(value)
    }
}
