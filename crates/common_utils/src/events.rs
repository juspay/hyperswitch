use common_enums::{PaymentMethod, PaymentMethodType};
use serde::Serialize;

use crate::{id_type, types::TimeRange};

pub trait ApiEventMetric {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "flow_type", rename_all = "snake_case")]
pub enum ApiEventsType {
    Payout {
        payout_id: String,
    },
    Payment {
        payment_id: id_type::PaymentId,
    },
    Refund {
        payment_id: Option<id_type::PaymentId>,
        refund_id: String,
    },
    PaymentMethod {
        payment_method_id: String,
        payment_method: Option<PaymentMethod>,
        payment_method_type: Option<PaymentMethodType>,
    },
    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    PaymentMethodCreate,
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    Customer {
        id: String,
    },
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    Customer {
        customer_id: id_type::CustomerId,
    },
    BusinessProfile {
        profile_id: id_type::ProfileId,
    },
    User {
        user_id: String,
    },
    PaymentMethodList {
        payment_id: Option<String>,
    },
    Webhooks {
        connector: String,
        payment_id: Option<id_type::PaymentId>,
    },
    Routing,
    ResourceListAPI,
    PaymentRedirectionResponse {
        connector: Option<String>,
        payment_id: Option<id_type::PaymentId>,
    },
    Gsm,
    // TODO: This has to be removed once the corresponding apiEventTypes are created
    Miscellaneous,
    Keymanager,
    RustLocker,
    ApplePayCertificatesMigration,
    FraudCheck,
    Recon,
    Dispute {
        dispute_id: String,
    },
    Events {
        merchant_id: id_type::MerchantId,
    },
    PaymentMethodCollectLink {
        link_id: String,
    },
    Poll {
        poll_id: String,
    },
    Analytics,
    PaymentsSessionV2API,
    PaymentMethodListV2API,
}

impl ApiEventMetric for serde_json::Value {}
impl ApiEventMetric for () {}

impl ApiEventMetric for id_type::PaymentId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.clone(),
        })
    }
}

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
macro_rules! impl_api_event_type {
    ($event: ident, ($($type:ty),+))=> {
        $(
            impl ApiEventMetric for $type {
                fn get_api_event_type(&self) -> Option<ApiEventsType> {
                    Some(ApiEventsType::$event)
                }
            }
        )+
     };
}

impl_api_event_type!(
    Miscellaneous,
    (
        String,
        id_type::MerchantId,
        (id_type::MerchantId, String),
        (id_type::MerchantId, &String),
        (&id_type::MerchantId, &String),
        (&String, &String),
        (Option<i64>, Option<i64>, String),
        (Option<i64>, Option<i64>, id_type::MerchantId),
        bool
    )
);

impl<T: ApiEventMetric> ApiEventMetric for &T {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        T::get_api_event_type(self)
    }
}

impl ApiEventMetric for TimeRange {}
