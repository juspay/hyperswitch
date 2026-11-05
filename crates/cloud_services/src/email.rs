//! Email delivery clients.

#[cfg(feature = "ses")]
pub mod ses;
#[cfg(feature = "smtp")]
pub mod smtp;

/// One email to send. Addresses are plain strings; the client sending it checks them.
#[derive(Clone, Debug)]
pub struct EmailMessage {
    /// The sender's address.
    pub from: String,
    /// Recipients.
    pub to: Vec<String>,
    /// Copied recipients.
    pub cc: Vec<String>,
    /// Blind-copied recipients.
    pub bcc: Vec<String>,
    /// The subject line.
    pub subject: String,
    /// The HTML body.
    pub html_body: String,
}
