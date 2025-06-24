---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

# Router Extension Points

---
**Parent:** [Router Overview](./overview.md)  
**Related Files:**
- [Core Module](./modules/core.md)
- [Router Configuration](./configuration/router_configuration.md)
- [Payment Flows](./flows/payment_flows.md)
- [Refund Flows](./flows/refund_flows.md)
- [Webhook Flows](./flows/webhook_flows.md)
---

[← Back to Router Overview](./overview.md)

## Overview

The Hyperswitch router is designed with extensibility in mind. It provides several well-defined extension points that allow developers to customize its behavior, add new functionality, and integrate with external systems. This document details these extension points and provides guidance on how to use them effectively.

## Connector Integration Framework

One of the primary extension points in the router is the connector integration framework, which allows integration with new payment processors.

### Implementing a New Connector

To implement a new payment connector, you need to:

1. Create a new module in `crates/hyperswitch_connectors/src/connectors/`
2. Implement the required traits for the connector
3. Register the connector in the connector registry

The main traits to implement are:

```rust
// Core trait that all connectors must implement
pub trait ConnectorCommon {
    fn id(&self) -> &'static str;
    fn common_get_content_type(&self) -> &'static str;
    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str;
    // ... other common methods
}

// For payment operations
pub trait PaymentAuthorize {
    fn get_headers(&self, _req: &types::PaymentsAuthorizeRouterData, _connectors: &settings::Connectors) -> CustomResult<Vec<(String, String)>, errors::ConnectorError>;
    fn get_content_type(&self) -> &'static str;
    fn get_url(&self, _req: &types::PaymentsAuthorizeRouterData, _connectors: &settings::Connectors) -> CustomResult<String, errors::ConnectorError>;
    fn get_request_body(&self, req: &types::PaymentsAuthorizeRouterData) -> CustomResult<Option<String>, errors::ConnectorError>;
    fn build_request(&self, req: &types::PaymentsAuthorizeRouterData, connectors: &settings::Connectors) -> CustomResult<Option<services::Request>, errors::ConnectorError>;
    fn handle_response(&self, data: &types::PaymentsAuthorizeRouterData, res: types::Response) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError>;
    // ... other methods
}
```

### Connector Transformers

The connector framework uses a transformer pattern to convert between Hyperswitch's domain models and connector-specific formats. Each connector implements its own transformers.

## Custom Routing Logic

The router includes a pluggable routing system that allows custom routing logic to be implemented.

### Implementing a Custom Router

To implement a custom routing strategy:

1. Create a new module in `crates/router/src/core/payments/routing/strategies/`
2. Implement the `RoutingStrategy` trait for your custom router
3. Register the strategy in the routing registry

```rust
// The core routing strategy trait
pub trait RoutingStrategy: Send + Sync + Debug {
    fn route(&self, ctx: &RoutingContext) -> CustomResult<RoutingResponse, errors::ApiErrorResponse>;
    fn id(&self) -> &'static str;
}
```

## Webhook Processors

The webhook system can be extended with custom webhook processors for both incoming and outgoing webhooks.

### Implementing a Custom Webhook Processor

To implement a custom webhook processor:

1. Create a new module in `crates/router/src/core/webhooks/processors/`
2. Implement the appropriate webhook processor trait
3. Register the processor in the webhook registry

```rust
// For incoming webhooks
pub trait IncomingWebhookProcessor: Send + Sync + Debug {
    fn process_webhook(
        &self,
        request: &api::IncomingWebhookRequestData,
        merchant_account: domain::MerchantAccount,
        connector_details: domain::ConnectorDetails,
    ) -> CustomResult<api::IncomingWebhookResponse, errors::ApiErrorResponse>;
    
    fn id(&self) -> &'static str;
}
```

## Custom Authentication Methods

The router supports pluggable authentication methods for API authentication.

### Implementing a Custom Authentication Method

To implement a custom authentication method:

1. Create a new module in `crates/router/src/core/authentication/methods/`
2. Implement the `AuthenticationMethod` trait for your custom method
3. Register the method in the authentication registry

