//! Request paths for telemetry. Keep resource IDs while hiding callback tokens.

use std::borrow::Cow;

use actix_web::HttpRequest;

/// Use the concrete path except for a matched callback token. An unmatched path
/// has no trustworthy token position, so its path is omitted from telemetry.
pub fn path_for_telemetry(request: &HttpRequest) -> Cow<'_, str> {
    match request.match_pattern() {
        Some(pattern) if pattern.contains("{callback_token}") => {
            let pattern_segments = pattern.split('/').collect::<Vec<_>>();
            let mut path_segments = request.path().split('/').collect::<Vec<_>>();
            if pattern_segments.len() != path_segments.len() {
                return Cow::Owned(pattern);
            }
            for (segment, pattern_segment) in path_segments.iter_mut().zip(pattern_segments) {
                if pattern_segment.contains("{callback_token}") {
                    *segment = pattern_segment;
                }
            }
            Cow::Owned(path_segments.join("/"))
        }
        Some(_) => Cow::Borrowed(request.path()),
        None => Cow::Borrowed("/unmatched"),
    }
}

#[cfg(test)]
mod tests;
