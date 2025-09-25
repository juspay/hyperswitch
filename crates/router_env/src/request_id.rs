#![deny(unused, missing_docs)]
//! Request ID middleware for Actix Web applications.
//!
//! This module provides middleware to associate every HTTP request with a unique identifier.
//! The request ID can be used for distributed tracing, error tracking, and request correlation
//! across microservices.
//!
//! # Features
//!
//! - **Automatic UUID generation**: Uses UUID v7 by default (time-ordered) for better performance
//! - **Header reuse**: Respects incoming `X-Request-Id` headers for request correlation
//! - **Configurable headers**: Custom header names supported
//! - **Multiple UUID versions**: UUID v4 support via feature flag
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
//!         RequestIdentifier::with_uuid()
//!             .use_incoming_id(IdReuse::UseIncoming)
//!     );
//! ```

use std::{
    fmt::{self, Display, Formatter},
    future::{ready, Future, Ready},
    pin::Pin,
    task::{Context, Poll},
};

use actix_web::{
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform},
    error::ResponseError,
    http::header::{HeaderName, HeaderValue},
    Error as ActixError, FromRequest, HttpMessage, HttpRequest,
};
use uuid::Uuid;

/// The default header name used for the request ID.
///
/// This follows the common convention used by load balancers and reverse proxies.
pub const DEFAULT_HEADER: &str = "x-request-id";

/// Errors that can occur when working with request IDs.
#[derive(Debug, Clone)]
pub enum Error {
    /// No request ID is associated with the current request.
    ///
    /// This typically occurs when the RequestId extractor is used without
    /// the RequestIdentifier middleware being properly configured.
    NoAssociatedId,
}

impl error_stack::Context for Error {}

impl ResponseError for Error {}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoAssociatedId => write!(fmt, "No request ID associated with this request"),
        }
    }
}

/// Configuration for handling incoming request ID headers.
///
/// This determines whether the middleware should reuse request IDs from incoming
/// headers or always generate new ones.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum IdReuse {
    /// Reuse the incoming request ID if present, otherwise generate a new one.
    ///
    /// This is the recommended default for most applications as it preserves
    /// request correlation across service boundaries.
    #[default]
    UseIncoming,
    /// Always generate a new request ID, ignoring any incoming headers.
    ///
    /// This can be useful for security-sensitive applications that don't want
    /// to trust externally provided request IDs.
    IgnoreIncoming,
}

/// Function type for generating request ID header values.
///
/// Generator functions should return unique, ASCII-only header values.
/// The returned `HeaderValue` will be used both for internal tracking
/// and as the response header value.
type Generator = fn() -> HeaderValue;

/// Configuration builder for the request ID middleware.
///
/// This struct provides a fluent interface for configuring how request IDs
/// are generated and handled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RequestIdentifier {
    header_name: &'static str,
    id_generator: Generator,
    use_incoming_id: IdReuse,
}

/// Request ID value that can be extracted in route handlers.
///
/// This wraps a `HeaderValue` and provides convenient methods for accessing
/// the request ID as a string.
///
/// # Examples
///
/// ```rust
/// use actix_web::{web, HttpResponse, Result};
/// use router_env::RequestId;
///
/// async fn handler(request_id: RequestId) -> Result<HttpResponse> {
///     println!("Processing request: {}", request_id);
///     Ok(HttpResponse::Ok().json(format!("Request ID: {}", request_id)))
/// }
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RequestId(HeaderValue);

/// The actual middleware implementation that processes requests.
///
/// This is created by the `RequestIdentifier` configuration and handles
/// the actual request processing logic.
#[derive(Debug)]
pub struct RequestIdMiddleware<S> {
    service: S,
    header_name: HeaderName,
    id_generator: Generator,
    use_incoming_id: IdReuse,
}

