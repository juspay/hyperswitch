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
    AuthorizeResponse, ChangePasswordRequest, ConnectAccountRequest, CreateInternalUserRequest,
    DashboardEntryResponse, ForgotPasswordRequest, GetUsersResponse, InviteUserRequest,
    InviteUserResponse, ResetPasswordRequest, SendVerifyEmailRequest, SignInResponse,
    SignUpRequest, SignUpWithMerchantIdRequest, SwitchMerchantIdRequest,
    UpdateUserAccountDetailsRequest, UserMerchantCreate, VerifyEmailRequest,
};

impl ApiEventMetric for DashboardEntryResponse {
        /// Returns the API event type as an Option. If the API event type is User, it includes the merchant ID and user ID associated with the event.
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            merchant_id: self.merchant_id.clone(),
            user_id: self.user_id.clone(),
        })
    }
}

#[cfg(feature = "recon")]
impl ApiEventMetric for VerifyTokenResponse {
        /// This method returns the API event type for the current instance. It creates an `ApiEventsType::User` variant with the merchant ID and user ID obtained from the current instance, and returns it wrapped in an `Option`.
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
    GetUsersResponse,
    AuthorizeResponse,
    ConnectAccountRequest,
    ForgotPasswordRequest,
    ResetPasswordRequest,
    InviteUserRequest,
    InviteUserResponse,
    VerifyEmailRequest,
    SendVerifyEmailRequest,
    SignInResponse,
    UpdateUserAccountDetailsRequest
);

#[cfg(feature = "dummy_connector")]
common_utils::impl_misc_api_event_type!(SampleDataRequest);
