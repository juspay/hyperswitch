use common_enums::{EventClass, EventType, WebhookDeliveryAttempt};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

/// The constraints to apply when filtering events.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EventListConstraints {
    /// Filter events created after the specified time.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_after: Option<PrimitiveDateTime>,

    /// Filter events created before the specified time.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_before: Option<PrimitiveDateTime>,

    /// Include at most the specified number of events.
    pub limit: Option<i64>,

    /// Include events after the specified offset.
    pub offset: Option<i64>,

    /// Filter all events associated with the specified object identifier (Payment Intent ID,
    /// Refund ID, etc.)
    pub object_id: Option<String>,
}

#[derive(Debug)]
pub enum EventListConstraintsInternal {
    GenericFilter {
        created_after: Option<PrimitiveDateTime>,
        created_before: Option<PrimitiveDateTime>,
        limit: Option<i64>,
        offset: Option<i64>,
    },
    ObjectIdFilter {
        object_id: String,
    },
}

/// The response body for each item when listing events.
#[derive(Debug, Serialize, ToSchema)]
pub struct EventListItemResponse {
    /// The identifier for the Event.
    #[schema(max_length = 64, example = "evt_018e31720d1b7a2b82677d3032cab959")]
    pub event_id: String,

    /// The identifier for the Merchant Account.
    #[schema(max_length = 64, example = "y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_id: String,

    /// The identifier for the Business Profile.
    #[schema(max_length = 64, example = "SqB0zwDGR5wHppWf0bx7GKr1f2")]
    pub profile_id: String,

    /// The identifier for the object (Payment Intent ID, Refund ID, etc.)
    #[schema(max_length = 64, example = "QHrfd5LUDdZaKtAjdJmMu0dMa1")]
    pub object_id: String,

    /// Specifies the type of event, which includes the object and its status.
    pub event_type: EventType,

    /// Specifies the class of event (the type of object: Payment, Refund, etc.)
    pub event_class: EventClass,

    /// Indicates whether the webhook delivery attempt was successful.
    pub is_delivery_successful: bool,

    /// The identifier for the initial delivery attempt. This will be the same as `event_id` for
    /// the initial delivery attempt.
    #[schema(max_length = 64, example = "evt_018e31720d1b7a2b82677d3032cab959")]
    pub initial_attempt_id: String,

    /// Time at which the event was created.
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created: PrimitiveDateTime,
}

/// The response body for retrieving an event.
#[derive(Debug, Serialize, ToSchema)]
pub struct EventRetrieveResponse {
    #[serde(flatten)]
    pub event_information: EventListItemResponse,

    /// The request information (headers and body) sent in the webhook.
    pub request: OutgoingWebhookRequestContent,

    /// The response information (headers, body and status code) received for the webhook sent.
    pub response: OutgoingWebhookResponseContent,

    /// Indicates the type of delivery attempt.
    pub delivery_attempt: Option<WebhookDeliveryAttempt>,
}

/// The request information (headers and body) sent in the webhook.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutgoingWebhookRequestContent {
    /// The request body sent in the webhook.
    #[schema(value_type = String)]
    #[serde(alias = "payload")]
    pub body: Secret<String>,

    /// The request headers sent in the webhook.
    #[schema(
        value_type = Vec<(String, String)>,
        example = json!([["content-type", "application/json"], ["content-length", "1024"]]))
    ]
    pub headers: Vec<(String, Secret<String>)>,
}

/// The response information (headers, body and status code) received for the webhook sent.
#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct OutgoingWebhookResponseContent {
    /// The response body received for the webhook sent.
    #[schema(value_type = String)]
    #[serde(alias = "payload")]
    pub body: Secret<String>,

    /// The response headers received for the webhook sent.
    #[schema(
        value_type = Vec<(String, String)>,
        example = json!([["content-type", "application/json"], ["content-length", "1024"]]))
    ]
    pub headers: Vec<(String, Secret<String>)>,

    /// The HTTP status code for the webhook sent.
    #[schema(example = 200)]
    pub status_code: u16,
}

#[derive(Debug, serde::Serialize)]
pub struct EventListRequestInternal {
    pub merchant_id: String,
    pub constraints: EventListConstraints,
}

impl common_utils::events::ApiEventMetric for EventListRequestInternal {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Events {
            merchant_id: self.merchant_id.clone(),
        })
    }
}
