use futures::StreamExt;
use router_env::{
    logger,
    tracing::{field::Empty, Instrument},
};
/// Middleware to include request ID in response header.
pub struct RequestId;

impl<S, B> actix_web::dev::Transform<S, actix_web::dev::ServiceRequest> for RequestId
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
    type Transform = RequestIdMiddleware<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RequestIdMiddleware { service }))
    }
}

pub struct RequestIdMiddleware<S> {
    service: S,
}

impl<S, B> actix_web::dev::Service<actix_web::dev::ServiceRequest> for RequestIdMiddleware<S>
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

    fn call(&self, req: actix_web::dev::ServiceRequest) -> Self::Future {
        let old_x_request_id = req.headers().get("x-request-id").cloned();
        let mut req = req;
        let request_id_fut = req.extract::<router_env::tracing_actix_web::RequestId>();
        let response_fut = self.service.call(req);

        Box::pin(
            async move {
                let request_id = request_id_fut.await?;
                let request_id = request_id.as_hyphenated().to_string();
                if let Some(upstream_request_id) = old_x_request_id {
                    // router_env::logger::info!(?request_id, ?upstream_request_id);
                }
                let mut response = response_fut.await?;
                response.headers_mut().append(
                    http::header::HeaderName::from_static("x-request-id"),
                    http::HeaderValue::from_str(&request_id)?,
                );

                Ok(response)
            }
            .in_current_span(),
        )
    }
}

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
        let response_fut = self.service.call(req);

        Box::pin(
            response_fut.instrument(
                router_env::tracing::info_span!(
                    "golden_log_line",
                    payment_id = Empty,
                    merchant_id = Empty,
                    connector_name = Empty
                )
                .or_current(),
            ),
        )
    }
}

fn get_request_details_from_value(json_value: &serde_json::Value, parent_key: &str) -> String {
    match json_value {
        serde_json::Value::Null => format!("{}: null", parent_key),
        serde_json::Value::Bool(b) => format!("{}: {}", parent_key, b),
        serde_json::Value::Number(num) => format!("{}: {}", parent_key, num.to_string().len()),
        serde_json::Value::String(s) => format!("{}: {}", parent_key, s.len()),
        serde_json::Value::Array(arr) => {
            let mut result = String::new();
            for (index, value) in arr.iter().enumerate() {
                let child_key = format!("{}[{}]", parent_key, index);
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
                let child_key = format!("{}[{}]", parent_key, key);
                result.push_str(&get_request_details_from_value(value, &child_key));
                if index < obj.len() - 1 {
                    result.push_str(", ");
                }
            }
            result
        }
    }
}

/// Middleware for Logging request_details of HTTP 400 Bad Requests
pub struct Http400RequestDetailsLogger;

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
        }))
    }
}

pub struct Http400RequestDetailsLoggerMiddleware<S> {
    service: std::rc::Rc<S>,
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
        let request_id_fut = req.extract::<router_env::tracing_actix_web::RequestId>();
        Box::pin(async move {
            let (http_req, payload) = req.into_parts();
            let result_payload: Vec<Result<bytes::Bytes, actix_web::error::PayloadError>> =
                payload.collect().await;
            let payload = result_payload
                .into_iter()
                .collect::<Result<Vec<bytes::Bytes>, actix_web::error::PayloadError>>()?;
            let bytes = payload.clone().concat().to_vec();
            // we are creating h1 payload manually from bytes, currently there's no way to create http2 payload with actix
            let (_, mut new_payload) = actix_http::h1::Payload::create(true);
            new_payload.unread_data(bytes.to_vec().clone().into());
            let new_req = actix_web::dev::ServiceRequest::from_parts(http_req, new_payload.into());
            let response_fut = svc.call(new_req);
            let response = response_fut.await?;
            // Log the request_details when we receive 400 status from the application
            if response.status() == 400 {
                let value: serde_json::Value = serde_json::from_slice(&bytes)?;
                let request_id = request_id_fut.await?.as_hyphenated().to_string();
                logger::info!(
                    "request_id: {}, request_details: {}",
                    request_id,
                    get_request_details_from_value(&value, "")
                );
            }
            Ok(response)
        })
    }
}
