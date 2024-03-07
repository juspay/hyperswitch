use data_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use serde::Serialize;

use super::EventsHandler;
use crate::{
    core::{errors::ApiErrorResponse, payments::PaymentData},
    services::kafka::KafkaMessage,
};

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

pub trait AuditEventEmitter {
    fn get_audit_event(&self) -> AuditEventType;

    fn emit_event(self, event_handler: &EventsHandler) -> Self
    where
        Self: Sized,
    {
        event_handler.log_event(&AuditEvent::new(self.get_audit_event()));
        self
    }
}

impl<T: AuditEventEmitter, E: AuditEventEmitter> AuditEventEmitter for Result<T, E> {
    fn get_audit_event(&self) -> AuditEventType {
        match self {
            Ok(i) => i.get_audit_event(),
            Err(e) => e.get_audit_event(),
        }
    }
}

impl<T: AuditEventEmitter + Send + Sync + 'static> AuditEventEmitter for error_stack::Report<T> {
    fn get_audit_event(&self) -> AuditEventType {
        self.current_context().get_audit_event()
    }
}

impl<T> AuditEventEmitter for (T, AuditEventType) {
    fn get_audit_event(&self) -> AuditEventType {
        self.1.clone()
    }
}

impl AuditEventEmitter for ApiErrorResponse {
    fn get_audit_event(&self) -> AuditEventType {
        AuditEventType::Error {
            error_message: format!("{:?} - {}", self.error_type(), self.error_message()),
        }
    }
}

impl<T, F: Clone> AuditEventEmitter for (T, PaymentData<F>) {
    fn get_audit_event(&self) -> AuditEventType {
        self.1.get_audit_event()
    }
}
