//! What deja records of an OpenSearch query, and how a replay rebuilds it.

use common_utils::errors::CustomResult;
use opensearch::http::{response::Response, Url};
use serde_json::{json, Value};

use super::{OpenSearchError, OpenSearchQueryBuilder};

/// Records a response as its status and body; `read` only reads and rebuilds.
pub(super) struct ResponseCodec;

type ResponseEnvelope = deja::codec::ResultCodec<Value, OpenSearchError>;

impl deja::codec::OwnedReplayCodec for ResponseCodec {
    type Value = CustomResult<Response, OpenSearchError>;
    /// The body as read, or why reading failed.
    type Read = Result<opensearch_reqwest::Body, String>;

    async fn read(value: Self::Value) -> (Self::Value, Result<Self::Read, String>) {
        let response = match value {
            Ok(response) => response,
            Err(report) => return (Err(report), Ok(Ok(opensearch_reqwest::Body::from("")))),
        };
        let status = response.status_code();
        let headers = response.headers().clone();
        let url = response.url().clone();
        let method = response.method();
        // Sized only if the original knew its length.
        let sized = response.content_length().is_some();
        let (body, read) = match response.bytes().await {
            Ok(bytes) => {
                let read = opensearch_reqwest::Body::from(bytes.clone());
                let body = if sized {
                    opensearch_reqwest::Body::from(bytes)
                } else {
                    opensearch_reqwest::Body::wrap_stream(futures::stream::once(
                        futures::future::ready(Ok::<_, std::convert::Infallible>(bytes)),
                    ))
                };
                (body, Ok(read))
            }
            Err(error) => {
                let reason = error.to_string();
                let body = opensearch_reqwest::Body::wrap_stream(futures::stream::once(
                    futures::future::ready(Err::<Vec<u8>, _>(error)),
                ));
                (body, Err(reason))
            }
        };
        (Ok(rebuilt(status, headers, url, method, body)), Ok(read))
    }

    /// A failed read records its status and why; replay fails the body the same way.
    fn record(read: Self::Read, value: &Self::Value) -> (Value, bool) {
        match value {
            Ok(response) => {
                let status = response.status_code().as_u16();
                let recorded = match read {
                    Ok(body) => {
                        let body = String::from_utf8_lossy(body.as_bytes().unwrap_or_default());
                        json!({ "status": status, "body": body })
                    }
                    Err(reason) => json!({ "status": status, "read_error": reason }),
                };
                (ResponseEnvelope::ok_envelope(&recorded), false)
            }
            Err(report) => (ResponseEnvelope::err_envelope(report), true),
        }
    }

    fn reconstruct(recorded: Value) -> Option<Self::Value> {
        use deja::codec::ReplayCodec;
        match ResponseEnvelope::reconstruct(recorded)? {
            Ok(recorded) => {
                let status = opensearch_http::StatusCode::from_u16(
                    u16::try_from(recorded.get("status")?.as_u64()?).ok()?,
                )
                .ok()?;
                if let Some(reason) = recorded.get("read_error").and_then(Value::as_str) {
                    let failure = std::io::Error::other(reason.to_owned());
                    let body = opensearch_reqwest::Body::wrap_stream(futures::stream::once(
                        futures::future::ready(Err::<Vec<u8>, _>(failure)),
                    ));
                    return Some(Ok(Self::replayed(status, body)));
                }
                let body = recorded.get("body")?.as_str()?.to_owned();
                Some(Ok(Self::replayed(status, body.into())))
            }
            Err(report) => Some(Err(report)),
        }
    }
}

impl ResponseCodec {
    /// A missed query's answer: an empty result set both callers read, as
    /// `OpensearchOutput::Success` and as an `OpenMsearchOutput` with no responses.
    pub(super) fn missed_query() -> Response {
        Self::replayed(
            opensearch_http::StatusCode::OK,
            Self::missed_query_body().into(),
        )
    }

    fn missed_query_body() -> String {
        json!({
            "hits": {"total": {"value": 0}, "hits": []},
            "responses": [],
        })
        .to_string()
    }

    /// A response rebuilt from the tape, which holds its status and body.
    fn replayed(status: opensearch_http::StatusCode, body: opensearch_reqwest::Body) -> Response {
        let mut response = opensearch_http::Response::new(body);
        *response.status_mut() = status;
        Response::new(
            opensearch_reqwest::Response::from(response),
            opensearch::http::Method::Post,
        )
    }
}

