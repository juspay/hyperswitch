#![warn(unused, missing_docs)]
//! Request ID middleware for Actix Web applications.
//!
//! This module provides middleware to associate every HTTP request with a unique identifier.
//! The request ID can be used for distributed tracing, error tracking, and request correlation
//! across microservices.
//!
//! # Features
//!
//! - **Automatic UUID v7 generation**: Uses time-ordered UUIDs for better performance
//! - **Header reuse**: Respects incoming request ID headers for request correlation
//! - **Configurable headers**: Custom header names supported for any request ID header
//! - **Request extraction**: RequestId can be extracted in route handlers
//! - **Response headers**: Automatically adds request ID to response headers
//! - **Upstream logging**: Logs incoming request IDs for debugging
//!
//! # Examples
//!
//! ```rust
//! use router_env::{RequestIdentifier, IdReuse};
//! use actix_web::{web, App, HttpServer};
//!
//! let app = App::new()
//!     .wrap(
//!         RequestIdentifier::new("x-request-id")
//!             .use_incoming_id(IdReuse::UseIncoming)
//!     );
//! ```

use std::{
    fmt::{self, Display, Formatter},
    future::{ready, Future, Ready},
    pin::Pin,
    str::FromStr,
    sync::Arc,
    task::{Context, Poll},
};

#[cfg(feature = "deja")]
use actix_web::body::EitherBody;
use actix_web::{
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform},
    error::ResponseError,
    http::header::{HeaderName, HeaderValue},
    Error as ActixError, FromRequest, HttpMessage, HttpRequest,
};
use error_stack::{report, ResultExt};
#[cfg(feature = "deja")]
use tracing::Instrument;
use uuid::Uuid;

#[cfg(feature = "deja")]
pub(crate) mod semantic_boundary {
    use std::{
        fmt,
        future::Future,
        panic::Location,
        pin::Pin,
        sync::{Arc, OnceLock},
        task::{Context, Poll},
    };

    use actix_web::{
        body::{BodySize, MessageBody},
        dev::{Payload, ServiceRequest, ServiceResponse},
        http::header::{HeaderMap, CONTENT_LENGTH, CONTENT_TYPE},
        Error as ActixError,
    };
    use bytes::Bytes;
    use deja::DejaHook;
    use serde_json::json;

    static HOOK: OnceLock<Option<Arc<deja::RuntimeHook>>> = OnceLock::new();

