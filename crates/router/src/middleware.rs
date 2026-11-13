use actix_web::HttpMessage;
use common_utils::consts::TENANT_HEADER;
use futures::StreamExt;
// Re-export RequestId from router_env for convenience
pub use router_env::RequestId;
use router_env::{
    logger,
    tracing::{field::Empty, Instrument},
};

use crate::{headers, routes::metrics};

/// Middleware for attaching default response headers. Headers with the same key already set in a
/// response will not be overwritten.
pub fn default_response_headers() -> actix_web::middleware::DefaultHeaders {
    use actix_web::http::header;

    let default_headers_middleware = actix_web::middleware::DefaultHeaders::new();

    #[cfg(feature = "vergen")]
    let default_headers_middleware =
        default_headers_middleware.add(("x-hyperswitch-version", router_env::git_tag!()));

    default_headers_middleware
        // Max age of 1 year in seconds, equal to `60 * 60 * 24 * 365` seconds.
        .add((header::STRICT_TRANSPORT_SECURITY, "max-age=31536000"))
        .add((header::VIA, "HyperSwitch"))
}

/// Middleware to build a TOP level domain span for each request.
pub struct LogSpanInitializer;

impl<S, B> actix_web::dev::Transform<S, actix_web::dev::ServiceRequest> for LogSpanInitializer
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = LogSpanInitializerMiddleware<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(LogSpanInitializerMiddleware { service }))
    }
}

pub struct LogSpanInitializerMiddleware<S> {
    service: S,
}

impl<S, B> actix_web::dev::Service<actix_web::dev::ServiceRequest>
    for LogSpanInitializerMiddleware<S>
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = futures::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    // TODO: have a common source of truth for the list of top level fields
    // /crates/router_env/src/logger/storage.rs also has a list of fields  called PERSISTENT_KEYS
    fn call(&self, req: actix_web::dev::ServiceRequest) -> Self::Future {
        let tenant_id = req
            .headers()
            .get(TENANT_HEADER)
            .and_then(|i| i.to_str().ok())
            .map(|s| s.to_owned());
        let response_fut = self.service.call(req);
        let tenant_id_clone = tenant_id.clone();
        Box::pin(
            async move {
                if let Some(tenant) = tenant_id_clone {
                    router_env::tracing::Span::current().record("tenant_id", tenant);
                }
                let response = response_fut.await;
                router_env::tracing::Span::current().record("golden_log_line", true);
                response
            }
            .instrument(
                router_env::tracing::info_span!(
                    "ROOT_SPAN",
                    payment_id = Empty,
                    merchant_id = Empty,
                    connector_name = Empty,
                    payment_method = Empty,
                    status_code = Empty,
                    flow = "UNKNOWN",
                    golden_log_line = Empty,
                    tenant_id = &tenant_id
                )
                .or_current(),
            ),
        )
    }
}

fn get_request_details_from_value(json_value: &serde_json::Value, parent_key: &str) -> String {
    match json_value {
        serde_json::Value::Null => format!("{parent_key}: null"),
        serde_json::Value::Bool(b) => format!("{parent_key}: {b}"),
        serde_json::Value::Number(num) => format!("{}: {}", parent_key, num.to_string().len()),
        serde_json::Value::String(s) => format!("{}: {}", parent_key, s.len()),
        serde_json::Value::Array(arr) => {
            let mut result = String::new();
            for (index, value) in arr.iter().enumerate() {
                let child_key = format!("{parent_key}[{index}]");
                result.push_str(&get_request_details_from_value(value, &child_key));
                if index < arr.len() - 1 {
                    result.push_str(", ");
                }
            }
            result
        }
        serde_json::Value::Object(obj) => {
            let mut result = String::new();
            for (index, (key, value)) in obj.iter().enumerate() {
                let child_key = format!("{parent_key}[{key}]");
                result.push_str(&get_request_details_from_value(value, &child_key));
                if index < obj.len() - 1 {
                    result.push_str(", ");
                }
            }
            result
        }
    }
}

/// Middleware for logging request details of HTTP 400 Bad Requests without buffering the payload
/// ahead of the application's body extractors.
pub struct Http400RequestDetailsLogger {
    capture_limit: usize,
}

impl Http400RequestDetailsLogger {
    /// Limits the logged copy; endpoint extractors enforce request size limits.
    pub fn new(capture_limit: usize) -> Self {
        Self { capture_limit }
    }
}

struct RequestBodyCapture {
    bytes: Option<bytes::BytesMut>,
    bytes_read: usize,
    complete: bool,
}

