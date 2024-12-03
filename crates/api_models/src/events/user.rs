use common_utils::events::{ApiEventMetric, ApiEventsType};
#[cfg(feature = "recon")]
use masking::PeekInterface;

#[cfg(feature = "dummy_connector")]
use crate::user::sample_data::SampleDataRequest;
#[cfg(feature = "recon")]
use crate::user::VerifyTokenResponse;
use crate::user::{
    dashboard_metadata::{
        GetMetaDataRequest, GetMetaDataResponse, GetMultipleMetaDataPayload, SetMetaDataRequest,
    },
    AcceptInviteFromEmailRequest, AuthSelectRequest, AuthorizeResponse, BeginTotpResponse,
    ChangePasswordRequest, ConnectAccountRequest, CreateInternalUserRequest, CreateTenantRequest,
    CreateUserAuthenticationMethodRequest, ForgotPasswordRequest, GetSsoAuthUrlRequest,
    GetUserAuthenticationMethodsRequest, GetUserDetailsResponse, GetUserRoleDetailsRequest,
    GetUserRoleDetailsResponseV2, InviteUserRequest, ReInviteUserRequest, RecoveryCodes,
    ResetPasswordRequest, RotatePasswordRequest, SendVerifyEmailRequest, SignUpRequest,
    SignUpWithMerchantIdRequest, SsoSignInRequest, SwitchMerchantRequest,
    SwitchOrganizationRequest, SwitchProfileRequest, TokenResponse, TwoFactorAuthStatusResponse,
    TwoFactorStatus, UpdateUserAccountDetailsRequest, UpdateUserAuthenticationMethodRequest,
    UserFromEmailRequest, UserMerchantCreate, UserOrgMerchantCreateRequest, VerifyEmailRequest,
    VerifyRecoveryCodeRequest, VerifyTotpRequest,
};

#[cfg(feature = "recon")]
impl ApiEventMetric for VerifyTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            user_id: self.user_email.peek().to_string(),
        })
    }
}

common_utils::impl_api_event_type!(
    Miscellaneous,
    (
        SignUpRequest,
        SignUpWithMerchantIdRequest,
        ChangePasswordRequest,
        GetMultipleMetaDataPayload,
        GetMetaDataResponse,
        GetMetaDataRequest,
        SetMetaDataRequest,
        SwitchOrganizationRequest,
        SwitchMerchantRequest,
        SwitchProfileRequest,
        CreateInternalUserRequest,
        CreateTenantRequest,
        UserOrgMerchantCreateRequest,
        UserMerchantCreate,
        AuthorizeResponse,
        ConnectAccountRequest,
        ForgotPasswordRequest,
        ResetPasswordRequest,
        RotatePasswordRequest,
        InviteUserRequest,
        ReInviteUserRequest,
        VerifyEmailRequest,
        SendVerifyEmailRequest,
        AcceptInviteFromEmailRequest,
        UpdateUserAccountDetailsRequest,
        GetUserDetailsResponse,
        GetUserRoleDetailsRequest,
        GetUserRoleDetailsResponseV2,
        TokenResponse,
        TwoFactorAuthStatusResponse,
        TwoFactorStatus,
        UserFromEmailRequest,
        BeginTotpResponse,
        VerifyRecoveryCodeRequest,
        VerifyTotpRequest,
        RecoveryCodes,
        GetUserAuthenticationMethodsRequest,
        CreateUserAuthenticationMethodRequest,
        UpdateUserAuthenticationMethodRequest,
        GetSsoAuthUrlRequest,
        SsoSignInRequest,
        AuthSelectRequest
    )
);

#[cfg(feature = "dummy_connector")]
common_utils::impl_api_event_type!(Miscellaneous, (SampleDataRequest));
