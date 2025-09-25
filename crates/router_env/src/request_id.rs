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
    str::FromStr,
    sync::Arc,
    task::{Context, Poll},
};

use actix_web::{
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform},
    error::ResponseError,
    http::header::{HeaderName, HeaderValue},
    Error as ActixError, FromRequest, HttpMessage, HttpRequest,
};
use tracing::Instrument;
use uuid::Uuid;

/// Custom result type for request ID operations.
pub type RequestIdResult<T> = Result<T, Error>;

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
    /// Failed to convert header value to request ID.
    InvalidHeaderValue {
        /// The invalid header value that caused the error.
        value: String,
    },
    /// Request ID generation failed.
    GenerationFailed {
        /// The reason why generation failed.
        reason: String,
    },
    /// Configuration error.
    Configuration {
        /// The configuration error message.
        message: String,
    },
}

impl error_stack::Context for Error {}

impl ResponseError for Error {}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoAssociatedId => write!(fmt, "No request ID associated with this request"),
            Error::InvalidHeaderValue { value } => write!(fmt, "Invalid header value: {}", value),
            Error::GenerationFailed { reason } => {
                write!(fmt, "Request ID generation failed: {}", reason)
            }
            Error::Configuration { message } => write!(fmt, "Configuration error: {}", message),
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

/// Trait for generating request IDs.
///
/// Implementing this trait allows for custom request ID generation strategies,
/// including integration with external systems, databases, or custom formats.
pub trait RequestIdGenerator: Send + Sync + fmt::Debug + 'static {
    /// Generate a new request ID string.
    ///
    /// Should return a unique identifier suitable for request tracking.
    /// The implementation should be thread-safe and efficient.
    fn generate(&self) -> RequestIdResult<String>;
}

/// Function-based generator wrapper for backwards compatibility.
#[derive(Clone, Copy)]
pub struct FunctionGenerator(pub fn() -> String);

impl fmt::Debug for FunctionGenerator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionGenerator")
            .field("function", &self.0)
            .finish()
    }
}

impl RequestIdGenerator for FunctionGenerator {
    fn generate(&self) -> RequestIdResult<String> {
        Ok((self.0)())
    }
}

/// UUID v7 generator with v4 fallback.
#[derive(Default, Clone, Copy, Debug)]
pub struct UuidV7Generator;

impl RequestIdGenerator for UuidV7Generator {
    fn generate(&self) -> RequestIdResult<String> {
        // Try UUID v7 first
        let uuid = Uuid::now_v7();
        let uuid_str = uuid.to_string();

        // UUID strings are always valid, but fallback to v4 just in case
        (!uuid_str.is_empty())
            .then_some(Ok(uuid_str))
            .unwrap_or_else(|| {
                tracing::warn!("UUID v7 generated empty string, falling back to UUID v4");
                UuidV4Generator
                    .generate()
                    .map_err(|_| Error::GenerationFailed {
                        reason: "Both UUID v7 and v4 generation failed".to_string(),
                    })
            })
    }
}

/// UUID v4 generator.
#[derive(Default, Clone, Copy, Debug)]
pub struct UuidV4Generator;

impl RequestIdGenerator for UuidV4Generator {
    fn generate(&self) -> RequestIdResult<String> {
        let uuid = Uuid::new_v4();
        let uuid_str = uuid.to_string();

        (!uuid_str.is_empty())
            .then_some(uuid_str)
            .ok_or(Error::GenerationFailed {
                reason: "UUID v4 generation failed".to_string(),
            })
    }
}

/// Configuration builder for the request ID middleware.
///
/// This struct provides a fluent interface for configuring how request IDs
/// are generated and handled.
///
/// ## Why `Arc<dyn RequestIdGenerator>`?
/// - **Type Erasure**: Can store any generator implementation (UuidV7, UuidV4, custom)
/// - **Shared Ownership**: Cheap cloning for middleware (Arc::clone is O(1))
/// - **Thread Safety**: `Send + Sync` for async web frameworks
/// - **Flexibility**: Supports stateful generators with custom logic
#[derive(Clone, Debug)]
pub struct RequestIdentifier {
    header_name: &'static str,
    id_generator: Arc<dyn RequestIdGenerator>,
    use_incoming_id: IdReuse,
}

