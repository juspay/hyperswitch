use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::customers::{
    CustomerDeleteResponse, CustomerId, CustomerRequest, CustomerResponse, CustomerUpdateRequest,
};

impl ApiEventMetric for CustomerDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone(),
        })
    }
}

impl ApiEventMetric for CustomerRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.get_merchant_reference_id()
            .clone()
            .map(|cid| ApiEventsType::Customer { customer_id: cid })
    }
}

impl ApiEventMetric for CustomerResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.get_merchant_reference_id()
            .clone()
            .map(|cid| ApiEventsType::Customer { customer_id: cid })
    }
}

impl ApiEventMetric for CustomerId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.get_merchant_reference_id().clone(),
        })
    }
}

// #[cfg(all(feature = "v2", feature = "customer_v2"))]
// impl ApiEventMetric for CustomerUpdateRequest {
//     fn get_api_event_type(&self) -> Option<ApiEventsType> {
//         self.id
//             .clone()
//             .map(|id| ApiEventsType::Customer { id })
//     }
// }

// #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl ApiEventMetric for CustomerUpdateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.get_merchant_reference_id()
            .clone()
            .map(|cid| ApiEventsType::Customer { customer_id: cid })
    }
}
// These needs to be fixed for v2