    fn hook() -> Option<&'static Arc<deja::RuntimeHook>> {
        HOOK.get_or_init(|| deja::global_runtime_hook_from_env())
            .as_ref()
    }

    pub(super) fn is_active() -> bool {
        hook().is_some_and(|hook| hook.is_active())
    }

    pub(super) fn record_id_generation(
        method_name: &'static str,
        caller: &'static Location<'static>,
        request: serde_json::Value,
        response: serde_json::Value,
    ) {
        let Some(hook) = hook() else {
            return;
        };
        if !hook.is_active() {
            return;
        }
        let event = deja::EventBuilder::start(
            hook.as_ref(),
            "id_generation",
            "router_env::request_id",
            method_name,
            caller,
            request,
        );
        event.finish(hook.as_ref(), response, false);
    }

    /// Replay substitution for id generation.
    ///
    /// In replay mode the active hook is a lookup-table hook; this returns the
    /// recorded id for this call site so the candidate reproduces the recorded
    /// run byte-for-byte instead of minting a fresh uuid. The hook emits an
    /// `ObservedCall` either way (hit or miss) for the divergence scorer.
    ///
    /// Returns `None` when recording (so the caller generates live) or on a
    /// replay miss (the scored divergence; the caller then generates live too).
    /// The recorded result shape mirrors `record_id_generation`'s `response`:
    /// `{"generated_value": "<id>"}`.
    pub(super) fn replay_id_generation(
        method_name: &'static str,
        caller: &'static Location<'static>,
        request: &serde_json::Value,
    ) -> Option<String> {
        let hook = hook()?;
        if !hook.is_active() {
            return None;
        }
        let recorded = hook.try_replay_with_context(deja::ReplayLookup {
            boundary: "id_generation",
            trait_name: "router_env::request_id",
            method_name,
            args: request,
            callsite_identity: None,
            caller_location: Some(caller),
        })?;
        recorded
            .get("generated_value")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
    }

    #[derive(Debug)]
    pub(super) struct IncomingHttpRecord {
        method: String,
        path: String,
        query: String,
        request_id: String,
        headers: serde_json::Value,
        content_type: Option<String>,
        content_length: Option<u64>,
        request_body: serde_json::Value,
    }

    impl IncomingHttpRecord {
        fn new(
            method: &str,
            path: &str,
            query: &str,
            request_id: &str,
            headers: serde_json::Value,
            content_type: Option<String>,
            content_length: Option<u64>,
            request_body: serde_json::Value,
        ) -> Self {
            Self {
                method: method.to_string(),
                path: path.to_string(),
                query: query.to_string(),
                request_id: request_id.to_string(),
                headers,
                content_type,
                content_length,
                request_body,
            }
        }

        fn args(&self) -> serde_json::Value {
            json!({
                "method": self.method.as_str(),
                "path": self.path.as_str(),
                "query": self.query.as_str(),
                "request_id": self.request_id.as_str(),
                "headers": self.headers.clone(),
                "content_type": self.content_type.as_deref(),
                "content_length": self.content_length,
                "request_body": self.request_body.clone(),
            })
        }
    }

    pub(super) async fn capture_incoming_request(
        mut request: ServiceRequest,
        request_id: &str,
    ) -> (ServiceRequest, IncomingHttpRecord) {
        let method = request.method().as_str().to_string();
        let path = request.path().to_string();
        let query = request.query_string().to_string();
        let headers = headers_json(request.headers());
        let content_type = request
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let content_length = request
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());

        let request_body = match request.extract::<Bytes>().await {
            Ok(bytes) => {
                let body = body_json(&bytes);
                request.set_payload(Payload::from(bytes));
                body
            }
            Err(error) => json!({
                "captured": false,
                "error": error.to_string(),
            }),
        };

        let record = IncomingHttpRecord::new(
            &method,
            &path,
            &query,
            request_id,
            headers,
            content_type,
            content_length,
            request_body,
        );
        (request, record)
    }

    fn headers_json(headers: &HeaderMap) -> serde_json::Value {
        deja::http::headers(headers.iter().map(|(name, value)| {
            let value = value
                .to_str()
                .map(str::to_string)
                .unwrap_or_else(|_| format!("{value:?}"));
            (name.as_str().to_string(), value)
        }))
    }

    fn body_json(bytes: &Bytes) -> serde_json::Value {
        deja::http::body(bytes)
    }

    // -----------------------------------------------------------------------
    // RecordingBody — wraps an Actix body to buffer chunks and finalize the
    // boundary event only after the full body has been streamed.
    // -----------------------------------------------------------------------
    pub struct RecordingBody<B> {
        inner: Pin<Box<B>>,
        finalizer: Option<deja::LazyEventFinalizer>,
    }

    impl<B> fmt::Debug for RecordingBody<B> {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter
                .debug_struct("RecordingBody")
                .field("has_finalizer", &self.finalizer.is_some())
                .finish_non_exhaustive()
        }
    }

    impl<B> RecordingBody<B> {
        fn new(body: B, finalizer: deja::LazyEventFinalizer) -> Self {
            Self {
                inner: Box::pin(body),
                finalizer: Some(finalizer),
            }
        }

        pub(super) fn from_body(body: B) -> Self {
            Self {
                inner: Box::pin(body),
                finalizer: None,
            }
        }
    }

    // Pin<Box<B>> is always Unpin, so RecordingBody is always Unpin regardless of B.
    impl<B> Unpin for RecordingBody<B> {}

    impl<B: MessageBody> MessageBody for RecordingBody<B> {
        type Error = B::Error;

        fn size(&self) -> BodySize {
            self.inner.size()
        }

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Bytes, Self::Error>>> {
            // Safe because RecordingBody<B> is Unpin.
            let this = Pin::get_mut(self);

            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(None) => {
                    if let Some(finalizer) = this.finalizer.take() {
                        finalizer.finalize();
                    }
                    Poll::Ready(None)
                }
                Poll::Ready(Some(Ok(chunk))) => {
                    if let Some(finalizer) = this.finalizer.as_mut() {
                        finalizer.capture_chunk(&chunk);
                    }
                    Poll::Ready(Some(Ok(chunk)))
                }
                other => other,
            }
        }
    }

    // -----------------------------------------------------------------------
    // RecordedIncomingFuture — polls the inner service, then swaps the
    // response body for a RecordingBody so the event finalizes lazily.
    // -----------------------------------------------------------------------
    pub(super) fn recorded_incoming<F, B>(
        future: F,
        record: IncomingHttpRecord,
    ) -> RecordedIncomingFuture<B>
    where
        F: Future<Output = Result<ServiceResponse<B>, ActixError>> + 'static,
        B: 'static,
    {
        RecordedIncomingFuture {
            inner: Box::pin(future),
            record: Some(record),
            event: None,
            caller: Location::caller(),
        }
    }

    pub(super) struct RecordedIncomingFuture<B> {
        inner: Pin<Box<dyn Future<Output = Result<ServiceResponse<B>, ActixError>> + 'static>>,
        record: Option<IncomingHttpRecord>,
        event: Option<deja::EventBuilder>,
        caller: &'static Location<'static>,
    }

    impl<B: MessageBody + 'static> Future for RecordedIncomingFuture<B> {
        type Output = Result<ServiceResponse<RecordingBody<B>>, ActixError>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.event.is_none() {
                if let (Some(hook), Some(record)) = (hook(), self.record.as_ref()) {
                    if hook.is_active() {
                        let args = record.args();
                        self.event = Some(deja::EventBuilder::start(
                            hook.as_ref(),
                            "http_incoming",
                            "RequestIdMiddleware",
                            "call",
                            self.caller,
                            args,
                        ));
                    }
                }
            }

            let result = match self.inner.as_mut().poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(result) => result,
            };

            match result {
                Ok(response) => {
                    let partial = if let Some(record) = self.record.take() {
                        partial_result_json(&record, &response)
                    } else {
                        json!({})
                    };

                    let event = self.event.take();
                    let finalizer = if let (Some(event), Some(hook)) = (event, hook()) {
                        let hook_clone: Arc<deja::RuntimeHook> = Arc::clone(hook);
                        let hook_dyn: Arc<dyn deja::DejaHook> = hook_clone;
                        Some(deja::LazyEventFinalizer::new(
                            event, hook_dyn, partial, false,
                        ))
                    } else {
                        None
                    };

                    let mapped = response.map_body(|_head, body| {
                        if let Some(finalizer) = finalizer {
                            RecordingBody::new(body, finalizer)
                        } else {
                            RecordingBody {
                                inner: Box::pin(body),
                                finalizer: None,
                            }
                        }
                    });

                    Poll::Ready(Ok(mapped))
                }
                Err(error) => {
                    if let (Some(hook), Some(event), Some(record)) =
                        (hook(), self.event.take(), self.record.take())
                    {
                        event.finish(hook.as_ref(), error_result_json(&record, &error), true);
                    }
                    Poll::Ready(Err(error))
                }
            }
        }
    }

    fn partial_result_json<B>(
        record: &IncomingHttpRecord,
        response: &ServiceResponse<B>,
    ) -> serde_json::Value {
        json!({
            "method": record.method.as_str(),
            "path": record.path.as_str(),
            "query": record.query.as_str(),
            "request_id": record.request_id.as_str(),
            "status": response.status().as_u16(),
        })
    }

    fn error_result_json(record: &IncomingHttpRecord, error: &ActixError) -> serde_json::Value {
        let status = error.as_response_error().status_code();
        json!({
            "method": record.method.as_str(),
            "path": record.path.as_str(),
            "query": record.query.as_str(),
            "request_id": record.request_id.as_str(),
            "status": status.as_u16(),
            "error": error.to_string(),
        })
    }
}

