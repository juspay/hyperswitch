# Gateway Abstraction Layer - Usage Guide

## Overview

The Gateway Abstraction Layer provides a unified interface for executing payment operations through either Direct connector integration or Unified Connector Service (UCS). This eliminates the need for flow developers to handle cutover logic manually.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Payment Flow                              │
│                 (authorize_flow.rs, etc.)                    │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                  GatewayFactory::create()                    │
│              (Decision logic centralized here)               │
└────────────────────────┬────────────────────────────────────┘
                         │
         ┌───────────────┼───────────────┐
         │                               │
         ▼                               ▼
┌──────────────────┐          ┌──────────────────────┐
│  DirectGateway   │          │ UCSGateway           │
│  (Traditional)   │          │ (gRPC Service)       │
└────────┬─────────┘          └──────────┬───────────┘
         │                               │
         ▼                               ▼
┌──────────────────┐          ┌──────────────────────┐
│ execute_         │          │ UCS gRPC Client      │
│ connector_       │          │ (payment_authorize,  │
│ processing_step  │          │  payment_get, etc.)  │
└──────────────────┘          └──────────────────────┘
```

## Basic Usage

### Before (Old Way - 50+ lines)

```rust
// In authorize_flow.rs
async fn call_connector_service(
    state: &SessionState,
    connector: &api::ConnectorData,
    router_data: &RouterData<...>,
    payment_data: &PaymentData,
    merchant_connector_account: &MerchantConnectorAccountType,
    call_connector_action: CallConnectorAction,
) -> RouterResult<RouterData<...>> {
    // Determine execution path
    let execution_path = decide_unified_connector_service_call(
        state,
        &connector.merchant_connector_id,
        router_data,
        Some(payment_data),
    ).await?;

    // Route based on execution path
    match execution_path {
        ExecutionPath::Direct => {
            let connector_integration = connector.connector.get_connector_integration();
            services::execute_connector_processing_step(
                state,
                connector_integration,
                router_data,
                call_connector_action,
                None,
                None,
            ).await
        }
        ExecutionPath::UnifiedConnectorService => {
            call_unified_connector_service_authorize(
                router_data,
                state,
                header_payload,
                lineage_ids,
                merchant_connector_account,
                merchant_context,
                execution_mode,
            ).await
        }
        ExecutionPath::ShadowUnifiedConnectorService => {
            process_through_direct_with_shadow_unified_connector_service(
                state,
                connector,
                router_data,
                payment_data,
                merchant_connector_account,
                call_connector_action,
            ).await
        }
    }
}
```

### After (New Way - 2 lines)

```rust
// In authorize_flow.rs
async fn call_connector_service(
    state: &SessionState,
    connector: &api::ConnectorData,
    router_data: &RouterData<...>,
    payment_data: &PaymentData,
    merchant_connector_account: &MerchantConnectorAccountType,
    call_connector_action: CallConnectorAction,
) -> RouterResult<RouterData<...>> {
    // Create gateway (decision logic handled internally)
    let gateway = GatewayFactory::create_authorize_gateway(
        state,
        connector,
        router_data,
        Some(payment_data),
    ).await?;

    // Execute through gateway
    gateway.execute(
        state,
        router_data.clone(),
        connector,
        merchant_connector_account,
        call_connector_action,
    ).await
}
```

## Flow-Specific Examples

### 1. Authorize Flow

```rust
use crate::core::payments::gateway::{GatewayFactory, PaymentGateway};

