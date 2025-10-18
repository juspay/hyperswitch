# Gateway Abstraction Layer

## Overview

The Gateway Abstraction Layer provides a unified interface for executing payment operations through either:
- **Direct Path**: Traditional connector integration via `execute_connector_processing_step`
- **UCS Path**: Unified Connector Service via gRPC

This abstraction eliminates the need for payment flows to handle cutover logic manually.

## Architecture

### Core Components

1. **PaymentGateway Trait** (`mod.rs`)
   - Defines the unified interface for all gateways
   - Single `execute()` method for all payment operations

2. **DirectGateway** (`direct.rs`)
   - Wraps `execute_connector_processing_step`
   - Maintains backward compatibility with existing flows

3. **UnifiedConnectorServiceGateway** (`ucs.rs`)
   - Handles gRPC transformations and calls
   - Implements flow-specific UCS methods (authorize, get, setup_mandate)

4. **GatewayFactory** (`factory.rs`)
   - Centralizes cutover decision logic
   - Creates appropriate gateway based on configuration

## Key Features

### ✅ Transparent Cutover
- Decision logic centralized in `GatewayFactory`
- Flows don't need to know about Direct vs UCS
- Reuses existing `should_call_unified_connector_service()` logic

### ✅ Type Safety
- Generic over Flow, Request, and Response types
- Compile-time verification of flow compatibility
- Prevents mismatched flow/gateway combinations

### ✅ Backward Compatible
- Can be added alongside existing code
- Feature flag support for gradual rollout
- Easy rollback to old implementation

### ✅ Developer Friendly
- Simple 2-line API: create gateway, execute
- No need to understand cutover complexity
- Consistent pattern across all flows

## Flow-to-Gateway Mapping

| Flow Type | Gateway Method | UCS Client Method | Status |
|-----------|---------------|-------------------|--------|
| `api::Authorize` (CIT) | `create_authorize_gateway()` | `payment_authorize()` | ✅ Implemented |
| `api::Authorize` (MIT) | `create_authorize_gateway()` | `payment_repeat()` | ✅ Implemented |
| `api::PSync` | `create_psync_gateway()` | `payment_get()` | ✅ Implemented |
| `api::SetupMandate` | `create_setup_mandate_gateway()` | `payment_setup_mandate()` | ✅ Implemented |
| `api::Capture` | `create_capture_gateway()` | *(Not in UCS yet)* | 🚧 Direct only |
| `api::Void` | `create_void_gateway()` | *(Not in UCS yet)* | 🚧 Direct only |

## Usage

### Quick Start

```rust
use crate::core::payments::gateway::{GatewayFactory, PaymentGateway};

// 1. Create gateway (decision logic handled internally)
let gateway = GatewayFactory::create_authorize_gateway(
    state,
    connector,
    &router_data,
    Some(payment_data),
).await?;

// 2. Execute through gateway
let result = gateway.execute(
    state,
    router_data,
    connector,
    merchant_connector_account,
    call_connector_action,
).await?;
```

### Complete Example

See [USAGE_EXAMPLE.md](./USAGE_EXAMPLE.md) for detailed examples of:
- Authorize flow integration
- PSync flow integration
- Setup Mandate flow integration
- Testing strategies
- Migration guide

## Decision Logic

The `GatewayFactory` determines the execution path based on:

1. **UCS Availability**: Is UCS client configured and enabled?
2. **Connector Type**: Is connector in `ucs_only_connectors` list?
3. **Rollout Config**: Percentage-based rollout per merchant/connector/flow
4. **Previous Gateway**: Transaction consistency (continue with same gateway)
5. **Shadow Mode**: A/B testing configuration

### Configuration

```toml
[grpc_client.unified_connector_service]
base_url = "http://localhost:8000"
connection_timeout = 10
ucs_only_connectors = "paytm,phonepe"
ucs_psync_disabled_connectors = "cashtocode"

# Rollout percentage (0-100)
[rollout]
ucs_rollout_percent_merchant123_stripe_card_authorize = 50
ucs_rollout_percent_merchant123_stripe_card_authorize_shadow = 100
```

## Implementation Details

### DirectGateway

```rust
pub struct DirectGateway<F, ResourceCommonData, Req, Resp> {
    pub connector_integration: BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>,
}

// Simply delegates to execute_connector_processing_step
async fn execute(...) -> RouterResult<RouterData<F, Req, Resp>> {
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

### UnifiedConnectorServiceGateway

```rust
pub struct UnifiedConnectorServiceGateway<F> {
    flow_type: PhantomData<F>,
}

// Handles transformation and gRPC calls
async fn execute(...) -> RouterResult<RouterData<F, Req, Resp>> {
    // 1. Get UCS client
    // 2. Transform RouterData → gRPC request
    // 3. Build auth metadata
    // 4. Call UCS method (authorize/get/setup_mandate)
    // 5. Transform gRPC response → RouterData
    // 6. Update router_data fields
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_direct_gateway_execution() {
        let gateway = DirectGateway::new(mock_integration());
        let result = gateway.execute(...).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ucs_gateway_authorize() {
        let gateway = UnifiedConnectorServiceGateway::<api::Authorize>::new();
        let result = gateway.execute(...).await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Test the factory decision logic:

```rust
#[tokio::test]
async fn test_factory_selects_direct_gateway() {
    let state = setup_state_with_direct_config();
    let gateway = GatewayFactory::create_authorize_gateway(...).await?;
    // Verify DirectGateway was created
}

#[tokio::test]
async fn test_factory_selects_ucs_gateway() {
    let state = setup_state_with_ucs_config();
    let gateway = GatewayFactory::create_authorize_gateway(...).await?;
    // Verify UnifiedConnectorServiceGateway was created
}
```

## Migration Strategy

### Phase 1: Add Gateway Layer (Non-Breaking)
- Add gateway module alongside existing code
- No changes to existing flows
- Feature flag: `use_gateway_abstraction = false`

### Phase 2: Gradual Migration
- Migrate one flow at a time (start with authorize_flow.rs)
- Feature flag: `use_gateway_abstraction = true` for specific merchants
- Monitor metrics and errors

### Phase 3: Full Migration
- All flows use gateway abstraction
- Remove old code paths
- Remove feature flag

## Monitoring

### Metrics

The gateway layer integrates with existing metrics:
- `CONNECTOR_CALL_COUNT`: Incremented for Direct path
- UCS-specific metrics: Handled by UCS client
- Gateway selection metrics: Added by factory

### Logging

All gateway operations are logged with:
- Gateway type (Direct/UCS)
- Flow type (Authorize/PSync/etc.)
- Execution time
- Success/failure status

## Future Enhancements

1. **Shadow Gateway**: Proper implementation with result comparison
2. **Fallback Gateway**: Automatic fallback to Direct on UCS failure
3. **Circuit Breaker**: Prevent cascading failures
4. **Retry Logic**: Configurable retry strategies
5. **Metrics Dashboard**: Gateway-specific metrics visualization
6. **Additional Flows**: Capture, Void, Refund support in UCS

## Contributing

When adding a new flow:

1. Implement `PaymentGateway` trait for the flow type in `ucs.rs`
2. Add factory method in `factory.rs` (e.g., `create_capture_gateway()`)
3. Update flow file to use gateway abstraction
4. Add tests for the new gateway implementation
5. Update this README with the new flow mapping

## Support

For questions or issues:
- Review [USAGE_EXAMPLE.md](./USAGE_EXAMPLE.md) for detailed examples
- Check existing UCS documentation
- Contact the payments team

## License

Same as parent project