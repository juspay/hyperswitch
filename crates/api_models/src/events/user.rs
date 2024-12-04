use common_utils::events::{ApiEventMetric, ApiEventsType};

#[cfg(feature = "dummy_connector")]
use crate::user::sample_data::SampleDataRequest;
#[cfg(feature = "theme")]
use crate::user::theme::{
    CreateThemeRequest, GetThemeResponse, UpdateThemeRequest, UploadFileRequest,
};
use crate::user::{
    dashboard_metadata::{
        GetMetaDataRequest, GetMetaDataResponse, GetMultipleMetaDataPayload, SetMetaDataRequest,
    },
    AcceptInviteFromEmailRequest, AuthSelectRequest, AuthorizeResponse, BeginTotpResponse,
    ChangePasswordRequest, ConnectAccountRequest, CreateInternalUserRequest,
    CreateUserAuthenticationMethodRequest, ForgotPasswordRequest, GetSsoAuthUrlRequest,
    GetUserAuthenticationMethodsRequest, GetUserDetailsResponse, GetUserRoleDetailsRequest,
    GetUserRoleDetailsResponseV2, InviteUserRequest, ReInviteUserRequest, RecoveryCodes,
    ResetPasswordRequest, RotatePasswordRequest, SendVerifyEmailRequest, SignUpRequest,
    SignUpWithMerchantIdRequest, SsoSignInRequest, SwitchMerchantRequest,
    SwitchOrganizationRequest, SwitchProfileRequest, TokenResponse, TwoFactorAuthStatusResponse,
    TwoFactorStatus, UpdateUserAccountDetailsRequest, UpdateUserAuthenticationMethodRequest,
    UserFromEmailRequest, UserMerchantCreate, VerifyEmailRequest, VerifyRecoveryCodeRequest,
    VerifyTotpRequest,
};

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

#[cfg(feature = "themes")]
common_utils::impl_api_event_type!(
    Miscellaneous,
    (
        GetThemeResponse,
        UploadFileRequest,
        CreateThemeRequest,
        UpdateThemeRequest
    )
);

#[cfg(feature = "dummy_connector")]
common_utils::impl_api_event_type!(Miscellaneous, (SampleDataRequest));
