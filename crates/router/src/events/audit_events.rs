use data_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use serde::Serialize;

use crate::services::kafka::KafkaMessage;

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
        payment_id: String,
        merchant_id: String,
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

impl KafkaMessage for AuditEvent {
    fn key(&self) -> String {
        format!("{}", self.created_at.assume_utc().unix_timestamp_nanos())
    }

    fn event_type(&self) -> super::EventType {
        super::EventType::AuditEvent
    }
}
