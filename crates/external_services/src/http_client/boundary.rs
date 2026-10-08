use common_utils::{
    errors::CustomResult,
    request::{Request, RequestContent},
};
use hyperswitch_interfaces::errors::HttpClientError;
use hyperswitch_masking::{ExposeInterface, Maskable, PeekInterface, Secret};
use reqwest::ResponseBuilderExt;
use serde_json::json;

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
    body: Result<&[u8], &str>,
) -> (Secret<serde_json::Value>, bool) {
    (
        Secret::new(match result {
            Ok(response) => json!({
                "status": response.status().as_u16(),
                "reason": response.status().canonical_reason(),
                "response_headers": response_headers_json(response).expose(),
                // A failed read records why; replay fails the body the same way.
                "response_body": match body {
                    Ok(body) => deja::http::body(body),
                    Err(reason) => json!({ "captured": false, "read_error": reason }),
                },
            }),
            // The typed variant, not report text: callers branch on it, and report
            // text varies per build.
            Err(error) => json!({
                "version": 1,
                "result": "Err",
                "kind": error.current_context(),
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

impl deja::codec::OwnedReplayCodec for HttpResponseCodec {
    type Value = CustomResult<reqwest::Response, HttpClientError>;
    /// The body as read, or why reading it failed.
    type Read = Result<bytes::Bytes, String>;

    /// Reads the body and rebuilds the response from the original's parts.
    async fn read(value: Self::Value) -> (Self::Value, Result<Self::Read, String>) {
        let response = match value {
            Ok(response) => response,
            Err(report) => return (Err(report), Ok(Ok(bytes::Bytes::new()))),
        };
        let status = response.status();
        let version = response.version();
        let headers = response.headers().clone();
        let url = response.url().clone();
        // Sized only if the original knew its length.
        let sized = response.content_length().is_some();
        let (body, read) = match response.bytes().await {
            Ok(bytes) => {
                let body =
                    if sized {
                        reqwest::Body::from(bytes.clone())
                    } else {
                        reqwest::Body::wrap_stream(futures::stream::once(futures::future::ready(
                            Ok::<_, std::convert::Infallible>(bytes.clone()),
                        )))
                    };
                (body, Ok(bytes))
            }
            Err(error) => {
                let reason = error.to_string();
                let body = reqwest::Body::wrap_stream(futures::stream::once(
                    futures::future::ready(Err::<bytes::Bytes, _>(error)),
                ));
                (body, Err(reason))
            }
        };
        let mut response = match http::Response::builder().url(url).body(()) {
            Ok(response) => response.map(|()| body),
            Err(_) => http::Response::new(body),
        };
        *response.status_mut() = status;
        *response.version_mut() = version;
        *response.headers_mut() = headers;
        (Ok(reqwest::Response::from(response)), Ok(read))
    }

    fn record(read: Self::Read, value: &Self::Value) -> (serde_json::Value, bool) {
        let (payload, is_error) = response_result(value, read.as_deref().map_err(String::as_str));
        // The deja handoff: the tape records the full-fidelity value.
        (payload.expose(), is_error)
    }

    fn reconstruct(recorded: serde_json::Value) -> Option<Self::Value> {
        if recorded.get("result").and_then(serde_json::Value::as_str) == Some("Err") {
            return replay_error(&recorded).map(Err);
        }
        replay_response(&recorded).map(Ok)
    }
}

/// Rebuilds a recorded `send_request` error from its typed variant.
fn replay_error(recorded: &serde_json::Value) -> Option<error_stack::Report<HttpClientError>> {
    let kind = recorded.get("kind")?.clone();
    serde_json::from_value::<HttpClientError>(kind)
        .ok()
        .map(|error| error_stack::report!(error))
}

/// Rebuilds a `reqwest::Response` from a recorded `response_result` payload, so
/// a replayed connector call is served from the tape and touches no network.
///
/// Headers are read as an array of values per name; any other shape is a
/// recording that lost its repeats, so it returns `None`, which fail-stops the call.
pub(super) fn replay_response(recorded: &serde_json::Value) -> Option<reqwest::Response> {
    let status_code = u16::try_from(recorded.get("status")?.as_u64()?).ok()?;
    let status = http::StatusCode::from_u16(status_code).ok()?;

    let read_error = recorded
        .get("response_body")
        .and_then(|body| body.get("read_error"))
        .and_then(serde_json::Value::as_str);
    let raw_bytes: Vec<u8> = recorded
        .get("response_body")
        .and_then(|body| body.get("raw_bytes"))
        .and_then(|value| value.as_array())
        .map(|array| array.iter().filter_map(byte_from_json).collect())
        .unwrap_or_default();

    let mut builder = http::Response::builder().status(status);
    if let Some(headers) = recorded.get("response_headers").and_then(|h| h.as_object()) {
        for (name, values) in headers {
            for value in values.as_array()?.iter().filter_map(|value| value.as_str()) {
                // `Builder::header` is `try_append`, so a repeated name keeps
                // every value rather than replacing the previous one.
                builder = builder.header(name.as_str(), value);
            }
        }
    }
    let body = match read_error {
        Some(reason) => {
            reqwest::Body::wrap_stream(futures::stream::once(futures::future::ready(Err::<
                bytes::Bytes,
                _,
            >(
                std::io::Error::other(reason.to_owned()),
            ))))
        }
        None => reqwest::Body::from(bytes::Bytes::from(raw_bytes)),
    };
    let http_response = builder.body(body).ok()?;
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
        let (captured, _) = response_result(&result, Ok(b"{}"));
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

    /// Recording a reconstructed response reproduces the whole recorded payload.
    #[test]
    fn capturing_a_reconstructed_response_reproduces_the_recording() {
        use deja::codec::OwnedReplayCodec;
        const BODY: &[u8] = b"{\"ok\":true}";

        let mut builder = http::Response::builder().status(200);
        for (name, value) in [("content-type", "application/json"), ("set-cookie", "a=1")] {
            builder = builder.header(name, value);
        }
        let source = builder
            .body(bytes::Bytes::from_static(BODY))
            .expect("failed to build the test response");
        let (first, read) = block_on(HttpResponseCodec::read(Ok(reqwest::Response::from(source))));
        let captured = HttpResponseCodec::record(read.expect("the body reads"), &first).0;

        let reconstructed = HttpResponseCodec::reconstruct(captured.clone())
            .expect("a captured response must reconstruct");
        let (second, read) = block_on(HttpResponseCodec::read(reconstructed));
        let recaptured = HttpResponseCodec::record(read.expect("the body reads"), &second).0;

        assert_eq!(
            recaptured, captured,
            "record(read(reconstruct(v))) must equal v"
        );
    }

    const GZIPPED: [u8; 52] = [
        31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 171, 86, 202, 200, 44, 41, 86, 178, 170, 86, 42, 201,
        47, 73, 204, 1, 49, 202, 18, 115, 74, 83, 149, 172, 12, 107, 117, 160, 114, 209, 177, 181,
        181, 0, 3, 122, 183, 221, 40, 0, 0, 0,
    ];

    /// Serves `reply` to every connection, so one fixture can be fetched twice.
    #[allow(
        clippy::expect_used,
        reason = "test helper: a free fn, so allow-expect-in-tests does not cover it; a fixture that cannot serve should fail the test loudly"
    )]
    fn serve(reply: Vec<u8>) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}/charge", listener.local_addr().expect("addr"));
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request);
                let _ = stream.write_all(&reply);
            }
        });
        url
    }

    /// What a caller can observe: parts, length, remote address, then the body.
    async fn observed(
        response: reqwest::Response,
    ) -> (String, Option<u64>, bool, Result<bytes::Bytes, String>) {
        let parts = format!(
            "{:?} {:?} {:?} {}",
            response.status(),
            response.version(),
            response.headers(),
            response.url(),
        );
        let length = response.content_length();
        let has_remote = response.remote_addr().is_some();
        let body = response.bytes().await.map_err(|error| error.to_string());
        (parts, length, has_remote, body)
    }

    /// Every accessor reads the same through the codec for sized, chunked, gzip and truncated
    /// bodies, except `remote_addr` (private to reqwest) and a truncated body's `content_length`.
    #[test]
    fn the_caller_observes_the_same_response_through_the_codec() {
        use deja::codec::OwnedReplayCodec;
        let body = br#"{"id":"ch_1","status":"succeeded"}"#;
        let reply = |head: String, payload: &[u8]| [head.as_bytes(), b"\r\n", payload].concat();
        let fixtures = [
            (
                "sized",
                reply(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
                         set-cookie: a=1\r\nset-cookie: b=2\r\ncontent-length: {}\r\n",
                        body.len()
                    ),
                    body,
                ),
                true,
            ),
            (
                "chunked",
                reply(
                    format!(
                        "HTTP/1.1 402 Payment Required\r\ntransfer-encoding: chunked\r\n\r\n{:x}",
                        body.len()
                    ),
                    &[&body[..], b"\r\n0\r\n\r\n"].concat(),
                ),
                true,
            ),
            (
                "gzip",
                reply(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-encoding: gzip\r\ncontent-length: {}\r\n",
                        GZIPPED.len()
                    ),
                    &GZIPPED,
                ),
                true,
            ),
            (
                "truncated",
                reply(
                    "HTTP/1.1 200 OK\r\ncontent-length: 400\r\n".to_owned(),
                    body,
                ),
                false,
            ),
        ];
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a test runtime");
        runtime.block_on(async {
            let client = reqwest::Client::new();
            for (name, fixture, reads) in fixtures {
                let url = serve(fixture);
                let fetch = || async {
                    client
                        .post(&url)
                        .send()
                        .await
                        .expect("the fixture server answers")
                };
                let (direct_parts, direct_length, direct_remote, direct_body) =
                    observed(fetch().await).await;
                let (rebuilt, read) = HttpResponseCodec::read(Ok(fetch().await)).await;
                assert_eq!(matches!(read, Ok(Ok(_))), reads, "{name}: {read:?}");
                let (parts, length, remote, rebuilt_body) =
                    observed(rebuilt.expect("an Ok stays Ok")).await;
                assert_eq!(direct_parts, parts, "{name}: the parts differ");
                assert_eq!(
                    (direct_remote, remote),
                    (true, false),
                    "{name}: the known difference, remote_addr"
                );
                assert_eq!(direct_body.is_ok(), reads, "{name}: {direct_body:?}");
                if reads {
                    assert_eq!(direct_length, length, "{name}: the length differs");
                    assert_eq!(direct_body, rebuilt_body, "{name}: the body differs");
                } else {
                    assert_eq!(
                        (direct_length, length),
                        (Some(400), None),
                        "{name}: the known difference, a failed body's length"
                    );
                    assert!(rebuilt_body.is_err(), "{name}: the read failure survives");
                }
            }
        });
    }

    /// A failed read records status, headers and why, and replays as a failing body.
    #[test]
    fn a_failed_read_replays_as_a_body_that_fails() {
        use deja::codec::OwnedReplayCodec;
        let url = serve(
            b"HTTP/1.1 502 Bad Gateway\r\nx-request-id: r-1\r\ncontent-length: 400\r\n\r\n{\"partial\":"
                .to_vec(),
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a test runtime");
        runtime.block_on(async {
            let response = reqwest::Client::new()
                .post(&url)
                .send()
                .await
                .expect("the fixture server answers");
            let (value, read) = HttpResponseCodec::read(Ok(response)).await;
            let (recorded, is_error) =
                HttpResponseCodec::record(read.expect("the read always reports"), &value);
            assert!(!is_error, "the call returned its response");
            assert!(
                recorded.pointer("/response_body/read_error").is_some(),
                "the tape names the failed read: {recorded}"
            );
            let replayed = HttpResponseCodec::reconstruct(recorded)
                .expect("a failed read rebuilds")
                .expect("as a response");
            assert_eq!(replayed.status().as_u16(), 502);
            assert_eq!(replayed.headers()["x-request-id"], "r-1");
            assert!(replayed.bytes().await.is_err(), "the replayed body fails");
        });
    }

    /// The codec's futures here never wait on I/O.
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        let mut future = std::pin::pin!(future);
        loop {
            if let std::task::Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
        }
    }

    /// A recorded error rebuilds as the same variant and re-records identically.
    #[test]
    fn a_recorded_error_rebuilds_its_variant() {
        for error in [
            HttpClientError::RequestNotSent("error sending request".to_string()),
            HttpClientError::RequestTimeoutReceived,
        ] {
            let first: CustomResult<reqwest::Response, HttpClientError> =
                Err(error_stack::report!(error.clone()));
            let (captured, is_error) = response_result(&first, Ok(&[]));
            let captured = captured.expose();
            assert!(is_error);

            let rebuilt =
                <HttpResponseCodec as deja::codec::OwnedReplayCodec>::reconstruct(captured.clone())
                    .expect("a typed error must reconstruct");
            let Err(report) = &rebuilt else {
                panic!("a recorded error must rebuild as an error");
            };
            assert_eq!(report.current_context(), &error);
            assert_eq!(response_result(&rebuilt, Ok(&[])).0.expose(), captured);
        }
    }

    /// A typed DNS failure, as recorded for an unreachable internal service, rebuilds.
    #[test]
    fn the_recorded_dns_failure_rebuilds() {
        let recorded = serde_json::json!({
            "version": 1,
            "result": "Err",
            "kind": {"RequestNotSent": "error sending request for url (http://service.internal/rule): error trying to connect: dns error: failed to lookup address information: Name or service not known"},
            "response_body": {"captured": false},
        });
        let rebuilt = <HttpResponseCodec as deja::codec::OwnedReplayCodec>::reconstruct(recorded)
            .expect("the recorded dns failure must reconstruct");
        assert!(matches!(
            rebuilt.as_ref().map_err(|report| report.current_context()),
            Err(HttpClientError::RequestNotSent(message)) if message.contains("dns error")
        ));
    }

    /// A text-only error recording has no variant to rebuild, so it is refused.
    #[test]
    fn a_text_only_error_recording_is_refused() {
        let recorded = serde_json::json!({
            "error": "Failed to send request to connector error sending request for url (http://service.internal/rule): error trying to connect: dns error: failed to lookup address information: Name or service not known",
            "response_body": {"captured": false},
        });
        assert!(
            <HttpResponseCodec as deja::codec::OwnedReplayCodec>::reconstruct(recorded).is_none()
        );
    }
}
