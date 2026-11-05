//! Request paths for telemetry. Keep resource IDs while hiding callback tokens.

use std::borrow::Cow;

use actix_web::HttpRequest;

/// Use the concrete path except for a matched callback token. An unmatched path
/// has no trustworthy token position, so its path is omitted from telemetry.
pub fn path_for_telemetry(request: &HttpRequest) -> Cow<'_, str> {
    match request.match_pattern() {
        Some(pattern) if pattern.ends_with("/callback/{callback_token}") => {
            let Some((prefix, _token)) = request.path().rsplit_once('/') else {
                return Cow::Borrowed("/unmatched");
            };
            Cow::Owned(format!("{prefix}/{{callback_token}}"))
        }
        Some(pattern) if pattern.contains("{callback_token}") => Cow::Owned(pattern),
        Some(_) => Cow::Borrowed(request.path()),
        None => Cow::Borrowed("/unmatched"),
    }
}
