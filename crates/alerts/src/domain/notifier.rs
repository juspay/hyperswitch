//! Notification: delivering an alert to a destination channel.
//!
//! **A `Notifier` is a pure sink.** It is told what to say; it does not decide, and it does not
//! mutate state another sink also reads. The reference alerting service learned this the hard way:
//! when deciding and delivering lived in one step, the first sink consumed the shared lifecycle
//! store and every later sink went permanently silent. Any implementation here that reaches for
//! shared delivery state reintroduces that bug.
//!
//! ## Refusal is an outcome, not an error
//!
//! [`Outcome`] is the reason this module exists in the shape it does. A provider that answers "no"
//! has answered: the notifier reached it, spoke to it, and got a documented reply. That is a
//! successful delivery *attempt* with a negative result, and it is reported through the `Ok` arm.
//!
//! The `Err` arm is reserved for the cases where nothing is known — the provider could not be
//! reached, or replied with something outside its documented envelope — plus our own faults. The
//! split is exactly the one payments draws between a connector declining a transaction and a
//! connector being unreachable.
//!
//! This matters beyond tidiness. Whether a refusal is "our fault" or "the caller's" depends on who
//! owns the destination, and today that is a config file while tomorrow it is a database row a
//! merchant edits. Encoding that ownership in an HTTP status would bake in an answer that changes
//! under us, and a status code is not a thing you can change once a caller depends on it.
//!
//! ## Two traits, not one
//!
//! [`chat::ChatNotifier`] and [`email::EmailNotifier`] have separate request types because chat and
//! email are not one operation in different clothes. Chat threads and returns a message id; email
//! has a subject and does neither. A single trait needs a union input, which gives every
//! implementation a match arm for a channel it can never serve, growing by one per channel.
//!
//! They exist at all — rather than calling `external_services::chat_service::ChatClient` directly
//! — because that client knows nothing about alerts and should not have to.

pub mod chat;
pub mod email;

use std::{collections::HashMap, sync::Arc};

/// What came of one delivery attempt.
///
/// `T` is whatever the channel has to hand back on success: a message id for chat, nothing for
/// email.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T> {
    /// The provider accepted the message.
    Delivered(T),

    /// The provider was reached and refused, for a reason it named.
    Refused(Refusal),
}

/// A provider's refusal, in terms a caller can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// A stable snake_case code — `msg_too_long`, `channel_not_found`, `rate_limited`.
    ///
    /// Never prose, so a caller can branch on it. Not always the exact bytes the provider sent,
    /// because `external_services` folds synonyms on the way in (`is_archived` arrives as
    /// `not_in_channel`); the guarantee is that one condition always yields one code.
    pub code: String,

    /// How long the provider asked us to wait, when it said. Only ever set alongside a
    /// rate-limiting code.
    pub retry_after_seconds: Option<u64>,
}

impl Refusal {
    /// A refusal carrying only a code.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            retry_after_seconds: None,
        }
    }

    /// A refusal that names how long to wait.
    pub fn retry_after(code: impl Into<String>, retry_after_seconds: Option<u64>) -> Self {
        Self {
            code: code.into(),
            retry_after_seconds,
        }
    }
}

/// The destinations of one channel, resolved at boot and looked up per request.
///
/// A destination id is the only thing a request names. Channel ids, recipient addresses and
/// credentials stay in configuration, so a caller cannot address a channel that was not set up for
/// it and no credential ever travels on the wire.
///
/// Built once at startup rather than per request: a chat destination holds a validated endpoint,
/// and re-reading configuration per alert would move a boot-time failure into the delivery path.
///
/// Deliberately not `Debug`: the notifier traits are not either, since requiring it forced a
/// hand-written impl on the one implementation holding a non-`Debug` client, and nothing formats a
/// registry.
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
    /// A legitimate state, not an error: a first deployment has none until credentials exist, and
    /// refusing to boot would make the service undeployable before them.
    pub fn is_empty(&self) -> bool {
        self.destinations.is_empty()
    }
}

impl<T: ?Sized> Default for Registry<T> {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}
