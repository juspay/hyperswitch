---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

# Connector Interface Requirements

---
**Parent:** [Hyperswitch Connectors Overview](./overview.md)  
**Related Files:**
- [Connector Implementation Guide](./connector_implementation_guide.md)
- [Connector Testing Guide](./connector_testing_guide.md)
- [Connector Configuration Guide](./connector_configuration_guide.md)
---

## Overview

This document provides a detailed specification of the interfaces that must be implemented when creating a new payment connector in Hyperswitch. Understanding these interfaces is essential for successfully integrating new payment processors into the platform.

## Core Connector Traits

### `ConnectorCommon` Trait

The `ConnectorCommon` trait is the foundation of every connector implementation. It defines basic properties and behaviors that all connectors must implement.

```rust
pub trait ConnectorCommon {
    // The unique identifier for the connector
    fn id(&self) -> &'static str;
    
    // The content type used for API communications
    fn common_get_content_type(&self) -> &'static str;
    
    // The base URL for the connector's API
    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str;
    
    // The currency unit (major/minor) used by the connector
    fn get_currency_unit(&self) -> api::CurrencyUnit;
    
    // Generate authentication headers
    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError>;
    
    // Build error response from connector's response
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError>;
}
```

### `ConnectorIntegration` Trait

The `ConnectorIntegration` trait is a generic trait that defines the interface for specific payment operations. Each operation (like authorize, capture, refund) has its own specialized version of this trait.

```rust
pub trait ConnectorIntegration<Flow, Request, Response> {
    // Get headers for the request
    fn get_headers(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError>;
    
    // Get content type for the request
    fn get_content_type(&self) -> &'static str;
    
    // Get URL for the request
    fn get_url(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError>;
    
    // Get request body
    fn get_request_body(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError>;
    
    // Build the complete request
    fn build_request(
        &self,
        req: &RouterData<Flow, Request, Response>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError>;
    
    // Handle the response from the connector
    fn handle_response(
        &self,
        data: &RouterData<Flow, Request, Response>,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<RouterData<Flow, Request, Response>, errors::ConnectorError>;
    
    // Get error response
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError>;
}
```

### Payment Operation Marker Traits

For each payment operation, there are marker traits that a connector must implement to indicate support for that operation:

```rust
pub trait Payment {}
pub trait PaymentSession {}
pub trait PaymentToken {}
pub trait ConnectorAccessToken {}
pub trait MandateSetup {}
pub trait PaymentAuthorize {}
pub trait PaymentSync {}
pub trait PaymentCapture {}
pub trait PaymentVoid {}
pub trait Refund {}
pub trait RefundExecute {}
pub trait RefundSync {}
```

### `IncomingWebhook` Trait

The `IncomingWebhook` trait defines methods for processing webhooks received from the payment processor:

```rust
#[async_trait::async_trait]
pub trait IncomingWebhook {
    // Get the algorithm used for webhook signature verification
    fn get_webhook_source_verification_algorithm(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError>;
    
    // Get the signature from the webhook for verification
    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError>;
    
    // Get the message to verify against the signature
    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError>;
    
    // Get the object reference ID from the webhook
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError>;
    
    // Get the event type from the webhook
    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError>;
    
    // Get the resource object from the webhook
    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError>;
}
```

### `ConnectorSpecifications` Trait

The `ConnectorSpecifications` trait provides metadata about the connector's capabilities:

```rust
pub trait ConnectorSpecifications {
    // Get information about the connector
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo>;
    
    // Get supported payment methods
    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods>;
    
    // Get supported webhook event classes
    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]>;
}
```

## Data Transformer Pattern

A critical part of implementing a connector is creating transformers that convert between Hyperswitch's domain models and connector-specific formats:

1. **Request Transformers**: Convert Hyperswitch request models to connector-specific request formats
2. **Response Transformers**: Convert connector-specific response formats to Hyperswitch response models

Example transformer implementation pattern:

```rust
// Request Transformer
impl TryFrom<&RouterData<Flow, Request, Response>> for ConnectorSpecificRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: &RouterData<Flow, Request, Response>) -> Result<Self, Self::Error> {
        // Transform Hyperswitch domain model to connector request format
    }
}

// Response Transformer
impl TryFrom<ResponseRouterData<Flow, ConnectorSpecificResponse, Request, Response>> 
    for RouterData<Flow, Request, Response> {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: ResponseRouterData<Flow, ConnectorSpecificResponse, Request, Response>) 
        -> Result<Self, Self::Error> {
        // Transform connector response to Hyperswitch domain model
    }
}
```

## Connector Specific Data Types

Each connector implementation must define its own request and response types that match the payment processor's API:

1. **Request Types**: For each supported operation (authorize, capture, refund, etc.)
2. **Response Types**: For each supported operation
3. **Error Types**: For handling connector-specific errors
4. **Webhook Types**: For handling incoming webhooks
5. **Status Enums**: For mapping connector-specific statuses to Hyperswitch statuses

## Error Handling

Connector implementations must handle various error scenarios:

1. **Validation Errors**: Errors in request validation
2. **Authentication Errors**: Errors in API authentication
3. **Network Errors**: Connection and timeout errors
4. **Processor Errors**: Errors returned by the payment processor
5. **Mapping Errors**: Errors in data transformation

All errors should be properly mapped to Hyperswitch's standardized error types using the `errors::ConnectorError` enum.

## Interface Requirements Matrix

| Operation | Required Traits | Required Methods | Optional Methods |
|-----------|----------------|------------------|------------------|
| Authorization | `Payment`, `PaymentAuthorize` | `get_headers`, `get_content_type`, `get_url`, `get_request_body`, `build_request`, `handle_response`, `get_error_response` | |
| Payment Sync | `Payment`, `PaymentSync` | `get_headers`, `get_content_type`, `get_url`, `build_request`, `handle_response`, `get_error_response` | `get_request_body` |
| Capture | `Payment`, `PaymentCapture` | `get_headers`, `get_content_type`, `get_url`, `get_request_body`, `build_request`, `handle_response`, `get_error_response` | |
| Void | `Payment`, `PaymentVoid` | `get_headers`, `get_content_type`, `get_url`, `get_request_body`, `build_request`, `handle_response`, `get_error_response` | |
| Refund | `Refund`, `RefundExecute` | `get_headers`, `get_content_type`, `get_url`, `get_request_body`, `build_request`, `handle_response`, `get_error_response` | |
| Refund Sync | `Refund`, `RefundSync` | `get_headers`, `get_content_type`, `get_url`, `build_request`, `handle_response`, `get_error_response` | `get_request_body` |
| Webhooks | `IncomingWebhook` | All methods in the `IncomingWebhook` trait | |

## Core Required Implementations

At minimum, every connector should implement:

1. `ConnectorCommon` trait
2. Payment authorization flow (`PaymentAuthorize` trait)
3. Payment status synchronization (`PaymentSync` trait)
4. `ConnectorSpecifications` trait

Implementing refunds, captures, voids, and webhooks are recommended but may depend on the payment processor's capabilities.

## Next Steps

After understanding the interface requirements, proceed to:

- [Connector Implementation Guide](./connector_implementation_guide.md) for step-by-step implementation instructions
- [Connector Testing Guide](./connector_testing_guide.md) for testing your connector implementation
- [Connector Configuration Guide](./connector_configuration_guide.md) for configuring your connector

## See Also

- [Hyperswitch Connectors Overview](./overview.md)
- [Common Connector Patterns](./connector_patterns.md)
- [Error Handling Guidelines](./error_handling.md)