#[cfg(feature = "v1")]
pub mod incoming;
#[cfg(feature = "v2")]
mod incoming_v2;
#[cfg(feature = "v1")]
mod network_tokenization_incoming;
#[cfg(feature = "v1")]
mod outgoing;
#[cfg(feature = "v2")]
mod outgoing_v2;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
pub mod recovery_incoming;
pub mod types;
pub mod utils;
#[cfg(feature = "olap")]
pub mod webhook_events;

#[cfg(feature = "v1")]
pub(crate) use self::{
    incoming::{incoming_webhooks_wrapper, network_token_incoming_webhooks_wrapper},
    outgoing::{
        create_event_and_trigger_outgoing_webhook, get_outgoing_webhook_request,
        trigger_webhook_and_raise_event,
    },
};
#[cfg(feature = "v2")]
pub(crate) use self::{
    incoming_v2::incoming_webhooks_wrapper, outgoing_v2::create_event_and_trigger_outgoing_webhook,
};

const MERCHANT_ID: &str = "merchant_id";
