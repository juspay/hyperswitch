//! Laying out an alert for a chat destination.
//!
//! The client in `external_services` sends `text`, with any attachments, as a Slack-compatible
//! message. What an alert looks like is decided here.
//!
//! - **Plain** — no alert: the caller's text is the whole message.
//! - **Alert** — a bold heading built from the alert's fields as the text, and the caller's text as
//!   the body in one section block of an attachment coloured by severity. Every caller's alerts
//!   look the same.

use external_services::chat_service::{ChatMessage, MessageId};
use hyperswitch_masking::PeekInterface;
use serde_json::{json, Value};

use super::{AlertSeverity, AlertState, ChatAlert};

/// Longest text one section block accepts. The provider refuses a longer block, so a longer body
/// is cut to fit.
const SECTION_MAX_CHARS: usize = 3000;

/// The message for `text`, laid out as an alert when `alert` is given, threaded under `reply_to`
/// when one is given.
///
/// `resolved` keeps the heading `alert` describes and turns the rail green: how an alert's
/// original message is marked once the alert has cleared.
pub(super) fn build(
    text: &str,
    alert: Option<&ChatAlert>,
    resolved: bool,
    reply_to: Option<MessageId>,
) -> ChatMessage {
    let (text, attachments) = content(text, alert, resolved);
    let message = match reply_to {
        Some(message_id) => ChatMessage::reply(text, message_id),
        None => ChatMessage::new(text),
    };
    match attachments {
        Some(attachments) => message.with_attachments(attachments),
        None => message,
    }
}

/// The message's text and attachments.
fn content(text: &str, alert: Option<&ChatAlert>, resolved: bool) -> (String, Option<Vec<Value>>) {
    let Some(alert) = alert else {
        return (text.to_owned(), None);
    };

    let body: String = text.chars().take(SECTION_MAX_CHARS).collect();

    (
        format!("*{}*", heading(alert)),
        Some(vec![json!({
            "color": colour(alert, resolved),
            "blocks": [{
                "type": "section",
                "text": { "type": "mrkdwn", "text": body },
            }],
        })]),
    )
}

/// `🔴 [eu-west-1] SEV1 · SR drop - connector (15m)`.
///
/// Severity belongs to the alert, not to everything said about it afterwards: a still-firing or a
/// resolved message leads with what changed rather than repeating the severity.
fn heading(alert: &ChatAlert) -> String {
    let marker = match (alert.state, alert.severity) {
        (AlertState::Resolved, _) => "🟢",
        (_, AlertSeverity::Sev1) => "🔴",
        (_, AlertSeverity::Sev2) => "🟠",
        (_, AlertSeverity::Sev3) => "🟡",
    };
    let lead = match (alert.state, alert.severity) {
        (AlertState::Persistent, _) => "STILL FIRING",
        (AlertState::Resolved, _) => "RESOLVED",
        (AlertState::Firing, AlertSeverity::Sev1) => "SEV1",
        (AlertState::Firing, AlertSeverity::Sev2) => "SEV2",
        (AlertState::Firing, AlertSeverity::Sev3) => "SEV3",
    };
    let title = alert.title.peek();

    match &alert.region {
        Some(region) => format!("{marker} [{region}] {lead} · {title}"),
        None => format!("{marker} {lead} · {title}"),
    }
}