impl Display for RequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl RequestId {
    /// Get the raw header value for this request ID.
    ///
    /// This can be useful when you need to pass the request ID to other
    /// HTTP clients or services that expect a `HeaderValue`.
    pub const fn header_value(&self) -> &HeaderValue {
        &self.0
    }

    /// Get a string representation of this request ID.
    ///
    /// # Panics
    ///
    /// Panics if the header value contains non-ASCII characters. This should
    /// never happen with properly generated UUIDs, but could occur if external
    /// request IDs contain invalid characters.
    pub fn as_str(&self) -> &str {
        self.0
            .to_str()
            .expect("Request ID contains non-ASCII characters")
    }
}

impl RequestIdentifier {
    /// Create a request ID middleware with default UUID generation.
    ///
    /// By default, this uses UUID v7 for time-ordered request IDs, which provides
    /// better database performance and natural sorting. If the `uuid-v4-generator`
    /// feature is enabled, it will use UUID v4 instead.
    ///
    /// Uses the default header name [`DEFAULT_HEADER`] and [`IdReuse::UseIncoming`].
    #[must_use]
    pub fn with_uuid() -> Self {
        Self::default()
    }

    /// Create a request ID middleware with explicit UUID v7 generation.
    ///
    /// This explicitly uses UUID v7 regardless of feature flags. UUID v7 provides
    /// time-ordered identifiers that are more efficient for database indexing
    /// and provide natural chronological sorting.
    #[must_use]
    pub fn with_uuid_v7() -> Self {
        Self::with_generator(uuid_v7_generator)
    }

    /// Create a request ID middleware with a custom header name.
    ///
    /// Uses the default UUID generator (v7 unless `uuid-v4-generator` feature is enabled)
    /// and [`IdReuse::UseIncoming`] behavior.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use router_env::RequestIdentifier;
    ///
    /// let middleware = RequestIdentifier::with_header("x-trace-id");
    /// ```
    #[must_use]
    pub fn with_header(header_name: &'static str) -> Self {
        Self {
            header_name,
            ..Default::default()
        }
    }

    /// Change the header name for this middleware configuration.
    ///
    /// This allows customizing which HTTP header is used for the request ID.
    /// Common alternatives include `x-trace-id`, `x-correlation-id`, or
    /// service-specific headers.
    #[must_use]
    pub const fn header(self, header_name: &'static str) -> Self {
        Self {
            header_name,
            ..self
        }
    }

    /// Create a request ID middleware with a custom ID generator.
    ///
    /// The generator function should return unique, ASCII-only header values.
    /// This is useful for integrating with existing ID generation systems
    /// or implementing custom ID formats.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use actix_web::http::header::HeaderValue;
    /// use router_env::RequestIdentifier;
    ///
    /// fn custom_generator() -> HeaderValue {
    ///     HeaderValue::from_static("custom-id-123")
    /// }
    ///
    /// let middleware = RequestIdentifier::with_generator(custom_generator);
    /// ```
    #[must_use]
    pub fn with_generator(id_generator: Generator) -> Self {
        Self {
            id_generator,
            header_name: DEFAULT_HEADER,
            use_incoming_id: IdReuse::default(),
        }
    }

    /// Change the ID generator for this middleware configuration.
    ///
    /// This allows switching between different UUID versions or implementing
    /// completely custom ID generation logic.
    #[must_use]
    pub fn generator(self, id_generator: Generator) -> Self {
        Self {
            id_generator,
            ..self
        }
    }

    /// Configure how incoming request ID headers should be handled.
    ///
    /// - [`IdReuse::UseIncoming`] (default): Reuse incoming request IDs when present
    /// - [`IdReuse::IgnoreIncoming`]: Always generate new request IDs
    ///
    /// # Examples
    ///
    /// ```rust
    /// use router_env::{RequestIdentifier, IdReuse};
    ///
    /// // Always generate new IDs (ignore incoming headers)
    /// let middleware = RequestIdentifier::with_uuid()
    ///     .use_incoming_id(IdReuse::IgnoreIncoming);
    /// ```
    #[must_use]
    pub fn use_incoming_id(self, use_incoming_id: IdReuse) -> Self {
        Self {
            use_incoming_id,
            ..self
        }
    }
}

