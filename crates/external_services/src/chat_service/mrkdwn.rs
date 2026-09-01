//! Slack's `mrkdwn`, the markup Slack-compatible backends render message text as.
//!
//! Deliberately thin: composing a *message* — which alerts to show, in what order, how many —
//! belongs to whoever is deciding what to say. What belongs here is the part that is a property of
//! the wire format, above all [`escape`]: an unescaped `<` in a merchant id or an error reason is
//! read as the start of markup and silently mangles the message.

/// Escape text so `mrkdwn` renders it literally.
///
/// Slack reserves exactly three characters in message text. `&` is replaced first, or the
/// ampersands introduced by the other two replacements would be escaped again.
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render `text` in bold.
pub fn bold(text: &str) -> String {
    format!("*{text}*")
}

/// Render `text` as inline code.
///
/// Backticks disable `mrkdwn` inside the span, so the content is not escaped — but a backtick in
/// the content would end the span early, and is stripped for that reason.
pub fn code(text: &str) -> String {
    format!("`{}`", text.replace('`', ""))
}

/// Render a link with its own label.
pub fn link(url: &str, label: &str) -> String {
    format!("<{}|{}>", escape(url), escape(label))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn escape_handles_the_three_reserved_characters() {
        assert_eq!(escape("a < b > c & d"), "a &lt; b &gt; c &amp; d");
    }

    #[test]
    fn escape_does_not_double_escape_its_own_ampersands() {
        assert_eq!(escape("<"), "&lt;");
        assert_eq!(escape("&lt;"), "&amp;lt;");
    }

    #[test]
    fn code_cannot_be_ended_early_by_its_content() {
        assert_eq!(code("a`b"), "`ab`");
    }

    #[test]
    fn link_escapes_both_halves() {
        assert_eq!(
            link("https://example.com/?a=1&b=2", "a & b"),
            "<https://example.com/?a=1&amp;b=2|a &amp; b>"
        );
    }

    #[test]
    fn bold_wraps_in_asterisks() {
        assert_eq!(bold("urgent"), "*urgent*");
    }
}
