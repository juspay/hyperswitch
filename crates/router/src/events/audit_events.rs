
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

pub struct AuditEvent {
    payment_id: String,
    event_type: AuditEventType,
    metadata: serde_json::Value,
}

