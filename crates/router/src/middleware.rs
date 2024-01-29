use router_env::tracing::{field::Empty, Instrument};
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
                    router_env::logger::info!(?request_id, ?upstream_request_id);
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
                    connector_name = Empty,
                    flow = Empty
                )
                .or_current(),
            ),
        )
    }
}
