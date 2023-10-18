use serde::Serialize;

pub mod api_logs;
pub mod event_logger;

pub trait EventHandler: Sync + Send + dyn_clone::DynClone {
    fn log_event(&self, event: RawEvent);
}

dyn_clone::clone_trait_object!(EventHandler);

pub struct RawEvent {
    event_type: EventType,
    key: String,
    payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    PaymentIntent,
    PaymentAttempt,
    Refund,
    ApiLogs,
}
