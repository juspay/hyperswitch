# Connector Integration Details

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Connector Integration Architecture

The `hyperswitch_interfaces` crate implements a multi-layered architecture for connector integration, providing flexibility, version compatibility, and standardized interaction patterns.

## Connector Version Evolution

Hyperswitch supports two generations of connector implementations:

### V1 Connector Architecture

The original connector architecture defines these key traits:

1. **Connector**: Base trait defining connector identity and common operations
2. **ConnectorIntegration**: Flow-specific trait for handling particular payment operations
3. **IncomingWebhook**: Trait for processing webhook events from the connector

### V2 Connector Architecture

The newer connector architecture adds improved abstraction:

1. **ConnectorV2**: Consolidated trait that combines multiple functional areas
2. **ConnectorIntegrationV2**: Enhanced integration trait with additional type parameter for resource data
3. **RouterDataV2**: Updated data structure with improved type parameterization

### Compatibility Layer

The `ConnectorEnum` and `ConnectorIntegrationEnum` types wrap both v1 and v2 implementations, allowing seamless interoperability:

```rust
// Dispatches to either v1 or v2 implementation
impl ConnectorIntegrationInterface<T, ResourceCommonData, Req, Resp>
    for ConnectorIntegrationEnum<'static, T, ResourceCommonData, Req, Resp>
{
    fn build_request(
        &self,
        req: &RouterData<T, Req, Resp>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        match self {
            ConnectorIntegrationEnum::Old(old_integration) => {
                old_integration.build_request(req, connectors)
            }
            ConnectorIntegrationEnum::New(new_integration) => {
                let new_router_data = ResourceCommonData::from_old_router_data(req)?;
                new_integration.build_request_v2(&new_router_data)
            }
        }
    }
    // ...other methods
}
```

## Connector Integration Flow

### Request Processing Flow

1. The core application creates a `RouterData` object with request parameters
2. The appropriate connector instance is selected based on configuration
3. The connector's integration implementation builds a request for the payment processor
4. The request is executed by the HTTP client
5. The response is passed back to the connector for processing
6. The connector transforms the response into a standardized Hyperswitch format
7. The processed result is returned to the application

```
Application → RouterData → ConnectorIntegration → Payment Processor
Application ← Processed Result ← ConnectorIntegration ← Payment Processor Response
```

### Error Handling Flow

1. Connector-specific errors are captured and transformed into standardized `ErrorResponse` objects
2. Different error types (client errors, server errors) are handled with specialized methods
3. Error responses include status codes, error messages, and additional metadata
4. Errors are properly propagated up the call stack with `error-stack` integration

## Implementing a New Connector

### V1 Connector Implementation

```rust
pub struct Stripe;

impl ConnectorCommon for Stripe {
    fn id(&self) -> &'static str {
        "stripe"
    }
    
    // ...other common methods
}

impl api::Payment for Stripe {}
impl api::PaymentAuthorize for Stripe {}
impl api::PaymentCapture for Stripe {}
// ...other payment operation traits

impl ConnectorIntegration<api::Authorize, AuthorizeRequest, AuthorizeResponse> for Stripe {
    fn build_request(
        &self,
        req: &RouterData<api::Authorize, AuthorizeRequest, AuthorizeResponse>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        // Connector-specific logic to build an authorization request
    }
    
    fn handle_response(
        &self,
        data: &RouterData<api::Authorize, AuthorizeRequest, AuthorizeResponse>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<RouterData<api::Authorize, AuthorizeRequest, AuthorizeResponse>, errors::ConnectorError> {
        // Connector-specific logic to process the response
    }
    
    // ...other methods
}
```

### V2 Connector Implementation

