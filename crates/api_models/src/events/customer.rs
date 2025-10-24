use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::customers::{
    CustomerDeleteResponse, CustomerListRequestWithConstraints, CustomerListResponse,
    CustomerRequest, CustomerResponse, CustomerUpdateRequestInternal,
};

#[cfg(feature = "v1")]
impl ApiEventMetric for CustomerDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for CustomerDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: Some(self.id.clone()),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for CustomerRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.get_merchant_reference_id()
            .clone()
            .map(|cid| ApiEventsType::Customer { customer_id: cid })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for CustomerRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer { customer_id: None })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for CustomerResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.get_merchant_reference_id()
            .clone()
            .map(|cid| ApiEventsType::Customer { customer_id: cid })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for CustomerResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: Some(self.id.clone()),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for CustomerUpdateRequestInternal {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for CustomerUpdateRequestInternal {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: Some(self.id.clone()),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for CustomerListRequestWithConstraints {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer {
            customer_id: self.customer_id.clone()?,
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for CustomerListRequestWithConstraints {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Customer { customer_id: None })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for CustomerListResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for CustomerListResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}
