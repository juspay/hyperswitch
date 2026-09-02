//! Notification: receiving alert data and delivering it to a destination channel.
//!
//! **A `Notifier` is a pure sink.** It is told what to say; it does not decide, and it does not
//! mutate state that another sink also reads. The `hyperswitch-alerts` R service learned this the
//! hard way: when deciding and delivering lived in the same step, the first sink consumed the
//! shared lifecycle store and every later sink went permanently silent. `R/alerts/announce.R`
//! exists because of that bug. Any implementation here that reaches for shared delivery state
//! reintroduces it.
//!
//! ## Two traits, not one
//!
//! [`chat::ChatNotifier`] and [`email::EmailNotifier`] are separate traits with separate request
//! types, because chat and email are not the same operation wearing different clothes. Chat
//! threads and hands back a message id; email has a subject and does neither. A single trait over
//! both would need a union input, which means every implementation carrying a match arm for a
//! variant it can never serve, growing by one with each channel added.
//!
//! They exist at all — rather than `alerts` calling `external_services::chat_service::ChatClient`
//! straight — because that client knows nothing about alerts and should not have to. These are
//! this crate's own seam, and alert-shaped behaviour that has nowhere else to go can grow here
//! without reaching into a crate the router also depends on.
//!
//! ## Where an implementation lives
//!
//! Here, in `alerts`. `external_services` deliberately carries no dependency on this crate
//! (hyperswitch-cloud#23108), so an adapter that knows both sides has to sit on this side of the
//! line.

pub mod chat;
pub mod email;

use std::{collections::HashMap, sync::Arc};

/// The destinations of one channel, resolved at boot and looked up per request.
///
/// A destination id is the only thing a request names. Channel ids, base URLs and credentials stay
/// in configuration, so a caller cannot address a channel that has not been configured for it and
/// no credential ever travels on the wire.
///
/// Built once at startup rather than per request: a chat destination holds a validated endpoint,
/// and re-reading configuration on every alert would move a boot-time failure into the delivery
/// path.
#[derive(Debug)]
pub struct Registry<T: ?Sized> {
    destinations: HashMap<String, Arc<T>>,
}

impl<T: ?Sized> Registry<T> {
    /// Build a registry from resolved destinations.
    pub fn new(destinations: HashMap<String, Arc<T>>) -> Self {
        Self { destinations }
    }

    /// The destination configured under `id`, if there is one.
    pub fn get(&self, id: &str) -> Option<&Arc<T>> {
        self.destinations.get(id)
    }

    /// How many destinations are configured.
    pub fn len(&self) -> usize {
        self.destinations.len()
    }

    /// Whether nothing is configured for this channel.
    ///
    /// A legitimate state, not an error: a first deployment has no destinations until credentials
    /// exist, and refusing to boot would mean the service could not be deployed before them.
    /// [`crate::state::AppState::new`] warns instead.
    pub fn is_empty(&self) -> bool {
        self.destinations.is_empty()
    }
}

impl<T: ?Sized> Default for Registry<T> {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}
