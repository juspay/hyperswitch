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
        let mut req = req;
        let request_id_fut = req.extract::<router_env::tracing_actix_web::RequestId>();
        let response_fut = self.service.call(req);

        Box::pin(async move {
            let request_id = request_id_fut.await?;
            let mut response = response_fut.await?;
            response.headers_mut().append(
                http::header::HeaderName::from_static("x-request-id"),
                http::HeaderValue::from_str(&request_id.as_hyphenated().to_string())?,
            );

            Ok(response)
        })
    }
}
