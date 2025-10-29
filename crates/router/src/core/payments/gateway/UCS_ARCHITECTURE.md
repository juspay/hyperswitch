# UCS (Unified Connector Service) Architecture

## Table of Contents
1. [Overview](#overview)
2. [Core Components](#core-components)
3. [Trait System](#trait-system)
4. [Data Flow](#data-flow)
5. [Lifetime Management](#lifetime-management)
6. [File Structure](#file-structure)
7. [Adding New Flows](#adding-new-flows)
8. [Best Practices](#best-practices)

---

## Overview

The UCS (Unified Connector Service) architecture provides a trait-based, generic framework for executing payment flows through GRPC endpoints. It uses Generic Associated Types (GATs) for flexible lifetime management and eliminates code duplication through polymorphism.

### Key Features
- ✅ **Trait-based polymorphism** - Separates common infrastructure from flow-specific logic
- ✅ **Generic Associated Types (GATs)** - Flexible lifetime management without `'static` constraints
- ✅ **Type-safe GRPC integration** - Compile-time guarantees for request/response handling
- ✅ **Reusable generic executor** - Single implementation for all flows
- ✅ **Comprehensive error handling** - Structured error propagation with context

---

## Core Components

### 1. Context Types

#### RouterGatewayContext
**Location**: `context.rs`

Entry point context passed to `PaymentGateway::execute()`. Contains all merchant and request information.

```rust
pub struct RouterGatewayContext {
    pub merchant_context: MerchantContext,
    pub header_payload: HeaderPayload,
    pub lineage_ids: LineageIds,
    pub merchant_connector_account: MerchantConnectorAccountType,
    pub execution_mode: ExecutionMode,
}
```

#### RouterUcsExecutionContext<'a>
**Location**: `ucs_execution_context.rs`

Execution context with Generic Associated Type (GAT) for flexible lifetimes. Groups all parameters needed for UCS execution.

```rust
pub struct RouterUcsExecutionContext<'a> {
    pub merchant_context: &'a MerchantContext,
    pub header_payload: &'a HeaderPayload,
    pub lineage_ids: LineageIds,
    pub merchant_connector_account: &'a MerchantConnectorAccountType,
    pub execution_mode: ExecutionMode,
}
```

**Key feature**: The lifetime `'a` allows references without requiring `'static`, enabling efficient borrowing.

#### RouterUcsContext
**Location**: `ucs_context.rs`

GRPC call context containing authentication and headers for a single GRPC request.

```rust
pub struct RouterUcsContext {
    auth: ConnectorAuthMetadata,
    headers: GrpcHeadersUcs,
    lineage_ids: LineageIds,
}
```

---

### 2. Trait System

All traits are defined in `crates/hyperswitch_interfaces/src/unified_connector_service/ucs_traits.rs`

#### UcsRequestTransformer<F, Req, Resp>
Transforms RouterData into GRPC request format.

```rust
pub trait UcsRequestTransformer<F, Req, Resp> {
    type GrpcRequest;
    
    fn transform_request(
        router_data: &RouterData<F, Req, Resp>
    ) -> CustomResult<Self::GrpcRequest, ConnectorError>;
}
```

**Purpose**: Converts domain-specific RouterData to GRPC-specific request types.

#### UcsResponseHandler<GrpcResp, Resp>
Handles GRPC responses and converts them to RouterData format.

```rust
pub trait UcsResponseHandler<GrpcResp, Resp> {
    fn handle_response(
        response: GrpcResp,
    ) -> CustomResult<
        (Result<(Resp, AttemptStatus), ErrorResponse>, u16),
        ConnectorError,
    >;
}
```

**Purpose**: Processes GRPC responses and extracts payment status and data.

#### UcsGrpcExecutor<Client, Ctx, GrpcReq, GrpcResp>
Executes the specific GRPC client method.

```rust
pub trait UcsGrpcExecutor<Client, Ctx, GrpcReq, GrpcResp>
where
    Ctx: UcsContext,
{
    type GrpcResponse;
    
    async fn execute_grpc_call(
        client: &Client,
        request: GrpcReq,
        context: Ctx,
    ) -> CustomResult<Self::GrpcResponse, ConnectorError>;
}
```

**Purpose**: Calls the appropriate GRPC endpoint (e.g., `payment_authorize`, `payment_get`).

#### UcsFlowExecutor<F, Req, Resp, State>
Main orchestrator trait that combines all operations.

```rust
pub trait UcsFlowExecutor<F, Req, Resp, State>: Send + Sync {
    type GrpcRequest: Send + Sync;
    type GrpcResponse: Send + Sync + Clone;
    type ExecCtx<'a>: UcsExecutionContextProvider;  // GAT!
    
    async fn execute_ucs_flow<'a>(
        state: &State,
        router_data: &RouterData<F, Req, Resp>,
        execution_context: Self::ExecCtx<'a>,
    ) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>;
}
```

**Purpose**: Orchestrates the complete UCS flow from request to response.

**Key feature**: Uses GAT `ExecCtx<'a>` to support flexible lifetimes.

---

### 3. Executors

#### Generic Executor Function
**Location**: `ucs_executors.rs`

Provides reusable implementation for all UCS flows.

```rust
pub async fn ucs_executor<'a, F, Exe, Req, Resp, GrpcReq, GrpcResp>(
    state: &SessionState,
    router_data: &RouterData<F, Req, Resp>,
    execution_context: RouterUcsExecutionContext<'a>,
) -> CustomResult<RouterData<F, Req, Resp>, ConnectorError>
where
    F: Send + Sync + Clone + Debug + 'static,
    Req: Send + Sync + Clone + Debug + 'static,
    Resp: Send + Sync + Clone + Debug + 'static,
    GrpcReq: Serialize + Debug + Send,
    GrpcResp: Serialize + Debug + Clone + Send,
    Exe: UcsRequestTransformer<F, Req, Resp, GrpcRequest = GrpcReq>
        + UcsResponseHandler<GrpcResp, Resp>
        + UcsGrpcExecutor<UnifiedConnectorServiceClient, RouterUcsContext, GrpcReq, GrpcResp>
        + UcsFlowExecutor<F, Req, Resp, SessionState>,
    for<'b> Exe: UcsFlowExecutor<F, Req, Resp, SessionState, ExecCtx<'b> = RouterUcsExecutionContext<'b>>,
{
    // 1. Prepare infrastructure (client, auth, headers)
    // 2. Transform request
    // 3. Execute GRPC call with logging
    // 4. Handle response
    // 5. Update and return RouterData
}
```

#### Concrete Executors
**Location**: `ucs_executors.rs`

Each executor implements all four UCS traits for a specific GRPC endpoint:

1. **AuthorizeUcsExecutor** - `payment_authorize` endpoint
2. **RepeatUcsExecutor** - `payment_repeat` endpoint (mandate payments)
3. **PSyncUcsExecutor** - `payment_get` endpoint (payment sync)
4. **SetupMandateUcsExecutor** - `payment_setup_mandate` endpoint

**Example: AuthorizeUcsExecutor**
```rust
#[derive(Debug, Clone, Copy)]
pub struct AuthorizeUcsExecutor;

impl UcsRequestTransformer<domain::Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for AuthorizeUcsExecutor
{
    type GrpcRequest = PaymentServiceAuthorizeRequest;
    
    fn transform_request(router_data: &RouterData<...>) -> CustomResult<...> {
        PaymentServiceAuthorizeRequest::foreign_try_from(router_data)
            .change_context(ConnectorError::RequestEncodingFailed)
    }
}

impl UcsResponseHandler<PaymentServiceAuthorizeResponse, PaymentsResponseData>
    for AuthorizeUcsExecutor
{
    fn handle_response(response: PaymentServiceAuthorizeResponse) -> CustomResult<...> {
        handle_unified_connector_service_response_for_payment_authorize(response)
            .change_context(ConnectorError::ResponseHandlingFailed)
    }
}

impl UcsGrpcExecutor<UnifiedConnectorServiceClient, RouterUcsContext, ...>
    for AuthorizeUcsExecutor
{
    type GrpcResponse = tonic::Response<PaymentServiceAuthorizeResponse>;
    
    async fn execute_grpc_call(...) -> CustomResult<...> {
        client.payment_authorize(request, context.auth(), context.headers()).await
            .change_context(ConnectorError::ProcessingStepFailed(None))
    }
}

impl UcsFlowExecutor<domain::Authorize, PaymentsAuthorizeData, PaymentsResponseData, SessionState>
    for AuthorizeUcsExecutor
{
    type GrpcRequest = PaymentServiceAuthorizeRequest;
    type GrpcResponse = PaymentServiceAuthorizeResponse;
    type ExecCtx<'a> = RouterUcsExecutionContext<'a>;
    
    async fn execute_ucs_flow<'a>(...) -> CustomResult<...> {
        // Implementation uses the generic ucs_executor or custom logic
    }
}
```

---

## Data Flow

### Complete Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. PaymentGateway::execute()                                    │
│    - Receives RouterGatewayContext                              │
│    - Creates RouterUcsExecutionContext                          │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. Executor::execute_ucs_flow()                                 │
│    - AuthorizeUcsExecutor, PSyncUcsExecutor, etc.               │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. prepare_ucs_infrastructure()                                 │
│    - Get GRPC client                                            │
│    - Build auth metadata                                        │
│    - Build GRPC headers                                         │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. UcsRequestTransformer::transform_request()                   │
│    - RouterData → GRPC Request                                  │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. ucs_logging_wrapper()                                        │
│    - Wraps GRPC call with logging                               │
│    - Handles telemetry and error tracking                       │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│ 6. UcsGrpcExecutor::execute_grpc_call()                         │
│    - Create RouterUcsContext                                    │
│    - Call GRPC endpoint (payment_authorize, payment_get, etc.)  │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│ 7. UcsResponseHandler::handle_response()                        │
│    - GRPC Response → (PaymentsResponseData, AttemptStatus)      │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│ 8. Update RouterData                                            │
│    - Set response data                                          │
│    - Set status                                                 │
│    - Set HTTP status code                                       │
│    - Return updated RouterData                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Step-by-Step Execution

1. **Entry Point**: `PaymentGateway::execute()` receives request
2. **Context Creation**: Creates `RouterUcsExecutionContext` from `RouterGatewayContext`
3. **Executor Selection**: Calls appropriate executor's `execute_ucs_flow()`
4. **Infrastructure Setup**: Prepares GRPC client, auth, and headers
5. **Request Transformation**: Converts RouterData to GRPC request
6. **GRPC Call**: Executes GRPC endpoint with logging
7. **Response Handling**: Processes GRPC response
8. **RouterData Update**: Updates and returns RouterData with results

---

## Lifetime Management

### The Problem with `'static`

**Before GATs:**
```rust
type ExecCtx = RouterUcsExecutionContext<'static>;  // ❌ Too restrictive
```

This required all references to live for the entire program duration, preventing efficient borrowing.

### The Solution: Generic Associated Types (GATs)

**With GATs:**
```rust
type ExecCtx<'a> = RouterUcsExecutionContext<'a>;  // ✅ Flexible lifetime
```

This allows references to live only as long as needed, enabling efficient borrowing from `RouterGatewayContext`.

### Lifetime Propagation

```rust
// 1. RouterGatewayContext (owned data)
let context = RouterGatewayContext { ... };

// 2. Create execution context with borrowed references
let execution_context = RouterUcsExecutionContext::new(
    &context.merchant_context,     // Borrows with lifetime 'a
    &context.header_payload,        // Borrows with lifetime 'a
    context.lineage_ids,            // Owned (moved)
    &context.merchant_connector_account,  // Borrows with lifetime 'a
    context.execution_mode,         // Copy
);

// 3. Pass to executor (lifetime 'a propagates)
Executor::execute_ucs_flow(state, router_data, execution_context).await
```

### Closure Lifetime Management

**Problem**: Async closures capture by move, so references must be extracted first.

```rust
// ❌ Wrong - captures execution_context by reference
|router_data, grpc_request, grpc_headers| async move {
    let context = RouterUcsContext::new(auth, grpc_headers, execution_context.lineage_ids());
    //                                                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    //                                                       Error: execution_context doesn't live long enough
}

// ✅ Correct - extract value before closure
let lineage_ids = execution_context.lineage_ids();  // Extract before closure
|router_data, grpc_request, grpc_headers| async move {
    let context = RouterUcsContext::new(auth, grpc_headers, lineage_ids);  // Use owned value
}
```

---

## File Structure

```
crates/router/src/core/payments/gateway/
├── mod.rs                          # Module exports
├── context.rs                      # RouterGatewayContext
├── ucs_context.rs                  # RouterUcsContext
├── ucs_execution_context.rs       # RouterUcsExecutionContext<'a>
├── ucs_executors.rs                # Generic executor + concrete implementations
│   ├── ucs_executor()              # Generic reusable function
│   ├── AuthorizeUcsExecutor        # payment_authorize
│   ├── RepeatUcsExecutor           # payment_repeat
│   ├── PSyncUcsExecutor            # payment_get
│   └── SetupMandateUcsExecutor     # payment_setup_mandate
├── helpers.rs                      # Shared utility functions
│   └── prepare_ucs_infrastructure()
├── authorize.rs                    # Authorize flow PaymentGateway impl
├── psync.rs                        # PSync flow PaymentGateway impl
├── setup_mandate.rs                # SetupMandate flow PaymentGateway impl
└── UCS_ARCHITECTURE.md             # This file
```

---

## Adding New Flows

### Step 1: Create Executor Struct

```rust
// In ucs_executors.rs
#[derive(Debug, Clone, Copy)]
pub struct MyNewFlowUcsExecutor;
```

### Step 2: Implement UcsRequestTransformer

```rust
impl UcsRequestTransformer<domain::MyNewFlow, MyRequestData, PaymentsResponseData>
    for MyNewFlowUcsExecutor
{
    type GrpcRequest = PaymentServiceMyNewFlowRequest;
    
    fn transform_request(
        router_data: &RouterData<domain::MyNewFlow, MyRequestData, PaymentsResponseData>
    ) -> CustomResult<Self::GrpcRequest, ConnectorError> {
        PaymentServiceMyNewFlowRequest::foreign_try_from(router_data)
            .change_context(ConnectorError::RequestEncodingFailed)
    }
}
```

### Step 3: Implement UcsResponseHandler

```rust
impl UcsResponseHandler<PaymentServiceMyNewFlowResponse, PaymentsResponseData>
    for MyNewFlowUcsExecutor
{
    fn handle_response(
        response: PaymentServiceMyNewFlowResponse,
    ) -> CustomResult<
        (Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>, u16),
        ConnectorError,
    > {
        handle_unified_connector_service_response_for_my_new_flow(response)
            .change_context(ConnectorError::ResponseHandlingFailed)
    }
}
```

### Step 4: Implement UcsGrpcExecutor

```rust
#[async_trait]
impl UcsGrpcExecutor<
        UnifiedConnectorServiceClient,
        RouterUcsContext,
        PaymentServiceMyNewFlowRequest,
        PaymentServiceMyNewFlowResponse,
    > for MyNewFlowUcsExecutor
{
    type GrpcResponse = tonic::Response<PaymentServiceMyNewFlowResponse>;
    
    async fn execute_grpc_call(
        client: &UnifiedConnectorServiceClient,
        request: PaymentServiceMyNewFlowRequest,
        context: RouterUcsContext,
    ) -> CustomResult<Self::GrpcResponse, ConnectorError> {
        client
            .my_new_flow_endpoint(request, context.auth(), context.headers())
            .await
            .change_context(ConnectorError::ProcessingStepFailed(None))
    }
}
```

### Step 5: Implement UcsFlowExecutor

```rust
#[async_trait]
impl UcsFlowExecutor<domain::MyNewFlow, MyRequestData, PaymentsResponseData, SessionState>
    for MyNewFlowUcsExecutor
{
    type GrpcRequest = PaymentServiceMyNewFlowRequest;
    type GrpcResponse = PaymentServiceMyNewFlowResponse;
    type ExecCtx<'a> = RouterUcsExecutionContext<'a>;
    
    async fn execute_ucs_flow<'a>(
        state: &SessionState,
        router_data: &RouterData<domain::MyNewFlow, MyRequestData, PaymentsResponseData>,
        execution_context: RouterUcsExecutionContext<'a>,
    ) -> CustomResult<
        RouterData<domain::MyNewFlow, MyRequestData, PaymentsResponseData>,
        ConnectorError,
    >
    where
        Self::GrpcRequest: serde::Serialize + std::fmt::Debug,
        Self::GrpcResponse: std::fmt::Debug,
    {
        // Use the generic ucs_executor or implement custom logic
        // Most flows can use the generic implementation
        ucs_executor::<domain::MyNewFlow, Self, MyRequestData, PaymentsResponseData, _, _>(
            state,
            router_data,
            execution_context,
        )
        .await
    }
}
```

### Step 6: Create Flow File

```rust
// In my_new_flow.rs
use super::ucs_executors::MyNewFlowUcsExecutor;

#[async_trait]
impl<RCD> PaymentGateway<...> for domain::MyNewFlow
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<...>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<...>,
        router_data: &RouterData<domain::MyNewFlow, MyRequestData, PaymentsResponseData>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<RouterData<domain::MyNewFlow, MyRequestData, PaymentsResponseData>, ConnectorError> {
        let execution_context = RouterUcsExecutionContext::new(
            &context.merchant_context,
            &context.header_payload,
            context.lineage_ids,
            &context.merchant_connector_account,
            context.execution_mode,
        );
        
        MyNewFlowUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
    }
}

impl<RCD> FlowGateway<...> for domain::MyNewFlow
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<...>,
{
    fn get_gateway(execution_path: ExecutionPath) -> Box<dyn PaymentGateway<...>> {
        match execution_path {
            ExecutionPath::Direct => Box::new(DirectGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => Box::new(domain::MyNewFlow),
        }
    }
}
```

---

## Best Practices

### 1. Trait Implementation Order
Always implement traits in this order:
1. `UcsRequestTransformer` - Request transformation
2. `UcsResponseHandler` - Response handling
3. `UcsGrpcExecutor` - GRPC call execution
4. `UcsFlowExecutor` - Flow orchestration

### 2. Error Handling
Always use `.change_context()` to add context to errors:
```rust
PaymentServiceRequest::foreign_try_from(router_data)
    .change_context(ConnectorError::RequestEncodingFailed)
```

### 3. Lifetime Management
- Extract values before async closures
- Use GATs for flexible lifetimes
- Avoid `'static` unless absolutely necessary

### 4. Type Safety
- Let the compiler infer types when possible (use `_`)
- Constrain associated types explicitly when needed
- Use Higher-Ranked Trait Bounds (HRTB) for GATs

### 5. Code Organization
- Keep executor implementations in `ucs_executors.rs`
- Keep flow-specific PaymentGateway implementations in separate files
- Use helpers for shared infrastructure

### 6. Documentation
- Document all public functions
- Explain complex trait bounds
- Provide usage examples

### 7. Testing
- Test each trait implementation independently
- Test complete flows end-to-end
- Test error scenarios

---

## Common Patterns

### Pattern 1: Using the Generic Executor

Most flows can use the generic `ucs_executor` function:

```rust
impl UcsFlowExecutor<...> for MyExecutor {
    async fn execute_ucs_flow<'a>(...) -> CustomResult<...> {
        ucs_executor::<Flow, Self, Req, Resp, _, _>(state, router_data, execution_context).await
    }
}
```

### Pattern 2: Custom Flow Logic

For flows with special requirements:

```rust
impl UcsFlowExecutor<...> for MyExecutor {
    async fn execute_ucs_flow<'a>(...) -> CustomResult<...> {
        // Custom pre-processing
        let modified_data = preprocess(router_data)?;
        
        // Use generic executor
        let result = ucs_executor::<Flow, Self, Req, Resp, _, _>(
            state,
            &modified_data,
            execution_context,
        ).await?;
        
        // Custom post-processing
        postprocess(result)
    }
}
```

### Pattern 3: Conditional Routing

Route to different executors based on request data:

```rust
async fn execute(...) -> CustomResult<...> {
    let execution_context = RouterUcsExecutionContext::new(...);
    
    if router_data.request.mandate_id.is_some() {
        RepeatUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
    } else {
        AuthorizeUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
    }
}
```

---

## Troubleshooting

### Lifetime Errors

**Error**: `does not live long enough`
**Solution**: Extract values before async closures

```rust
// ❌ Wrong
|...| async move {
    context.lineage_ids()  // Error: context doesn't live long enough
}

// ✅ Correct
let lineage_ids = context.lineage_ids();
|...| async move {
    lineage_ids  // OK: owned value
}
```

### Trait Bound Errors

**Error**: `the trait bound ... is not satisfied`
**Solution**: Add missing trait bounds

```rust
where
    GrpcReq: Serialize + Debug + Send,  // Add Send for async
    GrpcResp: Serialize + Debug + Clone + Send,  // Add Clone + Send
```

### Type Inference Errors

**Error**: `type annotations needed`
**Solution**: Provide explicit type parameters or use turbofish

```rust
// Explicit types
ucs_executor::<domain::Authorize, AuthorizeUcsExecutor, PaymentsAuthorizeData, PaymentsResponseData, _, _>(...)

// Or let compiler infer
let result: RouterData<...> = ucs_executor(...).await?;
```

---

## Summary

The UCS architecture provides:
- ✅ **Type-safe GRPC integration** with compile-time guarantees
- ✅ **Flexible lifetime management** using GATs
- ✅ **Code reuse** through trait-based polymorphism
- ✅ **Easy extensibility** for new flows
- ✅ **Comprehensive error handling** with context propagation
- ✅ **Clear separation of concerns** between infrastructure and business logic

For usage examples and implementation guides, see `UCS_USAGE_GUIDE.md`.