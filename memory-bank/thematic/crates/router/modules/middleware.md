# Router Middleware Module

---
**Parent:** [Router Overview](../overview.md)  
**Last Updated:** 2025-05-20  
**Related Files:**
- [Routes Module](./routes.md)
- [Services Module](./services.md)
---

[← Back to Router Overview](../overview.md)

## Overview

The `middleware` module contains Actix Web middleware components that provide cross-cutting functionality across the Hyperswitch API. Middleware intercepts HTTP requests and responses to add functionality that applies universally or conditionally to multiple routes without modifying the route handlers themselves.

## Key Components

### Logging Middleware

The logging middleware handles request and response logging:

- **Request Logging**: Logs incoming HTTP requests with relevant details
- **Response Logging**: Logs outgoing HTTP responses with status codes and timing
- **Correlation IDs**: Generates and propagates correlation IDs for request tracing
- **PII Masking**: Masks sensitive data in logs (using the `masking` crate)
- **Structured Logging**: Ensures logs are structured for easy parsing and analysis

### Authentication Middleware

The authentication middleware manages API key validation:

- **API Key Extraction**: Extracts API keys from request headers or query parameters
- **Key Verification**: Verifies API key validity against the database
- **Merchant Identification**: Identifies the merchant associated with the API key
- **Unauthorized Response Handling**: Formats appropriate responses for authentication failures
- **Authentication Context**: Adds authentication data to the request context for downstream use

### Authorization Middleware

The authorization middleware controls access to protected endpoints:

- **Permission Checking**: Verifies if the authenticated merchant has permission for the requested operation
- **Resource-level Authorization**: Controls access to specific resources based on ownership
- **Role-based Authorization**: Implements role-based access control
- **Forbidden Response Handling**: Formats appropriate responses for authorization failures
- **Audit Logging**: Records authorization decisions for security auditing

### Error Handling Middleware

The error handling middleware provides consistent error responses:

- **Error Catching**: Catches errors thrown during request processing
- **Error Mapping**: Maps internal errors to appropriate HTTP status codes
- **Error Formatting**: Ensures consistent error response format
- **Error Logging**: Logs errors with appropriate severity and context
- **Sensitive Error Handling**: Prevents leaking sensitive information in error responses

### Metrics Collection Middleware

The metrics middleware gathers performance metrics:

- **Request Timing**: Measures request processing time
- **Counter Metrics**: Counts requests by endpoint, method, and status
- **Histogram Metrics**: Records distribution of response times
- **Prometheus Integration**: Exposes metrics in Prometheus format
- **Custom Business Metrics**: Collects business-specific metrics

### CORS Middleware

The CORS (Cross-Origin Resource Sharing) middleware handles cross-origin requests:

- **Origin Validation**: Validates request origins against configured allowed origins
- **Headers Management**: Manages CORS headers in responses
- **Preflight Requests**: Handles OPTIONS preflight requests
- **Credentials Support**: Configures support for credentials in cross-origin requests
- **Configurability**: Allows different CORS settings for different endpoints

### Rate Limiting Middleware

The rate limiting middleware controls request frequency:

- **Rate Limit Calculation**: Enforces request rate limits based on configured rules
- **Merchant-specific Limits**: Applies different limits for different merchants
- **Endpoint-specific Limits**: Applies different limits for different endpoints
- **Rate Limit Headers**: Adds rate limit information headers to responses
- **Rate Limit Exceedance Handling**: Returns appropriate responses when limits are exceeded

### Request Context Middleware

The request context middleware adds contextual data to requests:

- **Request ID Generation**: Generates unique identifiers for each request
- **Timestamp Addition**: Adds request start timestamp
- **Client Information**: Adds client IP, user agent, etc.
- **Propagation**: Ensures context is available throughout the request lifecycle
- **Cleanup**: Handles context cleanup after request completion

## Implementation Details

### Middleware Registration

Middleware is registered with the Actix Web application during startup. The registration order is important, as middleware is applied in the order it is registered. A typical registration pattern:

```rust
pub fn configure_middleware(config: &mut web::ServiceConfig) {
    let app = config
        .wrap(logging::LoggingMiddleware::new())
        .wrap(error_handling::ErrorHandlingMiddleware::new())
        .wrap(metrics::MetricsMiddleware::new())
        .wrap(cors::configure_cors())
        .wrap(auth::AuthenticationMiddleware::new())
        // Other middleware registration...
}
```

### Middleware Implementation

Middleware is typically implemented using Actix Web's middleware framework:

```rust
pub struct CustomMiddleware {
    // Middleware state...
}

impl<S, B> Transform<S, ServiceRequest> for CustomMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    // Implementation...
}
```

### Conditional Middleware

Some middleware is applied conditionally based on route or request properties:

```rust
pub fn conditional_middleware<S>(
    req: ServiceRequest,
    srv: &S,
) -> impl Future<Output = Result<ServiceResponse, Error>>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
{
    // Conditional logic...
}
```

### Middleware Configuration

Middleware is configured through application configuration:

- Environment variables
- Configuration files (TOML)
- Programmatic configuration during application startup

## Performance Considerations

The middleware module is designed with performance in mind:

- **Efficient Processing**: Middleware is optimized for minimal overhead
- **Selective Application**: Middleware can be selectively applied to routes
- **Caching**: Cacheable operations use appropriate caching strategies
- **Asynchronous Design**: Middleware is designed for asynchronous operation

## Dependencies

The middleware module primarily depends on:

- **Actix Web**: For the middleware framework
- **Router Services**: For business logic needed by some middleware
- **Masking Crate**: For PII masking in logs
- **Router Environment**: For configuration and logging infrastructure
- **Metrics Libraries**: For metrics collection

## See Also

- [Routes Module Documentation](./routes.md)
- [Services Module Documentation](./services.md)
