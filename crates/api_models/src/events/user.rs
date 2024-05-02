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
    AcceptInviteFromEmailRequest, AuthorizeResponse, ChangePasswordRequest, ConnectAccountRequest,
    CreateInternalUserRequest, DashboardEntryResponse, ForgotPasswordRequest,
    GetUserDetailsResponse, GetUserRoleDetailsRequest, GetUserRoleDetailsResponse,
    InviteUserRequest, ListUsersResponse, ReInviteUserRequest, ResetPasswordRequest,
    SendVerifyEmailRequest, SignInResponse, SignInWithTokenResponse, SignUpRequest,
    SignUpWithMerchantIdRequest, SwitchMerchantIdRequest, UpdateUserAccountDetailsRequest,
    UserMerchantCreate, VerifyEmailRequest,
};

impl ApiEventMetric for DashboardEntryResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            merchant_id: self.merchant_id.clone(),
            user_id: self.user_id.clone(),
        })
    }
}

#[cfg(feature = "recon")]
impl ApiEventMetric for VerifyTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            merchant_id: self.merchant_id.clone(),
            user_id: self.user_email.peek().to_string(),
        })
    }
}

common_utils::impl_misc_api_event_type!(
    SignUpRequest,
    SignUpWithMerchantIdRequest,
    ChangePasswordRequest,
    GetMultipleMetaDataPayload,
    GetMetaDataResponse,
    GetMetaDataRequest,
    SetMetaDataRequest,
    SwitchMerchantIdRequest,
    CreateInternalUserRequest,
    UserMerchantCreate,
    ListUsersResponse,
    AuthorizeResponse,
    ConnectAccountRequest,
    ForgotPasswordRequest,
    ResetPasswordRequest,
    InviteUserRequest,
    ReInviteUserRequest,
    VerifyEmailRequest,
    SendVerifyEmailRequest,
    AcceptInviteFromEmailRequest,
    SignInResponse,
    UpdateUserAccountDetailsRequest,
    GetUserDetailsResponse,
    SignInWithTokenResponse,
    GetUserRoleDetailsRequest,
    GetUserRoleDetailsResponse
);

#[cfg(feature = "dummy_connector")]
common_utils::impl_misc_api_event_type!(SampleDataRequest);
