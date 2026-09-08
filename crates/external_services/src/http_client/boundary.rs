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

/// Captures response headers as an ORDERED LIST of `[name, value]` pairs.
///
/// Deliberately not a map keyed by header name, and the distinction is
/// load-bearing twice over.
///
/// A name-keyed map holds one value per name, so a response carrying three
/// `set-cookie` headers records one and the other two are gone. And
/// `serde_json::Map` is an `IndexMap` under serde_json's `preserve_order`
/// feature but a `BTreeMap` without it — this build enables it transitively
/// (josekit, thirtyfour, ucs_common_utils), so the same code records wire order
/// here and sorted order in a build that does not, and a tape written by one and
/// read by the other silently reorders.
///
/// `deja::http::headers` fixes the duplicate half by accumulating values into
/// arrays, but it still returns an object keyed by name, so it cannot fix the
/// ordering half. A JSON array is ordered under either backing and carries
/// repeats by construction, so the pair list is what crosses the boundary.
fn response_headers_json(response: &reqwest::Response) -> Secret<serde_json::Value> {
    let pairs = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            // A non-UTF-8 header value has no JSON representation; drop the one
            // entry rather than failing the whole capture.
            value.to_str().ok().map(|value| {
                serde_json::Value::Array(vec![
                    serde_json::Value::String(name.as_str().to_owned()),
                    serde_json::Value::String(value.to_owned()),
                ])
            })
        })
        .collect();
    Secret::new(serde_json::Value::Array(pairs))
}

