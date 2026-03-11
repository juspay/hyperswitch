# External Service Call Instrumentation - Problem Statement

## Overview

We want to capture all external service calls (KeyManager, Redis, PostgreSQL, HTTP connectors, etc.) as Kafka events for analytics. A separate service will consume these events, correlate them with API events using `request_id`, and write aggregated data to ClickHouse.

---

## Current State

### Existing Event Infrastructure

The codebase has multiple ways to emit events to Kafka:

#### 1. KafkaMessage Trait (router crate)
**File:** `crates/router/src/services/kafka.rs`

```rust
pub trait KafkaMessage: Serialize + Debug {
    fn key(&self) -> String;
    fn event_type(&self) -> EventType;
    fn value(&self) -> MQResult<Vec<u8>>;
    fn creation_timestamp(&self) -> Option<i64>;
}
```

**Usage:** All entity events (PaymentIntent, Refund, Dispute) and log events (ApiEvent, ConnectorEvent) use this trait.

**Emission:** `EventsHandler.log_event::<T: KafkaMessage>(&event)`

#### 2. MessagingInterface Trait (events crate)
**File:** `crates/events/src/lib.rs`

```rust
pub trait MessagingInterface {
    type MessageClass;
    fn send_message<T: Message>(
        &self,
        data: T,
        metadata: HashMap<String, String>,
        timestamp: PrimitiveDateTime,
    ) -> Result<(), EventsError>;
}
```

**Usage:** Only used by AuditEvent. Supports metadata accumulation via EventContext and Kafka headers.

#### 3. EventHandlerInterface Trait (hyperswitch_interfaces crate)
**File:** `crates/hyperswitch_interfaces/src/events.rs`

```rust
pub trait EventHandlerInterface: DynClone + Send + Sync {
    fn log_connector_event(&self, event: &ConnectorEvent);
}
```

**Purpose:** Allows crates outside `router` (like `subscriptions` microservice) to emit events without depending on `router`.

---

### Why Three Traits?

| Trait | Crate | Problem It Solves |
|-------|-------|-------------------|
| `KafkaMessage` | router | Events defined inside router can implement this directly |
| `MessagingInterface` | events | Generic event framework with metadata accumulation (AuditEvent) |
| `EventHandlerInterface` | hyperswitch_interfaces | Crates outside router need to emit events (subscriptions microservice) |

---

### Crate Dependency Graph

```
                    router
                      │
         ┌────────────┼────────────┐
         │            │            │
         ▼            ▼            ▼
   common_utils   hyperswitch_    events
                      interfaces
         │            │
         └─────◄──────┘
         (hyperswitch_interfaces depends on common_utils)
```

**Critical Constraints:**
- `common_utils` is a low-level crate - it CANNOT depend on `router` or `hyperswitch_interfaces`
- `hyperswitch_interfaces` depends on `common_utils`, so `common_utils` cannot depend on it
- Any trait that `common_utils` needs to use must be defined IN `common_utils` or in a crate that `common_utils` can import

---

## The Problem

We need to instrument `call_encryption_service()` in `common_utils/src/keymanager.rs` to emit `ExternalServiceCall` events.

```rust
// common_utils/src/keymanager.rs
pub async fn call_encryption_service<T, R>(
    state: &KeyManagerState,
    method: Method,
    endpoint: &str,
    request_body: T,
) -> errors::CustomResult<R, errors::KeyManagerClientError>
```

**Issues:**

1. **Where does `ExternalServiceCall` struct live?** - Must be accessible to crates outside router
2. **How does `call_encryption_service` emit events?** - Needs access to event emitter, but can't import from router
3. **How do we get `request_id`?** - Required for correlating events

---

## request_id Propagation Bug

`KeyManagerState` has a `request_id` field (feature-gated), but it's NOT reliably populated:

```
Timeline:
─────────────────────────────────────────────────────────────────────────
1. get_session_state() called
   └─► KeyManagerState created from AppState (request_id = None)
   └─► store.set_key_manager_state(key_manager_state)
       └─► Store's KeyManagerState.request_id = None

2. add_request_id(request_id) called  
   └─► SessionState.request_id = Some(request_id) ✓
   └─► KeyManagerState.request_id remains None! ✗ (not updated)

3. Code paths diverge:
   ├─► Payment flows: &state.into() → Gets request_id from SessionState ✓
   └─► Store operations: get_keymanager_state() → Gets None ✗
─────────────────────────────────────────────────────────────────────────
```

**Result:** Some call sites have `request_id`, others don't.

---

## Proposed Solution

### Step 1: Create Event Types in `common_utils`

**File:** `crates/common_utils/src/external_events.rs` (new file)

```rust
use serde::Serialize;
use time::PrimitiveDateTime;

/// External service call event for wide events capturing
#[derive(Debug, Clone, Serialize)]
pub struct ExternalServiceCall {
    pub request_id: String,
    pub service_name: String,
    pub event_id: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: Option<u16>,
    pub success: bool,
    pub latency_ms: u128,
    pub created_at: PrimitiveDateTime,
}

/// Type of external service being called
#[derive(Debug, Clone, Copy, Serialize)]
pub enum ServiceType {
    KeyManager,
    Redis,
    Postgres,
    Connector,
    Webhook,
}

/// Trait for emitting external service call events
/// Implement this trait in your service's event handler
pub trait EventEmitter: Send + Sync {
    fn emit(&self, event: &ExternalServiceCall);
}

/// Tracker for measuring external call latency
pub struct ExternalCallTracker {
    service_type: ServiceType,
    endpoint: String,
    method: String,
    start: std::time::Instant,
}

impl ExternalCallTracker {
    pub fn start(service_type: ServiceType, endpoint: impl Into<String>, method: impl Into<String>) -> Self;
    pub fn complete(self, request_id: String, status_code: Option<u16>, success: bool) -> ExternalServiceCall;
}
```

