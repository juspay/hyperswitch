# Hyperswitch Interfaces Crate

## Introduction and Purpose

The `hyperswitch_interfaces` crate serves as a critical abstraction layer for payment connector integrations within the Hyperswitch payment orchestration platform. It defines standardized interfaces, traits, and enums that create well-defined boundaries between Hyperswitch's core application logic and the various payment processors it integrates with.

This crate acts as the "connective tissue" between the main Hyperswitch application and payment connectors, ensuring that:

1. The core application can interact with all payment processors through a unified interface
2. Payment processors can be added, updated, or removed without affecting core functionality
3. Both legacy (v1) and new (v2) connector implementations are supported through a version compatibility layer
4. Webhook handling, authentication flows, and error handling are standardized across connectors

## Feature Flags

The crate supports several feature flags that enable different functionality:

```toml
[features]
default = ["dummy_connector", "frm", "payouts"]
dummy_connector = []
v1 = ["hyperswitch_domain_models/v1", "api_models/v1", "common_utils/v1"]
v2 = []
payouts = ["hyperswitch_domain_models/payouts"]
frm = ["hyperswitch_domain_models/frm"]
revenue_recovery = []
```

## Core Interfaces and Abstractions

### Connector Integration Interface

At the heart of this crate are two primary traits that define how connectors integrate with Hyperswitch:

1. **ConnectorIntegration (v1)**: The original integration interface defined in `connector_integration_interface.rs`
2. **ConnectorIntegrationV2**: The newer integration interface defined in `connector_integration_v2.rs` with improved abstraction

These traits define methods for building requests, handling responses, processing errors, and interacting with payment processors.

```rust
// Example of v1 Interface (simplified)
pub trait ConnectorIntegration<T, Req, Resp>: ConnectorCommon {
    fn build_request(
        &self,
        req: &RouterData<T, Req, Resp>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError>;
    
    fn handle_response(
        &self,
        data: &RouterData<T, Req, Resp>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>;
    
    // ... other methods
}

// Example of v2 Interface (simplified)
pub trait ConnectorIntegrationV2<Flow, ResourceCommonData, Req, Resp>:
    ConnectorIntegrationAnyV2<Flow, ResourceCommonData, Req, Resp> + Sync + api::ConnectorCommon
{
    fn build_request_v2(
        &self,
        req: &RouterDataV2<Flow, ResourceCommonData, Req, Resp>,
    ) -> CustomResult<Option<Request>, errors::ConnectorError>;
    
    fn handle_response_v2(
        &self,
        data: &RouterDataV2<Flow, ResourceCommonData, Req, Resp>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<RouterDataV2<Flow, ResourceCommonData, Req, Resp>, errors::ConnectorError>;
    
    // ... other methods
}
```

### Version Compatibility Layer

The `RouterDataConversion` trait enables seamless compatibility between v1 and v2 connectors by providing bidirectional conversion methods:

```rust
pub trait RouterDataConversion<T, Req: Clone, Resp: Clone> {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized;
        
    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized;
}
```

### Connector Abstraction Enums

The crate defines important abstractions for working with connectors:

1. **ConnectorEnum**: A unified representation of both v1 and v2 connectors
   ```rust
   pub enum ConnectorEnum {
       Old(BoxedConnector),
       New(BoxedConnectorV2),
   }
   ```

2. **ConnectorIntegrationEnum**: Wraps connector integration implementations for both versions
   ```rust
   pub enum ConnectorIntegrationEnum<'a, F, ResourceCommonData, Req, Resp> {
       Old(BoxedConnectorIntegration<'a, F, Req, Resp>),
       New(BoxedConnectorIntegrationV2<'a, F, ResourceCommonData, Req, Resp>),
   }
   ```

3. **BoxedConnector** and **BoxedConnectorV2**: Type aliases for boxed references to connector traits

## Key Components by Module

### Webhook Handling (`webhooks.rs`)

