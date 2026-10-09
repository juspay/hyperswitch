use common_utils::events::ApiEventMetric;

use crate::twilio_generic_pay::{TwilioGenericPayRequest, TwilioGenericPayResponse};

impl ApiEventMetric for TwilioGenericPayRequest {}

#[cfg(feature = "v1")]
impl ApiEventMetric for TwilioGenericPayResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        // `charge_id` is the Hyperswitch `payment_id`, so a successful charge is reported as an
        // ordinary payment event. A decline has no payment to point at.
        self.charge_id
            .as_ref()
            .and_then(|charge_id| {
                common_utils::id_type::PaymentId::try_from(std::borrow::Cow::Owned(
                    charge_id.clone(),
                ))
                .ok()
            })
            .map(|payment_id| common_utils::events::ApiEventsType::Payment { payment_id })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for TwilioGenericPayResponse {}
