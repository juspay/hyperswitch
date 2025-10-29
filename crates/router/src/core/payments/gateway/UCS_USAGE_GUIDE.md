# UCS Usage Guide

## Table of Contents
1. [Quick Start](#quick-start)
2. [Implementing a New Flow](#implementing-a-new-flow)
3. [Common Use Cases](#common-use-cases)
4. [Testing Guide](#testing-guide)
5. [Troubleshooting](#troubleshooting)
6. [FAQ](#faq)

---

## Quick Start

### Prerequisites
- Understanding of Rust async/await
- Familiarity with trait-based programming
- Basic knowledge of GRPC
- Read `UCS_ARCHITECTURE.md` for architecture overview

### Minimal Example

Here's a complete minimal example of implementing a new UCS flow:

```rust
// 1. Define the executor
#[derive(Debug, Clone, Copy)]
pub struct CaptureUcsExecutor;

// 2. Implement request transformation
impl UcsRequestTransformer<domain::Capture, PaymentsCaptureData, PaymentsResponseData>
    for CaptureUcsExecutor
{
    type GrpcRequest = PaymentServiceCaptureRequest;
    
    fn transform_request(
        router_data: &RouterData<domain::Capture, PaymentsCaptureData, PaymentsResponseData>
    ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
        PaymentServiceCaptureRequest::foreign_try_from(router_data)
            .change_context(ConnectorError::RequestEncodingFailed)
    }
}

// 3. Implement response handling
impl UcsResponseHandler<PaymentServiceCaptureResponse, PaymentsResponseData>
    for CaptureUcsExecutor
{
    fn handle_response(
        response: PaymentServiceCaptureResponse,
    ) -> CustomResult<
        (Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>, u16),
        ConnectorError,
    > {
        handle_unified_connector_service_response_for_payment_capture(response)
            .change_context(ConnectorError::ResponseHandlingFailed)
    }
}

// 4. Implement GRPC execution
#[async_trait]
impl UcsGrpcExecutor<
        UnifiedConnectorServiceClient,
        RouterUcsContext,
        PaymentServiceCaptureRequest,
        PaymentServiceCaptureResponse,
    > for CaptureUcsExecutor
{
    type GrpcResponse = tonic::Response<PaymentServiceCaptureResponse>;
    
    async fn execute_grpc_call(
        client: &UnifiedConnectorServiceClient,
        request: PaymentServiceCaptureRequest,
        context: RouterUcsContext,
    ) -> CustomResult<Self::GrpcResponse, ConnectorError> {
        client
            .payment_capture(request, context.auth(), context.headers())
            .await
            .change_context(ConnectorError::ProcessingStepFailed(None))
    }
}

// 5. Implement flow orchestration
#[async_trait]
impl UcsFlowExecutor<domain::Capture, PaymentsCaptureData, PaymentsResponseData, SessionState>
    for CaptureUcsExecutor
{
    type GrpcRequest = PaymentServiceCaptureRequest;
    type GrpcResponse = PaymentServiceCaptureResponse;
    type ExecCtx<'a> = RouterUcsExecutionContext<'a>;
    
    async fn execute_ucs_flow<'a>(
        state: &SessionState,
        router_data: &RouterData<domain::Capture, PaymentsCaptureData, PaymentsResponseData>,
        execution_context: RouterUcsExecutionContext<'a>,
    ) -> CustomResult<
        RouterData<domain::Capture, PaymentsCaptureData, PaymentsResponseData>,
        ConnectorError,
    >
    where
        Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
        Self::GrpcResponse: std::fmt::Debug,
    {
        // Use the generic executor
        ucs_executor::<domain::Capture, Self, PaymentsCaptureData, PaymentsResponseData, _, _>(
            state,
            router_data,
            execution_context,
        )
        .await
    }
}

// 6. Implement PaymentGateway
#[async_trait]
impl<RCD> PaymentGateway<
        SessionState,
        RCD,
        domain::Capture,
        PaymentsCaptureData,
        PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Capture
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::Capture,
        PaymentsCaptureData,
        PaymentsResponseData,
    >,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::Capture,
            RCD,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
        router_data: &RouterData<domain::Capture, PaymentsCaptureData, PaymentsResponseData>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<domain::Capture, PaymentsCaptureData, PaymentsResponseData>,
        ConnectorError,
    > {
        let execution_context = RouterUcsExecutionContext::new(
            &context.merchant_context,
            &context.header_payload,
            context.lineage_ids,
            &context.merchant_connector_account,
            context.execution_mode,
        );
        
        CaptureUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
    }
}

// 7. Implement FlowGateway
impl<RCD> FlowGateway<
        SessionState,
        RCD,
        PaymentsCaptureData,
        PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Capture
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::Capture,
        PaymentsCaptureData,
        PaymentsResponseData,
    >,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn PaymentGateway<
            SessionState,
            RCD,
            Self,
            PaymentsCaptureData,
            PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => Box::new(DirectGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => Box::new(domain::Capture),
        }
    }
}
```

---

## Implementing a New Flow

### Step-by-Step Guide

#### Step 1: Identify Requirements

Before starting, gather:
- ✅ GRPC endpoint name (e.g., `payment_capture`)
- ✅ GRPC request type (e.g., `PaymentServiceCaptureRequest`)
- ✅ GRPC response type (e.g., `PaymentServiceCaptureResponse`)
- ✅ Domain flow type (e.g., `domain::Capture`)
- ✅ Request data type (e.g., `PaymentsCaptureData`)
- ✅ Response data type (usually `PaymentsResponseData`)

#### Step 2: Create Executor Struct

**Location**: `crates/router/src/core/payments/gateway/ucs_executors.rs`

```rust
/// Executor for payment_capture GRPC endpoint
#[derive(Debug, Clone, Copy)]
pub struct CaptureUcsExecutor;
```

**Best practices:**
- Use descriptive names ending in `UcsExecutor`
- Add documentation explaining which GRPC endpoint it handles
- Derive `Debug`, `Clone`, `Copy` for zero-sized types

#### Step 3: Implement UcsRequestTransformer

```rust
impl UcsRequestTransformer<domain::Capture, PaymentsCaptureData, PaymentsResponseData>
    for CaptureUcsExecutor
{
    type GrpcRequest = PaymentServiceCaptureRequest;
    
    fn transform_request(
        router_data: &RouterData<domain::Capture, PaymentsCaptureData, PaymentsResponseData>
    ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
        // Use ForeignTryFrom for conversion
        PaymentServiceCaptureRequest::foreign_try_from(router_data)
            .change_context(ConnectorError::RequestEncodingFailed)
    }
}
```

**Key points:**
- Use `ForeignTryFrom` trait for conversions
- Always add error context with `.change_context()`
- Return `ConnectorError::RequestEncodingFailed` for conversion errors

#### Step 4: Implement UcsResponseHandler

```rust
impl UcsResponseHandler<PaymentServiceCaptureResponse, PaymentsResponseData>
    for CaptureUcsExecutor
{
    fn handle_response(
        response: PaymentServiceCaptureResponse,
    ) -> CustomResult<
        (Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>, u16),
        ConnectorError,
    > {
        // Use existing response handler or create new one
        handle_unified_connector_service_response_for_payment_capture(response)
            .change_context(ConnectorError::ResponseHandlingFailed)
    }
}
```

**Key points:**
- Reuse existing response handlers when possible
- Return tuple of `(Result<(ResponseData, Status), Error>, StatusCode)`
- Add error context for debugging

#### Step 5: Implement UcsGrpcExecutor

```rust
#[async_trait]
impl UcsGrpcExecutor<
        UnifiedConnectorServiceClient,
        RouterUcsContext,
        PaymentServiceCaptureRequest,
        PaymentServiceCaptureResponse,
    > for CaptureUcsExecutor
{
    type GrpcResponse = tonic::Response<PaymentServiceCaptureResponse>;
    
    async fn execute_grpc_call(
        client: &UnifiedConnectorServiceClient,
        request: PaymentServiceCaptureRequest,
        context: RouterUcsContext,
    ) -> CustomResult<Self::GrpcResponse, ConnectorError> {
        client
            .payment_capture(request, context.auth(), context.headers())
            .await
            .change_context(ConnectorError::ProcessingStepFailed(None))
    }
}
```

**Key points:**
- Call the specific GRPC client method
- Pass `context.auth()` and `context.headers()`
- Wrap response in `tonic::Response<T>`
- Add error context

#### Step 6: Implement UcsFlowExecutor

```rust
#[async_trait]
impl UcsFlowExecutor<domain::Capture, PaymentsCaptureData, PaymentsResponseData, SessionState>
    for CaptureUcsExecutor
{
    type GrpcRequest = PaymentServiceCaptureRequest;
    type GrpcResponse = PaymentServiceCaptureResponse;
    type ExecCtx<'a> = RouterUcsExecutionContext<'a>;  // GAT!
    
    async fn execute_ucs_flow<'a>(
        state: &SessionState,
        router_data: &RouterData<domain::Capture, PaymentsCaptureData, PaymentsResponseData>,
        execution_context: RouterUcsExecutionContext<'a>,
    ) -> CustomResult<
        RouterData<domain::Capture, PaymentsCaptureData, PaymentsResponseData>,
        ConnectorError,
    >
    where
        Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
        Self::GrpcResponse: std::fmt::Debug,
    {
        // Most flows can use the generic executor
        ucs_executor::<domain::Capture, Self, PaymentsCaptureData, PaymentsResponseData, _, _>(
            state,
            router_data,
            execution_context,
        )
        .await
    }
}
```

**Key points:**
- Use GAT `ExecCtx<'a>` for flexible lifetimes
- Add lifetime parameter `<'a>` to method
- Use generic `ucs_executor` for standard flows
- Add where clauses for serialization and debugging

#### Step 7: Create Flow File

**Location**: `crates/router/src/core/payments/gateway/capture.rs`

```rust
//! PaymentGateway implementation for api::Capture flow

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_flow_types as domain,
};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};

use super::{
    context::RouterGatewayContext,
    ucs_execution_context::RouterUcsExecutionContext,
    ucs_executors::CaptureUcsExecutor,
};
use crate::{
    routes::SessionState,
    types,
};

#[async_trait]
impl<RCD> payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Capture
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    >,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::Capture,
            RCD,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<
            domain::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<domain::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        let execution_context = RouterUcsExecutionContext::new(
            &context.merchant_context,
            &context.header_payload,
            context.lineage_ids,
            &context.merchant_connector_account,
            context.execution_mode,
        );
        
        CaptureUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
    }
}

impl<RCD> payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::Capture
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    >,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => Box::new(payment_gateway::DirectGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => Box::new(domain::Capture),
        }
    }
}
```

#### Step 8: Update Module Exports

**Location**: `crates/router/src/core/payments/gateway/mod.rs`

```rust
pub mod capture;  // Add new module
pub use capture::*;  // Export if needed
```

---

## Common Use Cases

### Use Case 1: Standard Flow (No Custom Logic)

For flows that follow the standard pattern, use the generic executor:

```rust
async fn execute_ucs_flow<'a>(...) -> CustomResult<...> {
    ucs_executor::<Flow, Self, Req, Resp, _, _>(state, router_data, execution_context).await
}
```

### Use Case 2: Conditional Routing

Route to different executors based on request data:

```rust
async fn execute(...) -> CustomResult<...> {
    let execution_context = RouterUcsExecutionContext::new(...);
    
    if router_data.request.mandate_id.is_some() {
        // Use mandate payment flow
        RepeatUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
    } else {
        // Use regular payment flow
        AuthorizeUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
    }
}
```

### Use Case 3: Pre/Post Processing

Add custom logic before or after the generic executor:

```rust
async fn execute_ucs_flow<'a>(...) -> CustomResult<...> {
    // Pre-processing
    validate_request(router_data)?;
    let modified_data = transform_data(router_data)?;
    
    // Execute flow
    let mut result = ucs_executor::<Flow, Self, Req, Resp, _, _>(
        state,
        &modified_data,
        execution_context,
    ).await?;
    
    // Post-processing
    enrich_response(&mut result)?;
    
    Ok(result)
}
```

### Use Case 4: Custom Error Handling

Add flow-specific error handling:

```rust
async fn execute_ucs_flow<'a>(...) -> CustomResult<...> {
    match ucs_executor::<Flow, Self, Req, Resp, _, _>(state, router_data, execution_context).await {
        Ok(result) => Ok(result),
        Err(e) if is_retryable_error(&e) => {
            // Retry logic
            retry_with_backoff(|| {
                ucs_executor::<Flow, Self, Req, Resp, _, _>(state, router_data, execution_context)
            }).await
        }
        Err(e) => Err(e),
    }
}
```

### Use Case 5: Feature Flags

Enable/disable flows based on configuration:

```rust
async fn execute(...) -> CustomResult<...> {
    if !state.conf.features.ucs_capture_enabled {
        return Err(ConnectorError::NotImplemented(
            "UCS Capture flow is disabled".to_string()
        ).into());
    }
    
    let execution_context = RouterUcsExecutionContext::new(...);
    CaptureUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
}
```

---

## Testing Guide

### Unit Testing Executors

Test each trait implementation independently:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_request_transformation() {
        let router_data = create_test_router_data();
        let result = CaptureUcsExecutor::transform_request(&router_data);
        
        assert!(result.is_ok());
        let grpc_request = result.unwrap();
        assert_eq!(grpc_request.amount, router_data.request.amount);
    }
    
    #[test]
    fn test_response_handling() {
        let grpc_response = create_test_grpc_response();
        let result = CaptureUcsExecutor::handle_response(grpc_response);
        
        assert!(result.is_ok());
        let (response_result, status_code) = result.unwrap();
        assert_eq!(status_code, 200);
    }
    
    #[tokio::test]
    async fn test_grpc_execution() {
        let client = create_mock_client();
        let request = create_test_request();
        let context = create_test_context();
        
        let result = CaptureUcsExecutor::execute_grpc_call(&client, request, context).await;
        assert!(result.is_ok());
    }
}
```

### Integration Testing

Test complete flows end-to-end:

```rust
#[tokio::test]
async fn test_capture_flow_success() {
    let state = create_test_state();
    let router_data = create_test_router_data();
    let context = create_test_gateway_context();
    
    let execution_context = RouterUcsExecutionContext::new(
        &context.merchant_context,
        &context.header_payload,
        context.lineage_ids,
        &context.merchant_connector_account,
        context.execution_mode,
    );
    
    let result = CaptureUcsExecutor::execute_ucs_flow(&state, &router_data, execution_context).await;
    
    assert!(result.is_ok());
    let updated_data = result.unwrap();
    assert_eq!(updated_data.status, AttemptStatus::Charged);
}

#[tokio::test]
async fn test_capture_flow_error_handling() {
    let state = create_test_state_with_failing_client();
    let router_data = create_test_router_data();
    let context = create_test_gateway_context();
    
    let execution_context = RouterUcsExecutionContext::new(...);
    
    let result = CaptureUcsExecutor::execute_ucs_flow(&state, &router_data, execution_context).await;
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error.current_context(), ConnectorError::ProcessingStepFailed(_)));
}
```

### Mock Helpers

Create reusable test helpers:

```rust
fn create_test_router_data() -> RouterData<domain::Capture, PaymentsCaptureData, PaymentsResponseData> {
    RouterData {
        flow: PhantomData,
        merchant_id: "test_merchant".to_string(),
        customer_id: Some("test_customer".to_string()),
        connector: "stripe".to_string(),
        request: PaymentsCaptureData {
            amount: 1000,
            currency: Currency::USD,
            // ... other fields
        },
        response: Err(ErrorResponse::default()),
        // ... other fields
    }
}

fn create_test_gateway_context() -> RouterGatewayContext {
    RouterGatewayContext {
        merchant_context: MerchantContext::default(),
        header_payload: HeaderPayload::default(),
        lineage_ids: LineageIds::new(),
        merchant_connector_account: create_test_mca(),
        execution_mode: ExecutionMode::Test,
    }
}
```

---

## Troubleshooting

### Common Error 1: Lifetime Issues

**Error:**
```
error[E0597]: `context.merchant_context` does not live long enough
```

**Solution:**
Extract values before async closures:

```rust
// ❌ Wrong
|...| async move {
    let value = execution_context.lineage_ids();  // Error!
}

// ✅ Correct
let lineage_ids = execution_context.lineage_ids();  // Extract first
|...| async move {
    let value = lineage_ids;  // OK
}
```

### Common Error 2: Missing Trait Bounds

**Error:**
```
error[E0277]: the trait bound `GrpcReq: Send` is not satisfied
```

**Solution:**
Add missing trait bounds:

```rust
where
    GrpcReq: Serialize + Debug + Send,  // Add Send
    GrpcResp: Serialize + Debug + Clone + Send,  // Add Send
```

### Common Error 3: Type Mismatch

**Error:**
```
error[E0308]: mismatched types
expected `RouterData<F, Req, Resp>`
found `RouterData<_, _, PaymentsResponseData>`
```

**Solution:**
Ensure response type matches:

```rust
// Make sure Resp type parameter matches
impl UcsResponseHandler<GrpcResp, Resp> for MyExecutor  // Resp must match
```

### Common Error 4: GAT Syntax

**Error:**
```
error[E0637]: `'_` cannot be used here
```

**Solution:**
Use named lifetime in trait bounds:

```rust
// ❌ Wrong
for<'_> Exe: UcsFlowExecutor<..., ExecCtx<'_> = ...>

// ✅ Correct
for<'b> Exe: UcsFlowExecutor<..., ExecCtx<'b> = ...>
```

---

## FAQ

### Q: When should I use the generic `ucs_executor` vs custom implementation?

**A:** Use the generic executor for standard flows. Implement custom logic only when you need:
- Pre/post processing
- Conditional routing
- Custom error handling
- Special validation

### Q: How do I handle flows that don't have GRPC endpoints yet?

**A:** Use `todo!()` with detailed documentation:

```rust
async fn execute(...) -> CustomResult<...> {
    todo!("UCS GRPC endpoint for XYZ not available - decision pending")
}
```

Add comments explaining what's needed to implement it.

### Q: Can I reuse executors across different flows?

**A:** No, each executor is specific to one GRPC endpoint. However, you can share helper functions and response handlers.

### Q: How do I add logging to my flow?

**A:** The `ucs_logging_wrapper` automatically handles logging. For custom logging:

```rust
async fn execute_ucs_flow<'a>(...) -> CustomResult<...> {
    logger::info!("Starting custom flow");
    let result = ucs_executor::<...>(...).await?;
    logger::info!("Flow completed successfully");
    Ok(result)
}
```

### Q: What's the difference between `PaymentGateway` and `FlowGateway`?

**A:**
- `PaymentGateway` - Executes the actual flow logic
- `FlowGateway` - Factory that returns the appropriate gateway based on execution path

### Q: How do I test with real GRPC endpoints?

**A:** Use integration tests with test credentials:

```rust
#[tokio::test]
#[ignore]  // Run only when explicitly requested
async fn test_real_grpc_endpoint() {
    let state = create_real_state_with_test_credentials();
    // ... test with real endpoint
}
```

### Q: Can I modify RouterData in the executor?

**A:** Yes, but only in the response handling phase:

```rust
|mut router_data, grpc_request, grpc_headers| async move {
    // ... execute GRPC call
    
    // Modify router_data
    router_data.status = new_status;
    router_data.response = new_response;
    
    Ok((router_data, grpc_response))
}
```

---

## Additional Resources

- **Architecture Overview**: See `UCS_ARCHITECTURE.md`
- **Trait Definitions**: `crates/hyperswitch_interfaces/src/unified_connector_service/ucs_traits.rs`
- **Example Implementations**: `crates/router/src/core/payments/gateway/ucs_executors.rs`
- **Helper Functions**: `crates/router/src/core/payments/gateway/helpers.rs`

---

## Getting Help

If you encounter issues:
1. Check this guide and `UCS_ARCHITECTURE.md`
2. Review existing implementations in `ucs_executors.rs`
3. Check compiler error messages carefully
4. Ask in the team chat or create an issue

Happy coding! 🚀