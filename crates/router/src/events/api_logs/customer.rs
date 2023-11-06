use api_models::{
    customers::{CustomerDeleteResponse, CustomerId, CustomerRequest, CustomerResponse},
    payment_methods::PaymentMethodListRequest,
};

use super::{ApiEventMetric, ApiEventsType};

impl ApiEventMetric for CustomerDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone(),
        })
    }
}

impl ApiEventMetric for CustomerRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone(),
        })
    }
}

impl ApiEventMetric for CustomerResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone(),
        })
    }
}

impl ApiEventMetric for CustomerId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone(),
        })
    }
}

impl ApiEventMetric for (PaymentMethodListRequest, CustomerId) {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.1.get_api_event_type()
    }
}
