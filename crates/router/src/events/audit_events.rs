use serde::Serialize;

use crate::services::kafka::KafkaMessage;

#[derive(Debug, Clone, Serialize)]
pub enum AuditEventType {
    Error,
    PaymentCreated,
    ConnectorDecided,
    ConnectorCalled,
    PaymentSuccess,
    PaymentFail,
    RefundCreated,
    RefundSuccess,
    RefundFail,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    payment_id: String,
    event_type: AuditEventType,
    metadata: serde_json::Value,
    created_at: time::PrimitiveDateTime,
}

impl KafkaMessage for AuditEvent {
    fn key(&self) -> String {
        format!(
            "audit_event_{}_{}",
            self.payment_id,
            self.created_at.assume_utc().unix_timestamp_nanos()
        )
    }

    fn event_type(&self) -> super::EventType {
        super::EventType::AuditEvent
    }
}
