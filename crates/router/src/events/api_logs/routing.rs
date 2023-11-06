use api_models::routing::{
    LinkedRoutingConfigRetrieveResponse, MerchantRoutingAlgorithm, RoutableConnectorChoice,
    RoutingAlgorithmId, RoutingConfigRequest, RoutingDictionaryRecord, RoutingKind,
};
#[cfg(feature = "business_profile_routing")]
use api_models::routing::{RoutingRetrieveLinkQuery, RoutingRetrieveQuery};

use super::{ApiEventMetric, ApiEventsType};

impl ApiEventMetric for RoutingKind {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Routing)
    }
}

impl ApiEventMetric for MerchantRoutingAlgorithm {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Routing)
    }
}

impl ApiEventMetric for RoutingAlgorithmId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Routing)
    }
}

impl ApiEventMetric for RoutingDictionaryRecord {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Routing)
    }
}

impl ApiEventMetric for Vec<RoutableConnectorChoice> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Routing)
    }
}

impl ApiEventMetric for LinkedRoutingConfigRetrieveResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Routing)
    }
}

#[cfg(feature = "business_profile_routing")]
impl ApiEventMetric for RoutingRetrieveQuery {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Routing)
    }
}

impl ApiEventMetric for RoutingConfigRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Routing)
    }
}

#[cfg(feature = "business_profile_routing")]
impl ApiEventMetric for RoutingRetrieveLinkQuery {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Routing)
    }
}
