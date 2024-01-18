mod core;
pub mod events;

pub trait OutgoingWebhookEventAnalytics: events::OutgoingWebhookLogsFilterAnalytics {}

pub use self::core::outgoing_webhook_events_core;
