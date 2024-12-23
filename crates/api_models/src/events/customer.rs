use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::customers::{
    CustomerDeleteResponse, CustomerRequest, CustomerResponse, CustomerUpdateRequestInternal,
};

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl ApiEventMetric for CustomerDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone(),
        })
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl ApiEventMetric for CustomerDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: Some(self.id.clone()),
        })
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl ApiEventMetric for CustomerRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.get_merchant_reference_id()
            .clone()
            .map(|cid| ApiEventsType::Customer { customer_id: cid })
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl ApiEventMetric for CustomerRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer { customer_id: None })
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl ApiEventMetric for CustomerResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.get_merchant_reference_id()
            .clone()
            .map(|cid| ApiEventsType::Customer { customer_id: cid })
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl ApiEventMetric for CustomerResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: Some(self.id.clone()),
        })
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl ApiEventMetric for CustomerUpdateRequestInternal {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone(),
        })
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl ApiEventMetric for CustomerUpdateRequestInternal {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: Some(self.id.clone()),
        })
    }
}
