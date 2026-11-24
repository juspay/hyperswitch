//! Custom RootSpanBuilder tracing-actix-web

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::StatusCode,
    Error, HttpMessage, ResponseError,
};
use tracing::Span;
use tracing_actix_web::{root_span, RootSpanBuilder};

use crate::request_id::RequestId;

/// Custom RootSpanBuilder that captures x-request-id header in spans
#[derive(Debug)]
pub struct CustomRootSpanBuilder;

// Override only the URI observed by the tracing macro. The actual request,
// including its body, headers, and routing data, remains untouched.
struct RouteTargetRequest<'a> {
    request: &'a ServiceRequest,
    uri: actix_web::http::Uri,
}

impl std::ops::Deref for RouteTargetRequest<'_> {
    type Target = ServiceRequest;

    fn deref(&self) -> &Self::Target {
        self.request
    }
}

impl RouteTargetRequest<'_> {
    fn uri(&self) -> &actix_web::http::Uri {
        &self.uri
    }
}

impl RootSpanBuilder for CustomRootSpanBuilder {
    fn on_request_start(request: &ServiceRequest) -> Span {
        // Extract the RequestId from extensions (set by RequestIdentifier middleware)
        // We clone the string to avoid lifetime issues with the temporary Ref guard
        let request_id = request
            .extensions()
            .get::<RequestId>()
            .map(|id| id.to_string())
            .unwrap_or_default();

        // root_span! records http.target as soon as the span is created.
        let target = crate::request_path::path_for_telemetry(request.request());
        if target.as_ref() == request.path() {
            return root_span!(
                level = crate::Level::INFO,
                request,
                request_id = request_id.as_str()
            );
        }
        let route_request = RouteTargetRequest {
            request,
            uri: target
                .parse()
                .unwrap_or_else(|_| actix_web::http::Uri::from_static("/unmatched")),
        };
        let request = &route_request;
        root_span!(
            level = crate::Level::INFO,
            request,
            request_id = request_id.as_str()
        )
    }

    fn on_request_end<B: MessageBody>(span: Span, outcome: &Result<ServiceResponse<B>, Error>) {
        match &outcome {
            Ok(response) => {
                if let Some(error) = response.response().error() {
                    // use the status code already constructed for the outgoing HTTP response
                    handle_error(span, response.status(), error.as_response_error());
                } else {
                    let code: i32 = response.response().status().as_u16().into();
                    span.record("http.status_code", code);
                    span.record("otel.status_code", "OK");
                }
            }
            Err(error) => {
                let response_error = error.as_response_error();
                handle_error(span, response_error.status_code(), response_error);
            }
        };
    }
}

fn handle_error(span: Span, status_code: StatusCode, response_error: &dyn ResponseError) {
    // pre-formatting errors is a workaround for https://github.com/tokio-rs/tracing/issues/1565
    let display = format!("{response_error}");
    let debug = format!("{response_error:?}");
    span.record("exception.message", tracing::field::display(display));
    span.record("exception.details", tracing::field::display(debug));
    let code: i32 = status_code.as_u16().into();

    span.record("http.status_code", code);

    if status_code.is_client_error() {
        span.record("otel.status_code", "OK");
    } else {
        span.record("otel.status_code", "ERROR");
    }
}

#[cfg(test)]
mod tests;
