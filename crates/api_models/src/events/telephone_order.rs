use common_utils::events::ApiEventMetric;

use crate::telephone_order::twilio::{TwilioGenericPayRequest, TwilioGenericPayResponse};

impl ApiEventMetric for TwilioGenericPayRequest {}

#[cfg(feature = "v1")]
impl ApiEventMetric for TwilioGenericPayResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        // A charge reports its `payment_id`, a tokenize the payment method it stored, a failure
        // neither.
        self.charge_id
            .clone()
            .map(|payment_id| common_utils::events::ApiEventsType::Payment { payment_id })
            .or_else(|| {
                self.token_id.clone().map(|payment_method_id| {
                    common_utils::events::ApiEventsType::PaymentMethod {
                        payment_method_id,
                        payment_method: Some(common_enums::PaymentMethod::Card),
                        payment_method_type: None,
                    }
                })
            })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for TwilioGenericPayResponse {}
