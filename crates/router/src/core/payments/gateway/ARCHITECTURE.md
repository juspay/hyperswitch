# Gateway Abstraction Layer - Architecture Deep Dive

## System Architecture

### High-Level Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Application Layer                                │
│                    (Payment API Endpoints)                               │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      Payment Operations Layer                            │
│     (PaymentConfirm, PaymentCapture, PaymentStatus, etc.)              │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       Payment Flows Layer                                │
│   (authorize_flow, psync_flow, setup_mandate_flow, etc.)               │
│                                                                          │
│   OLD WAY:                          NEW WAY (Gateway):                  │
│   ┌──────────────────────┐          ┌──────────────────────┐           │
│   │ decide_ucs_call()    │          │ GatewayFactory::     │           │
│   │ match execution_path │          │   create_*_gateway() │           │
│   │   Direct => exec...  │   ──►    │ gateway.execute()    │           │
│   │   UCS => call_ucs... │          │                      │           │
│   │   Shadow => both...  │          │ (2 lines!)           │           │
│   └──────────────────────┘          └──────────────────────┘           │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    🆕 GATEWAY ABSTRACTION LAYER 🆕                       │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────┐    │
│  │                     GatewayFactory                              │    │
│  │  ┌──────────────────────────────────────────────────────────┐  │    │
│  │  │ determine_execution_path()                                │  │    │
│  │  │   ↓                                                       │  │    │
│  │  │ should_call_unified_connector_service()                  │  │    │
│  │  │   ├─ check_ucs_availability()                           │  │    │
│  │  │   ├─ determine_connector_integration_type()             │  │    │
│  │  │   ├─ extract_previous_gateway()                         │  │    │
│  │  │   └─ decide_execution_path()                            │  │    │
│  │  │       ├─ Direct                                          │  │    │
│  │  │       ├─ UnifiedConnectorService                         │  │    │
│  │  │       └─ ShadowUnifiedConnectorService                   │  │    │
│  │  └──────────────────────────────────────────────────────────┘  │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                             │                                            │
│              ┌──────────────┼──────────────┐                            │
│              │                              │                            │
│              ▼                              ▼                            │
│  ┌─────────────────────┐        ┌─────────────────────────┐            │
│  │   DirectGateway     │        │ UCSGateway              │            │
│  │                     │        │                         │            │
│  │ ┌─────────────────┐ │        │ ┌─────────────────────┐ │            │
│  │ │ execute()       │ │        │ │ execute()           │ │            │
│  │ │   ↓             │ │        │ │   ↓                 │ │            │
│  │ │ execute_        │ │        │ │ match flow_type:    │ │            │
│  │ │ connector_      │ │        │ │   Authorize:        │ │            │
│  │ │ processing_step │ │        │ │     CIT → authorize │ │            │
│  │ │                 │ │        │ │     MIT → repeat    │ │            │
│  │ └─────────────────┘ │        │ │   PSync → get       │ │            │
│  └─────────────────────┘        │ │   SetupMandate →    │ │            │
│                                  │ │     setup_mandate   │ │            │
│                                  │ └─────────────────────┘ │            │
│                                  └─────────────────────────┘            │
└────────────────────────────┬─────────────────┬───────────────────────────┘
                             │                 │
                             ▼                 ▼
┌──────────────────────────────────┐  ┌────────────────────────────────┐
│   Traditional Connector Layer    │  │  Unified Connector Service     │
│                                   │  │         (gRPC)                 │
│  ┌────────────────────────────┐  │  │  ┌──────────────────────────┐ │
│  │ ConnectorIntegration       │  │  │  │ PaymentServiceClient     │ │
│  │   build_request()          │  │  │  │   payment_authorize()    │ │
│  │   handle_response()        │  │  │  │   payment_get()          │ │
│  │   get_error_response()     │  │  │  │   payment_setup_mandate()│ │
│  └────────────────────────────┘  │  │  │   payment_repeat()       │ │
│              ↓                    │  │  └──────────────────────────┘ │
│  ┌────────────────────────────┐  │  │              ↓                 │
│  │ HTTP Client                │  │  │  ┌──────────────────────────┐ │
│  │   call_connector_api()     │  │  │  │ gRPC Transport           │ │
│  └────────────────────────────┘  │  │  └──────────────────────────┘ │
└──────────────────────────────────┘  └────────────────────────────────┘
                 ↓                                    ↓
