use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::user::{ConnectAccountRequest, ConnectAccountResponse};

impl ApiEventMetric for ConnectAccountResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            merchant_id: self.merchant_id.clone(),
            user_id: self.user_id.clone(),
        })
    }
}

impl ApiEventMetric for ConnectAccountRequest {}