The `IncomingWebhook` trait standardizes how connectors process and validate incoming webhook events. It provides methods for:

- Decoding and verifying webhook request bodies
- Extracting event types and reference IDs
- Generating API responses for webhook callbacks
- Processing dispute and mandate-related webhook events

```rust
#[async_trait::async_trait]
pub trait IncomingWebhook: ConnectorCommon + Sync {
    async fn decode_webhook_body(&self, request: &IncomingWebhookRequestDetails<'_>, ...);
    async fn verify_webhook_source(&self, request: &IncomingWebhookRequestDetails<'_>, ...);
    fn get_webhook_object_reference_id(&self, request: &IncomingWebhookRequestDetails<'_>);
    fn get_webhook_event_type(&self, request: &IncomingWebhookRequestDetails<'_>);
    // ... other methods
}
```

### Authentication (`authentication.rs`)

Defines interfaces for handling authentication with payment processors, including:
- External authentication payloads
- Authentication flow verification
- Authentication-related webhook processing

### Disputes (`disputes.rs`)

Provides standardized interfaces for processing dispute-related events and data, including:
- Dispute payloads and status models
- Methods for extracting dispute details from webhooks
- Dispute resolution workflows

### Error Handling (`errors.rs`)

Defines the error types and handling mechanisms for connector integrations, including:
- ConnectorError types and descriptions
- Standardized error response formats
- Error conversion between connector-specific and application-level errors

## Integration Patterns

### Connector Implementation Flow

1. Define a connector struct that implements either `Connector` (v1) or `ConnectorV2` (v2)
2. Implement required traits based on supported features (payments, refunds, webhooks, etc.)
3. For each payment flow (authorize, capture, etc.), implement the corresponding connector integration trait
4. Define request/response transformers for converting between connector-specific and Hyperswitch formats
5. Implement webhook processors for handling connector-specific event formats

### Webhook Processing Flow

```
Incoming Webhook Request
↓
Decode Webhook Body (connector-specific)
↓
Verify Webhook Source (signature validation)
↓
Extract Object Reference ID and Event Type
↓
Extract Resource Object (payment/refund/dispute details)
↓
Process Event Based on Type (via core application)
↓
Return API Response to Webhook Caller
```

## Relationship with Other Crates

The `hyperswitch_interfaces` crate has dependencies on several other Hyperswitch crates:

1. **hyperswitch_domain_models**: Provides core data models and types used in payment flows
2. **api_models**: Defines API request/response structures used in connector interfaces
3. **common_enums**: Provides shared enumerations for payment methods, currencies, etc.
4. **common_utils**: Offers utility functions for error handling, request building, etc.
5. **router_derive**: Provides procedural macros for deriving connector-related functionality
6. **masking**: Handles sensitive data masking and PII protection

## Design Patterns and Principles

1. **Adapter Pattern**: Connectors adapt external payment processor APIs to Hyperswitch's internal interfaces
2. **Strategy Pattern**: Different payment flows can be selected and executed at runtime
3. **Trait Abstractions**: Heavy use of traits for defining behavior contracts
4. **Type-driven Development**: Leverages Rust's type system for expressing complex relationships
5. **Error Handling**: Comprehensive error propagation and conversion mechanisms

## Key Extension Points

The crate is designed for extensibility in several ways:

1. **New Connectors**: Adding new payment processors by implementing the connector traits
2. **New Payment Flows**: Supporting additional payment operations through flow-specific traits
3. **Feature Flags**: Enabling/disabling specific functionality through Cargo features
4. **Version Evolution**: Supporting both v1 and v2 interfaces during transition periods

## Summary

The `hyperswitch_interfaces` crate forms the foundation of Hyperswitch's connector ecosystem, providing a standardized, type-safe approach to integrating with diverse payment processors. Through its carefully designed traits and abstractions, it enables a pluggable architecture where connectors can be seamlessly added or updated without disrupting the core payment orchestration functionality.
