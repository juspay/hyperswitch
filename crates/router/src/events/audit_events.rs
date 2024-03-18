use std::collections::HashMap;

use data_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use events::{Event, EventInfo};
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize)]
pub enum AuditEventType {
    Error {
        error_message: String,
    },
    PaymentCreated,
    ConnectorDecided,
    ConnectorCalled,
    RefundCreated,
    RefundSuccess,
    RefundFail,
    PaymentUpdate {
        payment_intent: PaymentIntent,
        payment_attempt: PaymentAttempt,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    event_type: AuditEventType,
    created_at: time::PrimitiveDateTime,
}

impl AuditEvent {
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            event_type,
            created_at: common_utils::date_time::now(),
        }
    }
}

impl Event for AuditEvent {
    type EventType = super::EventType;

    fn timestamp(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn identifier(&self) -> String {
        let event_type = match &self.event_type {
            AuditEventType::Error { .. } => "error",
            AuditEventType::PaymentCreated => "payment_created",
            AuditEventType::ConnectorDecided => "connector_decided",
            AuditEventType::ConnectorCalled => "connector_called",
            AuditEventType::RefundCreated => "refund_created",
            AuditEventType::RefundSuccess => "refund_success",
            AuditEventType::RefundFail => "refund_fail",
            AuditEventType::PaymentUpdate { .. } => "payment_update",
        };
        format!("{event_type}-{}", self.timestamp().unix_timestamp_nanos())
    }

    fn class(&self) -> Self::EventType {
        super::EventType::AuditEvent
    }
}

impl EventInfo for AuditEvent {
    fn data(&self) -> error_stack::Result<HashMap<String, serde_json::Value>, events::EventsError> {
        serde_json::to_value(self)
            .map_err(|e| {
                error_stack::report!(events::EventsError::SerializationError(e.to_string()))
            })
            .and_then(|v| match v {
                serde_json::Value::Object(map) => Ok(map),
                _ => Err(error_stack::report!(
                    events::EventsError::SerializationError(
                        "Expected a serialized map".to_string()
                    )
                )),
            })
            .map(|i| i.into_iter().collect())
    }

    fn key(&self) -> String {
        "event".to_string()
    }
}