/// Custom result type for request ID operations.
pub type RequestIdResult<T> = Result<T, error_stack::Report<RequestIdError>>;

/// Errors that can occur when working with request IDs.
#[derive(Debug, Clone)]
pub enum RequestIdError {
    /// No request ID is associated with the current request.
    NoAssociatedId,
    /// Failed to convert header value to request ID.
    InvalidHeaderValue {
        /// The invalid header value that caused the error.
        value: String,
    },
}

impl error_stack::Context for RequestIdError {}

impl ResponseError for RequestIdError {}

impl Display for RequestIdError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAssociatedId => write!(fmt, "No request ID associated with this request"),
            Self::InvalidHeaderValue { value } => write!(fmt, "Invalid header value: {}", value),
        }
    }
}

/// Configuration for handling incoming request ID headers.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdReuse {
    /// Reuse the incoming request ID if present, otherwise generate a new one.
    UseIncoming,
    /// Always generate a new request ID, ignoring any incoming headers.
    #[default]
    IgnoreIncoming,
}

/// Generate a new UUID v7 request ID.
#[track_caller]
fn generate_uuid_v7() -> String {
    #[cfg(feature = "deja")]
    {
        // Only INTERNAL generations — fired inside a correlated request, so
        // `current_correlation_id()` is `Some` — are instrumented as entropy seams
        // (recorded + substituted so a mid-request sub-id reproduces byte-exact).
        //
        // The TOP-LEVEL bootstrap request-id is minted in the ingress middleware
        // BEFORE the correlation scope exists (`current_correlation_id() == None`).
        // It is not a standalone seam: it IS the `http_incoming` boundary's
        // identity, captured there and anchored on replay via the kernel's
        // x-request-id injection (the candidate reuses it instead of regenerating).
        // Recording it separately only produced omitted-on-replay noise, so we fold
        // it into `http_incoming` by generating it live without recording.
        if deja::__private::current_correlation_id().is_some() {
            let caller = std::panic::Location::caller();
            let request = serde_json::json!({ "source": "uuid_v7" });

            // REPLAY: serve the recorded id for this call site (correlated, so it
            // matches robustly). A miss falls through to live generation.
            if let Some(recorded) =
                semantic_boundary::replay_id_generation("generate_uuid_v7", caller, &request)
            {
                return recorded;
            }

            // RECORD (or replay miss): generate live and record the value.
            let generated_value = Uuid::now_v7().to_string();
            semantic_boundary::record_id_generation(
                "generate_uuid_v7",
                caller,
                request,
                serde_json::json!({ "generated_value": generated_value.clone() }),
            );
            return generated_value;
        }

        // Bootstrap / uncorrelated: generate live; identity folds into http_incoming.
        Uuid::now_v7().to_string()
    }

    #[cfg(not(feature = "deja"))]
    {
        Uuid::now_v7().to_string()
    }
}