┌──────────────────────────────────┐  ┌────────────────────────────────┐
│   External Payment Connectors    │  │  UCS Microservice              │
│   (Stripe, Adyen, PayPal, etc.)  │  │  (Handles all connectors)      │
└──────────────────────────────────┘  └────────────────────────────────┘
```

## Component Interaction Flow

### 1. Authorize Flow (CIT) - Direct Path

```
┌─────────────┐
│ API Request │
│ POST /      │
│ payments/   │
│ confirm     │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ PaymentConfirm Operation                │
│ - Validate request                      │
│ - Load payment data from DB             │
│ - Construct PaymentData                 │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ authorize_flow.rs                       │
│ ConstructFlowSpecificData trait         │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ GatewayFactory::create_authorize_gateway│
│                                         │
│ 1. Call should_call_ucs()               │
│    ├─ UCS available? ✓                  │
│    ├─ Connector in ucs_only? ✗          │
│    ├─ Rollout enabled? ✗                │
│    └─ Result: ExecutionPath::Direct     │
│                                         │
│ 2. Create DirectGateway                 │
│    └─ Get connector_integration         │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ DirectGateway::execute()                │
│                                         │
│ Call execute_connector_processing_step  │
│   ├─ connector_integration              │
│   │   .build_request(router_data)       │
│   │   → Request { url, headers, body }  │
│   │                                     │
│   ├─ call_connector_api(request)        │
│   │   → HTTP POST to Stripe             │
│   │   ← Response { status: 200, body }  │
│   │                                     │
│   └─ connector_integration              │
│       .handle_response(response)        │
│       → PaymentsResponseData            │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Updated RouterData                      │
│ - response: Ok(PaymentsResponseData)    │
│ - status: Charged                       │
│ - connector_http_status_code: 200       │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Post-Processing                         │
│ - Update payment_attempt in DB          │
│ - Update payment_intent in DB           │
│ - Trigger webhooks                      │
│ - Return response to client             │
└─────────────────────────────────────────┘
```

### 2. Authorize Flow (CIT) - UCS Path

```
┌─────────────┐
│ API Request │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ PaymentConfirm Operation                │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ authorize_flow.rs                       │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ GatewayFactory::create_authorize_gateway│
│                                         │
│ 1. Call should_call_ucs()               │
│    ├─ UCS available? ✓                  │
│    ├─ Connector in ucs_only? ✓ (paytm)  │
│    ├─ Previous gateway? None            │
│    └─ Result: ExecutionPath::UCS        │
│                                         │
│ 2. Create UnifiedConnectorServiceGateway│
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ UnifiedConnectorServiceGateway::execute │
│                                         │
│ 1. Get UCS client                       │
│    └─ state.grpc_client.ucs_client      │
│                                         │
│ 2. Check mandate_id                     │
│    └─ None (CIT flow)                   │
│                                         │
│ 3. Transform RouterData → gRPC          │
│    PaymentServiceAuthorizeRequest {     │
│      amount: 1000,                      │
│      currency: USD,                     │
│      payment_method: Card {...},        │
│      address: {...},                    │
│      ...                                │
│    }                                    │
│                                         │
│ 4. Build auth metadata                  │
│    ConnectorAuthMetadata {              │
│      connector_name: "paytm",           │
│      auth_type: "HeaderKey",            │
│      api_key: Secret("..."),            │
│      merchant_id: Secret("..."),        │
│    }                                    │
│                                         │
│ 5. Build gRPC headers                   │
│    GrpcHeadersUcs {                     │
│      lineage_ids: [...],                │
│      request_id: "...",                 │
│      tenant_id: "...",                  │
│    }                                    │
│                                         │
│ 6. Call UCS                             │
│    client.payment_authorize(            │
│      request, auth_metadata, headers    │
│    )                                    │
│    → gRPC call to UCS service           │
│    ← PaymentServiceAuthorizeResponse    │
│                                         │
│ 7. Handle response                      │
│    handle_ucs_response_for_authorize()  │
│    → (PaymentsResponseData,             │
│       AttemptStatus::Charged,           │
│       200)                              │
│                                         │
│ 8. Update router_data                   │
│    router_data.response = Ok(...)       │
│    router_data.status = Charged         │
│    router_data.connector_http_status... │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Updated RouterData                      │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Post-Processing                         │
└─────────────────────────────────────────┘
```

### 3. PSync Flow - UCS Path

```
┌─────────────┐
│ GET /       │
│ payments/   │
│ {id}        │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ PaymentStatus Operation                 │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ psync_flow.rs                           │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ GatewayFactory::create_psync_gateway    │
│                                         │
│ Result: ExecutionPath::UCS              │
│ Create: UnifiedConnectorServiceGateway  │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ UnifiedConnectorServiceGateway::execute │
│                                         │
│ 1. Transform RouterData → gRPC          │
│    PaymentServiceGetRequest {           │
│      transaction_id: "txn_123",         │
│      request_ref_id: "ref_456",         │
│    }                                    │
│                                         │
│ 2. Call UCS                             │
│    client.payment_get(...)              │
│    ← PaymentServiceGetResponse          │
│                                         │
│ 3. Handle response                      │
│    handle_ucs_response_for_get()        │
│    → (PaymentsResponseData, status)     │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Updated RouterData with sync status     │
└─────────────────────────────────────────┘
```

## Decision Logic Deep Dive

### should_call_unified_connector_service() Flow

```
┌─────────────────────────────────────────────────────────────┐
│ should_call_unified_connector_service()                     │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 1: check_ucs_availability()                            │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ Is UCS client initialized?                             │  │
│ │   state.grpc_client.unified_connector_service_client   │  │
│ │                                                         │  │
│ │ Is UCS enabled in config?                              │  │
│ │   config.get("consts::UCS_ENABLED")                    │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ Result: UcsAvailability::Enabled | Disabled                 │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 2: determine_connector_integration_type()              │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ Is connector in ucs_only_connectors list?              │  │
│ │   config.ucs_only_connectors.contains(connector)       │  │
│ │   Example: ["paytm", "phonepe"]                        │  │
│ │                                                         │  │
│ │ OR                                                      │  │
│ │                                                         │  │
│ │ Is rollout enabled for this combination?               │  │
│ │   Key: ucs_rollout_percent_{merchant}_{connector}_     │  │
│ │        {payment_method}_{flow}                         │  │
│ │   Example: ucs_rollout_percent_merchant123_stripe_     │  │
│ │            card_authorize = 50                         │  │
│ │                                                         │  │
│ │   Random(0-100) < rollout_percent?                     │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ Result: ConnectorIntegrationType::UcsConnector | Direct     │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 3: extract_previous_gateway()                          │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ Check payment_intent.feature_metadata.gateway_system   │  │
│ │                                                         │  │
│ │ Values:                                                 │  │
│ │   - GatewaySystem::Direct                              │  │
│ │   - GatewaySystem::UnifiedConnectorService             │  │
│ │   - None (first attempt)                               │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ Purpose: Transaction consistency - continue with same       │
│          gateway for subsequent operations                  │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 4: check_shadow_rollout()                              │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ Is shadow mode enabled?                                │  │
│ │   Key: {rollout_key}_shadow                            │  │
│ │   Example: ucs_rollout_percent_merchant123_stripe_     │  │
│ │            card_authorize_shadow = 100                 │  │
│ │                                                         │  │
│ │   Random(0-100) < shadow_percent?                      │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ Result: ShadowRollout::Available | NotAvailable             │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 5: decide_execution_path()                             │
│                                                              │
│ Decision Matrix (10 cases):                                 │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ Case 1: DirectConnector + No Previous + No Shadow     │  │
│ │   → ExecutionPath::Direct                             │  │
│ │                                                         │  │
│ │ Case 2: DirectConnector + Direct Previous + No Shadow │  │
│ │   → ExecutionPath::Direct                             │  │
│ │                                                         │  │
│ │ Case 3: DirectConnector + UCS Previous + No Shadow    │  │
│ │   → ExecutionPath::Direct (migration back)            │  │
│ │                                                         │  │
│ │ Case 4: UcsConnector + Direct Previous + No Shadow    │  │
│ │   → ExecutionPath::Direct (consistency)               │  │
│ │                                                         │  │
│ │ Case 5-8: DirectConnector + Shadow Available          │  │
│ │   → ExecutionPath::ShadowUnifiedConnectorService      │  │
│ │   (Execute Direct as primary, UCS in background)      │  │
│ │                                                         │  │
│ │ Case 9: UcsConnector + No Previous + No Shadow        │  │
│ │   → ExecutionPath::UnifiedConnectorService            │  │
│ │                                                         │  │
│ │ Case 10: UcsConnector + UCS Previous + No Shadow      │  │
│ │   → ExecutionPath::UnifiedConnectorService            │  │
│ └────────────────────────────────────────────────────────┘  │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ Return: ExecutionPath                                       │
│   - Direct                                                  │
│   - UnifiedConnectorService                                 │
│   - ShadowUnifiedConnectorService                           │
└─────────────────────────────────────────────────────────────┘
```

## Data Transformation Flow

### RouterData → gRPC Request (Authorize)

```
RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>
│
├─ request: PaymentsAuthorizeData
│  ├─ amount: 1000 (minor units)
│  ├─ currency: Currency::USD
│  ├─ payment_method_data: PaymentMethodData::Card(Card {...})
│  ├─ capture_method: CaptureMethod::Automatic
│  ├─ browser_info: BrowserInformation {...}
│  ├─ customer_id: "cust_123"
│  └─ ...
│
├─ merchant_id: "merchant_123"
├─ connector: "stripe"
├─ connector_auth_type: ConnectorAuthType::HeaderKey {...}
├─ address: PaymentAddress {...}
└─ ...

                    ↓ ForeignTryFrom

