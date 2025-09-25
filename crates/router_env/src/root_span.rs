//! Custom RootSpanBuilder for integrating actix-request-identifier with tracing-actix-web

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpMessage,
};
use tracing::Span;
use tracing_actix_web::{DefaultRootSpanBuilder, RootSpanBuilder};

/// Custom RootSpanBuilder that captures x-request-id header in spans
/// This integrates actix-request-identifier's request ID with tracing-actix-web's spans
#[derive(Debug)]
pub struct RequestIdRootSpanBuilder;

impl RootSpanBuilder for RequestIdRootSpanBuilder {
    fn on_request_start(request: &ServiceRequest) -> Span {
        // Extract request_id from request extensions (set by RequestIdentifier middleware)
        let request_id = request
            .extensions()
            .get::<crate::RequestId>()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Create a custom root span instead of using the root_span! macro
        // This avoids the macro's automatic request_id generation
        tracing::info_span!(
            "HTTP request",
            http.method = %request.method(),
            http.route = %request.match_pattern().unwrap_or("unknown".to_string()),
            http.flavor = ?request.version(),
            http.scheme = %request.connection_info().scheme(),
            http.host = %request.connection_info().host(),
            http.client_ip = %request.connection_info().peer_addr().unwrap_or("unknown"),
            http.user_agent = %request.headers().get("user-agent")
                .and_then(|h| h.to_str().ok())
                .unwrap_or(""),
            http.target = %request.uri().path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or(request.uri().path()),
            otel.name = %format!("{} {}", request.method(),
                request.match_pattern().unwrap_or(request.uri().path().to_string())),
            otel.kind = "server",
            request_id = %request_id,
            trace_id = tracing::field::Empty
        )
    }

    fn on_request_end<B: MessageBody>(span: Span, outcome: &Result<ServiceResponse<B>, Error>) {
        // Delegate to default implementation for response handling
        DefaultRootSpanBuilder::on_request_end(span, outcome);
    }
}