```rust
pub struct StripeV2;

impl ConnectorCommon for StripeV2 {
    fn id(&self) -> &'static str {
        "stripe_v2"
    }
    
    // ...other common methods
}

impl api::payments_v2::PaymentV2 for StripeV2 {}
impl api::refunds_v2::RefundV2 for StripeV2 {}
// ...other v2 operation traits

impl<F, Req, Resp> ConnectorIntegrationV2<F, StripeData, Req, Resp> for StripeV2
where
    F: Clone,
    StripeData: Clone + RouterDataConversion<F, Req, Resp>,
    Req: Clone,
    Resp: Clone,
{
    fn build_request_v2(
        &self,
        req: &RouterDataV2<F, StripeData, Req, Resp>,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        // Connector-specific logic to build a request
    }
    
    fn handle_response_v2(
        &self,
        data: &RouterDataV2<F, StripeData, Req, Resp>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<RouterDataV2<F, StripeData, Req, Resp>, errors::ConnectorError> {
        // Connector-specific logic to process the response
    }
    
    // ...other methods
}
```

## Webhook Processing

Webhooks are an essential part of connector integration, allowing payment processors to asynchronously notify Hyperswitch about event updates.

### Webhook Processing Steps

1. **Verification**: Authenticating the webhook source using signatures or other mechanisms
2. **Decoding**: Parsing the webhook payload into a structured format
3. **Event Classification**: Determining the event type (payment, refund, dispute, etc.)
4. **Reference Resolution**: Identifying the Hyperswitch resources related to the event
5. **Event Processing**: Updating the system state based on the event
6. **Response Generation**: Returning an appropriate HTTP response to the webhook caller

### Implementing Webhook Handling

```rust
impl IncomingWebhook for Stripe {
    fn get_webhook_source_verification_algorithm(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        // Return appropriate signature verification algorithm
    }
    
    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        // Extract signature from request headers
    }
    
    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        // Extract message to verify from request
    }
    
    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        // Extract reference ID from webhook payload
    }
    
    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        // Determine event type from webhook payload
    }
    
    // ...other webhook methods
}
```

## Advanced Features and Extensions

### Connector Authentication

Connectors support various authentication mechanisms:

```rust
fn get_auth_header(
    &self,
    auth_type: &ConnectorAuthType,
) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
    match auth_type {
        ConnectorAuthType::HeaderKey { api_key } => {
            Ok(vec![("Authorization".to_string(), format!("Bearer {}", api_key.expose()).into())])
        }
        ConnectorAuthType::BodyKey { api_key, key_name } => {
            // Authentication via request body
            // ...
        }
        // ... other auth types
    }
}
```

### Connector Validation

Connectors implement validation logic to ensure they support requested operations:

```rust
fn validate_connector_against_payment_request(
    &self,
    capture_method: Option<common_enums::CaptureMethod>,
    payment_method: common_enums::PaymentMethod,
    pmt: Option<common_enums::PaymentMethodType>,
) -> CustomResult<(), errors::ConnectorError> {
    // Validate that the connector supports the requested operation
    // ...
}
```

### Dispute Handling

Specialized interfaces for processing disputes and chargebacks:

```rust
fn get_dispute_details(
    &self,
    request: &IncomingWebhookRequestDetails<'_>,
) -> CustomResult<disputes::DisputePayload, errors::ConnectorError> {
    // Extract dispute details from webhook payload
    // ...
}
```

## Connector Specifications

Connectors provide metadata about their capabilities:

```rust
fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
    Some(&SupportedPaymentMethods {
        payment_methods: HashSet::from([
            PaymentMethod::Card,
            PaymentMethod::BankRedirect,
            // ...other supported methods
        ])
    })
}

fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
    Some(&[
        common_enums::EventClass::PaymentConfirmation,
        common_enums::EventClass::Dispute,
        // ...other supported event types
    ])
}
```

## Summary

The connector integration architecture in Hyperswitch provides a powerful, flexible system for integrating with diverse payment processors. Through well-defined interfaces, version compatibility mechanisms, and standardized processing flows, it enables a modular, extensible payment orchestration platform.

## Document History

| Date | Changes |
|------|---------|
| 2025-05-27 | Added metadata and document history section |
| Prior | Initial version |