// In authorize_flow.rs
impl ConstructFlowSpecificData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for PaymentData
{
    async fn construct_router_data(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_connector_account: &MerchantConnectorAccountType,
        call_connector_action: CallConnectorAction,
    ) -> RouterResult<RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>> {
        // Build router_data as usual
        let router_data = build_router_data(...)?;

        // Create gateway and execute
        let gateway = GatewayFactory::create_authorize_gateway(
            state,
            connector,
            &router_data,
            Some(self),
        ).await?;

        gateway.execute(
            state,
            router_data,
            connector,
            merchant_connector_account,
            call_connector_action,
        ).await
    }
}
```

### 2. PSync Flow

```rust
// In psync_flow.rs
impl ConstructFlowSpecificData<api::PSync, PaymentsSyncData, PaymentsResponseData>
    for PaymentData
{
    async fn construct_router_data(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_connector_account: &MerchantConnectorAccountType,
        call_connector_action: CallConnectorAction,
    ) -> RouterResult<RouterData<api::PSync, PaymentsSyncData, PaymentsResponseData>> {
        let router_data = build_router_data(...)?;

        let gateway = GatewayFactory::create_psync_gateway(
            state,
            connector,
            &router_data,
            Some(self),
        ).await?;

        gateway.execute(
            state,
            router_data,
            connector,
            merchant_connector_account,
            call_connector_action,
        ).await
    }
}
```

### 3. Setup Mandate Flow

```rust
// In setup_mandate_flow.rs
impl ConstructFlowSpecificData<api::SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for PaymentData
{
    async fn construct_router_data(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_connector_account: &MerchantConnectorAccountType,
        call_connector_action: CallConnectorAction,
    ) -> RouterResult<RouterData<api::SetupMandate, SetupMandateRequestData, PaymentsResponseData>> {
        let router_data = build_router_data(...)?;

        let gateway = GatewayFactory::create_setup_mandate_gateway(
            state,
            connector,
            &router_data,
            Some(self),
        ).await?;

        gateway.execute(
            state,
            router_data,
            connector,
            merchant_connector_account,
            call_connector_action,
        ).await
    }
}
```

## How It Works

### 1. Gateway Factory Decision Logic

The `GatewayFactory` internally calls the existing `should_call_unified_connector_service()` function to determine the execution path:

```rust
// Internal implementation
async fn determine_execution_path(...) -> RouterResult<GatewayExecutionPath> {
    let execution_path = ucs::should_call_unified_connector_service(
        state,
        merchant_connector_id,
        router_data,
        payment_data,
    ).await?;

    Ok(match execution_path {
        ExecutionPath::Direct => GatewayExecutionPath::Direct,
        ExecutionPath::UnifiedConnectorService => GatewayExecutionPath::UnifiedConnectorService,
        ExecutionPath::ShadowUnifiedConnectorService => GatewayExecutionPath::ShadowUnifiedConnectorService,
    })
}
```

### 2. Direct Gateway Execution

For Direct path, the gateway simply wraps `execute_connector_processing_step`:

```rust
// DirectGateway implementation
async fn execute(...) -> RouterResult<RouterData<...>> {
    services::execute_connector_processing_step(
        state,
        self.connector_integration.clone(),
        &router_data,
        call_connector_action,
        None,
        None,
    ).await
}
```

### 3. UCS Gateway Execution

For UCS path, the gateway handles transformation and gRPC calls:

```rust
// UnifiedConnectorServiceGateway implementation
async fn execute(...) -> RouterResult<RouterData<...>> {
    // 1. Get UCS client
    let client = state.grpc_client.unified_connector_service_client.as_ref()?;

    // 2. Transform RouterData → gRPC request
    let grpc_request = PaymentServiceAuthorizeRequest::foreign_try_from(&router_data)?;

    // 3. Build auth metadata
    let auth_metadata = build_unified_connector_service_auth_metadata(...)?;

    // 4. Call UCS
    let response = client.payment_authorize(grpc_request, auth_metadata, headers).await?;

    // 5. Transform gRPC response → RouterData
    let (payments_response, attempt_status, http_status_code) =
        handle_unified_connector_service_response_for_payment_authorize(response.into_inner())?;

    // 6. Update router_data
    router_data.response = payments_response;
    router_data.status = attempt_status;
    router_data.connector_http_status_code = Some(http_status_code);

    Ok(router_data)
}
```

## Benefits

### For Flow Developers

1. **No Cutover Logic**: Just call `GatewayFactory::create_*_gateway()` and `gateway.execute()`
2. **Consistent API**: Same interface for all flows
3. **Type Safety**: Compiler ensures correct flow types
4. **Easy Testing**: Mock gateway implementations for unit tests

### For Maintainers

1. **Centralized Decision Logic**: All cutover logic in `GatewayFactory`
2. **Single Source of Truth**: Reuses existing `should_call_unified_connector_service()`
3. **Easy to Extend**: Add new gateway types (fallback, circuit breaker, etc.)
4. **No Duplication**: 20+ flows use same pattern

### For Operations

1. **Gradual Rollout**: Existing rollout configs work unchanged
2. **Shadow Mode**: A/B testing continues to work
3. **Easy Rollback**: Change config to switch back to Direct
4. **Monitoring**: Centralized metrics and logging

## Migration Guide

### Phase 1: Add Gateway Layer (Non-Breaking)

Add gateway calls alongside existing code:

```rust
// Keep old code
let router_data = services::execute_connector_processing_step(...).await?;

