use common_utils::{
    errors::CustomResult,
    request::{Request, RequestContent},
};
use deja::DejaHook;
use error_stack::ResultExt;
use hyperswitch_interfaces::errors::HttpClientError;
use hyperswitch_masking::{ExposeInterface, Maskable, PeekInterface, Secret};
use reqwest::ResponseBuilderExt;
use serde_json::json;

/// Extension key used to smuggle the captured response body out of the
/// `send_request` function so that the boundary codec can report it without
/// consuming the response stream.
pub(super) struct CapturedResponseBody(pub(super) bytes::Bytes);

pub(super) fn is_active() -> bool {
    deja::global_hook_from_env().is_some_and(|hook| hook.is_active())
}

pub(super) async fn response_with_captured_body(
    response: reqwest::Response,
) -> CustomResult<reqwest::Response, HttpClientError> {
    // Eagerly read the response body so the boundary result extractor can
    // report it without consuming the stream a second time. The body is then
    // cloned: one copy is rebuilt into a new reqwest::Response as a reusable
    // Body, the other is stashed in http::Extensions so `response_result` can
    // read it.
    let status = response.status();
    let headers = response.headers().clone();
    let version = response.version();
    let url = response.url().clone();
    let body_bytes = response
        .bytes()
        .await
        .change_context(HttpClientError::ResponseDecodingFailed)
        .attach_printable("Failed to read response body for boundary capture")?;

    let mut builder = http::Response::builder()
        .status(status)
        .version(version)
        .url(url);
    for (key, value) in &headers {
        builder = builder.header(key, value);
    }
    let mut http_response = builder.body(body_bytes.clone()).map_err(|_| {
        error_stack::report!(HttpClientError::UnexpectedState)
            .attach_printable("Failed to rebuild HTTP response")
    })?;
    http_response
        .extensions_mut()
        .insert(CapturedResponseBody(body_bytes));

    Ok(reqwest::Response::from(http_response))
}

pub(super) fn request_id(request: &Request) -> Option<String> {
    request.headers.iter().find_map(|(key, value)| {
        if key.eq_ignore_ascii_case(common_utils::consts::X_REQUEST_ID) {
            Some(header_value(value))
        } else {
            None
        }
    })
}

pub(super) fn request_args(
    request: &Request,
    timeout_secs: Option<u64>,
) -> Secret<serde_json::Value> {
    // Header storage iterates in non-deterministic (HashMap) order, so the raw
    // sequence differs between record and replay even when the header SET is
    // identical. The args matcher compares serialized JSON arrays
    // order-sensitively, so an unsorted list misses the lookup and the outgoing
    // call falls through to a LIVE network request. Sort by (key, value) to
    // produce a canonical, byte-stable representation. (Computed outside the
    // json! macro, which cannot parse a block containing type annotations.)
    let mut headers: Vec<(String, String)> = request
        .headers
        .iter()
        .map(|(key, value)| (key.to_string(), header_value(value)))
        .collect();
    headers.sort();
    let headers: Vec<serde_json::Value> = headers
        .into_iter()
        .map(|(key, value)| json!({ "key": key, "value": value }))
        .collect();
    Secret::new(json!({
        "method": format!("{:?}", request.method),
        "url": request.url.as_str(),
        "request_id": request_id(request),
        "headers": headers,
        "query_params": request.query_params.clone(),
        "timeout_secs": timeout_secs,
        "request_body": request.body.as_ref().map(request_body),
        "client_tls": {
            "certificate": request.certificate.is_some(),
            "certificate_key": request.certificate_key.is_some(),
            "ca_certificate": request.ca_certificate.is_some(),
        },
    }))
}

// Captured payloads travel as `Secret<serde_json::Value>` between helpers so
// their `Debug` output is redacted (response bodies/headers can carry PII or
// credentials); they are `.expose()`d only at the deja handoff, where the tape
// keeps full fidelity.
fn captured_body_json(response: &reqwest::Response) -> Secret<serde_json::Value> {
    Secret::new(match response.extensions().get::<CapturedResponseBody>() {
        Some(CapturedResponseBody(bytes)) => deja::http::body(bytes),
        None => deja::http::missing_body("response body not captured (missing extension)"),
    })
}

/// Captures response headers, keeping every value under a repeated name.
///
/// This used to `insert` one value per name, so a response carrying three
/// `set-cookie` headers recorded one. The loss only bites on replay, and not at
/// this boundary: record builds the key-manager payload from the live response
/// and replay builds it from the reconstructed one, so the two payloads differ
/// in entry count and diverge downstream at `km` — twelve of the fifty-seven
/// value divergences on the 42-tape sweep.
///
/// Order across distinct names is deliberately not recorded. The comparison
/// sorts arrays before comparing (`bag_canon`), so a permutation of the same
/// values is already absorbed; only a change in what was recorded can diverge.
fn response_headers_json(response: &reqwest::Response) -> Secret<serde_json::Value> {
    // A non-UTF-8 header value has no JSON representation; drop the one entry
    // rather than failing the whole capture.
    Secret::new(deja::http::headers(response.headers().iter().filter_map(
        |(name, value)| value.to_str().ok().map(|value| (name.as_str(), value)),
    )))
}

pub(super) fn response_result(
    result: &CustomResult<reqwest::Response, HttpClientError>,
) -> (Secret<serde_json::Value>, bool) {
    (
        Secret::new(match result {
            Ok(response) => json!({
                "status": response.status().as_u16(),
                "reason": response.status().canonical_reason(),
                "response_headers": response_headers_json(response).expose(),
                "response_body": captured_body_json(response).expose(),
            }),
            Err(error) => json!({
                "error": format!("{error:?}"),
                "response_body": {
                    "captured": false,
                },
            }),
        }),
        result.is_err(),
    )
}