/// Request ID middleware that takes a configurable header name
/// and determines how to handle incoming request IDs.
#[derive(Clone, Debug)]
pub struct RequestIdentifier {
    header_name: String,
    use_incoming_id: IdReuse,
}

/// Request ID value that can be extracted in route handlers.
///
/// Wraps an `Arc<str>` for optimal performance in web middleware.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct RequestId(Arc<str>);

/// The middleware implementation that processes requests.
#[derive(Debug)]
pub struct RequestIdMiddleware<S> {
    service: Arc<S>,
    header_name: HeaderName,
    use_incoming_id: IdReuse,
}

impl Display for RequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RequestId {
    type Err = error_stack::Report<RequestIdError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err(report!(RequestIdError::InvalidHeaderValue {
                value: s.to_string(),
            }))
        } else {
            Ok(Self(s.into()))
        }
    }
}

impl TryFrom<HeaderValue> for RequestId {
    type Error = error_stack::Report<RequestIdError>;

    fn try_from(value: HeaderValue) -> Result<Self, Self::Error> {
        let s = value
            .to_str()
            .change_context(RequestIdError::InvalidHeaderValue {
                value: format!("{:?}", value),
            })?;
        Self::from_str(s)
    }
}

impl From<RequestId> for String {
    fn from(request_id: RequestId) -> Self {
        request_id.0.to_string()
    }
}

impl TryFrom<String> for RequestId {
    type Error = error_stack::Report<RequestIdError>;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(s.as_str())
    }
}

