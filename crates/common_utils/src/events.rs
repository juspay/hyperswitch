use common_enums::{PaymentMethod, PaymentMethodType};
use serde::Serialize;

pub trait ApiEventMetric {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "flow_type", rename_all = "snake_case")]
pub enum ApiEventsType {
    Payout,
    Payment {
        payment_id: String,
    },
    Refund {
        payment_id: Option<String>,
        refund_id: String,
    },
    PaymentMethod {
        payment_method_id: String,
        payment_method: Option<PaymentMethod>,
        payment_method_type: Option<PaymentMethodType>,
    },
    Customer {
        customer_id: String,
    },
    User {
        //specified merchant_id will overridden on global defined
        merchant_id: String,
        user_id: String,
    },
    PaymentMethodList {
        payment_id: Option<String>,
    },
    Webhooks {
        connector: String,
        payment_id: Option<String>,
    },
    Routing,
    ResourceListAPI,
    PaymentRedirectionResponse {
        connector: Option<String>,
        payment_id: Option<String>,
    },
    Gsm,
    // TODO: This has to be removed once the corresponding apiEventTypes are created
    Miscellaneous,
    RustLocker,
    FraudCheck,
    Recon,
    Dispute {
        dispute_id: String,
    },
}

impl ApiEventMetric for serde_json::Value {}
impl ApiEventMetric for () {}

impl<Q: ApiEventMetric, E> ApiEventMetric for Result<Q, E> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        match self {
            Ok(q) => q.get_api_event_type(),
            Err(_) => None,
        }
    }
}

// TODO: Ideally all these types should be replaced by newtype responses
impl<T> ApiEventMetric for Vec<T> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[macro_export]
macro_rules! impl_misc_api_event_type {
    ($($type:ty),+) => {
        $(
            impl ApiEventMetric for $type {
                fn get_api_event_type(&self) -> Option<ApiEventsType> {
                    Some(ApiEventsType::Miscellaneous)
                }
            }
        )+
     };
}

impl_misc_api_event_type!(
    String,
    (&String, &String),
    (Option<i64>, Option<i64>, String),
    bool
);

impl<T: ApiEventMetric> ApiEventMetric for &T {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        T::get_api_event_type(self)
    }
}