/// Default configuration uses UUID v7 generation.
///
/// UUID v7 provides time-ordered identifiers that are more efficient for
/// database operations and provide natural chronological sorting.
#[cfg(not(feature = "uuid-v4-generator"))]
impl Default for RequestIdentifier {
    fn default() -> Self {
        Self {
            header_name: DEFAULT_HEADER,
            id_generator: uuid_v7_generator,
            use_incoming_id: IdReuse::default(),
        }
    }
}

/// When the `uuid-v4-generator` feature is enabled, use UUID v4 by default.
///
/// UUID v4 provides fully random identifiers without timestamp information.
#[cfg(feature = "uuid-v4-generator")]
impl Default for RequestIdentifier {
    fn default() -> Self {
        Self {
            header_name: DEFAULT_HEADER,
            id_generator: uuid_v4_generator,
            use_incoming_id: IdReuse::default(),
        }
    }
}

/// Generate a UUID v4 based request ID.
///
/// UUID v4 uses random numbers and provides no ordering guarantees.
/// This generator is only available when the `uuid-generator` feature is enabled.
#[cfg(feature = "uuid-generator")]
fn uuid_v4_generator() -> HeaderValue {
    let uuid = Uuid::new_v4();
    HeaderValue::from_str(&uuid.to_string())
        // This unwrap is safe because UUID v4 strings are always ASCII
        .expect("UUID v4 should always be valid ASCII")
}

/// Generate a UUID v7 based request ID.
///
/// UUID v7 includes timestamp information, making the IDs naturally ordered
/// by creation time. This provides better database performance and enables
/// chronological sorting without additional metadata.
fn uuid_v7_generator() -> HeaderValue {
    let uuid = Uuid::now_v7();
    HeaderValue::from_str(&uuid.to_string())
        // This unwrap is safe because UUID v7 strings are always ASCII
        .expect("UUID v7 should always be valid ASCII")
}

impl<S, B> Transform<S, ServiceRequest> for RequestIdentifier
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Transform = RequestIdMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddleware {
            service,
            header_name: HeaderName::from_static(self.header_name),
            id_generator: self.id_generator,
            use_incoming_id: self.use_incoming_id,
        }))
    }
}

#[allow(clippy::type_complexity)]
impl<S, B> Service<ServiceRequest> for RequestIdMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
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
        let header_name = self.header_name.clone();

        // Capture the incoming request ID for logging and correlation
        let incoming_request_id = request.headers().get(&header_name).cloned();

        // Determine the request ID to use based on configuration
        let header_value = match self.use_incoming_id {
            IdReuse::UseIncoming => request
                .headers()
                .get(&header_name)
                .map_or_else(self.id_generator, Clone::clone),
            IdReuse::IgnoreIncoming => (self.id_generator)(),
        };

        // Store the request ID in request extensions for handler extraction
        let request_id = RequestId(header_value.clone());
        request.extensions_mut().insert(request_id);

        let fut = self.service.call(request);
        Box::pin(async move {
            // Log incoming request IDs for debugging and request correlation
            if let Some(upstream_request_id) = incoming_request_id {
                tracing::debug!(
                    ?upstream_request_id,
                    "Received upstream request ID for correlation"
                );
            }

            let mut response = fut.await?;

            // Add the request ID to the response headers for client correlation
            response.headers_mut().insert(header_name, header_value);

            Ok(response)
        })
    }
}

impl FromRequest for RequestId {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    /// Extract the request ID from the current request.
    ///
    /// This will return an error if the `RequestIdentifier` middleware
    /// is not properly configured or if it runs after this extraction.
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(
            req.extensions()
                .get::<RequestId>()
                .cloned()
                .ok_or(Error::NoAssociatedId),
        )
    }
}