fn capture_request_payload(
    mut payload: actix_web::dev::Payload,
    limit: usize,
) -> (
    actix_web::dev::Payload,
    std::rc::Rc<std::cell::RefCell<RequestBodyCapture>>,
) {
    let capture = std::rc::Rc::new(std::cell::RefCell::new(RequestBodyCapture {
        bytes: Some(bytes::BytesMut::new()),
        bytes_read: 0,
        complete: false,
    }));
    let captured = capture.clone();
    let stream: actix_http::BoxedPayloadStream = Box::pin(futures::stream::poll_fn(move |cx| {
        let next = payload.poll_next_unpin(cx);
        let mut capture = captured.borrow_mut();
        match &next {
            std::task::Poll::Ready(Some(Ok(chunk))) => {
                capture.bytes_read = capture.bytes_read.saturating_add(chunk.len());
                if let Some(bytes) = capture.bytes.as_mut() {
                    if chunk.len() <= limit.saturating_sub(bytes.len()) {
                        bytes.extend_from_slice(chunk);
                    } else {
                        // Drop the captured prefix as well: partial JSON is not useful to log.
                        capture.bytes = None;
                    }
                }
            }
            std::task::Poll::Ready(Some(Err(_))) => capture.bytes = None,
            std::task::Poll::Ready(None) => capture.complete = true,
            std::task::Poll::Pending => {}
        }
        next
    }));
    (stream.into(), capture)
}

impl<S: 'static, B> actix_web::dev::Transform<S, actix_web::dev::ServiceRequest>
    for Http400RequestDetailsLogger
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = Http400RequestDetailsLoggerMiddleware<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(Http400RequestDetailsLoggerMiddleware {
            service: std::rc::Rc::new(service),
            capture_limit: self.capture_limit,
        }))
    }
}

pub struct Http400RequestDetailsLoggerMiddleware<S> {
    service: std::rc::Rc<S>,
    capture_limit: usize,
}

impl<S, B> actix_web::dev::Service<actix_web::dev::ServiceRequest>
    for Http400RequestDetailsLoggerMiddleware<S>
where
    S: actix_web::dev::Service<
            actix_web::dev::ServiceRequest,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        > + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = futures::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, mut req: actix_web::dev::ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let request_id_fut = req.extract::<RequestId>();
        let (payload, captured) = capture_request_payload(req.take_payload(), self.capture_limit);
        req.set_payload(payload);
        Box::pin(async move {
            let content_length_header = req
                .headers()
                .get(headers::CONTENT_LENGTH)
                .map(ToOwned::to_owned);
            let response_fut = svc.call(req);
            let response = response_fut.await?;
            // Log the request_details when we receive 400 status from the application
            if response.status() == 400 {
                let request_id = request_id_fut.await?.to_string();
                let content_length_header_string = content_length_header
                    .map(|content_length_header| {
                        content_length_header.to_str().map(ToOwned::to_owned)
                    })
                    .transpose()
                    .inspect_err(|error| {
                        logger::warn!("Could not convert content length to string {error:?}");
                    })
                    .ok()
                    .flatten();

                let capture = captured.borrow();
                let bytes_read = capture.bytes_read;
                logger::info!("Content length from header: {content_length_header_string:?}, Bytes read: {bytes_read}");

                let Some(bytes) = capture.bytes.as_ref().filter(|_| capture.complete) else {
                    logger::info!("request_id: {request_id}, request_details: Body not fully captured (unread, capture limit exceeded, or payload error)");
                    return Ok(response);
                };

                if !bytes.is_empty() {
                    let value_result: Result<serde_json::Value, serde_json::Error> =
                        serde_json::from_slice(bytes);
                    match value_result {
                        Ok(value) => {
                            logger::info!(
                                "request_id: {request_id}, request_details: {}",
                                get_request_details_from_value(&value, "")
                            );
                        }
                        Err(err) => {
                            logger::warn!("error while parsing the request in json value: {err}");
                        }
                    }
                } else {
                    logger::info!("request_id: {request_id}, request_details: Empty Body");
                }
            }
            Ok(response)
        })
    }
}

/// Middleware for Adding Accept-Language header based on query params
pub struct AddAcceptLanguageHeader;

impl<S: 'static, B> actix_web::dev::Transform<S, actix_web::dev::ServiceRequest>
    for AddAcceptLanguageHeader
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = AddAcceptLanguageHeaderMiddleware<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(AddAcceptLanguageHeaderMiddleware {
            service: std::rc::Rc::new(service),
        }))
    }
}

pub struct AddAcceptLanguageHeaderMiddleware<S> {
    service: std::rc::Rc<S>,
}

impl<S, B> actix_web::dev::Service<actix_web::dev::ServiceRequest>
    for AddAcceptLanguageHeaderMiddleware<S>
