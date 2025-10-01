#![deny(unused, missing_docs)]
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
//! - **Exact parity**: Supports `.as_hyphenated()` method like tracing_actix_web::RequestId
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

/// Errors that can occur when working with request IDs.
#[derive(Debug, Clone)]
pub enum Error {
    /// No request ID is associated with the current request.
    NoAssociatedId,
    /// Failed to convert header value to request ID.
    InvalidHeaderValue {
        /// The invalid header value that caused the error.
        value: String,
    },
}

impl error_stack::Context for Error {}

impl ResponseError for Error {}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAssociatedId => write!(fmt, "No request ID associated with this request"),
            Self::InvalidHeaderValue { value } => write!(fmt, "Invalid header value: {}", value),
        }
    }
}

/// Configuration for handling incoming request ID headers.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum IdReuse {
    /// Reuse the incoming request ID if present, otherwise generate a new one.
    #[default]
    UseIncoming,
    /// Always generate a new request ID, ignoring any incoming headers.
    IgnoreIncoming,
}

/// Generate a new UUID v7 request ID.
fn generate_uuid_v7() -> String {
    Uuid::now_v7().to_string()
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
    service: S,
    header_name: HeaderName,
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
        Self::from_str(&request_id.0).unwrap_or_else(|_| Self::from_static("invalid-request-id"))
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
    pub fn new_from_string(value: impl Into<Arc<str>>) -> Self {
        Self(value.into())
    }

    /// Extract request ID from ServiceRequest header or generate UUID v7.
    ///
    /// This is the core logic: try to extract from the specified header,
    /// if not possible or not desired, generate a new UUID v7.
    pub fn extract_or_generate(
        request: &ServiceRequest,
        header_name: &HeaderName,
        use_incoming_id: IdReuse,
    ) -> Self {
        let request_id_string = match use_incoming_id {
            IdReuse::UseIncoming => {
                // Try to extract from incoming header
                if let Some(existing_header) = request.headers().get(header_name) {
                    existing_header
                        .to_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| {
                            tracing::warn!("Invalid request ID header, generating new UUID v7");
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
        HeaderValue::from_str(&self.0).map_err(|_| Error::InvalidHeaderValue {
            value: self.0.to_string(),
        })
    }

    /// Get a string representation of this request ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get a hyphenated string representation of this request ID.
    ///
    /// This method provides exact parity with `tracing_actix_web::RequestId.as_hyphenated()`.
    /// Since UUID v7's default string representation is already hyphenated, this returns
    /// the same value as `to_string()` and `as_str()`.
    pub fn as_hyphenated(&self) -> &str {
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
        let header_name = HeaderName::from_str(&self.header_name).unwrap_or_else(|_| {
            tracing::error!(
                "Invalid header name '{}', using fallback 'x-request-id'",
                self.header_name
            );
            HeaderName::from_static("x-request-id")
        });

        ready(Ok(RequestIdMiddleware {
            service,
            header_name,
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

        // Capture incoming request ID for logging
        let incoming_request_id = request.headers().get(&header_name).cloned();

        // Extract from header or generate UUID v7 - simple and direct
        let request_id =
            RequestId::extract_or_generate(&request, &header_name, self.use_incoming_id);

        // Create header value for response
        let header_value = request_id.to_header_value().unwrap_or_else(|e| {
            tracing::error!("Failed to convert request ID to header value: {}", e);
            HeaderValue::from_static("invalid-request-id")
        });

        // Store request ID for extraction in handlers
        request.extensions_mut().insert(request_id);

        let fut = self.service.call(request);
        Box::pin(
            async move {
                // Log incoming request IDs for correlation
                if let Some(upstream_request_id) = incoming_request_id {
                    tracing::debug!(
                        ?upstream_request_id,
                        "Received upstream request ID for correlation"
                    );
                }

                let mut response = fut.await?;

                // Add request ID to response headers
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

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(
            req.extensions()
                .get::<Self>()
                .cloned()
                .ok_or(Error::NoAssociatedId),
        )
    }
}
