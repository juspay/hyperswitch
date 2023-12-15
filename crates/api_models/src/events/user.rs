use common_utils::events::{ApiEventMetric, ApiEventsType};

#[cfg(feature = "dummy_connector")]
use crate::user::sample_data::SampleDataRequest;
use crate::user::{
    dashboard_metadata::{
        GetMetaDataRequest, GetMetaDataResponse, GetMultipleMetaDataPayload, SetMetaDataRequest,
    },
    AuthorizeResponse, ChangePasswordRequest, ConnectAccountRequest, CreateInternalUserRequest,
    DashboardEntryResponse, ForgotPasswordRequest, GetUsersResponse, InviteUserRequest,
    InviteUserResponse, ResetPasswordRequest, SendVerifyEmailRequest, SignUpRequest,
    SignUpWithMerchantIdRequest, SwitchMerchantIdRequest, UserMerchantCreate, VerifyEmailRequest,
};

impl ApiEventMetric for DashboardEntryResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            merchant_id: self.merchant_id.clone(),
            user_id: self.user_id.clone(),
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
    GetUsersResponse,
    AuthorizeResponse,
    ConnectAccountRequest,
    ForgotPasswordRequest,
    ResetPasswordRequest,
    InviteUserRequest,
    InviteUserResponse,
    VerifyEmailRequest,
    SendVerifyEmailRequest
);

#[cfg(feature = "dummy_connector")]
common_utils::impl_misc_api_event_type!(SampleDataRequest);
