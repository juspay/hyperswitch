use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::telephone_order::twilio::{TwilioGenericPayRequest, TwilioGenericPayResponse};

#[cfg(feature = "v1")]
impl ApiEventMetric for TwilioGenericPayRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        // Only an explicit `parameters.payment_id` — the derived one is the core's. Reported all
        // the same, so a request rejected before the core still lands as a telephone order.
        Some(ApiEventsType::TelephoneOrder {
            payment_id: self
                .parameters
                .as_ref()
                .and_then(|parameters| parameters.payment_id.clone()),
            payment_method_id: None,
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for TwilioGenericPayRequest {}

#[cfg(feature = "v1")]
impl ApiEventMetric for TwilioGenericPayResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        // Read the response, so the ids reach the audit trail on a decline as well as on a success.
        Some(ApiEventsType::TelephoneOrder {
            payment_id: self.payment_id(),
            payment_method_id: self.payment_method_id(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for TwilioGenericPayResponse {}