/// The client's own `Response` from the original's parts and a body.
fn rebuilt(
    status: opensearch_http::StatusCode,
    headers: opensearch_http::HeaderMap,
    url: Url,
    method: opensearch::http::Method,
    body: opensearch_reqwest::Body,
) -> Response {
    use opensearch_reqwest::ResponseBuilderExt;
    let mut response = match opensearch_http::Response::builder().url(url).body(()) {
        Ok(response) => response.map(|()| body),
        Err(_) => opensearch_http::Response::new(body),
    };
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Response::new(opensearch_reqwest::Response::from(response), method)
}

impl OpenSearchQueryBuilder {
    /// Args for the OpenSearch seam, built field by field because the builder's `Debug`
    /// renders a `HashSet` in random order.
    pub fn deja_args(&self) -> Value {
        let mut auth_scope = self.build_auth_array();
        // Auth fragments and the IDs in `terms` are sets: sort the args, not the query.
        for fragment in &mut auth_scope {
            for field in [
                "merchant_id.keyword",
                "processor_merchant_id.keyword",
                "profile_id.keyword",
            ] {
                if let Some(values) = fragment
                    .pointer_mut("/bool/must")
                    .and_then(Value::as_array_mut)
                    .and_then(|clauses| {
                        clauses.iter_mut().find_map(|clause| {
                            clause
                                .get_mut("terms")
                                .and_then(|terms| terms.get_mut(field))
                                .and_then(Value::as_array_mut)
                        })
                    })
                {
                    values.sort_by_cached_key(Value::to_string);
                }
            }
        }
        auth_scope.sort_by_cached_key(Value::to_string);

        serde_json::json!({
            "query": self.query,
            "indexes": format!("{:?}", self.query_type),
            "offset": self.offset,
            "count": self.count,
            "filters": format!("{:?}", self.filters),
            "time_range": format!("{:?}", self.time_range),
            "amount_range": format!("{:?}", self.amount_range),
            "order": format!("{:?}", self.order),
            "auth_scope": auth_scope,
        })
    }
}

#[cfg(test)]
mod tests {
    use api_models::analytics::search::SearchIndex;

    use crate::{
        enums::AuthInfo,
        opensearch::{OpenSearchQuery, OpenSearchQueryBuilder},
    };

