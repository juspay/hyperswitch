//! Delivering an alert to an email destination.
//!
//! **Only [`LogEmailNotifier`] exists today, and that is deliberate.** Putting a real
//! implementation here means choosing how to reach `external_services::email`, whose object-safe
//! trait `EmailService` exposes exactly one method, built for templated product emails:
//! `compose_and_send_email(base_url, Box<dyn EmailData>, proxy_url)`. `alerts` has a subject and a
//! body already in hand, no template and no meaningful base URL. That choice — implement
//! `EmailData` and ignore `base_url`, add a plain-send method to a trait `router` also depends on,
//! or drop to the concrete backends — belongs to hyperswitch-cloud#23111, which owns it explicitly
//! and asks for the reasoning in its PR. This ticket fixes the contract; that one fills it in.
//!
//! Chat is not in the same position, which is why it has a real implementation here:
//! `ChatClient::post_message` maps onto [`ChatNotifier`](super::chat::ChatNotifier) one to one,
//! with nothing to decide.

use crate::{errors::AlertsApiResult, logger};

/// A message to send to one email destination.
#[derive(Debug, Clone)]
pub struct EmailNotification {
    /// The subject line, delivered unchanged.
    pub subject: String,

    /// The body, as HTML. Both backends in `external_services::email` hardcode an HTML body, so
    /// there is nothing else to send until hyperswitch-cloud#23160 widens them.
    pub body: String,
}

/// Sends an alert to one email destination.
///
/// One implementation is bound to one recipient. A destination that should reach several people is
/// several destinations today, because `EmailClient::send_email` takes a single `pii::Email` and
/// both backends build a single-recipient message. hyperswitch-cloud#23160 tracks lifting that;
/// when it lands, a destination widens to a recipient list without the wire contract changing,
/// since a request only ever names an id.
#[async_trait::async_trait]
pub trait EmailNotifier: Send + Sync + std::fmt::Debug {
    /// Deliver `notification`.
    ///
    /// Returns nothing on success. There is no message id to hand back and nothing to thread
    /// under, so the status code carries the whole answer.
    async fn notify(&self, notification: EmailNotification) -> AlertsApiResult<()>;
}

/// An [`EmailNotifier`] that delivers nothing and says so.
///
/// The counterpart of [`LogChatNotifier`](super::chat::LogChatNotifier), and for now the only
/// implementation, so `/email/notify` answers with the real contract while delivery waits on
/// hyperswitch-cloud#23111.
///
/// It logs the destination and the sizes, never the subject or the body: a subject carries
/// merchant ids and a body carries payment volumes, and this writes to the same log stream as
/// everything else.
#[derive(Debug)]
pub struct LogEmailNotifier {
    destination: String,
}

impl LogEmailNotifier {
    /// Build a log destination under the id it was configured with.
    pub fn new(destination: String) -> Self {
        Self { destination }
    }
}

#[async_trait::async_trait]
impl EmailNotifier for LogEmailNotifier {
    async fn notify(&self, notification: EmailNotification) -> AlertsApiResult<()> {
        logger::info!(
            tag = "email_notify_skipped",
            destination = %self.destination,
            subject_chars = notification.subject.chars().count(),
            body_chars = notification.body.chars().count(),
            "not delivered: no email transport is wired yet"
        );

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_log_destination_accepts_a_message() {
        LogEmailNotifier::new("oncall".to_owned())
            .notify(EmailNotification {
                subject: "[Hyperswitch] 3 merchants not converting".to_owned(),
                body: "<pre>...</pre>".to_owned(),
            })
            .await
            .unwrap();
    }
}
