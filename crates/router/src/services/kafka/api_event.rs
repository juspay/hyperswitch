use api_models::enums as api_enums;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "flow_type")]
pub enum ApiEventsType {
    Payment {
        payment_id: String,
    },
    Refund {
        payment_id: String,
        refund_id: String,
    },
    Default,
    PaymentMethod {
        payment_method_id: String,
        payment_method: Option<api_enums::PaymentMethod>,
        payment_method_type: Option<api_enums::PaymentMethodType>,
    },
    Customer {
        customer_id: String,
    },
    User {
        //specified merchant_id will overridden on global defined
        merchant_id: String,
        user_id: String,
    },
    Webhooks {
        connector: String,
        payment_id: Option<String>,
    },
    OutgoingEvent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApiEvents {
    pub api_name: String,
    pub request_id: Option<String>,
    //It is require to solve ambiquity in case of event_type is User
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_id: Option<String>,
    pub request: String,
    pub response: String,
    pub status_code: u16,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    pub latency: u128,
    //conflicting fields underlying enums will be used
    #[serde(flatten)]
    pub event_type: ApiEventsType,
    pub user_agent: Option<String>,
    pub ip_addr: Option<String>,
    pub url_path: Option<String>,
    pub api_event_type: Option<ApiCallEventType>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ApiCallEventType {
    IncomingApiEvent,
    OutgoingApiEvent,
}

impl super::KafkaMessage for ApiEvents {
    fn key(&self) -> String {
        match &self.event_type {
            ApiEventsType::Payment { payment_id } => format!(
                "{}_{}",
                self.merchant_id
                    .as_ref()
                    .unwrap_or(&"default_merchant_id".to_string()),
                payment_id
            ),
            ApiEventsType::Refund {
                payment_id,
                refund_id,
            } => format!("{payment_id}_{refund_id}"),
            ApiEventsType::Default => "key".to_string(),
            ApiEventsType::PaymentMethod {
                payment_method_id,
                payment_method,
                payment_method_type,
            } => format!(
                "{:?}_{:?}_{:?}",
                payment_method_id.clone(),
                payment_method.clone(),
                payment_method_type.clone(),
            ),
            ApiEventsType::Customer { customer_id } => customer_id.to_string(),
            ApiEventsType::User {
                merchant_id,
                user_id,
            } => format!("{}_{}", merchant_id, user_id),
            ApiEventsType::Webhooks {
                connector,
                payment_id,
            } => format!(
                "webhook_{}_{connector}",
                payment_id.clone().unwrap_or_default()
            ),
            ApiEventsType::OutgoingEvent => "outgoing_event".to_string(),
        }
    }

    fn creation_timestamp(&self) -> Option<i64> {
        Some(self.created_at.unix_timestamp())
    }
}