where
    S: actix_web::dev::Service<
            actix_web::dev::ServiceRequest,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        > + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = futures::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, mut req: actix_web::dev::ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        Box::pin(async move {
            #[derive(serde::Deserialize)]
            struct LocaleQueryParam {
                locale: Option<String>,
            }
            let query_params = req.query_string();
            let locale_param =
                serde_qs::from_str::<LocaleQueryParam>(query_params).map_err(|error| {
                    actix_web::error::ErrorBadRequest(format!(
                        "Could not convert query params to locale query parmas: {error:?}",
                    ))
                })?;
            let accept_language_header = req.headers().get(http::header::ACCEPT_LANGUAGE);
            if let Some(locale) = locale_param.locale {
                req.headers_mut().insert(
                    http::header::ACCEPT_LANGUAGE,
                    http::HeaderValue::from_str(&locale)?,
                );
            } else if accept_language_header.is_none() {
                req.headers_mut().insert(
                    http::header::ACCEPT_LANGUAGE,
                    http::HeaderValue::from_static("en"),
                );
            }
            let response_fut = svc.call(req);
            let response = response_fut.await?;
            Ok(response)
        })
    }
}

/// Middleware for recording request-response metrics
pub struct RequestResponseMetrics;

impl<S: 'static, B> actix_web::dev::Transform<S, actix_web::dev::ServiceRequest>
    for RequestResponseMetrics
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = RequestResponseMetricsMiddleware<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RequestResponseMetricsMiddleware {
            service: std::rc::Rc::new(service),
        }))
    }
}

pub struct RequestResponseMetricsMiddleware<S> {
    service: std::rc::Rc<S>,
}

impl<S, B> actix_web::dev::Service<actix_web::dev::ServiceRequest>
    for RequestResponseMetricsMiddleware<S>
