use std::fmt::Debug;

use common_enums::{EntityType, TokenPurpose};
use common_utils::{crypto::OptionalEncryptableName, id_type, pii};
use masking::Secret;

use crate::user_role::UserStatus;
pub mod dashboard_metadata;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;
#[cfg(feature = "control_center_theme")]
pub mod theme;

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

pub type SignInRequest = SignUpRequest;

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

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct RotatePasswordRequest {
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
pub struct SwitchOrganizationRequest {
    pub org_id: id_type::OrganizationId,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SwitchMerchantRequest {
    pub merchant_id: id_type::MerchantId,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SwitchProfileRequest {
    pub profile_id: id_type::ProfileId,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct CreateInternalUserRequest {
    pub name: Secret<String>,
    pub email: pii::Email,
    pub password: Secret<String>,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct CreateTenantUserRequest {
    pub name: Secret<String>,
    pub email: pii::Email,
    pub password: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct UserOrgMerchantCreateRequest {
    pub organization_name: Secret<String>,
    pub organization_details: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub merchant_name: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserMerchantCreate {
    pub company_name: String,
    pub product_type: Option<common_enums::MerchantProductType>,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct GetUserDetailsResponse {
    pub merchant_id: id_type::MerchantId,
    pub name: Secret<String>,
    pub email: pii::Email,
    pub verification_days_left: Option<i64>,
    pub role_id: String,
    // This field is added for audit/debug reasons
    #[serde(skip_serializing)]
    pub user_id: String,
    pub org_id: id_type::OrganizationId,
    pub is_two_factor_auth_setup: bool,
    pub recovery_codes_left: Option<usize>,
    pub profile_id: id_type::ProfileId,
    pub entity_type: EntityType,
    pub theme_id: Option<String>,
    pub version: common_enums::ApiVersion,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetUserRoleDetailsRequest {
    pub email: pii::Email,
}

#[derive(Debug, serde::Serialize)]
pub struct GetUserRoleDetailsResponseV2 {
    pub role_id: String,
    pub org: NameIdUnit<Option<String>, id_type::OrganizationId>,
    pub merchant: Option<NameIdUnit<OptionalEncryptableName, id_type::MerchantId>>,
    pub profile: Option<NameIdUnit<String, id_type::ProfileId>>,
    pub status: UserStatus,
    pub entity_type: EntityType,
    pub role_name: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct NameIdUnit<N: Debug + Clone, I: Debug + Clone> {
    pub name: N,
    pub id: I,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VerifyEmailRequest {
    pub token: Secret<String>,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct SendVerifyEmailRequest {
    pub email: pii::Email,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateUserAccountDetailsRequest {
    pub name: Option<Secret<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SkipTwoFactorAuthQueryParam {
    pub skip_two_factor_auth: Option<bool>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TokenResponse {
    pub token: Secret<String>,
    pub token_type: TokenPurpose,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TwoFactorAuthStatusResponse {
    pub totp: bool,
    pub recovery_code: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TwoFactorAuthAttempts {
    pub is_completed: bool,
    pub remaining_attempts: u8,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TwoFactorAuthStatusResponseWithAttempts {
    pub totp: TwoFactorAuthAttempts,
    pub recovery_code: TwoFactorAuthAttempts,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TwoFactorStatus {
    pub status: Option<TwoFactorAuthStatusResponseWithAttempts>,
    pub is_skippable: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserFromEmailRequest {
    pub token: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BeginTotpResponse {
    pub secret: Option<TotpSecret>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TotpSecret {
    pub secret: Secret<String>,
    pub totp_url: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VerifyTotpRequest {
    pub totp: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VerifyRecoveryCodeRequest {
    pub recovery_code: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RecoveryCodes {
    pub recovery_codes: Vec<Secret<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "auth_type")]
#[serde(rename_all = "snake_case")]
pub enum AuthConfig {
    OpenIdConnect {
        private_config: OpenIdConnectPrivateConfig,
        public_config: OpenIdConnectPublicConfig,
    },
    MagicLink,
    Password,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct OpenIdConnectPrivateConfig {
    pub base_url: String,
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub private_key: Option<Secret<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct OpenIdConnectPublicConfig {
    pub name: OpenIdProvider,
}

#[derive(
    Debug, serde::Deserialize, serde::Serialize, Copy, Clone, strum::Display, Eq, PartialEq,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OpenIdProvider {
    Okta,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OpenIdConnect {
    pub name: OpenIdProvider,
    pub base_url: String,
    pub client_id: String,
    pub client_secret: Secret<String>,
    pub private_key: Option<Secret<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CreateUserAuthenticationMethodRequest {
    pub owner_id: String,
    pub owner_type: common_enums::Owner,
    pub auth_method: AuthConfig,
    pub allow_signup: bool,
    pub email_domain: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateUserAuthenticationMethodRequest {
    AuthMethod {
        id: String,
        auth_config: AuthConfig,
    },
    EmailDomain {
        owner_id: String,
        email_domain: String,
    },
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetUserAuthenticationMethodsRequest {
    pub auth_id: Option<String>,
    pub email_domain: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserAuthenticationMethodResponse {
    pub id: String,
    pub auth_id: String,
    pub auth_method: AuthMethodDetails,
    pub allow_signup: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AuthMethodDetails {
    #[serde(rename = "type")]
    pub auth_type: common_enums::UserAuthType,
    pub name: Option<OpenIdProvider>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetSsoAuthUrlRequest {
    pub id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SsoSignInRequest {
    pub state: Secret<String>,
    pub code: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AuthIdAndThemeIdQueryParam {
    pub auth_id: Option<String>,
    pub theme_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AuthSelectRequest {
    pub id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UserKeyTransferRequest {
    pub from: u32,
    pub limit: u32,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserTransferKeyResponse {
    pub total_transferred: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct ListOrgsForUserResponse {
    pub org_id: id_type::OrganizationId,
    pub org_name: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct UserMerchantAccountResponse {
    pub merchant_id: id_type::MerchantId,
    pub merchant_name: OptionalEncryptableName,
    pub product_type: Option<common_enums::MerchantProductType>,
    pub version: common_enums::ApiVersion,
}

#[derive(Debug, serde::Serialize)]
pub struct ListProfilesForUserInOrgAndMerchantAccountResponse {
    pub profile_id: id_type::ProfileId,
    pub profile_name: String,
}
