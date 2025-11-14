use common_utils::events::{ApiEventMetric, ApiEventsType};

#[cfg(feature = "dummy_connector")]
use crate::user::sample_data::SampleDataRequest;
#[cfg(feature = "control_center_theme")]
use crate::user::theme::{
    CreateThemeRequest, CreateUserThemeRequest, GetThemeResponse, UpdateThemeRequest,
    UploadFileRequest,
};
use crate::user::{
    dashboard_metadata::{
        GetMetaDataRequest, GetMetaDataResponse, GetMultipleMetaDataPayload, SetMetaDataRequest,
    },
    AcceptInviteFromEmailRequest, AcceptInviteResponse, AuthSelectRequest, AuthorizeResponse,
    BeginTotpResponse, ChangePasswordRequest, CloneConnectorRequest, ConnectAccountRequest,
    CreateInternalUserRequest, CreateTenantUserRequest, CreateUserAuthenticationMethodRequest,
    CreateUserAuthenticationMethodResponse, ForgotPasswordRequest, GetSsoAuthUrlRequest,
    GetUserAuthenticationMethodsRequest, GetUserDetailsResponse, GetUserRoleDetailsRequest,
    GetUserRoleDetailsResponseV2, InviteUserRequest, PlatformAccountCreateRequest,
    PlatformAccountCreateResponse, ReInviteUserRequest, RecoveryCodes, ResetPasswordRequest,
    RotatePasswordRequest, SendVerifyEmailRequest, SignUpRequest, SignUpWithMerchantIdRequest,
    SsoSignInRequest, SwitchMerchantRequest, SwitchOrganizationRequest, SwitchProfileRequest,
    TokenResponse, TwoFactorAuthStatusResponse, TwoFactorStatus, UpdateUserAccountDetailsRequest,
    UpdateUserAuthenticationMethodRequest, UserFromEmailRequest, UserMerchantAccountResponse,
    UserMerchantCreate, UserOrgMerchantCreateRequest, VerifyEmailRequest,
    VerifyRecoveryCodeRequest, VerifyTotpRequest,
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
        CreateTenantUserRequest,
        PlatformAccountCreateRequest,
        PlatformAccountCreateResponse,
        UserOrgMerchantCreateRequest,
        UserMerchantAccountResponse,
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
        AcceptInviteResponse,
        TwoFactorAuthStatusResponse,
        TwoFactorStatus,
        UserFromEmailRequest,
        BeginTotpResponse,
        VerifyRecoveryCodeRequest,
        VerifyTotpRequest,
        RecoveryCodes,
        GetUserAuthenticationMethodsRequest,
        CreateUserAuthenticationMethodRequest,
        CreateUserAuthenticationMethodResponse,
        UpdateUserAuthenticationMethodRequest,
        GetSsoAuthUrlRequest,
        SsoSignInRequest,
        AuthSelectRequest,
        CloneConnectorRequest
    )
);

#[cfg(feature = "control_center_theme")]
common_utils::impl_api_event_type!(
    Miscellaneous,
    (
        GetThemeResponse,
        UploadFileRequest,
        CreateThemeRequest,
        CreateUserThemeRequest,
        UpdateThemeRequest
    )
);

#[cfg(feature = "dummy_connector")]
common_utils::impl_api_event_type!(Miscellaneous, (SampleDataRequest));