impl TryFrom<&str> for RequestId {
    type Error = error_stack::Report<RequestIdError>;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl RequestId {
    /// Generate a new request ID using UUID v7.
    #[track_caller]
    pub fn new_generated() -> Self {
        Self(generate_uuid_v7().into())
    }

    /// Extract request ID from ServiceRequest header or generate UUID v7.
    ///
    /// This is the core logic: try to extract from the specified header,
    /// if not possible or not desired, generate a new UUID v7.
    #[track_caller]
    pub fn extract_or_generate(
        request: &ServiceRequest,
        header_name: &HeaderName,
        use_incoming_id: IdReuse,
    ) -> Self {
        let request_id_string = match use_incoming_id {
            IdReuse::UseIncoming => {
                // Try to extract from incoming header
                if let Some(existing_header) = request.headers().get(header_name) {
                    Self::try_from(existing_header.clone())
                        .map(|id| id.0.to_string())
                        .unwrap_or_else(|e| {
                            tracing::warn!(
                                error = %e,
                                "Invalid request ID header, generating new UUID v7"
                            );
                            generate_uuid_v7()
                        })
                } else {
                    // No header found, generate new UUID v7
                    generate_uuid_v7()
                }
            }
            IdReuse::IgnoreIncoming => {
                // Always generate new UUID v7
                generate_uuid_v7()
            }
        };

        Self(request_id_string.into())
    }

    /// Convert this request ID to a `HeaderValue`.
    pub fn to_header_value(&self) -> RequestIdResult<HeaderValue> {
        HeaderValue::from_str(&self.0).change_context(RequestIdError::InvalidHeaderValue {
            value: self.0.to_string(),
        })
    }

    /// Get a string representation of this request ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl RequestIdentifier {
    /// Create a request ID middleware with a custom header name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use router_env::RequestIdentifier;
    ///
    /// // Use any custom header name for request ID extraction
    /// let middleware = RequestIdentifier::new("x-request-id");
    /// ```
    #[must_use]
    pub fn new(header_name: &str) -> Self {
        Self {
            header_name: header_name.to_string(),
            use_incoming_id: IdReuse::default(),
        }
    }

    /// Configure how incoming request ID headers should be handled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use router_env::{RequestIdentifier, IdReuse};
    ///
    /// let middleware = RequestIdentifier::new("x-request-id")
    ///     .use_incoming_id(IdReuse::IgnoreIncoming);
    /// ```
    #[must_use]
    pub fn use_incoming_id(self, use_incoming_id: IdReuse) -> Self {
        Self {
            use_incoming_id,
            ..self
        }
    }

    /// Get the header name used for request ID extraction.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use router_env::RequestIdentifier;
    ///
    /// let identifier = RequestIdentifier::new("x-request-id");
    /// assert_eq!(identifier.header_name(), "x-request-id");
    /// ```
    pub fn header_name(&self) -> &str {
        &self.header_name
    }

    /// Get the configured strategy for reusing incoming request IDs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use router_env::{IdReuse, RequestIdentifier};
    ///
    /// let identifier = RequestIdentifier::new("x-request-id")
    ///     .use_incoming_id(IdReuse::IgnoreIncoming);
    /// assert_eq!(identifier.id_reuse_strategy(), IdReuse::IgnoreIncoming);
    /// ```
    pub fn id_reuse_strategy(&self) -> IdReuse {
        self.use_incoming_id
    }
}

impl IdReuse {
    /// Reuse the existing request ID or create a new one based on the configured strategy.
    ///
    /// Returns the request ID and a flag indicating whether it was newly generated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use router_env::{IdReuse, RequestId};
    ///
    /// let existing = RequestId::new_generated();
    /// let (id, generated) = IdReuse::UseIncoming.get_or_create_request_id(Some(&existing));
    /// assert_eq!(id, existing);
    /// assert!(!generated);
    ///
    /// let (_id, generated) = IdReuse::IgnoreIncoming.get_or_create_request_id(None);
    /// assert!(generated);
    /// ```
    #[track_caller]
    pub fn get_or_create_request_id(&self, existing: Option<&RequestId>) -> (RequestId, bool) {
        match self {
            Self::UseIncoming => existing
                .cloned()
                .map(|id| (id, false))
                .unwrap_or_else(|| (RequestId::new_generated(), true)),
            Self::IgnoreIncoming => (RequestId::new_generated(), true),
        }
    }
}

// ---------------------------------------------------------------------------
// Transform / Service — note that when deja is active the response body type
// becomes RecordingBody<B> so that body bytes can be captured lazily.
// ---------------------------------------------------------------------------

#[cfg(feature = "deja")]
impl<S, B> Transform<S, ServiceRequest> for RequestIdentifier
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    S::Future: 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<semantic_boundary::RecordingBody<B>, B>>;
    type Error = S::Error;
    type Transform = RequestIdMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let header_name = HeaderName::from_str(&self.header_name).unwrap_or_else(|_| {
            tracing::error!(
                "Invalid header name '{}', using fallback 'x-request-id'",
                self.header_name
            );
            HeaderName::from_static("x-request-id")
        });