where
    S: actix_web::dev::Service<
            actix_web::dev::ServiceRequest,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        > + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = futures::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: actix_web::dev::ServiceRequest) -> Self::Future {
        use std::borrow::Cow;

        let svc = self.service.clone();

        let request_path = req
            .match_pattern()
            .map(Cow::<'static, str>::from)
            .unwrap_or_else(|| "UNKNOWN".into());
        let request_method = Cow::<'static, str>::from(req.method().as_str().to_owned());

        Box::pin(async move {
            let mut attributes =
                router_env::metric_attributes!(("path", request_path), ("method", request_method))
                    .to_vec();

            let response_fut = svc.call(req);

            metrics::REQUESTS_RECEIVED.add(1, &attributes);

            let (response_result, request_duration) =
                common_utils::metrics::utils::time_future(response_fut).await;
            let response = response_result?;

            attributes.extend_from_slice(router_env::metric_attributes!((
                "status_code",
                i64::from(response.status().as_u16())
            )));

            metrics::REQUEST_TIME.record(request_duration.as_secs_f64(), &attributes);

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use actix_web::{http::StatusCode, test, web, App, HttpResponse};
    use bytes::Bytes;

    use super::*;

    fn streamed_payload(
        chunks: Vec<Result<Bytes, actix_web::error::PayloadError>>,
    ) -> actix_web::dev::Payload {
        let stream: actix_http::BoxedPayloadStream = Box::pin(futures::stream::iter(chunks));
        stream.into()
    }

    #[actix_web::test]
    async fn captures_complete_body_at_limit() {
        let payload = streamed_payload(vec![
            Ok(Bytes::from_static(b"abcd")),
            Ok(Bytes::from_static(b"efgh")),
        ]);
        let (mut payload, captured) = capture_request_payload(payload, 8);

        assert_eq!(captured.borrow().bytes_read, 0);
        assert!(!captured.borrow().complete);
        assert_eq!(payload.next().await.unwrap().unwrap(), b"abcd"[..]);
        assert_eq!(payload.next().await.unwrap().unwrap(), b"efgh"[..]);
        assert!(payload.next().await.is_none());
        let capture = captured.borrow();
        assert_eq!(capture.bytes.as_deref(), Some(&b"abcdefgh"[..]));
        assert_eq!(capture.bytes_read, 8);
        assert!(capture.complete);
    }

    #[actix_web::test]
    async fn zero_capture_limit_handles_empty_and_nonempty_bodies() {
        let (mut payload, captured) = capture_request_payload(streamed_payload(vec![]), 0);
        assert!(payload.next().await.is_none());
        assert_eq!(captured.borrow().bytes.as_deref(), Some(&b""[..]));
        assert!(captured.borrow().complete);

        let (mut payload, captured) = capture_request_payload(
            streamed_payload(vec![Ok(Bytes::from_static(b"larger than the limit"))]),
            0,
        );
        assert_eq!(
            payload.next().await.unwrap().unwrap(),
            b"larger than the limit"[..]
        );
        assert!(captured.borrow().bytes.is_none());
    }

    #[actix_web::test]
    async fn drops_capture_over_limit_and_keeps_forwarding_chunks() {
        let payload = streamed_payload(vec![
            Ok(Bytes::from_static(b"abcd")),
            Ok(Bytes::from_static(b"efgh")),
            Ok(Bytes::from_static(b"ij")),
        ]);
        let (mut payload, captured) = capture_request_payload(payload, 4);

        assert_eq!(payload.next().await.unwrap().unwrap(), b"abcd"[..]);
        assert!(captured.borrow().bytes.is_some());
        assert_eq!(payload.next().await.unwrap().unwrap(), b"efgh"[..]);
        assert!(captured.borrow().bytes.is_none());
        assert_eq!(payload.next().await.unwrap().unwrap(), b"ij"[..]);
        assert!(payload.next().await.is_none());
        assert!(captured.borrow().bytes.is_none());
        assert_eq!(captured.borrow().bytes_read, 10);
    }

    #[actix_web::test]
    async fn forwards_payload_errors_and_discards_partial_capture() {
        let payload = streamed_payload(vec![
            Ok(Bytes::from_static(b"abcd")),
            Err(actix_web::error::PayloadError::Incomplete(None)),
        ]);
        let (mut payload, captured) = capture_request_payload(payload, 8);

        assert!(payload.next().await.unwrap().is_ok());
        assert!(matches!(
            payload.next().await,
            Some(Err(actix_web::error::PayloadError::Incomplete(None)))
        ));
        assert!(captured.borrow().bytes.is_none());
        assert!(!captured.borrow().complete);
    }

    #[actix_web::test]
    async fn json_limit_rejects_without_draining_payload() {
        let app = test::init_service(
            App::new()
                .app_data(
                    web::JsonConfig::default()
                        .limit(16)
                        .error_handler(|err, _| actix_web::error::ErrorBadRequest(err)),
                )
                .wrap(Http400RequestDetailsLogger::new(16))
                .default_service(web::to(|_: web::Json<serde_json::Value>| async {
                    HttpResponse::Ok().finish()
                })),
        )
        .await;

        // Exercise streamed bodies, misleading lengths, and early header-based rejection.
        for content_length in [None, Some("1"), Some("1024")] {
            let chunks_read = Rc::new(Cell::new(0));
            let counter = chunks_read.clone();
            let stream: actix_http::BoxedPayloadStream =
                Box::pin(futures::stream::iter(0..1000).map(move |_| {
                    counter.set(counter.get() + 1);
                    assert!(counter.get() <= 3, "logger drained the oversized body");
                    Ok(Bytes::from_static(b"12345678"))
                }));
            let mut request = test::TestRequest::post()
                .insert_header((actix_web::http::header::CONTENT_TYPE, "application/json"))
                .to_request();
            request
                .extensions_mut()
                .insert(RequestId::try_from("test-request-id").expect("valid request identifier"));
            if let Some(length) = content_length {
                request.headers_mut().insert(
                    actix_web::http::header::CONTENT_LENGTH,
                    actix_web::http::header::HeaderValue::from_static(length),
                );
            }
            let (request, _) = request.replace_payload(stream.into());

            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert_eq!(
                chunks_read.get(),
                if content_length == Some("1024") { 0 } else { 3 }
            );
        }
    }

    #[actix_web::test]
    async fn does_not_read_body_when_handler_rejects_without_consuming_it() {
        let app = test::init_service(
            App::new()
                .wrap(Http400RequestDetailsLogger::new(16))
                .default_service(web::to(|| async { HttpResponse::BadRequest().finish() })),
        )
        .await;
        let stream: actix_http::BoxedPayloadStream = Box::pin(futures::stream::poll_fn(|_| {
            panic!("logger must not poll an unread request body")
        }));
        let request = test::TestRequest::post().to_request();
        request
            .extensions_mut()
            .insert(RequestId::try_from("test-request-id").expect("valid request identifier"));
        let (request, _) = request.replace_payload(stream.into());

        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn capture_limit_does_not_limit_raw_body_handlers() {
        let app = test::init_service(
            App::new()
                .app_data(web::PayloadConfig::new(64))
                .wrap(Http400RequestDetailsLogger::new(4))
                .default_service(web::to(|body: Bytes| async {
                    HttpResponse::BadRequest().body(body)
                })),
        )
        .await;
        let request = test::TestRequest::post()
            .set_payload("body larger than the capture limit")
            .to_request();
        request
            .extensions_mut()
            .insert(RequestId::try_from("test-request-id").expect("valid request identifier"));

        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            test::read_body(response).await,
            b"body larger than the capture limit"[..]
        );
    }
}
