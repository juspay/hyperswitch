use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::customers::CustomerId;
#[cfg(all(feature = "v2", feature = "customer_v2"))]
use crate::customers::GlobalId;
use crate::customers::{
    CustomerDeleteResponse, CustomerRequest, CustomerResponse, CustomerUpdateRequest,
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
            id: self.id.clone(),
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
        Some(ApiEventsType::Customer {
            id: "temp_id".to_string(),
        })
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
            id: self.id.clone(),
        })
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl ApiEventMetric for CustomerId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.get_merchant_reference_id().clone(),
        })
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl ApiEventMetric for GlobalId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            id: self.id.clone(),
        })
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl ApiEventMetric for CustomerId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            id: self
                .get_merchant_reference_id()
                .get_string_repr()
                .to_string(),
        })
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
impl ApiEventMetric for CustomerUpdateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.get_merchant_reference_id()
            .clone()
            .map(|cid| ApiEventsType::Customer { customer_id: cid })
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
impl ApiEventMetric for CustomerUpdateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            id: "temo_id".to_string(),
        })
    }
}
// These needs to be fixed for v2
