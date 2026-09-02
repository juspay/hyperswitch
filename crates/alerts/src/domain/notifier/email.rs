//! Delivering an alert to an email destination.
//!
//! ## Reaching `external_services::email`
//!
//! That module's object-safe trait was built for templated product emails:
//! `compose_and_send_email(base_url, Box<dyn EmailData>, proxy_url)`, where `EmailData` renders a
//! template against a base URL. This crate has a subject and a body already in hand, no template,
//! and no base URL.
//!
//! Rather than implement `EmailData` to hand back what it was given — and invent a `base_url` for
//! that impl to ignore — `EmailService` gained [`send_contents`], and composition is now defined in
//! terms of it. The trait was missing an operation; it was not missing a clever caller.
//!
//! That costs existing callers nothing. `EmailService` has exactly one implementor, the blanket
//! `impl<T> EmailService for T where T: EmailClient`, so SES, SMTP and `no_email` gained the method
//! for free and every `compose_and_send_email` call site is untouched.
//!
//! Dropping to the concrete backends instead was the other option, and it loses the SES / SMTP /
//! no-email selection `EmailClientConfigs` gives for free — and with it the ability to turn email
//! off by configuration.
//!
//! [`send_contents`]: EmailService::send_contents
//!
//! ## Email cannot refuse
//!
//! `EmailError` has no refusal vocabulary: a rejected recipient, a throttle and an unverified SES
//! sender all arrive as `EmailSendingFailure`. So this channel only ever produces
//! [`Outcome::Delivered`] or an error, and `status: "refused"` is unreachable for it until that
//! enum grows. The response shape stays uniform across channels; email simply never uses half of
//! it yet.

use std::sync::Arc;

use common_utils::pii;
use external_services::email::{EmailContents, EmailService, IntermediateString};
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
pub trait EmailNotifier: Send + Sync {
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
///
/// Not `Debug`, and neither is [`EmailNotifier`]. Deriving would need `dyn EmailService` to be
/// `Debug`, and the alternative is a hand-written impl whose only job is to skip the one field
/// that cannot be — so the bound is dropped rather than paid for. Nothing formats a notifier.
/// [`ChatNotifier`](super::chat::ChatNotifier) keeps it, because every chat implementation derives
/// it for free.
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

#[async_trait::async_trait]
impl EmailNotifier for EmailServiceNotifier {
    async fn notify(&self, notification: EmailNotification) -> AlertsApiResult<EmailOutcome> {
        self.client
            .send_contents(
                EmailContents {
                    subject: notification.subject.peek().clone(),
                    body: IntermediateString::new(notification.body.peek().clone()),
                    recipient: self.recipient.clone(),
                },
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

    /// `NoEmailClient` accepts and logs, so this exercises the real path to the transport — with no
    /// `EmailData` in it any more — without needing credentials.
    #[tokio::test]
    async fn the_no_email_backend_reports_delivery() {
        let outcome = no_email_notifier()
            .await
            .notify(notification())
            .await
            .unwrap();

        assert_eq!(outcome, Outcome::Delivered(()));
    }

    /// A subject carries merchant ids and a body carries volumes, and neither belongs in a log
    /// line. `Secret` is what enforces that, so `Debug` can be derived everywhere rather than
    /// hand-written and kept in step by hand.
    #[test]
    fn debug_leaks_no_content() {
        let rendered = format!("{:?}", notification());

        assert!(!rendered.contains("merchant_1234"));
        assert!(!rendered.contains("4,201"));
    }

    /// A destination with no address would accept alerts and send them nowhere, so it has to fail
    /// at boot rather than at delivery.
    #[test]
    fn an_absent_recipient_is_not_usable() {
        assert!(is_usable_recipient(&recipient()));
        assert!(!is_usable_recipient(&pii::Email::default()));
    }
}
