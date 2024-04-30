use common_enums::{PermissionGroup, RoleScope};
use common_utils::{crypto::OptionalEncryptableName, pii};
use masking::Secret;

use crate::user_role::UserStatus;
pub mod dashboard_metadata;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;

#[derive(serde::Deserialize, Debug, Clone, serde::Serialize)]
pub struct SignUpWithMerchantIdRequest {
    pub name: Secret<String>,
    pub email: pii::Email,
    pub password: Secret<String>,
    pub company_name: String,
}

pub type SignUpWithMerchantIdResponse = AuthorizeResponse;

#[derive(serde::Deserialize, Debug, Clone, serde::Serialize)]
pub struct SignUpRequest {
    pub email: pii::Email,
    pub password: Secret<String>,
}

pub type SignUpResponse = DashboardEntryResponse;

#[derive(serde::Serialize, Debug, Clone)]
pub struct DashboardEntryResponse {
    pub token: Secret<String>,
    pub merchant_id: String,
    pub name: Secret<String>,
    pub email: pii::Email,
    pub verification_days_left: Option<i64>,
    pub user_role: String,
    //this field is added for audit/debug reasons
    #[serde(skip_serializing)]
    pub user_id: String,
}

pub type SignInRequest = SignUpRequest;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "flow_type", rename_all = "snake_case")]
pub enum SignInResponse {
    MerchantSelect(MerchantSelectResponse),
    DashboardEntry(DashboardEntryResponse),
}

#[derive(Debug, serde::Serialize)]
pub struct MerchantSelectResponse {
    pub token: Secret<String>,
    pub name: Secret<String>,
    pub email: pii::Email,
    pub verification_days_left: Option<i64>,
    pub merchants: Vec<UserMerchantAccount>,
}

#[derive(serde::Deserialize, Debug, Clone, serde::Serialize)]
pub struct ConnectAccountRequest {
    pub email: pii::Email,
}

pub type ConnectAccountResponse = AuthorizeResponse;

#[derive(serde::Serialize, Debug, Clone)]
pub struct AuthorizeResponse {
    pub is_email_sent: bool,
    //this field is added for audit/debug reasons
    #[serde(skip_serializing)]
    pub user_id: String,
    //this field is added for audit/debug reasons
    #[serde(skip_serializing)]
    pub merchant_id: String,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct ChangePasswordRequest {
    pub new_password: Secret<String>,
    pub old_password: Secret<String>,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct ForgotPasswordRequest {
    pub email: pii::Email,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct ResetPasswordRequest {
    pub token: Secret<String>,
    pub password: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct InviteUserRequest {
    pub email: pii::Email,
    pub name: Secret<String>,
    pub role_id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct InviteMultipleUserResponse {
    pub email: pii::Email,
    pub is_email_sent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ReInviteUserRequest {
    pub email: pii::Email,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct AcceptInviteFromEmailRequest {
    pub token: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SwitchMerchantIdRequest {
    pub merchant_id: String,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct CreateInternalUserRequest {
    pub name: Secret<String>,
    pub email: pii::Email,
    pub password: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserMerchantCreate {
    pub company_name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ListUsersResponse(pub Vec<UserDetails>);

#[derive(Debug, serde::Serialize)]
pub struct UserDetails {
    pub email: pii::Email,
    pub name: Secret<String>,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: time::PrimitiveDateTime,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct GetUserDetailsResponse {
    pub merchant_id: String,
    pub name: Secret<String>,
    pub email: pii::Email,
    pub verification_days_left: Option<i64>,
    pub role_id: String,
    //this field is added for audit/debug reasons
    #[serde(skip_serializing)]
    pub user_id: String,
    pub org_id: String,
}
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetUserRoleDetailsRequest {
    pub email: pii::Email,
}

#[derive(Debug, serde::Serialize)]
pub struct GetUserRoleDetailsResponse {
    pub email: pii::Email,
    pub name: Secret<String>,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: time::PrimitiveDateTime,
    pub groups: Vec<PermissionGroup>,
    pub role_scope: RoleScope,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VerifyEmailRequest {
    pub token: Secret<String>,
}

pub type VerifyEmailResponse = SignInResponse;

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct SendVerifyEmailRequest {
    pub email: pii::Email,
}

#[derive(Debug, serde::Serialize)]
pub struct UserMerchantAccount {
    pub merchant_id: String,
    pub merchant_name: OptionalEncryptableName,
    pub is_active: bool,
    pub role_id: String,
    pub role_name: String,
    pub org_id: String,
}

#[cfg(feature = "recon")]
#[derive(serde::Serialize, Debug)]
pub struct VerifyTokenResponse {
    pub merchant_id: String,
    pub user_email: pii::Email,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateUserAccountDetailsRequest {
    pub name: Option<Secret<String>>,
    pub preferred_merchant_id: Option<String>,
}