/// One recorded `[name, value]` header entry.
fn header_pair(pair: &serde_json::Value) -> Option<(&str, &str)> {
    let entry = pair.as_array()?;
    Some((entry.first()?.as_str()?, entry.get(1)?.as_str()?))
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
/// A recorded ERROR carries no `status` field and reconstructs to `None`, which
/// the boundary treats as a lookup miss and answers by executing LIVE. Replaying
/// a request whose connector call failed therefore issues a real outbound request
/// to the real endpoint. This is the current Ok-only replay policy: replay is
/// egress-free only for as long as every recorded call succeeded, so it must not
/// be relied on as an egress guarantee.
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
    // `Builder::header` appends (`HeaderMap::try_append`), so repeated names
    // survive here as long as the recording carried them in the first place.
    match recorded.get("response_headers") {
        // Current form: an ordered list of `[name, value]` pairs.
        Some(serde_json::Value::Array(pairs)) => {
            for pair in pairs {
                if let Some((name, value)) = header_pair(pair) {
                    builder = builder.header(name, value);
                }
            }
        }
        // Legacy form: an object keyed by name, one value each, written before
        // the pair list. Their repeats and wire order were already lost at
        // capture and cannot be recovered here; reading the shape keeps those
        // recordings replayable rather than failing them.
        Some(serde_json::Value::Object(map)) => {
            for (name, value) in map {
                if let Some(value) = value.as_str() {
                    builder = builder.header(name.as_str(), value);
                }
            }
        }
        _ => {}
    }
    let body = bytes::Bytes::from(raw_bytes);
    let mut http_response = builder.body(body.clone()).ok()?;
    // Restore the extension `response_result` reads the body from. Without it a
    // reconstructed response re-captures as "body not captured (missing
    // extension)", so `capture(reconstruct(v))` would not equal `v` — the codec
    // round-trip property in juspay/deja#121. Nothing re-captures on a replay
    // hit today, so this is inert at runtime; the codec should not rely on that
    // staying true.
    http_response
        .extensions_mut()
        .insert(CapturedResponseBody(body));
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

    /// A response carrying repeated `set-cookie` headers must survive capture and
    /// reconstruct with every value intact and in the same order.
    ///
    /// Fails on the name-keyed capture this replaced: that recorded one entry per
    /// name, so three cookies became one and the order that came back was the
    /// map's rather than the response's.
    ///
    /// The expectation is taken from the source response's own header iteration
    /// rather than from the insertion list, because `HeaderMap` groups the values
    /// of a repeated name together — so the property under test is the round trip
    /// (`capture` then `reconstruct` preserves what the response had), not a
    /// guess about what order `HeaderMap` chooses.
    #[test]
    fn repeated_headers_survive_capture_and_reconstruct_in_order() {
        // Deliberately not alphabetical, so a sorted representation is visible.
        let wire = [
            ("x-request-id", "req-1"),
            ("set-cookie", "a=1"),
            ("content-type", "application/json"),
            ("set-cookie", "b=2"),
            ("set-cookie", "c=3"),
        ];

        let mut builder = http::Response::builder().status(200);
        for (name, value) in wire {
            builder = builder.header(name, value);
        }
        let mut source = builder
            .body(bytes::Bytes::from_static(b"{}"))
            .expect("failed to build the test response");
        source
            .extensions_mut()
            .insert(CapturedResponseBody(bytes::Bytes::from_static(b"{}")));
        let source = reqwest::Response::from(source);

        let expected = header_sequence(source.headers());
        assert_eq!(
            expected
                .iter()
                .filter(|(name, _)| name == "set-cookie")
                .count(),
            3,
            "the fixture must actually carry three set-cookie values"
        );

        let result: CustomResult<reqwest::Response, HttpClientError> = Ok(source);
        let (captured, is_error) = response_result(&result);
        assert!(!is_error, "a 200 response must not capture as an error");
        let captured = captured.expose();

        // Pin the representation itself: an object can carry neither repeats nor
        // order, so this would catch a regression even on a build whose
        // `serde_json::Map` happens to preserve insertion order.
        let recorded_headers = captured
            .get("response_headers")
            .expect("capture must record response_headers");
        assert!(
            recorded_headers.is_array(),
            "headers must record as an ordered pair list, got {recorded_headers}"
        );

        let replayed = replay_response(&captured).expect("reconstruct returned None");

        assert_eq!(
            header_sequence(replayed.headers()),
            expected,
            "every header value must survive the round trip in the same order"
        );

        // The codec round-trip property from juspay/deja#121: capturing a
        // reconstructed value must reproduce the recording it came from.
        // Asserted on the whole payload, not just the headers, so a future
        // change that loses something elsewhere in the envelope is caught here
        // rather than as a replay divergence.
        let recaptured = {
            let result: CustomResult<reqwest::Response, HttpClientError> = Ok(replayed);
            response_result(&result).0.expose()
        };
        assert_eq!(recaptured, captured, "capture(reconstruct(v)) must equal v");
    }

    /// Tapes recorded before the pair list stored a name-keyed object with one
    /// value per name. Those recordings must still reconstruct — their repeats
    /// and ordering were lost when they were written and cannot be recovered,
    /// but replaying them must not fail outright.
    #[test]
    fn legacy_object_form_still_reconstructs() {
        let recorded = serde_json::json!({
            "status": 200,
            "response_headers": {
                "content-type": "application/json",
                "set-cookie": "only=1",
            },
            "response_body": { "raw_bytes": [123, 125] },
        });

        let replayed = replay_response(&recorded).expect("legacy form must reconstruct");

        assert_eq!(replayed.status().as_u16(), 200);
        let observed = header_sequence(replayed.headers());
        assert!(
            observed.contains(&("set-cookie".to_owned(), "only=1".to_owned())),
            "legacy header must survive, got {observed:?}"
        );
        assert!(
            observed.contains(&("content-type".to_owned(), "application/json".to_owned())),
            "legacy header must survive, got {observed:?}"
        );
    }

    fn header_sequence(headers: &http::HeaderMap) -> Vec<(String, String)> {
        headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    value
                        .to_str()
                        .expect("test fixture headers are UTF-8")
                        .to_owned(),
                )
            })
            .collect()
    }
}