// Add new code (feature flagged)
if state.conf.features.use_gateway_abstraction {
    let gateway = GatewayFactory::create_authorize_gateway(...).await?;
    let router_data = gateway.execute(...).await?;
}
```

### Phase 2: Feature Flag Rollout

Enable for specific merchants/connectors:

```toml
[features]
use_gateway_abstraction = true

[gateway_rollout]
enabled_merchants = ["merchant_1", "merchant_2"]
enabled_connectors = ["stripe", "adyen"]
```

### Phase 3: Full Migration

Remove old code paths after validation:

```rust
// Final state - only gateway code
let gateway = GatewayFactory::create_authorize_gateway(...).await?;
let router_data = gateway.execute(...).await?;
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_direct_gateway() {
        let gateway = DirectGateway::new(mock_connector_integration());
        let result = gateway.execute(...).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ucs_gateway() {
        let gateway = UnifiedConnectorServiceGateway::<api::Authorize>::new();
        let result = gateway.execute(...).await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_gateway_factory_direct_path() {
    let state = setup_test_state_with_direct_config();
    let gateway = GatewayFactory::create_authorize_gateway(...).await?;
    // Verify DirectGateway was created
}

#[tokio::test]
async fn test_gateway_factory_ucs_path() {
    let state = setup_test_state_with_ucs_config();
    let gateway = GatewayFactory::create_authorize_gateway(...).await?;
    // Verify UnifiedConnectorServiceGateway was created
}
```

## Troubleshooting

### Issue: Gateway not using UCS even though configured

**Solution**: Check UCS availability and rollout configs:
- Verify `grpc_client.unified_connector_service_client` is initialized
- Check `ucs_only_connectors` list includes your connector
- Verify rollout percentage config for merchant/connector/flow

### Issue: Compilation errors with gateway types

**Solution**: Ensure correct flow type parameters:
- Use `api::Authorize` for authorization flows
- Use `api::PSync` for payment sync flows
- Use `api::SetupMandate` for mandate setup flows

### Issue: UCS calls failing

**Solution**: Check UCS client configuration:
- Verify `base_url` is correct
- Check `connection_timeout` is sufficient
- Ensure auth metadata is properly configured

## Future Enhancements

1. **Shadow Gateway**: Implement proper shadow execution with comparison
2. **Fallback Gateway**: Automatic fallback to Direct on UCS failure
3. **Circuit Breaker**: Prevent cascading failures
4. **Retry Logic**: Configurable retry strategies per gateway
5. **Metrics**: Detailed metrics per gateway type
6. **Tracing**: Distributed tracing across gateways

## Support

For questions or issues:
1. Check existing UCS documentation
2. Review `should_call_unified_connector_service()` decision logic
3. Examine flow-specific examples in this guide
4. Contact the payments team