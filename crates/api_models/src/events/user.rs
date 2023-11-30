use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user::{
    dashboard_metadata::{
        GetMetaDataRequest, GetMetaDataResponse, GetMultipleMetaDataPayload, SetMetaDataRequest,
    },
    ChangePasswordRequest, ConnectAccountRequest, ConnectAccountResponse,
    CreateInternalUserRequest, SwitchMerchantIdRequest, UserMerchantCreate,
};

impl ApiEventMetric for ConnectAccountResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            merchant_id: self.merchant_id.clone(),
            user_id: self.user_id.clone(),
        })
    }
}

impl ApiEventMetric for ConnectAccountRequest {}

common_utils::impl_misc_api_event_type!(
    ChangePasswordRequest,
    GetMultipleMetaDataPayload,
    GetMetaDataResponse,
    GetMetaDataRequest,
    SetMetaDataRequest,
    SwitchMerchantIdRequest,
    CreateInternalUserRequest,
    UserMerchantCreate
);
