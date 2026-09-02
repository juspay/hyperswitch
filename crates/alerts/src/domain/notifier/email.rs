//! Delivering an alert to an email destination.
//!
//! ## Reaching `external_services::email`
//!
//! That module's object-safe trait exposes exactly one method, and it is built for templated
//! product emails: `compose_and_send_email(base_url, Box<dyn EmailData>, proxy_url)`. `EmailData`
//! is the composition hook, where an implementation renders a template against a base URL. This
//! crate has a subject and a body already in hand, no template, and no meaningful base URL.
//!
//! Three ways to bridge that were open, and the choice is less free than it looks:
//!
//! - **Implement `EmailData` and ignore `base_url`.** What [`AlertEmail`] does. Nothing outside
//!   this crate changes.
//! - **Add a plain-send method to `EmailService`.** Honest, and smaller here — but it widens a
//!   trait `router` depends on, to suit one caller.
//! - **Depend on the concrete backends.** Loses the SES / SMTP / no-email selection that
//!   `EmailClientConfigs` gives for free, and with it the ability to turn email off by config.
//!
//! The first is the only one whose cost stays inside this crate, so a dummy `base_url` is the
//! price. It is one ignored parameter in one function, and it buys not touching a shared trait.
//!
//! ## Email cannot refuse
//!
//! `EmailError` has no refusal vocabulary: a rejected recipient, a throttle and an unverified SES
//! sender all arrive as `EmailSendingFailure`. So this channel only ever produces
//! [`Outcome::Delivered`] or an error, and `status: "refused"` is unreachable for it until that
//! enum grows. The response shape stays uniform across channels; email simply never uses half of
//! it yet.

use std::sync::Arc;

use common_utils::{errors::CustomResult, pii};
use external_services::email::{
    EmailContents, EmailData, EmailError, EmailService, IntermediateString,
};
use hyperswitch_masking::{PeekInterface, Secret};

use super::Outcome;
use crate::errors::{AlertsApiResult, AlertsError};

/// A message to send to one email destination.
#[derive(Debug, Clone)]
pub struct EmailNotification {
    /// The subject line, delivered unchanged.
    pub subject: Secret<String>,

    /// The body, as HTML. Both backends hardcode an HTML body — `email/ses.rs` builds it with
    /// `.html(...)`, `email/smtp.rs` sets `ContentType::TEXT_HTML` — so there is nothing else to
    /// send until that module grows a plain-text path.
    pub body: Secret<String>,
}

/// The result of one email delivery attempt.
///
/// Nothing accompanies a success: there is no message id to hand back and nothing to thread under,
/// so the outcome carries the whole answer.
pub type EmailOutcome = Outcome<()>;

/// Sends an alert to one email destination.
///
/// One implementation is bound to one recipient. A destination that should reach several people is
/// several destinations today, because `EmailClient::send_email` takes a single `pii::Email` and
/// both backends build a single-recipient message. When that is lifted, a destination widens to a
/// recipient list without the wire contract changing, since a request only ever names an id.
#[async_trait::async_trait]
pub trait EmailNotifier: Send + Sync + std::fmt::Debug {
    /// Attempt delivery.
    ///
    /// A provider that refuses returns `Ok(Outcome::Refused)`, not an error. Email cannot reach
    /// that arm today — see the module docs.
    async fn notify(&self, notification: EmailNotification) -> AlertsApiResult<EmailOutcome>;
}

/// An [`EmailNotifier`] backed by the shared `external_services` email client.
///
/// The client is shared across destinations and the recipient is not, which is the whole shape of
/// the type: one transport, many addresses.
#[derive(Clone)]
pub struct EmailServiceNotifier {
    destination: String,
    client: Arc<Box<dyn EmailService>>,
    recipient: pii::Email,
    proxy_url: Option<String>,
}

impl EmailServiceNotifier {
    /// Bind the shared client to one destination's recipient.
    pub fn new(
        destination: String,
        client: Arc<Box<dyn EmailService>>,
        recipient: pii::Email,
        proxy_url: Option<String>,
    ) -> Self {
        Self {
            destination,
            client,
            recipient,
            proxy_url,
        }
    }
}

