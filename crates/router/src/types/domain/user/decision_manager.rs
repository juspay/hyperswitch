use common_enums::TokenPurpose;
use diesel_models::{enums::UserStatus, user_role::UserRole};
use masking::Secret;

use super::UserFromStorage;
use crate::{
    core::errors::{StorageErrorExt, UserErrors, UserResult},
    routes::AppState,
    services::authentication as auth,
};

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum UserFlow {
    SPTFlow(SPTFlow),
    JWTFlow(JWTFlow),
}

impl UserFlow {
    async fn is_required(&self, user: &UserFromStorage, state: &AppState) -> UserResult<bool> {
        match self {
            Self::SPTFlow(flow) => flow.is_required(user, state).await,
            Self::JWTFlow(flow) => flow.is_required(user, state).await,
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
            Self::MerchantSelect => user
                .get_roles_from_db(state)
                .await
                .map(|roles| !roles.iter().any(|role| role.status == UserStatus::Active)),
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
    UserInfo,
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
#[serde(rename_all = "snake_case")]
pub enum Origin {
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
            Self::SignIn => &SIGNIN_FLOW,
            Self::SignUp => &SIGNUP_FLOW,
            Self::VerifyEmail => &VERIFY_EMAIL_FLOW,
            Self::MagicLink => &MAGIC_LINK_FLOW,
            Self::AcceptInvitationFromEmail => &ACCEPT_INVITATION_FROM_EMAIL_FLOW,
            Self::ResetPassword => &RESET_PASSWORD_FLOW,
        }
    }
}

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

const ACCEPT_INVITATION_FROM_EMAIL_FLOW: [UserFlow; 4] = [
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
}

impl CurrentFlow {
    pub fn new(origin: Origin, current_flow: UserFlow) -> UserResult<Self> {
        let flows = origin.get_flows();
        let index = flows
            .iter()
            .position(|flow| flow == &current_flow)
            .ok_or(UserErrors::InternalServerError)?;

        Ok(Self {
            origin,
            current_flow_index: index,
        })
    }

    pub async fn next(&self, user: UserFromStorage, state: &AppState) -> UserResult<NextFlow> {
        let flows = self.origin.get_flows();
        let remaining_flows = flows.iter().skip(self.current_flow_index + 1);
        for flow in remaining_flows {
            if flow.is_required(&user, state).await? {
                return Ok(NextFlow {
                    origin: self.origin.clone(),
                    next_flow: *flow,
                    user,
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
                    next_flow: *flow,
                    user,
                });
            }
        }
        Err(UserErrors::InternalServerError.into())
    }

    pub fn get_flow(&self) -> UserFlow {
        self.next_flow
    }

    pub async fn get_token(&self, state: &AppState) -> UserResult<Secret<String>> {
        match self.next_flow {
            UserFlow::SPTFlow(spt_flow) => spt_flow.generate_spt(state, self).await,
            UserFlow::JWTFlow(jwt_flow) => {
                #[cfg(feature = "email")]
                {
                    self.user.get_verification_days_left(state)?;
                }

                let user_role = self
                    .user
                    .get_preferred_or_active_user_role_from_db(state)
                    .await
                    .to_not_found_response(UserErrors::InternalServerError)?;
                jwt_flow.generate_jwt(state, self, &user_role).await
            }
        }
    }

    pub async fn get_token_with_user_role(
        &self,
        state: &AppState,
        user_role: &UserRole,
    ) -> UserResult<Secret<String>> {
        match self.next_flow {
            UserFlow::SPTFlow(spt_flow) => spt_flow.generate_spt(state, self).await,
            UserFlow::JWTFlow(jwt_flow) => {
                #[cfg(feature = "email")]
                {
                    self.user.get_verification_days_left(state)?;
                }

                jwt_flow.generate_jwt(state, self, &user_role).await
            }
        }
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
            SPTFlow::TOTP => Self::TOTP,
            SPTFlow::VerifyEmail => Self::VerifyEmail,
            SPTFlow::AcceptInvitationFromEmail => Self::AcceptInvitationFromEmail,
            SPTFlow::MerchantSelect => Self::AcceptInvite,
            SPTFlow::ResetPassword | SPTFlow::ForceSetPassword => Self::ResetPassword,
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