### Step 2: Implement EventEmitter in Router

**File:** `crates/router/src/events.rs`

```rust
impl common_utils::EventEmitter for EventsHandler {
    fn emit(&self, event: &ExternalServiceCall) {
        self.log_event(event);  // ExternalServiceCall implements KafkaMessage
    }
}
```

### Step 3: Implement KafkaMessage for ExternalServiceCall

**File:** `crates/router/src/events/external_service_call.rs`

```rust
use common_utils::external_events::ExternalServiceCall;
use crate::services::kafka::{KafkaMessage, EventType, MQResult};

impl KafkaMessage for ExternalServiceCall {
    fn key(&self) -> String {
        self.event_id.clone()
    }
    
    fn event_type(&self) -> EventType {
        EventType::ExternalServiceCall  // NEW variant needed
    }
    
    fn value(&self) -> MQResult<Vec<u8>> {
        serde_json::to_vec(self).map_err(|_| MQError::JsonSerializationFailed)
    }
    
    fn creation_timestamp(&self) -> Option<i64> {
        Some(self.created_at.assume_utc().unix_timestamp())
    }
}
```

### Step 4: Add EventType Variant and Topic

**File:** `crates/router/src/events.rs`

```rust
pub enum EventType {
    // ... existing variants
    ExternalServiceCall,  // NEW
}
```

**File:** `crates/router/src/configs/settings.rs`

```rust
pub struct KafkaSettings {
    // ... existing topics
    pub external_service_calls_topic: String,  // NEW
}
```

### Step 5: Instrument call_encryption_service

**File:** `crates/common_utils/src/keymanager.rs`

```rust
use crate::external_events::{EventEmitter, ExternalCallTracker, ServiceType};

pub async fn call_encryption_service<T, R>(
    state: &KeyManagerState,
    method: Method,
    endpoint: &str,
    request_body: T,
    request_id: Option<&str>,           // NEW: explicit request_id
    event_emitter: Option<&dyn EventEmitter>,  // NEW: event emitter
) -> errors::CustomResult<R, errors::KeyManagerClientError>
{
    let tracker = ExternalCallTracker::start(ServiceType::KeyManager, endpoint, method.as_str());
    
    // ... existing logic ...
    
    let (status_code, success) = match &result {
        Ok(resp) => (Some(resp.status().as_u16()), resp.status().is_success()),
        Err(_) => (None, false),
    };
    
    if let (Some(emitter), Some(rid)) = (event_emitter, request_id) {
        let event = tracker.complete(rid.to_string(), status_code, success);
        emitter.emit(&event);
    }
    
    result
}
```

### Step 6: Update Call Sites

All callers of `call_encryption_service` must pass `request_id` and `event_emitter`:

```rust
// Example: payment_create.rs
call_encryption_service(
    state,
    Method::POST,
    "key/create",
    request_body,
    state.request_id.as_ref().map(|r| r.as_str()),
    Some(&state.event_handler),
).await
```

---

## Files to Create/Modify

| File | Action |
|------|--------|
| `crates/common_utils/src/external_events.rs` | **CREATE** - ExternalServiceCall, EventEmitter trait, tracker |
| `crates/common_utils/src/lib.rs` | **MODIFY** - Add `pub mod external_events;` |
| `crates/common_utils/src/keymanager.rs` | **MODIFY** - Add request_id and emitter params |
| `crates/router/src/events.rs` | **MODIFY** - Add EventType variant, impl EventEmitter |
| `crates/router/src/events/external_service_call.rs` | **CREATE** - KafkaMessage impl |
| `crates/router/src/configs/settings.rs` | **MODIFY** - Add topic config |
| `crates/router/src/services/kafka.rs` | **MODIFY** - Add topic field and mapping |

---

## Open Questions

1. **request_id propagation fix:** Should we also fix the `KeyManagerState.request_id` propagation bug, or rely on explicit parameter passing?

2. **ExternalServiceCall struct location:** Should it be in `common_utils` or a separate shared crate for maximum reusability?

3. **EventEmitter vs EventHandlerInterface:** Should we extend `EventHandlerInterface` in `hyperswitch_interfaces` instead of creating `EventEmitter` in `common_utils`?
   - Pro: Reuses existing trait
   - Con: `common_utils` would need to depend on `hyperswitch_interfaces` (reverse of current direction)

4. **Batch operations:** Should we support batch emission for high-volume scenarios?

---

## Success Criteria

1. Every external service call (KeyManager, Redis, Postgres, Connectors) emits an `ExternalServiceCall` event
2. Each event has a `request_id` for correlation with `ApiEvent`
3. Events are sent to `external_service_calls_topic` in Kafka
4. Fire-and-forget pattern - event emission failures don't affect the main operation
5. Minimal changes to existing call sites
6. Reusable by other microservices outside the router crate

---

## Future Extensions

1. **Redis instrumentation:** Wrap `RedisConnectionPool` methods
2. **PostgreSQL instrumentation:** Wrap generic DB operations in `diesel_models/src/query/generics.rs`
3. **Connector HTTP calls:** Already have `ConnectorEvent` - may merge or coexist
4. **Correlation service:** New service that joins `ApiEvent` + `ExternalServiceCall` events
