use serde::Serialize;

pub mod event_logger;

pub trait EventHandler: Sync + Send + dyn_clone::DynClone {
    fn log_event<T: Event>(&self, event: T, previous: Option<T>);
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    PaymentIntent,
    PaymentAttempt,
    Refund,
    ApiLogs,
}

pub trait Event
where
    Self: Serialize,
{
    fn event_type() -> EventType;

    fn key(&self) -> String;
}