/// Capture and rebuild a `send_request` outcome on the `http_outgoing`
/// boundary. `reconstruct` returning `None` is what fail-stops a substituted
/// egress call, so the tape-conformance gate calls it directly.
#[derive(Debug)]
pub struct HttpResponseCodec;

impl deja::codec::ReplayCodec for HttpResponseCodec {
    type Value = CustomResult<reqwest::Response, HttpClientError>;

    fn capture(value: &Self::Value) -> (serde_json::Value, bool) {
        let (payload, is_error) = response_result(value);
        // The deja handoff: the tape records the full-fidelity value.
        (payload.expose(), is_error)
    }

    fn reconstruct(recorded: serde_json::Value) -> Option<Self::Value> {
        replay_response(&recorded).map(Ok)
    }
}

/// Replay: reconstruct a `reqwest::Response` from a recorded `response_result`
/// payload (`{status, response_headers, response_body: {raw_bytes: [...]}}`).
///
/// A recorded SUCCESS reconstructs verbatim: connectors read status, headers
/// and body bytes, and all three come from the tape, so that call touches no
/// network.
///
/// Returning `None` does not fall through to a live call. deja maps it to
/// `Reconstructed::Failed`, which fail-stops the request with a named reason —
/// so a recorded value this build cannot read halts the replay rather than
/// quietly reaching the real endpoint.
///
/// Headers are read as an array of values per name and no other shape is
/// accepted. A recording written before that capture stored a single string per
/// name and had already lost every repeat; reading one would replay a tape that
/// still carries the defect this fixes, so it fail-stops instead.
pub(super) fn replay_response(recorded: &serde_json::Value) -> Option<reqwest::Response> {
    let status_code = u16::try_from(recorded.get("status")?.as_u64()?).ok()?;
    let status = http::StatusCode::from_u16(status_code).ok()?;

    let raw_bytes: Vec<u8> = recorded
        .get("response_body")
        .and_then(|body| body.get("raw_bytes"))
        .and_then(|value| value.as_array())
        .map(|array| array.iter().filter_map(byte_from_json).collect())
        .unwrap_or_default();

    let mut builder = http::Response::builder().status(status);
    if let Some(headers) = recorded.get("response_headers").and_then(|h| h.as_object()) {
        for (name, values) in headers {
            // `?` rather than a skip: a name whose values are not an array is a
            // pre-fix recording, and replaying it with its headers dropped would
            // be worse than refusing it.
            for value in values.as_array()?.iter().filter_map(|value| value.as_str()) {
                // `Builder::header` is `try_append`, so a repeated name keeps
                // every value rather than replacing the previous one.
                builder = builder.header(name.as_str(), value);
            }
        }
    }
    let http_response = builder.body(bytes::Bytes::from(raw_bytes)).ok()?;
    Some(reqwest::Response::from(http_response))
}

fn byte_from_json(value: &serde_json::Value) -> Option<u8> {
    value.as_u64().and_then(|byte| byte.try_into().ok())
}

fn header_value(value: &Maskable<String>) -> String {
    match value {
        Maskable::Masked(value) => value.peek().to_string(),
        Maskable::Normal(value) => value.clone(),
    }
}

fn request_body(body: &RequestContent) -> serde_json::Value {
    match body {
        RequestContent::RawBytes(bytes) => {
            let mut body = deja::http::body(bytes);
            if let serde_json::Value::Object(ref mut object) = body {
                object.insert(
                    "kind".to_string(),
                    serde_json::Value::String("RawBytesRequestBody".to_string()),
                );
            }
            body
        }
        _ => {
            let value = body.get_inner_value();
            let text = value.peek();
            let kind = format!("{body:?}");
            let mut captured = deja::http::body(text.as_bytes());
            if let serde_json::Value::Object(ref mut object) = captured {
                object.insert("kind".to_string(), serde_json::Value::String(kind));
            }
            captured
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three `set-cookie` headers must survive capture and reconstruct as three.
    ///
    /// Fails on the `insert`-per-name capture this replaced, which kept one value
    /// per name and so recorded a single cookie.
    #[test]
    fn repeated_headers_survive_capture_and_reconstruct() {
        let mut builder = http::Response::builder().status(200);
        for (name, value) in [
            ("x-request-id", "req-1"),
            ("set-cookie", "a=1"),
            ("set-cookie", "b=2"),
            ("set-cookie", "c=3"),
        ] {
            builder = builder.header(name, value);
        }
        let source = reqwest::Response::from(
            builder
                .body(bytes::Bytes::from_static(b"{}"))
                .expect("failed to build the test response"),
        );

        let result: CustomResult<reqwest::Response, HttpClientError> = Ok(source);
        let (captured, _) = response_result(&result);
        let captured = captured.expose();

        let reconstructed =
            replay_response(&captured).expect("a captured response must reconstruct");
        let cookies: Vec<&str> = reconstructed
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect();
        assert_eq!(
            cookies,
            ["a=1", "b=2", "c=3"],
            "every set-cookie value must survive the round trip"
        );
    }

    /// A recording written before the capture kept repeats stored one string per
    /// name, having already lost every repeated value. Reconstructing it would
    /// replay a tape that still carries the defect this fixes, so it refuses —
    /// and deja turns that refusal into a fail-stop with a named reason rather
    /// than a live call.
    #[test]
    fn a_recording_with_one_value_per_name_is_refused() {
        let recorded = serde_json::json!({
            "status": 200,
            "response_headers": { "content-type": "application/json" },
            "response_body": { "raw_bytes": [] },
        });
        assert!(
            replay_response(&recorded).is_none(),
            "a pre-fix recording must refuse rather than replay with its headers dropped"
        );
    }
}
