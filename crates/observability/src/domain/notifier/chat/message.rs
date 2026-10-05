//! Laying out an alert for a chat destination.
//!
//! The client in `external_services` sends `text`, with any attachments, as a Slack-compatible
//! message. What an alert looks like is decided here.
//!
//! - **Plain** — no alert: the caller's text is the whole message.
//! - **Alert** — a bold heading built from the alert's domain data as the text, and the caller's
//!   text as the body inside a coloured attachment. The rules match what the R alerts service sends
//!   Xyne directly (`xyne_body_payload`, `alert_heading`, `alert_state_marker`,
//!   `xyne_attachment_color`), so a message looks the same whichever path delivered it.

use external_services::chat_service::{ChatMessage, MessageId};
use hyperswitch_masking::PeekInterface;
use serde_json::{json, Value};

use super::{AlertSeverity, AlertState, ChatAlert};

/// Longest heading a section-style message carries. The heading is one line; anything longer is
/// clipped, as the R direct sink clips it.
const HEADING_MAX_CHARS: usize = 150;

/// What ends a clipped heading.
const CLIP_MARKER: &str = "...";

/// Longest text one section block accepts. An oversized block is rejected rather than trimmed, so
/// a longer body is spread across several blocks.
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

    let blocks: Vec<Value> = sections(text)
        .into_iter()
        .map(|section| {
            json!({
                "type": "section",
                "text": { "type": "mrkdwn", "text": section },
            })
        })
        .collect();

    (
        format!("*{}*", clip(&heading(alert), HEADING_MAX_CHARS)),
        Some(vec![json!({
            "color": colour(alert, resolved),
            "blocks": blocks,
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

    match alert
        .region
        .as_deref()
        .map(str::trim)
        .filter(|region| !region.is_empty())
    {
        Some(region) => format!("{marker} [{region}] {lead} · {title}"),
        None => format!("{marker} {lead} · {title}"),
    }
}

/// The attachment's rail. The severity hexes are the ones the R alerts' report chart uses for the
/// same levels, so a message and its chart agree.
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

/// Spread `body` across as many section blocks as it needs, breaking between lines.
///
/// A single line longer than a block is split mid-line. Nothing is dropped.
fn sections(body: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0;

    for line in body.split_inclusive('\n') {
        for piece in split_chars(line, SECTION_MAX_CHARS) {
            let piece_chars = piece.chars().count();
            if current_chars + piece_chars > SECTION_MAX_CHARS {
                sections.push(std::mem::take(&mut current));
                current_chars = 0;
            }
            current.push_str(&piece);
            current_chars += piece_chars;
        }
    }
    sections.push(current);

    // A block ending on the line break it was split after would render a blank line.
    sections
        .into_iter()
        .map(|section| section.trim_end_matches('\n').to_owned())
        .filter(|section| !section.is_empty())
        .collect()
}

/// `text` in pieces of at most `max_chars` characters.
fn split_chars(text: &str, max_chars: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(max_chars)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

/// Shorten `text` to `max_chars`, ending in [`CLIP_MARKER`].
fn clip(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let keep = max_chars.saturating_sub(CLIP_MARKER.chars().count());
    let mut clipped: String = text.chars().take(keep).collect();
    clipped.push_str(CLIP_MARKER);
    clipped
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

    fn sections_of(shaped: &Value) -> Vec<String> {
        shaped["attachments"][0]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|block| {
                assert_eq!(block["type"], "section");
                assert_eq!(block["text"]["type"], "mrkdwn");
                block["text"]["text"].as_str().unwrap().to_owned()
            })
            .collect()
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

        with_region.region = Some("  ".to_owned());
        assert_eq!(
            shaped("body", Some(&with_region), false)["text"],
            "*🟡 SEV3 · SR drop - connector (15m)*"
        );
    }

    #[test]
    fn a_long_heading_is_clipped_to_one_line() {
        let mut long = alert(AlertState::Firing, AlertSeverity::Sev1);
        long.title = Secret::new("t".repeat(400));

        let text = shaped("body", Some(&long), false)["text"]
            .as_str()
            .unwrap()
            .to_owned();
        let heading = text.trim_matches('*');
        assert_eq!(heading.chars().count(), HEADING_MAX_CHARS);
        assert!(heading.ends_with(CLIP_MARKER));
    }

    #[test]
    fn a_long_body_is_spread_across_sections_between_lines_and_loses_nothing() {
        let line = format!("{}\n", "a".repeat(999));
        let body = line.repeat(7);

        let sections = sections_of(&shaped(
            &body,
            Some(&alert(AlertState::Firing, AlertSeverity::Sev1)),
            false,
        ));

        assert_eq!(sections.len(), 3);
        assert!(sections
            .iter()
            .all(|section| section.chars().count() <= SECTION_MAX_CHARS));
        assert_eq!(sections.join("\n"), body.trim_end_matches('\n'));
    }

    #[test]
    fn a_line_longer_than_a_section_is_split_mid_line() {
        let body = "b".repeat(SECTION_MAX_CHARS + 500);
        let sections = sections_of(&shaped(
            &body,
            Some(&alert(AlertState::Firing, AlertSeverity::Sev1)),
            false,
        ));

        assert_eq!(sections.len(), 2);
        assert_eq!(sections.concat(), body);
    }
}