PaymentServiceAuthorizeRequest (gRPC protobuf)
│
├─ amount: 1000
├─ currency: payments_grpc::Currency::Usd
├─ payment_method: payments_grpc::PaymentMethod {
│    card: Some(payments_grpc::Card {
│      number: "4242424242424242",
│      expiry_month: "12",
│      expiry_year: "2025",
│      cvv: "123",
│    })
│  }
├─ capture_method: payments_grpc::CaptureMethod::Automatic
├─ browser_info: payments_grpc::BrowserInformation {...}
├─ address: payments_grpc::PaymentAddress {...}
├─ connector_customer_id: "cust_123"
└─ ...
```

### gRPC Response → RouterData (Authorize)

```
PaymentServiceAuthorizeResponse (gRPC protobuf)
│
├─ status: payments_grpc::PaymentStatus::Charged
├─ connector_transaction_id: "ch_3abc123"
├─ connector_reference_id: "ref_456"
├─ raw_connector_response: "{\"id\":\"ch_3abc123\",...}"
├─ error_message: None
└─ ...

                    ↓ handle_unified_connector_service_response_for_payment_authorize

(PaymentsResponseData, AttemptStatus, u16)
│
├─ PaymentsResponseData {
│    status: enums::AttemptStatus::Charged,
│    connector_transaction_id: Some("ch_3abc123"),
│    connector_metadata: Some(serde_json::Value {...}),
│    ...
│  }
│
├─ AttemptStatus::Charged
│
└─ 200 (HTTP status code)

                    ↓ Update RouterData

