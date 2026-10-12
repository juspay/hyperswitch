use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::external_service_auth::{
    ExternalSignoutTokenRequest, ExternalTokenResponse, ExternalVerifyTokenRequest,
    ExternalVerifyTokenResponse, ValidateTokenRequest,
};

const HYPERSENSE: &str = "hypersense";

impl ApiEventMetric for ExternalTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceAuth {
            service: HYPERSENSE.to_string(),
        })
    }
}

impl ApiEventMetric for ExternalVerifyTokenRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceAuth {
            service: HYPERSENSE.to_string(),
        })
    }
}

impl ApiEventMetric for ExternalVerifyTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        let service = match self {
            Self::Hypersense { .. } => HYPERSENSE.to_string(),
            Self::OfferEngine { .. } => {
                crate::external_service_auth::ValidatingService::OfferEngine.to_string()
            }
        };

        Some(ApiEventsType::ExternalServiceAuth { service })
    }
}

impl ApiEventMetric for ExternalSignoutTokenRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceAuth {
            service: HYPERSENSE.to_string(),
        })
    }
}

impl ApiEventMetric for ValidateTokenRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceAuth {
            service: self.service.to_string(),
        })
    }
}
