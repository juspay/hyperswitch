use serde::Serialize;

pub mod api_logs;
pub mod event_logger;

pub trait EventHandler: Sync + Send + dyn_clone::DynClone {
    fn log_event(&self, event: RawEvent);
}

dyn_clone::clone_trait_object!(EventHandler);

#[derive(Debug)]
pub struct RawEvent {
    pub event_type: EventType,
    pub key: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    PaymentIntent,
    PaymentAttempt,
    Refund,
    ApiLogs,
}
