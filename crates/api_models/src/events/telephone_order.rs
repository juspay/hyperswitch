use common_utils::events::ApiEventMetric;

use crate::telephone_order::twilio::{TwilioGenericPayRequest, TwilioGenericPayResponse};

impl ApiEventMetric for TwilioGenericPayRequest {}

#[cfg(feature = "v1")]
impl ApiEventMetric for TwilioGenericPayResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        // `charge_id` is the Hyperswitch `payment_id`, so a successful charge is reported as an
        // ordinary payment event. A decline has no payment to point at.
        self.charge_id
            .clone()
            .map(|payment_id| common_utils::events::ApiEventsType::Payment { payment_id })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for TwilioGenericPayResponse {}