/// The attachment's rail: red, orange or yellow by severity, green once resolved.
///
/// Resolved is a hex too — Slack's own "good" green — rather than the name `good`. Xyne reads the
/// name, but Slack draws a named colour grey on an attachment carrying blocks.
fn colour(alert: &ChatAlert, resolved: bool) -> &'static str {
    if resolved || alert.state == AlertState::Resolved {
        return "#2eb886";
    }
    match alert.severity {
        AlertSeverity::Sev1 => "#b71c1c",
        AlertSeverity::Sev2 => "#e65100",
        AlertSeverity::Sev3 => "#f9a825",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use hyperswitch_masking::Secret;

    use super::*;

    fn alert(state: AlertState, severity: AlertSeverity) -> ChatAlert {
        ChatAlert {
            state,
            severity,
            title: Secret::new("SR drop - connector (15m)".to_owned()),
            region: None,
        }
    }

    /// The laid-out message as one value, the way the provider receives text and attachments.
    fn shaped(text: &str, alert: Option<&ChatAlert>, resolved: bool) -> Value {
        let (text, attachments) = content(text, alert, resolved);
        json!({ "text": text, "attachments": attachments })
    }

    #[test]
    fn a_plain_message_is_the_text_alone() {
        assert_eq!(
            content("*3 merchants*", None, false),
            ("*3 merchants*".to_owned(), None)
        );
    }

    #[test]
    fn a_firing_alert_is_a_heading_over_a_coloured_body() {
        assert_eq!(
            shaped(
                "SR fell to `42%`",
                Some(&alert(AlertState::Firing, AlertSeverity::Sev1)),
                false,
            ),
            json!({
                "text": "*🔴 SEV1 · SR drop - connector (15m)*",
                "attachments": [{
                    "color": "#b71c1c",
                    "blocks": [{
                        "type": "section",
                        "text": { "type": "mrkdwn", "text": "SR fell to `42%`" }
                    }]
                }]
            })
        );
    }

    #[test]
    fn an_alert_is_built_as_a_threaded_message_with_its_heading() {
        let message = build(
            "body",
            Some(&alert(AlertState::Persistent, AlertSeverity::Sev1)),
            false,
            Some(MessageId::ts("1.1")),
        );

        assert_eq!(
            message.text(),
            "*🔴 STILL FIRING · SR drop - connector (15m)*"
        );
        assert_eq!(message.reply_target(), Some(&MessageId::ts("1.1")));
    }

    #[test]
    fn severity_sets_the_marker_lead_and_rail_while_firing() {
        for (severity, marker, lead, colour) in [
            (AlertSeverity::Sev1, "🔴", "SEV1", "#b71c1c"),
            (AlertSeverity::Sev2, "🟠", "SEV2", "#e65100"),
            (AlertSeverity::Sev3, "🟡", "SEV3", "#f9a825"),
        ] {
            let built = shaped("body", Some(&alert(AlertState::Firing, severity)), false);
            assert_eq!(
                built["text"],
                format!("*{marker} {lead} · SR drop - connector (15m)*")
            );
            assert_eq!(built["attachments"][0]["color"], colour);
        }
    }

    #[test]
    fn a_reminder_keeps_the_severity_rail_and_leads_with_still_firing() {
        let built = shaped(
            "body",
            Some(&alert(AlertState::Persistent, AlertSeverity::Sev2)),
            false,
        );
        assert_eq!(
            built["text"],
            "*🟠 STILL FIRING · SR drop - connector (15m)*"
        );
        assert_eq!(built["attachments"][0]["color"], "#e65100");
    }

    #[test]
    fn a_resolved_message_is_green_whatever_the_severity() {
        let built = shaped(
            "body",
            Some(&alert(AlertState::Resolved, AlertSeverity::Sev1)),
            false,
        );
        assert_eq!(built["text"], "*🟢 RESOLVED · SR drop - connector (15m)*");
        assert_eq!(built["attachments"][0]["color"], "#2eb886");
    }

    /// How an alert's original message is marked once it clears: the heading it was raised with,
    /// on a green rail.
    #[test]
    fn marking_resolved_keeps_the_original_heading_and_turns_the_rail_green() {
        let built = shaped(
            "body",
            Some(&alert(AlertState::Firing, AlertSeverity::Sev1)),
            true,
        );
        assert_eq!(built["text"], "*🔴 SEV1 · SR drop - connector (15m)*");
        assert_eq!(built["attachments"][0]["color"], "#2eb886");
    }

    #[test]
    fn a_region_is_named_in_the_heading() {
        let mut with_region = alert(AlertState::Firing, AlertSeverity::Sev3);
        with_region.region = Some("eu-west-1".to_owned());
        assert_eq!(
            shaped("body", Some(&with_region), false)["text"],
            "*🟡 [eu-west-1] SEV3 · SR drop - connector (15m)*"
        );
    }

    #[test]
    fn a_long_body_is_cut_to_one_section() {
        let body = "b".repeat(SECTION_MAX_CHARS + 500);

        let built = shaped(
            &body,
            Some(&alert(AlertState::Firing, AlertSeverity::Sev1)),
            false,
        );

        let blocks = built["attachments"][0]["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0]["text"]["text"],
            "b".repeat(SECTION_MAX_CHARS).as_str()
        );
    }
}
