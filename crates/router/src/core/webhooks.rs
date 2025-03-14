#[cfg(feature = "v1")]
mod incoming;
#[cfg(feature = "v2")]
mod incoming_v2;
#[cfg(feature = "v1")]
mod outgoing;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
mod recovery_incoming;
pub mod types;
pub mod utils;
#[cfg(feature = "olap")]
pub mod webhook_events;

#[cfg(feature = "v2")]
pub(crate) use self::incoming_v2::incoming_webhooks_wrapper;
#[cfg(feature = "v1")]
pub(crate) use self::{
    incoming::incoming_webhooks_wrapper,
    outgoing::{
        create_event_and_trigger_outgoing_webhook, get_outgoing_webhook_request,
        trigger_webhook_and_raise_event,
    },
};

const MERCHANT_ID: &str = "merchant_id";