    #[allow(
        clippy::expect_used,
        reason = "test helper: a free fn, so allow-expect-in-tests does not cover it; a fixture whose org id will not parse should fail the test loudly"
    )]
    fn org_scope(org_id: &str) -> AuthInfo {
        AuthInfo::OrgLevel {
            org_id: common_utils::id_type::OrganizationId::try_from(std::borrow::Cow::Owned(
                org_id.to_owned(),
            ))
            .expect("valid organization id"),
        }
    }

    fn builder(search_params: Vec<AuthInfo>) -> OpenSearchQueryBuilder {
        OpenSearchQueryBuilder::new(
            OpenSearchQuery::Search(SearchIndex::PaymentIntents),
            "example".to_string(),
            search_params,
            None,
        )
    }

    /// The missed-query body deserializes for both callers; the miss arm cannot tell which asked.
    #[test]
    fn the_missed_query_body_deserializes_for_both_callers() {
        let body = super::ResponseCodec::missed_query_body();

        let single = serde_json::from_str::<api_models::analytics::search::OpensearchOutput>(&body)
            .expect("the single-index caller must be able to read the missed-query body");
        match single {
            api_models::analytics::search::OpensearchOutput::Success(success) => {
                assert_eq!(success.hits.total.value, 0);
                assert!(success.hits.hits.is_empty());
            }
            api_models::analytics::search::OpensearchOutput::Error(error) => {
                panic!("the untagged enum matched Error, not Success: {error:?}")
            }
        }

        let multi = serde_json::from_str::<api_models::analytics::search::OpenMsearchOutput>(&body)
            .expect("the multi-index caller must be able to read the missed-query body");
        assert!(
            multi.responses.is_empty(),
            "the multi-index caller zips responses against its index list"
        );
        assert!(multi.error.is_none());
    }

    #[test]
    fn query_args_include_stable_auth_scope_identity() {
        let first = builder(vec![org_scope("org_a"), org_scope("org_b")]);
        let same = builder(vec![org_scope("org_a"), org_scope("org_b")]);
        let reversed = builder(vec![org_scope("org_b"), org_scope("org_a")]);
        let different = builder(vec![org_scope("org_a"), org_scope("org_c")]);

        let make_merchant_scope =
            |merchant_ids: Vec<common_utils::id_type::MerchantId>| AuthInfo::MerchantLevel {
                org_id: common_utils::id_type::OrganizationId::try_from(
                    std::borrow::Cow::Borrowed("org_a"),
                )
                .expect("valid organization id"),
                merchant_ids,
                processor_merchant_ids: None,
            };
        let merchant_ids_reordered = builder(vec![make_merchant_scope(vec![
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_a"))
                .expect("valid merchant id"),
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_b"))
                .expect("valid merchant id"),
        ])]);
        let merchant_ids_sorted = builder(vec![make_merchant_scope(vec![
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_b"))
                .expect("valid merchant id"),
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_a"))
                .expect("valid merchant id"),
        ])]);

        assert_eq!(
            merchant_ids_reordered.deja_args(),
            merchant_ids_sorted.deja_args()
        );
        let expected = first.deja_args();
        assert_eq!(expected, same.deja_args());
        assert_eq!(expected, reversed.deja_args());
        assert_ne!(expected, different.deja_args());
    }

    /// The seam on `execute` names the `on_miss` and `args` helpers the tests above exercise.
    #[test]
    fn the_seam_names_the_helpers_the_tests_exercise() {
        let source = include_str!("../opensearch.rs");
        // Built from parts so this test's own text is not a match.
        let start = source
            .find(&["deja::", "boundary("].concat())
            .expect("the seam attribute is present");
        let end = source[start..]
            .find(&["pub async fn ", "execute("].concat())
            .expect("the seam is declared on execute");
        let declaration = &source[start..start + end];
        for expected in [
            "owned_codec = recording::ResponseCodec",
            "on_miss = Ok(recording::ResponseCodec::missed_query())",
            "args = query_builder.deja_args()",
        ] {
            assert!(
                declaration.contains(expected),
                "the seam must name `{expected}`: {declaration}"
            );
        }
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
        let url = format!(
            "http://{}/idx/_search",
            listener.local_addr().expect("addr")
        );
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request);
                let _ = stream.write_all(&reply);
            }
        });
        url
    }

    fn reply(head: &str, body: &[u8]) -> Vec<u8> {
        [head.as_bytes(), b"\r\n", body].concat()
    }

    #[allow(
        clippy::expect_used,
        reason = "test helper: a free fn, so allow-expect-in-tests does not cover it; a fixture that cannot serve should fail the test loudly"
    )]
    async fn fetch(url: &str) -> opensearch::http::response::Response {
        let response = opensearch_reqwest::Client::new()
            .post(url)
            .send()
            .await
            .expect("the fixture server answers");
        opensearch::http::response::Response::new(response, opensearch::http::Method::Post)
    }

    /// What a caller can observe: parts, `content_length`, then the body.
    async fn observed(
        response: opensearch::http::response::Response,
    ) -> (String, Option<u64>, Result<String, String>) {
        let parts = format!(
            "{:?} {:?} {:?} {} {:?} {:?}",
            response.status_code(),
            response.headers(),
            response.content_type(),
            response.url(),
            response.method(),
            response.warning_headers().collect::<Vec<_>>(),
        );
        let length = response.content_length();
        (
            parts,
            length,
            response.text().await.map_err(|error| error.to_string()),
        )
    }

    /// Every accessor reads the same through the codec for sized, chunked, gzip and truncated
    /// bodies, except a truncated body's `content_length` (reqwest has no sized failing body).
    #[tokio::test]
    async fn the_caller_observes_the_same_response_through_the_codec() {
        use deja::codec::OwnedReplayCodec;
        let body = br#"{"hits":{"total":{"value":1},"hits":[]}}"#;
        let fixtures = [
            (
                "sized",
                reply(
                    &format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json; charset=UTF-8\r\n\
                         warning: 299 OpenSearch \"deprecated\"\r\ncontent-length: {}\r\n",
                        body.len()
                    ),
                    body,
                ),
                true,
            ),
            (
                "chunked",
                reply(
                    &format!(
                        "HTTP/1.1 404 Not Found\r\ncontent-type: application/json\r\n\
                         transfer-encoding: chunked\r\n\r\n{:x}",
                        body.len()
                    ),
                    &[&body[..], b"\r\n0\r\n\r\n"].concat(),
                ),
                true,
            ),
            (
                "gzip",
                reply(
                    &format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
                         content-encoding: gzip\r\ncontent-length: {}\r\n",
                        GZIPPED.len()
                    ),
                    &GZIPPED,
                ),
                true,
            ),
            (
                "truncated",
                reply(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 400\r\n",
                    body,
                ),
                false,
            ),
        ];
        for (name, fixture, reads) in fixtures {
            let url = serve(fixture);
            let (direct_parts, direct_length, direct_body) = observed(fetch(&url).await).await;
            let (rebuilt, read) = super::ResponseCodec::read(Ok(fetch(&url).await)).await;
            assert_eq!(matches!(read, Ok(Ok(_))), reads, "{name}: {read:?}");
            let (rebuilt_parts, rebuilt_length, rebuilt_body) =
                observed(rebuilt.expect("an Ok stays Ok")).await;
            assert_eq!(direct_parts, rebuilt_parts, "{name}: the parts differ");
            assert_eq!(direct_body.is_ok(), reads, "{name}: {direct_body:?}");
            if reads {
                assert_eq!(direct_length, rebuilt_length, "{name}: the length differs");
                assert_eq!(direct_body, rebuilt_body, "{name}: the body differs");
            } else {
                assert_eq!(
                    (direct_length, rebuilt_length),
                    (Some(400), None),
                    "{name}: the one known difference, a failed body's length"
                );
                assert!(rebuilt_body.is_err(), "{name}: the read failure survives");
            }
        }
    }

    /// What `read` saw goes on the tape and rebuilds on replay, error arm too.
    #[tokio::test]
    async fn a_recorded_response_replays_as_what_the_caller_read() {
        use deja::codec::OwnedReplayCodec;
        use error_stack::report;
        let url = serve(reply(
            "HTTP/1.1 404 Not Found\r\ncontent-length: 15\r\n",
            br#"{"found":false}"#,
        ));
        let (value, read) = super::ResponseCodec::read(Ok(fetch(&url).await)).await;
        let (recorded, is_error) =
            super::ResponseCodec::record(read.expect("the body reads"), &value);
        assert!(!is_error);
        let replayed = super::ResponseCodec::reconstruct(recorded)
            .expect("the envelope rebuilds")
            .expect("an Ok envelope");
        assert_eq!(replayed.status_code().as_u16(), 404);
        assert_eq!(
            replayed.text().await.expect("the replayed body reads"),
            r#"{"found":false}"#
        );

        // A failed read replays as a failing body at the recorded status.
        let url = serve(reply(
            "HTTP/1.1 503 Service Unavailable\r\ncontent-length: 400\r\n",
            br#"{"partial":"#,
        ));
        let (value, read) = super::ResponseCodec::read(Ok(fetch(&url).await)).await;
        let (recorded, is_error) =
            super::ResponseCodec::record(read.expect("the read always reports"), &value);
        assert!(!is_error, "the call returned its response");
        assert!(
            recorded.pointer("/value/read_error").is_some(),
            "the tape names the failed read: {recorded}"
        );
        let replayed = super::ResponseCodec::reconstruct(recorded)
            .expect("the envelope rebuilds")
            .expect("an Ok envelope");
        assert_eq!(replayed.status_code().as_u16(), 503);
        assert!(replayed.text().await.is_err(), "the replayed body fails");

        let failed: common_utils::errors::CustomResult<
            opensearch::http::response::Response,
            super::OpenSearchError,
        > = Err(report!(super::OpenSearchError::ConnectionError));
        let (value, read) = super::ResponseCodec::read(failed).await;
        let (recorded, is_error) =
            super::ResponseCodec::record(read.expect("an error arm reads nothing"), &value);
        assert!(is_error);
        let replayed = super::ResponseCodec::reconstruct(recorded).expect("the envelope rebuilds");
        assert!(matches!(
            replayed.map(|_| ()).map_err(|report| report.current_context().to_string()),
            Err(message) if message == super::OpenSearchError::ConnectionError.to_string()
        ));
    }
}
