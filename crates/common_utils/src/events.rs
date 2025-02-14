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
    #[cfg(feature = "v1")]
    Payment {
        payment_id: id_type::PaymentId,
    },
    #[cfg(feature = "v2")]
    Payment {
        payment_id: id_type::GlobalPaymentId,
    },
    #[cfg(feature = "v1")]
    Refund {
        payment_id: Option<id_type::PaymentId>,
        refund_id: String,
    },
    #[cfg(feature = "v2")]
    Refund {
        payment_id: id_type::GlobalPaymentId,
        refund_id: id_type::GlobalRefundId,
    },
    #[cfg(feature = "v1")]
    PaymentMethod {
        payment_method_id: String,
        payment_method: Option<common_enums::PaymentMethod>,
        payment_method_type: Option<common_enums::PaymentMethodType>,
    },
    #[cfg(feature = "v2")]
    PaymentMethod {
        payment_method_id: id_type::GlobalPaymentMethodId,
        payment_method_type: Option<common_enums::PaymentMethod>,
        payment_method_subtype: Option<common_enums::PaymentMethodType>,
    },
    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    PaymentMethodCreate,
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    Customer {
        customer_id: Option<id_type::GlobalCustomerId>,
    },
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    Customer {
        customer_id: id_type::CustomerId,
    },
    BusinessProfile {
        profile_id: id_type::ProfileId,
    },
    ApiKey {
        key_id: id_type::ApiKeyId,
    },
    User {
        user_id: String,
    },
    PaymentMethodList {
        payment_id: Option<String>,
    },
    #[cfg(feature = "v2")]
    PaymentMethodListForPaymentMethods {
        payment_method_id: id_type::GlobalPaymentMethodId,
    },
    #[cfg(feature = "v1")]
    Webhooks {
        connector: String,
        payment_id: Option<id_type::PaymentId>,
    },
    #[cfg(feature = "v2")]
    Webhooks {
        connector: id_type::MerchantConnectorAccountId,
        payment_id: Option<id_type::GlobalPaymentId>,
    },
    Routing,
    ResourceListAPI,
    #[cfg(feature = "v1")]
    PaymentRedirectionResponse {
        connector: Option<String>,
        payment_id: Option<id_type::PaymentId>,
    },
    #[cfg(feature = "v2")]
    PaymentRedirectionResponse {
        payment_id: id_type::GlobalPaymentId,
    },
    Gsm,
    // TODO: This has to be removed once the corresponding apiEventTypes are created
    Miscellaneous,
    Keymanager,
    RustLocker,
    ApplePayCertificatesMigration,
    FraudCheck,
    Recon,
    ExternalServiceAuth,
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
    #[cfg(feature = "v2")]
    ClientSecret {
        key_id: id_type::ClientSecretId,
    },
    #[cfg(feature = "v2")]
    PaymentMethodSession {
        payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
    },
}

impl ApiEventMetric for serde_json::Value {}
impl ApiEventMetric for () {}

#[cfg(feature = "v1")]
impl ApiEventMetric for id_type::PaymentId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for id_type::GlobalPaymentId {
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
