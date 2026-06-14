use common_utils::{
    errors::CustomResult,
    request::{Request, RequestContent},
};
use deja::DejaHook;
use hyperswitch_interfaces::errors::HttpClientError;
use hyperswitch_masking::{Maskable, PeekInterface};
use serde_json::json;

/// Extension key used to smuggle the captured response body out of the
/// `send_request` function so that the boundary `result` extractor can
/// report it without consuming the response stream.
pub(super) struct CapturedResponseBody(pub(super) bytes::Bytes);

pub(super) fn is_active() -> bool {
    deja::global_hook_from_env().is_some_and(|hook| hook.is_active())
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

pub(super) fn request_args(request: &Request, timeout_secs: Option<u64>) -> serde_json::Value {
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
    json!({
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
    })
}

fn captured_body_json(response: &reqwest::Response) -> serde_json::Value {
    match response.extensions().get::<CapturedResponseBody>() {
        Some(CapturedResponseBody(bytes)) => deja::http::body(bytes),
        None => deja::http::missing_body("response body not captured (missing extension)"),
    }
}

fn response_headers_json(response: &reqwest::Response) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, value) in response.headers() {
        if let Ok(value) = value.to_str() {
            map.insert(
                key.as_str().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }
    serde_json::Value::Object(map)
}

pub(super) fn response_result(
    result: &CustomResult<reqwest::Response, HttpClientError>,
) -> (serde_json::Value, bool) {
    (
        match result {
            Ok(response) => json!({
                "status": response.status().as_u16(),
                "reason": response.status().canonical_reason(),
                "response_headers": response_headers_json(response),
                "response_body": captured_body_json(response),
            }),
            Err(error) => json!({
                "error": format!("{error:?}"),
                "response_body": {
                    "captured": false,
                },
            }),
        },
        result.is_err(),
    )
}

/// Replay: reconstruct a `reqwest::Response` from a recorded `response_result`
/// payload (`{status, response_headers, response_body: {raw_bytes: [...]}}`).
///
/// Returns `None` for a recorded error (no `status` field) so the boundary
/// falls through to live execution (the V1 Ok-only policy). Connectors consume
/// status + headers + body bytes from the response, all of which are
/// reconstructed verbatim from the recording — so a replayed outgoing call
/// (e.g. to Stripe) is served entirely from the lookup table with no network.
pub(super) fn replay_response(recorded: &serde_json::Value) -> Option<reqwest::Response> {
    let status_code = u16::try_from(recorded.get("status")?.as_u64()?).ok()?;
    let status = http::StatusCode::from_u16(status_code).ok()?;

    let raw_bytes: Vec<u8> = recorded
        .get("response_body")
        .and_then(|body| body.get("raw_bytes"))
        .and_then(|value| value.as_array())
        .map(|array| {
            array
                .iter()
                .filter_map(|value| value.as_u64().map(|byte| byte as u8))
                .collect()
        })
        .unwrap_or_default();

    let mut builder = http::Response::builder().status(status);
    if let Some(headers) = recorded.get("response_headers").and_then(|h| h.as_object()) {
        for (name, value) in headers {
            if let Some(value) = value.as_str() {
                builder = builder.header(name.as_str(), value);
            }
        }
    }
    let http_response = builder.body(bytes::Bytes::from(raw_bytes)).ok()?;
    Some(reqwest::Response::from(http_response))
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