```rust
// The authentication method trait
pub trait AuthenticationMethod: Send + Sync + Debug {
    fn authenticate(
        &self,
        request: &http::Request<actix_web::web::Bytes>,
        db: &db::DB,
    ) -> CustomResult<domain::MerchantAccount, errors::ApiErrorResponse>;
    
    fn id(&self) -> &'static str;
}
```

## API Extensions

The router API can be extended with custom endpoints and handlers.

### Adding Custom API Endpoints

To add custom API endpoints:

1. Create a new module in `crates/router/src/routes/custom/`
2. Implement your API handlers
3. Register the routes in the API configuration

## Event Listeners

The router includes an event system that allows custom event listeners to be registered for various system events.

### Implementing a Custom Event Listener

To implement a custom event listener:

1. Create a new module in `crates/router/src/events/listeners/`
2. Implement the `EventListener` trait for your custom listener
3. Register the listener in the event registry

```rust
// The event listener trait
pub trait EventListener: Send + Sync + Debug {
    fn handle_event(
        &self,
        event: &domain::Event,
        db: &db::DB,
    ) -> CustomResult<(), errors::ApiErrorResponse>;
    
    fn interested_in(&self, event_type: &domain::EventType) -> bool;
    
    fn id(&self) -> &'static str;
}
```

## Database Extensions

The router's database layer can be extended with custom storage implementations.

### Implementing a Custom Storage Interface

To implement a custom storage interface:

1. Create a new module in `crates/storage_impl/src/custom/`
2. Implement the appropriate storage trait for your custom storage
3. Register the storage implementation in the database registry

## Middleware Extensions

The router's middleware stack can be extended with custom middleware components.

### Implementing Custom Middleware

To implement custom middleware:

1. Create a new module in `crates/router/src/middleware/`
2. Implement the actix_web middleware trait for your custom middleware
3. Register the middleware in the application configuration

## Integration with External Systems

The router can be extended to integrate with external systems such as fraud detection, analytics, or compliance services.

### Implementing an External System Integration

To implement an integration with an external system:

1. Create a new module in `crates/router/src/services/external/`
2. Implement the appropriate service interface for your integration
3. Configure the integration in the application setup

## Best Practices for Extensions

1. **Follow Existing Patterns**: Study the existing codebase to understand the design patterns and follow them in your extensions
2. **Write Tests**: Include comprehensive tests for your extensions, including unit tests and integration tests
3. **Handle Errors Properly**: Use the error handling mechanisms provided by the router, including the `CustomResult` type and proper error mapping
4. **Document Extensions**: Include comprehensive documentation for your extensions, including usage examples
5. **Performance Considerations**: Ensure your extensions are optimized for performance, especially for high-throughput scenarios
6. **Security Best Practices**: Follow security best practices, particularly for extensions that handle sensitive payment data
7. **Backward Compatibility**: Maintain backward compatibility with existing APIs and interfaces when extending the system

## Extension Deployment

There are several ways to deploy extensions:

1. **Fork the Repository**: Create a fork of the Hyperswitch repository and add your extensions
2. **Plugin Architecture**: For some extension points, implement a plugin that can be loaded at runtime
3. **Configuration-Based**: For simpler extensions, use the configuration system to enable and configure your extensions

The appropriate deployment method depends on the type and complexity of your extension.

## Extension Configuration

Extensions often need configuration options. The router provides several ways to configure extensions:

1. **Configuration Files**: Add configuration options to the TOML configuration files
2. **Environment Variables**: Use environment variables for runtime configuration
3. **Database Configuration**: Store extension configuration in the database for merchant-specific settings
4. **Dynamic Configuration**: Implement dynamic configuration through the admin API

## Versioning and Compatibility

When developing extensions, consider versioning and compatibility:

1. **API Versioning**: Ensure your extensions work with the current API version
2. **Backward Compatibility**: Maintain compatibility with existing code and clients
3. **Forward Compatibility**: Design extensions to be adaptable to future changes
4. **Version Constraints**: Specify version constraints for dependencies

## Document History
| Date | Changes |
|------|----------|
| 2025-05-27 | Initial version |