/// Request ID value that can be extracted in route handlers.
///
/// This wraps an `Arc<str>` for optimal performance in web middleware.
///
/// ## Why `Arc<str>`?
/// - **Performance**: ~1000x faster cloning vs `String` (atomic increment vs heap allocation)
/// - **Memory Efficiency**: Shared string data, 8-byte pointer vs 24-byte `String`
/// - **Thread Safety**: `Send + Sync` for async request handling
/// - **Immutable**: Perfect for IDs that are created once, cloned many times
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
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct RequestId(Arc<str>);

/// The actual middleware implementation that processes requests.
///
/// This is created by the `RequestIdentifier` configuration and handles
/// the actual request processing logic.
#[derive(Debug)]
pub struct RequestIdMiddleware<S> {
    service: S,
    header_name: HeaderName,
    id_generator: Arc<dyn RequestIdGenerator>,
    use_incoming_id: IdReuse,
}

impl Display for RequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RequestId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err(Error::InvalidHeaderValue {
                value: s.to_string(),
            })
        } else {
            Ok(Self(s.into()))
        }
    }
}

impl TryFrom<HeaderValue> for RequestId {
    type Error = Error;

    fn try_from(value: HeaderValue) -> Result<Self, Self::Error> {
        let s = value.to_str().map_err(|_| Error::InvalidHeaderValue {
            value: format!("{:?}", value),
        })?;
        Self::from_str(s)
    }
}

impl From<RequestId> for HeaderValue {
    fn from(request_id: RequestId) -> Self {
        // This should never fail since we validate on creation
        HeaderValue::from_str(&request_id.0)
            .unwrap_or_else(|_| HeaderValue::from_static("invalid-request-id"))
    }
}