RouterData {
  response: Ok(PaymentsResponseData {...}),
  status: AttemptStatus::Charged,
  connector_http_status_code: Some(200),
  ...
}
```

## Configuration Examples

### UCS-Only Connectors

```toml
[grpc_client.unified_connector_service]
base_url = "http://ucs-service:8000"
connection_timeout = 10
ucs_only_connectors = "paytm,phonepe,cashfree"
```

**Effect**: These connectors ALWAYS use UCS path (no rollout needed)

### Percentage-Based Rollout

```toml
# 50% of Stripe card authorizations for merchant_123 go to UCS
ucs_rollout_percent_merchant123_stripe_card_authorize = 50

# 100% of Adyen card authorizations for merchant_456 go to UCS
ucs_rollout_percent_merchant456_adyen_card_authorize = 100

# 0% = disabled (all go to Direct)
ucs_rollout_percent_merchant789_paypal_wallet_authorize = 0
```

### Shadow Mode

```toml
# Primary: Direct, Shadow: UCS (100% shadow execution)
ucs_rollout_percent_merchant123_stripe_card_authorize = 0
ucs_rollout_percent_merchant123_stripe_card_authorize_shadow = 100
```

**Effect**: 
- All requests go through Direct path (primary)
- 100% also execute through UCS in background (shadow)
- Results are compared for validation
- User sees Direct path response

## Error Handling

### Direct Gateway Errors

```
DirectGateway::execute()
  ↓