        ready(Ok(RequestIdMiddleware {
            service: Arc::new(service),
            header_name,
            use_incoming_id: self.use_incoming_id,
        }))
    }
}

#[cfg(not(feature = "deja"))]
impl<S, B> Transform<S, ServiceRequest> for RequestIdentifier
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Transform = RequestIdMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let header_name = HeaderName::from_str(&self.header_name).unwrap_or_else(|_| {
            tracing::error!(
                "Invalid header name '{}', using fallback 'x-request-id'",
                self.header_name
            );
            HeaderName::from_static("x-request-id")
        });

        ready(Ok(RequestIdMiddleware {
            service: Arc::new(service),
            header_name,
            use_incoming_id: self.use_incoming_id,
        }))
    }
}

#[allow(clippy::type_complexity)]
#[cfg(feature = "deja")]
impl<S, B> Service<ServiceRequest> for RequestIdMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    S::Future: 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<semantic_boundary::RecordingBody<B>, B>>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let service = Arc::clone(&self.service);
        let header_name = self.header_name.clone();

        let incoming_request_id = request.headers().get(&header_name).cloned();
        let request_id =
            RequestId::extract_or_generate(&request, &header_name, self.use_incoming_id);
        let header_value = request_id.to_header_value().unwrap_or_else(|e| {
            tracing::error!(
                error = %e,
                request_id = %request_id,
                "Failed to convert request ID to header value"
            );
            HeaderValue::from_static("invalid-request-id")
        });
        request.extensions_mut().insert(request_id.clone());

        #[cfg(feature = "deja")]
        let (span_method, span_path) = (
            request.method().as_str().to_owned(),
            request.path().to_owned(),
        );

        Box::pin(async move {
            if let Some(upstream_request_id) = incoming_request_id {
                tracing::debug!(
                    ?upstream_request_id,
                    generated_request_id = %request_id,
                    "Received upstream request ID for correlation"
                );
            }

            let mut response: ServiceResponse<EitherBody<semantic_boundary::RecordingBody<B>, B>> =
                if semantic_boundary::is_active() {
                    let (request, incoming_record) =
                        semantic_boundary::capture_incoming_request(request, request_id.as_str())
                            .await;
                    let fut = service.call(request);
                    let request_span = tracing::info_span!(
                        "deja::http_incoming",
                        method = %span_method,
                        path = %span_path,
                        request_id = %request_id,
                    );
                    let recorded = deja::__private::scope_correlation(
                        request_id.to_string(),
                        semantic_boundary::recorded_incoming(fut, incoming_record)
                            .instrument(request_span),
                    )
                    .await?;
                    recorded.map_body(|_head, body| EitherBody::left(body))
                } else {
                    let resp = service.call(request).await?;
                    resp.map_body(|_head, body| EitherBody::right(body))
                };

            response.headers_mut().insert(header_name, header_value);
            Ok(response)
        })
    }
}

#[allow(clippy::type_complexity)]
#[cfg(not(feature = "deja"))]
impl<S, B> Service<ServiceRequest> for RequestIdMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let service = Arc::clone(&self.service);
        let header_name = self.header_name.clone();

        let incoming_request_id = request.headers().get(&header_name).cloned();
        let request_id =
            RequestId::extract_or_generate(&request, &header_name, self.use_incoming_id);
        let header_value = request_id.to_header_value().unwrap_or_else(|e| {
            tracing::error!(
                error = %e,
                request_id = %request_id,
                "Failed to convert request ID to header value"
            );
            HeaderValue::from_static("invalid-request-id")
        });
        request.extensions_mut().insert(request_id.clone());

        Box::pin(async move {
            if let Some(upstream_request_id) = incoming_request_id {
                tracing::debug!(
                    ?upstream_request_id,
                    generated_request_id = %request_id,
                    "Received upstream request ID for correlation"
                );
            }

            let mut response = service.call(request).await?;
            response.headers_mut().insert(header_name, header_value);
            Ok(response)
        })
    }
}

impl FromRequest for RequestId {
    type Error = RequestIdError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(
            req.extensions()
                .get::<Self>()
                .cloned()
                .ok_or(RequestIdError::NoAssociatedId),
        )
    }
}