// Written out rather than derived, because `Box<dyn EmailService>` is not `Debug`. It names only
// the destination: the recipient masks itself, but a field nobody prints cannot be peeked into a
// log line by accident either.
impl std::fmt::Debug for EmailServiceNotifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailServiceNotifier")
            .field("destination", &self.destination)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl EmailNotifier for EmailServiceNotifier {
    async fn notify(&self, notification: EmailNotification) -> AlertsApiResult<EmailOutcome> {
        self.client
            .compose_and_send_email(
                // No template renders against this, so there is no URL to give. `EmailData` takes
                // one because the router's product emails build links with it.
                "",
                Box::new(AlertEmail {
                    subject: notification.subject,
                    body: notification.body,
                    recipient: self.recipient.clone(),
                }),
                self.proxy_url.as_ref(),
            )
            .await
            .map_err(|report| {
                report.change_context(AlertsError::ProviderUnavailable {
                    destination: self.destination.clone(),
                })
            })?;

        Ok(Outcome::Delivered(()))
    }
}

/// An alert, in the shape `EmailService` composes from.
///
/// `get_email_data` hands back what it was given. There is no template and nothing to render, which
/// is why `base_url` is ignored — see the module docs for why this trait is the bridge anyway.
///
/// The content stays [`Secret`] right up to the moment it is handed over, so nothing between the
/// route and the transport holds an unwrapped subject or body.
struct AlertEmail {
    subject: Secret<String>,
    body: Secret<String>,
    recipient: pii::Email,
}

#[async_trait::async_trait]
impl EmailData for AlertEmail {
    async fn get_email_data(&self, _base_url: &str) -> CustomResult<EmailContents, EmailError> {
        Ok(EmailContents {
            subject: self.subject.peek().clone(),
            body: IntermediateString::new(self.body.peek().clone()),
            recipient: self.recipient.clone(),
        })
    }
}

/// Whether an address is usable as a recipient.
///
/// `pii::Email` validates the format on the way in, so this only catches the empty default that a
/// `#[serde(default)]` configuration produces when the field is missing entirely — which would
/// otherwise become a destination that accepts alerts and sends them nowhere.
pub fn is_usable_recipient(recipient: &pii::Email) -> bool {
    !recipient.peek().trim().is_empty()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use external_services::email::no_email::NoEmailClient;

    use super::*;

    fn recipient() -> pii::Email {
        serde_json::from_value(serde_json::json!("oncall@example.com")).unwrap()
    }

    async fn no_email_notifier() -> EmailServiceNotifier {
        EmailServiceNotifier::new(
            "oncall".to_owned(),
            Arc::new(Box::new(NoEmailClient::create().await)),
            recipient(),
            None,
        )
    }

    fn notification() -> EmailNotification {
        EmailNotification {
            subject: "merchant_1234 not converting".to_owned().into(),
            body: "<pre>4,201 of 5,000 payments lost</pre>".to_owned().into(),
        }
    }

    /// `NoEmailClient` accepts and logs, so this exercises the whole composition path — the
    /// `EmailData` bridge included — without a transport.
    #[tokio::test]
    async fn the_no_email_backend_reports_delivery() {
        let outcome = no_email_notifier()
            .await
            .notify(notification())
            .await
            .unwrap();

        assert_eq!(outcome, Outcome::Delivered(()));
    }

    /// A subject carries merchant ids and a body carries volumes. Neither belongs in a log line,
    /// and nor does the recipient.
    #[tokio::test]
    async fn debug_leaks_neither_the_content_nor_the_recipient() {
        let rendered = format!("{:?} {:?}", no_email_notifier().await, notification());

        assert!(rendered.contains("oncall"));
        assert!(!rendered.contains("example.com"));
        assert!(!rendered.contains("merchant_1234"));
        assert!(!rendered.contains("4,201"));
    }

    #[tokio::test]
    async fn the_composed_email_carries_the_notification_unchanged() {
        let contents = AlertEmail {
            subject: "subject".to_owned().into(),
            body: "<pre>body</pre>".to_owned().into(),
            recipient: recipient(),
        }
        .get_email_data("")
        .await
        .unwrap();

        assert_eq!(contents.subject, "subject");
        assert_eq!(contents.body.into_inner(), "<pre>body</pre>");
    }

    /// A destination with no address would accept alerts and send them nowhere, so it has to fail
    /// at boot rather than at delivery.
    #[test]
    fn an_absent_recipient_is_not_usable() {
        assert!(is_usable_recipient(&recipient()));
        assert!(!is_usable_recipient(&pii::Email::default()));
    }
}