impl From<RequestId> for String {
    fn from(request_id: RequestId) -> Self {
        request_id.0.to_string()
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl RequestId {
    /// Create a new RequestId from a string.
    pub fn new(value: impl Into<Arc<str>>) -> Self {
        Self(value.into())
    }

    /// Convert this request ID to a `HeaderValue`.
    ///
    /// This can be useful when you need to pass the request ID to other
    /// HTTP clients or services that expect a `HeaderValue`.
    pub fn to_header_value(&self) -> RequestIdResult<HeaderValue> {
        HeaderValue::from_str(&self.0).map_err(|_| Error::InvalidHeaderValue {
            value: self.0.to_string(),
        })
    }

    /// Get a string representation of this request ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl RequestIdentifier {
    /// Create a request ID middleware with default UUID generation.
    ///
    /// By default, this uses UUID v7 for time-ordered request IDs, which provides
    /// better database performance and natural sorting.
    ///
    /// Uses the default header name [`DEFAULT_HEADER`] and [`IdReuse::UseIncoming`].
    #[must_use]
    pub fn with_uuid() -> Self {
        Self::default()
    }

    /// Create a request ID middleware with explicit UUID v7 generation.
    ///
    /// This explicitly uses UUID v7 with v4 fallback. UUID v7 provides
    /// time-ordered identifiers that are more efficient for database indexing
    /// and provide natural chronological sorting.
    #[must_use]
    pub fn with_uuid_v7() -> Self {
        Self::with_generator(UuidV7Generator)
    }

    /// Create a request ID middleware with UUID v4 generation.
    ///
    /// UUID v4 provides fully random identifiers without timestamp information.
    #[must_use]
    pub fn with_uuid_v4() -> Self {
        Self::with_generator(UuidV4Generator)
    }

    /// Create a request ID middleware with a custom header name.
    ///
    /// Uses the default UUID generator (v7 with v4 fallback)
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
    pub fn header(self, header_name: &'static str) -> Self {
        Self {
            header_name,
            ..self
        }
    }

    /// Create a request ID middleware with a custom ID generator.
    ///
    /// The generator should implement [`RequestIdGenerator`] and provide
    /// unique identifiers suitable for request tracking.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use router_env::{RequestIdentifier, UuidV4Generator};
    ///
    /// let middleware = RequestIdentifier::with_generator(UuidV4Generator);
    /// ```
    #[must_use]
    pub fn with_generator<G: RequestIdGenerator>(generator: G) -> Self {
        Self {
            id_generator: Arc::new(generator),
            header_name: DEFAULT_HEADER,
            use_incoming_id: IdReuse::default(),
        }
    }

    /// Change the ID generator for this middleware configuration.
    ///
    /// This allows switching between different UUID versions or implementing
    /// completely custom ID generation logic.
    #[must_use]
    pub fn generator<G: RequestIdGenerator>(self, generator: G) -> Self {
        Self {
            id_generator: Arc::new(generator),
            ..self
        }
    }

    /// Create a request ID middleware with a function-based generator.
    ///
    /// This is provided for backwards compatibility with function-based generators.
    /// For new code, prefer implementing [`RequestIdGenerator`] directly.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use router_env::RequestIdentifier;
    ///
    /// fn custom_generator() -> String {
    ///     \"custom-id-123\".to_string()
    /// }
    ///
    /// let middleware = RequestIdentifier::with_function_generator(custom_generator);
    /// ```
    #[must_use]
    pub fn with_function_generator(generator: fn() -> String) -> Self {
        Self::with_generator(FunctionGenerator(generator))
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
/// Default configuration uses UUID v7 generation with UUID v4 fallback.
///
/// UUID v7 provides time-ordered identifiers that are more efficient for
/// database operations and provide natural chronological sorting.
impl Default for RequestIdentifier {
    fn default() -> Self {
        Self {
            header_name: DEFAULT_HEADER,
            id_generator: Arc::new(UuidV7Generator),
            use_incoming_id: IdReuse::default(),
        }
    }
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
        let header_name = HeaderName::from_str(self.header_name).unwrap_or_else(|_| {
            tracing::error!("Invalid header name '{}', using default", self.header_name);
            HeaderName::from_static("x-request-id")
        });

        ready(Ok(RequestIdMiddleware {
            service,
            header_name,
            id_generator: Arc::clone(&self.id_generator),
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
        let request_id_string = match self.use_incoming_id {
            IdReuse::UseIncoming => {
                // Try to use incoming header first
                if let Some(existing_header) = request.headers().get(&header_name) {
                    existing_header.to_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| {
                            tracing::warn!("Incoming request ID header contains non-ASCII characters, generating new one");
                            self.id_generator.generate().unwrap_or_else(|e| {
                                tracing::error!("Failed to generate request ID: {}", e);
                                "fallback-request-id".to_string()
                            })
                        })
                } else {
                    // No incoming header, generate new one
                    self.id_generator.generate().unwrap_or_else(|e| {
                        tracing::error!("Failed to generate request ID: {}", e);
                        "fallback-request-id".to_string()
                    })
                }
            }
            IdReuse::IgnoreIncoming => {
                // Always generate new request ID
                self.id_generator.generate().unwrap_or_else(|e| {
                    tracing::error!("Failed to generate request ID: {}", e);
                    "fallback-request-id".to_string()
                })
            }
        };

        let request_id = RequestId(request_id_string.into());

        // Create header value for response
        let header_value = request_id.to_header_value().unwrap_or_else(|e| {
            tracing::error!("Failed to convert request ID to header value: {}", e);
            HeaderValue::from_static("invalid-request-id")
        });
        // Store the request ID in request extensions for handler extraction
        request.extensions_mut().insert(request_id);

        let fut = self.service.call(request);
        Box::pin(
            async move {
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
            }
            .in_current_span(),
        )
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