execute_connector_processing_step()
  ↓
call_connector_api()
  ↓
[HTTP Error: 500]
  ↓
connector_integration.get_5xx_error_response()
  ↓
RouterData {
  response: Err(ErrorResponse {
    code: "500",
    message: "Internal Server Error",
    status_code: 500,
    attempt_status: Some(AttemptStatus::Failure),
  }),
  status: AttemptStatus::Failure,
}
```

### UCS Gateway Errors

```
UnifiedConnectorServiceGateway::execute()
  ↓
client.payment_authorize()
  ↓
[gRPC Error: UNAVAILABLE]
  ↓
.change_context(ApiErrorResponse::InternalServerError)
.attach_printable("UCS payment_authorize call failed")
  ↓
RouterResult::Err(...)
  ↓
Error propagated to flow layer
```

## Performance Considerations

### Direct Path
- **Latency**: HTTP request to connector (~200-500ms)
- **Overhead**: Minimal (direct HTTP call)
- **Scalability**: Limited by connector rate limits

### UCS Path
- **Latency**: gRPC call to UCS + UCS to connector (~250-600ms)
- **Overhead**: gRPC serialization + transformation (~10-20ms)
- **Scalability**: Better (UCS handles rate limiting, retries)

### Gateway Abstraction Overhead
- **Factory creation**: ~1-2ms (decision logic)
- **Trait dispatch**: ~0.1ms (virtual function call)
- **Total overhead**: ~1-3ms (negligible)

## Monitoring & Observability

### Metrics

```
# Gateway selection
gateway_selection_total{gateway="direct"} 1000
gateway_selection_total{gateway="ucs"} 500
gateway_selection_total{gateway="shadow"} 200

# Execution time
gateway_execution_duration_seconds{gateway="direct", flow="authorize"} 0.250
gateway_execution_duration_seconds{gateway="ucs", flow="authorize"} 0.300

# Success rate
gateway_success_rate{gateway="direct"} 0.98
gateway_success_rate{gateway="ucs"} 0.97
```

### Logging

```
[INFO] GatewayFactory: Creating authorize gateway
  merchant_id=merchant_123
  connector=stripe
  execution_path=UnifiedConnectorService

[INFO] UnifiedConnectorServiceGateway: Executing authorize
  flow=Authorize
  is_mandate=false
  method=payment_authorize

[INFO] UnifiedConnectorServiceGateway: UCS call successful
  duration_ms=280
  status=Charged
  http_status=200
```

## Summary

The Gateway Abstraction Layer provides:

1. **Unified Interface**: Single API for all execution paths
2. **Transparent Cutover**: Decision logic hidden from flows
3. **Type Safety**: Compile-time verification
4. **Flexibility**: Easy to add new gateway types
5. **Observability**: Comprehensive metrics and logging
6. **Performance**: Minimal overhead (~1-3ms)

**Key Achievement**: Reduced flow integration complexity from 50+ lines to 2 lines while maintaining all existing functionality.