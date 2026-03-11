# Kafka Exploration Context Log

## Entry 1: Understanding Kafka in Router Crate

### User Query
User wants to understand everything about Kafka usage in the router crate - how it's used, where it's used, how it's initialized.

### Approach
Sequential exploration starting from entry point `crates/router/src/bin/router.rs`, following the initialization flow.

### Findings

#### 1. Initialization Flow (Server Startup)

```
router.rs::main()
    ↓
Settings::with_config_path()  → Loads config file
    ↓
router::start_server(conf)
    ↓
AppState::new(conf) → AppState::with_storage()
    ↓
conf.events.get_event_handler()  → Creates EventsHandler
    ↓
If Kafka: KafkaProducer::create() → Connects to brokers (FAILS if unreachable)
If Logs: EventLogger::new() → Always succeeds (default)
    ↓
KafkaStore::new(diesel_store, kafka_producer, tenant_id)
```

**Key files for initialization:**
- `crates/router/src/bin/router.rs` - Entry point
- `crates/router/src/routes/app.rs` - AppState::with_storage(), get_store_interface()
- `crates/router/src/events.rs` - EventsConfig, EventsHandler, EventType enum

**Config structure:**
```rust
pub enum EventsConfig {
    Kafka { kafka: Box<KafkaSettings> },  // Has 14 topic names + brokers
    #[default] Logs,                       // Default fallback
}

pub enum EventsHandler {
    Kafka(KafkaProducer),
    Logs(EventLogger),
}
```

**What happens if Kafka not available:**
- Config = `Logs` (default): Server starts fine, events logged to stdout
- Config = `Kafka` but brokers unreachable: Server FAILS to start

#### 2. API Call → Kafka Flow

```
POST /payments
    ↓
Core layer: payments_create()
    ↓
store.insert_payment_intent(new_intent)  // store = KafkaStore
    ↓
KafkaStore::insert_payment_intent()
    ↓
1. diesel_store.insert_payment_intent() → DB write FIRST
2. kafka_producer.log_payment_intent() → Kafka publish SECOND (fire-and-forget)
    ↓
KafkaProducer sends TWO messages:
  - To intent_analytics_topic (specific)
  - To consolidated_events_topic (aggregated)
```

**Key files for data flow:**
- `crates/router/src/db/kafka_store.rs` - KafkaStore wrapper, intercepts DB writes
- `crates/router/src/services/kafka.rs` - KafkaProducer implementation
- `crates/router/src/services/kafka/payment_intent.rs` - KafkaPaymentIntent message
- `crates/router/src/services/kafka/payment_attempt.rs` - KafkaPaymentAttempt message

#### 3. Key Design Decisions

1. **Fire-and-forget**: Kafka failures logged but don't fail DB operations
2. **DB first, Kafka second**: Data integrity from DB, Kafka is for analytics
3. **Dual publish**: Each entity sends to specific topic + consolidated topic
4. **sign_flag**: +1 for new record, -1 for delete (ClickHouse upsert optimization)
5. **Encrypted fields**: PII encrypted before Kafka (billing_details, shipping_details)
6. **Hashed email**: customer_email hashed for privacy

#### 4. Topics (14 total)

- intent_analytics_topic, attempt_analytics_topic, refund_analytics_topic
- dispute_analytics_topic, payout_analytics_topic, authentication_analytics_topic
- fraud_check_analytics_topic, api_logs_topic, connector_logs_topic
- outgoing_webhook_logs_topic, audit_events_topic, routing_logs_topic
- revenue_recovery_topic, consolidated_events_topic

#### 5. Entities Publishing to Kafka

| Entity | Kafka Method | Operations |
|--------|--------------|------------|
| PaymentIntent | log_payment_intent | insert, update |
| PaymentAttempt | log_payment_attempt | insert, update |
| Refund | log_refund | insert, update |
| Dispute | log_dispute | insert, update |
| Payout | log_payout | insert, update |
| Authentication | log_authentication | insert, update |
| FraudCheck | log_fraud_check | insert, update |

#### 6. Message Format

```rust
// KafkaEvent wrapper adds metadata
{
    sign_flag: i32,        // +1 = new, -1 = delete
    tenant_id: TenantID,
    clickhouse_database: Option<String>,
    inner: <Entity data serialized to JSON>
}
```

#### 7. Known Message Structures

**PaymentIntent** (60+ fields): payment_id, merchant_id, status, amount, currency, customer_id, customer_email (hashed), metadata, billing_details (encrypted), shipping_details (encrypted), created_at, modified_at, infra_values, etc.

**PaymentAttempt** (70+ fields): payment_id, attempt_id, merchant_id, status, amount, connector, error_message, payment_method, card_network, created_at, etc.

### Missing Areas to Explore

1. **Other entity messages**: Refund, Dispute, Payout, Authentication, FraudCheck - their KafkaMessage impls and field structures
2. **Logs events**: ApiLogs, ConnectorApiLogs, OutgoingWebhookLogs, RoutingApiLogs - who triggers these, different flow?
3. **infra_values**: What is this? Where does it come from in log_payment_intent?
4. **Encryption/hashing**: How customer_email is hashed, how billing_details is encrypted
5. **Producer config**: Beyond bootstrap.servers - retries, acks, batch settings?
6. **Batch operations**: BatchSampleDataInterface - how bulk Kafka publish works
7. **RevenueRecovery**: Mentioned in topics but usage not found - what triggers it?

---

## Entry 2: Missing Areas Explored

### 1. Other Entity Messages

#### Refund - KafkaRefund
**File:** `crates/router/src/services/kafka/refund.rs`
**Fields (v1: 26, v2: 38):** internal_reference_id, refund_id, payment_id, merchant_id, connector_transaction_id, connector, connector_refund_id, external_reference_id, refund_type, total_amount, currency, refund_amount, refund_status, sent_to_gateway, refund_error_message, refund_arn, created_at, modified_at, description, attempt_id, refund_reason, refund_error_code, profile_id, organization_id, metadata, updated_by, merchant_connector_id, charges, connector_refund_data, connector_transaction_data, split_refunds, unified_code, unified_message, processor_refund_data, processor_transaction_data
**Key:** `{merchant_id}_{payment_id}_{attempt_id}_{refund_id}` (v1) / `{merchant_id}_{payment_id}_{attempt_id}_{merchant_reference_id}` (v2)
**EventType:** `Refund`

#### Dispute - KafkaDispute
**File:** `crates/router/src/kafka/dispute.rs`
**Fields (22):** dispute_id, dispute_amount, currency, dispute_stage, dispute_status, payment_id, attempt_id, merchant_id, connector_status, connector_dispute_id, connector_reason, connector_reason_code, challenge_required_by, connector_created_at, connector_updated_at, created_at, modified_at, connector, evidence, profile_id, merchant_connector_id, organization_id
**Key:** `{merchant_id}_{payment_id}_{dispute_id}`
**EventType:** `Dispute`

#### Payout - KafkaPayout
**File:** `crates/router/src/services/kafka/payout.rs`
**Fields (32):** payout_id, payout_attempt_id, merchant_id, customer_id, address_id, profile_id, payout_method_id, payout_type, amount, destination_currency, source_currency, description, recurring, auto_fulfill, return_url, entity_type, metadata, created_at, last_modified_at, attempt_count, status, priority, connector, connector_payout_id, is_eligible, error_message, error_code, business_country, business_label, merchant_connector_id, organization_id
**Key:** `{merchant_id}_{payout_attempt_id}`
**EventType:** `Payout`
**Note:** Combines data from both `Payouts` and `PayoutAttempt` tables

#### Authentication - KafkaAuthentication
**File:** `crates/router/src/services/kafka/authentication.rs`
**Fields (60):** authentication_id, merchant_id, authentication_connector, connector_authentication_id, authentication_data, payment_method_id, authentication_type, authentication_status, authentication_lifecycle_status, created_at, modified_at, error_message, error_code, connector_metadata, maximum_supported_version, threeds_server_transaction_id, cavv, authentication_flow_type, message_version, eci, trans_status, acquirer_bin, acquirer_merchant_id, three_ds_method_data, three_ds_method_url, acs_url, challenge_request, acs_reference_number, acs_trans_id, acs_signed_content, profile_id, payment_id, merchant_connector_id, ds_trans_id, directory_server_id, acquirer_country_code, organization_id, platform, mcc, currency, amount, merchant_country, billing_country, shipping_country, issuer_country, earliest_supported_version, latest_supported_version, device_type, device_brand, device_os, device_display, browser_name, browser_version, issuer_id, scheme_name, exemption_requested, exemption_accepted
**Key:** `{merchant_id}_{authentication_id}`
**EventType:** `Authentication`
**Note:** Timestamps serialized with milliseconds precision

#### FraudCheck - KafkaFraudCheck
**File:** `crates/router/src/services/kafka/fraud_check.rs`
**Fields (16):** frm_id, payment_id, merchant_id, attempt_id, created_at, frm_name, frm_transaction_id, frm_transaction_type, frm_status, frm_score, frm_reason, frm_error, payment_details, metadata, modified_at, payment_capture_method
**Key:** `{merchant_id}_{payment_id}_{attempt_id}_{frm_id}`
**EventType:** `FraudCheck`

---

### 2. Logs Events Flow

**Key Difference from Entity Events:**
| Aspect | Entity Events | Log Events |
|--------|--------------|------------|
| Trigger Source | KafkaStore wrapper (DB ops) | Direct business logic code |
| Wrapper | KafkaEvent (adds sign_flag, tenant_id) | No wrapper - sent directly |
| Dual Publish | Yes (specific + consolidated topic) | No - only specific topic |
| sign_flag | +1 for new, -1 for delete | N/A |

#### ApiLogs
**Trigger:** `services/api.rs:350-368` (server_wrap), `core/webhooks/incoming.rs` (webhooks)
**Key:** `request_id`
**Fields:** tenant_id, merchant_id, api_flow, created_at_timestamp, request_id, latency, status_code, auth_type, request, user_agent, ip_addr, url_path, response, error, event_type, hs_latency, http_method, infra_components

#### ConnectorApiLogs
**Trigger:** `hyperswitch_interfaces/src/api_client.rs:286-419`, `core/unified_connector_service.rs:2274-2304`
**Key:** `request_id`
**Fields:** tenant_id, connector_name, flow, request, masked_response, error, url, method, merchant_id, created_at, request_id, latency, status_code, api_flow

#### OutgoingWebhookLogs
**Trigger:** `core/webhooks/outgoing.rs:549-560`, `core/webhooks/outgoing_v2.rs:301`
**Key:** `event_id`
**Fields:** tenant_id, merchant_id, event_id, event_type, outgoing_webhook_event_type, payment_id, content, is_error, error, created_at_timestamp, initial_attempt_id, status_code, delivery_attempt

#### RoutingApiLogs
**Trigger:** `core/payments/routing/utils.rs:1655-1705`
**Key:** `{merchant_id}-{profile_id}-{payment_id}`
**Fields:** tenant_id, routable_connectors, payment_connector, flow, request, response, error, url, method, payment_id, profile_id, merchant_id, created_at, status_code, request_id, routing_engine, routing_approach

---

### 3. infra_values

**Definition:** Optional HashMap populated from environment variables for analytics metadata.

**Config:** `configs/settings.rs:180`
```rust
pub infra_values: Option<HashMap<String, String>>,
```

**Processing:** `routes/app.rs:678-696` - Maps config keys to env var values at startup.
- Config provides mapping like `{"env_key": "ENV_VAR_NAME"}`
- At startup, resolves env var names to actual values

**Usage:**
- `kafka_store.rs:1869` - Adds `is_confirm_operation: true` for confirm operations
- Flattened into KafkaPaymentIntent message as top-level fields

---

### 4. Encryption/Hashing

**Email Hashing:** `common_utils/src/hashing.rs`
- Uses **blake3** hashing algorithm
- Email hashed to hex string on serialization

**Billing/Shipping Encryption:** `core/payment_methods/cards.rs:5369-5399`
- Uses Key Manager service for encryption
- Data serialized to JSON, wrapped in Secret, encrypted via `crypto_operation`

**Current State in KafkaPaymentIntent:**
- billing_details and shipping_details set to `None` with TODO comment (PII concern)

---

### 5. Producer Config

**File:** `services/kafka.rs:308-334`

**Only config set:** `bootstrap.servers`

**Uses rdkafka defaults:**
- `acks`: all (strongest guarantee)
- `retries`: 2^31-1 (essentially infinite)
- `batch.size`: 16384 bytes
- `linger.ms`: 0 (immediate send)
- `message.timeout.ms`: 300000 (5 minutes)

---

### 6. Batch Operations

**File:** `db/kafka_store.rs:3612-3702`

**Pattern:**
1. Batch insert to DB via diesel_store
2. Iterate and publish each record to Kafka individually (fire-and-forget)
3. No batch Kafka publish API - individual messages per record

**Methods:** insert_payment_intents_batch_for_sample_data, insert_payment_attempts_batch_for_sample_data, insert_refunds_batch_for_sample_data, insert_disputes_batch_for_sample_data

---

### 7. RevenueRecovery

**File:** `services/kafka/revenue_recovery.rs`

**Fields:** merchant_id, invoice_amount, invoice_currency, invoice_due_date, invoice_date, billing_country, billing_state, billing_city, attempt_amount, attempt_currency, attempt_status, pg_error_code, network_advice_code, network_error_code, first_pg_error_code, first_network_advice_code, first_network_error_code, attempt_created_at, payment_method_type, payment_method_subtype, card_network, card_issuer, retry_count, payment_gateway

**Trigger Points:**
1. `core/webhooks/recovery_incoming.rs:156-171` - Incoming webhooks from billing connector (Stripe Billing) for failed invoice payments
2. `core/revenue_recovery/types.rs:162-173, 236-248` - Revenue recovery retry workflow

**Flow:**
1. Billing connector sends webhook about failed invoice payment
2. Hyperswitch creates recovery payment intent/attempt
3. Kafka event published for each recovery attempt
4. Process tracker handles retries (Monitoring/Cascading/Smart algorithms)
5. Each retry outcome publishes another Kafka event

---

### 8. AuditEvents

**File:** `events/audit_events.rs`

**Struct:** `AuditEvent` (NOT wrapped in KafkaEvent)
```rust
pub struct AuditEvent {
    #[serde(flatten)]
    event_type: AuditEventType,
    created_at: PrimitiveDateTime,
}
```

**AuditEventType (16 variants, 9 used):**
| Variant | Fields | Status |
|---------|--------|--------|
| PaymentCreate | - | Used |
| PaymentConfirm | client_src, client_ver, frm_message | Used |
| PaymentCancelled | cancellation_reason | Used |
| PaymentCapture | capture_amount, multiple_capture_count | Used |
| PaymentUpdate | amount | Used |
| PaymentApprove | - | Used |
| PaymentStatus | - | Used |
| PaymentCompleteAuthorize | - | Used |
| PaymentReject | error_code, error_message | Used |
| Error, PaymentCreated, ConnectorDecided, ConnectorCalled, RefundCreated, RefundSuccess, RefundFail | - | **UNUSED** |

**Trigger Points:** Payment operations in `core/payments/operations/`
- payment_create.rs, payment_confirm.rs, payment_cancel.rs, payment_capture.rs, payment_update.rs, payment_approve.rs, payment_status.rs, payment_complete_authorize.rs, payment_reject.rs

**Key Format:** `{event_type}-{timestamp_nanos}` (e.g., `payment_confirm-1705315800000000000`)

**Message Structure:**
```json
{
    "event_type": "payment_confirm",
    "created_at": "2024-01-15T10:30:00.000Z",
    "payment": { "payment_intent": {...}, "payment_attempt": {...} },
    "client_src": "...",
    "client_ver": "...",
    "frm_message": {...}
}
```

**Key Differences:**
| Aspect | Entity Events | Log Events | Audit Events |
|--------|---------------|------------|--------------|
| Trigger | KafkaStore (DB ops) | Business logic | Business logic |
| Wrapper | KafkaEvent | None | None |
| Dual Publish | Yes | No | No |
| Key Format | {merchant_id}_{...} | request_id | {event_type}-{nanos} |
| Trait | KafkaMessage | KafkaMessage | Event + EventInfo |
| Send Path | log_event() | log_event() | send_message() |
| Purpose | Data replication | API logging | Audit trail |

---

## Entry 3: API Flow Traces with Kafka Events

### 1. POST /payments (Create Payment)

**Route:** `routes/payments.rs::payments_create` → `services/api.rs::server_wrap`

**Flow:**
```
POST /payments
    ↓
server_wrap() [auth, timing]
    ↓
core/payments.rs::payments_core()
    ↓
operations/payment_create.rs::get_trackers()
    ├── store.insert_payment_intent() → KafkaStore
    │       ├── DB insert
    │       └── kafka.log_payment_intent() → intent_analytics_topic + consolidated_events_topic
    └── store.insert_payment_attempt() → KafkaStore
            ├── DB insert
            └── kafka.log_payment_attempt() → attempt_analytics_topic + consolidated_events_topic
    ↓
operations/payment_create.rs::update_trackers()
    ├── store.update_payment_attempt() → KafkaStore
    │       └── kafka.log_payment_attempt(old=-1, new=+1)
    ├── store.update_payment_intent() → KafkaStore
    │       └── kafka.log_payment_intent(old=-1, new=+1)
    └── event_context.emit(AuditEvent::PaymentCreate) → audit_events_topic
    ↓
server_wrap() completes
    └── event_handler.log_event(ApiEvent) → api_logs_topic
```

**Kafka Events (12 messages):**
| Order | Event | Topic |
|-------|-------|-------|
| 1 | PaymentIntent insert (+1) | intent_analytics_topic |
| 2 | PaymentIntent insert | consolidated_events_topic |
| 3 | PaymentAttempt insert (+1) | attempt_analytics_topic |
| 4 | PaymentAttempt insert | consolidated_events_topic |
| 5 | PaymentAttempt update (-1) | attempt_analytics_topic |
| 6 | PaymentAttempt update (+1) | attempt_analytics_topic |
| 7 | PaymentAttempt update | consolidated_events_topic |
| 8 | PaymentIntent update (-1) | intent_analytics_topic |
| 9 | PaymentIntent update (+1) | intent_analytics_topic |
| 10 | PaymentIntent update | consolidated_events_topic |
| 11 | AuditEvent PaymentCreate | audit_events_topic |
| 12 | ApiLogs | api_logs_topic |

---

### 2. POST /refunds (Create Refund)

**Route:** `routes/refunds.rs::refunds_create` → `services/api.rs::server_wrap`

**Flow:**
```
POST /refunds
    ↓
server_wrap() [auth, timing]
    ↓
core/refunds.rs::refund_create_core()
    ├── Validate payment status
    └── validate_and_create_refund()
            └── store.insert_refund() → KafkaStore
                    ├── DB insert
                    └── kafka.log_refund() → refund_analytics_topic + consolidated_events_topic
    ↓
schedule_refund_execution() [if instant]
    └── trigger_refund_to_gateway()
            ├── execute_connector_processing_step()
            │       └── event_handler.log_connector_event() → connector_logs_topic
            ├── store.update_refund() → KafkaStore
            │       └── kafka.log_refund(old=-1, new=+1)
            └── trigger_refund_outgoing_webhook() [async]
                    └── log_event(OutgoingWebhookEvent) → outgoing_webhook_logs_topic
    ↓
server_wrap() completes
    └── event_handler.log_event(ApiEvent) → api_logs_topic
```

**Kafka Events (5-7 messages):**
| Order | Event | Topic |
|-------|-------|-------|
| 1 | Refund insert (+1) | refund_analytics_topic |
| 2 | Refund insert | consolidated_events_topic |
| 3 | ConnectorApiLogs | connector_logs_topic |
| 4 | Refund update (-1) | refund_analytics_topic |
| 5 | Refund update (+1) | refund_analytics_topic |
| 6 | Refund update | consolidated_events_topic |
| 7 | OutgoingWebhookLogs | outgoing_webhook_logs_topic |
| 8 | ApiLogs | api_logs_topic |

**Note:** AuditEvent variants for Refund (RefundCreated, RefundSuccess, RefundFail) are defined but NOT used.

---

### 3. POST /payouts (Create Payout)

**Route:** `routes/payouts.rs::payouts_create` → `services/api.rs::server_wrap`

**Flow:**
```
POST /payouts
    ↓
server_wrap() [auth, timing]
    ↓
core/payouts.rs::payouts_create_core()
    ├── payout_create_db_entries()
    │       ├── store.insert_payout() → NO Kafka event
    │       └── store.insert_payout_attempt() → KafkaStore
    │               ├── DB insert
    │               └── kafka.log_payout() → payout_analytics_topic
    ├── [if confirm=true] payouts_core()
    │       └── call_connector_payout()
    │               ├── execute_connector_processing_step()
    │               │       └── log_connector_event() → connector_logs_topic
    │               ├── store.update_payout_attempt() → KafkaStore
    │               │       └── kafka.log_payout(old=-1, new=+1)
    │               └── store.update_payout() → KafkaStore
    │                       └── kafka.log_payout(old=-1, new=+1)
    └── trigger_webhook_and_handle_response() [async]
            └── log_event(OutgoingWebhookEvent) → outgoing_webhook_logs_topic
    ↓
server_wrap() completes
    └── event_handler.log_event(ApiEvent) → api_logs_topic
```

**Kafka Events (4+ messages):**
| Order | Event | Topic |
|-------|-------|-------|
| 1 | Payout insert (+1) | payout_analytics_topic |
| 2+ | ConnectorApiLogs (per call) | connector_logs_topic |
| 3+ | Payout updates (-1/+1) | payout_analytics_topic |
| N-1 | OutgoingWebhookLogs | outgoing_webhook_logs_topic |
| N | ApiLogs | api_logs_topic |

**Key Differences:**
- No consolidated_events_topic for Payout
- insert_payout() does NOT publish to Kafka (only insert_payout_attempt does)
- No AuditEvents for Payout

---

### 4. POST /webhooks/{merchant_id}/{connector} (Incoming Webhook)

**Route:** `routes/webhooks.rs::receive_incoming_webhook`

**Flow:**
```
POST /webhooks/{merchant_id}/{connector}
    ↓
server_wrap() [MerchantIdAuth]
    ↓
core/webhooks/incoming.rs::incoming_webhooks_wrapper()
    └── log_event(ApiEvent with Webhooks type) → api_logs_topic [SECOND ApiLog!]
    ↓
incoming_webhooks_core()
    ├── Verify webhook signature
    ├── Determine event type
    └── process_webhook_business_logic()
            │
            ├── [Payment] payments_incoming_webhook_flow()
            │       ├── payments_core(PaymentStatus)
            │       │       ├── update_payment_attempt() → attempt_analytics_topic
            │       │       └── update_payment_intent() → intent_analytics_topic
            │       └── trigger_outgoing_webhook() → outgoing_webhook_logs_topic
            │
            ├── [Refund] refunds_incoming_webhook_flow()
            │       ├── update_refund() → refund_analytics_topic
            │       └── trigger_outgoing_webhook() → outgoing_webhook_logs_topic
            │
            ├── [Dispute] disputes_incoming_webhook_flow()
            │       ├── insert/update_dispute() → dispute_analytics_topic
            │       └── trigger_outgoing_webhook() → outgoing_webhook_logs_topic
            │
            └── [Payout] payouts_incoming_webhook_flow()
                    ├── update_payout() → payout_analytics_topic
                    └── trigger_outgoing_webhook() → outgoing_webhook_logs_topic
    ↓
Response to connector (HTTP 200)
    ↓
server_wrap() completes
    └── event_handler.log_event(ApiEvent) → api_logs_topic
```

**Kafka Events (Payment Webhook - 9 messages):**
| Order | Event | Topic |
|-------|-------|-------|
| 1 | ApiLogs (server_wrap) | api_logs_topic |
| 2 | ApiLogs (webhook wrapper) | api_logs_topic |
| 3 | PaymentAttempt update (-1) | attempt_analytics_topic |
| 4 | PaymentAttempt update (+1) | attempt_analytics_topic |
| 5 | PaymentAttempt update | consolidated_events_topic |
| 6 | PaymentIntent update (-1) | intent_analytics_topic |
| 7 | PaymentIntent update (+1) | intent_analytics_topic |
| 8 | PaymentIntent update | consolidated_events_topic |
| 9 | OutgoingWebhookLogs | outgoing_webhook_logs_topic |

**Key Observations:**
- TWO ApiLogs per incoming webhook (server_wrap + webhook wrapper)
- Entity updates use sign_flag (-1 for old, +1 for new)
- Outgoing webhook is async (doesn't block response to connector)

---

## Entry 4: Kafka Architecture Deep Dive

### 1. Two Layers of Kafka Logging

The Kafka system has two distinct layers:

**Layer 1: `log_event<T: KafkaMessage>(&self, event: &T)` - The Primitive**
```rust
pub fn log_event<T: KafkaMessage>(&self, event: &T) -> MQResult<()> {
    let topic = self.get_topic(event.event_type());
    self.producer.send(
        BaseRecord::to(topic)
            .key(&event.key())
            .payload(&event.value()?)
    )
}
```
- Takes ANY type implementing `KafkaMessage`
- Sends exactly **1 message** to Kafka
- No business logic, just serialization and send

**Layer 2: `log_payment_intent()`, `log_refund()`, etc. - Convenience Methods**
```rust
pub async fn log_payment_intent(&self, intent, old_intent, tenant_id, infra_values) {
    // 1. If update, send OLD with sign_flag=-1
    if let Some(old) = old_intent {
        self.log_event(&KafkaEvent::old(...));
    }
    // 2. Send NEW with sign_flag=+1
    self.log_event(&KafkaEvent::new(...));
    // 3. Send to consolidated topic
    self.log_event(&KafkaConsolidatedEvent::new(...));
}
```
- Call `log_event()` 2-3 times
- Handle sign_flag pattern (old=-1, new=+1)
- Handle dual publish (specific + consolidated topics)
- **Code duplication:** Same pattern repeated for 7 entities

---

### 2. Why Two Message Types Per Entity

Each entity has TWO Kafka message types:

| Type | Topic | Timestamp Precision | Example |
|------|-------|---------------------|---------|
| `KafkaPaymentIntent` | intent_analytics_topic | Milliseconds | `time::serde::timestamp` |
| `KafkaPaymentIntentEvent` | consolidated_events_topic | Nanoseconds | `time::serde::timestamp::nanoseconds` |

**Why different precision?**
- Specific topics: Milliseconds sufficient for ClickHouse partitioning
- Consolidated topic: Nanoseconds for finer-grained ordering across entity types

**How they route to different topics:**
```rust
impl KafkaMessage for KafkaPaymentIntent {
    fn event_type(&self) -> EventType { EventType::PaymentIntent }
    // → get_topic(PaymentIntent) → intent_analytics_topic
}

impl KafkaMessage for KafkaPaymentIntentEvent {
    fn event_type(&self) -> EventType { EventType::Consolidated }
    // → get_topic(Consolidated) → consolidated_events_topic
}
```

---

### 3. ClickHouse Integration

**Table Structure Pattern (for each entity):**

```sql
-- 1. Queue table: Consumes from Kafka topic
CREATE TABLE payment_intents_queue (...) 
ENGINE = Kafka SETTINGS 
    kafka_topic_list = 'hyperswitch-payment-intent-events',
    ...;

-- 2. Main table: Uses CollapsingMergeTree with sign_flag
CREATE TABLE payment_intents (
    payment_id String,
    status String,
    sign_flag Int8,  -- REQUIRED for collapse
    ...
) ENGINE = CollapsingMergeTree(sign_flag)
ORDER BY (merchant_id, payment_id, created_at);

-- 3. Materialized view: Pipes queue → main table
CREATE MATERIALIZED VIEW payment_intents_mv TO payment_intents AS
SELECT * FROM payment_intents_queue;
```

**Tables using this pattern:**
- payment_intents, payment_attempts, refunds, disputes
- payouts, authentications, fraud_check
- api_events, connector_events, routing_events, outgoing_webhook_events

**Critical:** `sign_flag` is NOT optional. `CollapsingMergeTree(sign_flag)` requires:
- +1 for new/updated records
- -1 for "deleting" old state during updates
- ClickHouse collapses matching +/- pairs, keeping only +1

**Files:** `crates/analytics/docs/clickhouse/scripts/*.sql`

---

### 4. consolidated_events_topic Status

**Finding:** The topic is published to but has NO ClickHouse consumer.

**Evidence:**
- Topic defined in config: `consolidated_events_topic = "hyperswitch-consolidated-events"`
- Events ARE published (EventType::Consolidated maps to it)
- NO `consolidated_events.sql` in `crates/analytics/docs/clickhouse/scripts/`

**Possible states:**
1. Consumed by external system (not in this repo)
2. Dead code / unused
3. Planned for future use

**Entities publishing to consolidated:**
- PaymentIntent ✓
- PaymentAttempt ✓
- Refund ✓
- Dispute ✓
- Authentication ✓
- FraudCheck ✓
- Payout ✗ (does NOT publish to consolidated)

---

### 5. Trait Hierarchy

**Three crates define event-related traits:**

| Crate | Traits | Purpose |
|-------|--------|---------|
| `events` | `MessagingInterface`, `Message`, `Event` | Generic event framework |
| `router` | `KafkaMessage` | Kafka-specific message interface |
| `hyperswitch_interfaces` | `EventHandlerInterface` | Shared interface for services |

**Trait details:**

```rust
// events crate - Generic (crates/events/src/lib.rs)
pub trait MessagingInterface {
    type MessageClass;
    fn send_message<T: Message>(&self, data, metadata, timestamp);
}

pub trait Message {
    type Class;
    fn identifier(&self) -> String;
    fn get_message_class(&self) -> Self::Class;
}

// router crate - Kafka-specific (crates/router/src/services/kafka.rs)
pub trait KafkaMessage: Serialize {
    fn key(&self) -> String;
    fn event_type(&self) -> EventType;
    fn value(&self) -> Result<Vec<u8>, KafkaError>;
    fn creation_timestamp(&self) -> Option<i64>;
}

// hyperswitch_interfaces crate - For cross-service use
pub trait EventHandlerInterface: DynClone + Send + Sync {
    fn log_connector_event(&self, event: &ConnectorEvent);
}
```

**Problem:** `KafkaMessage` is locked in router crate. Other microservices cannot implement Kafka event publishing using the same trait system.

---

### 6. KafkaEvent Wrapper Behavior

The wrapper delegates to inner but serializes itself:

```rust
struct KafkaEvent<'a, T: KafkaMessage> {
    sign_flag: i32,
    tenant_id: TenantID,
    clickhouse_database: Option<String>,
    inner: &'a T,
}

impl<T: KafkaMessage> KafkaMessage for KafkaEvent<'_, T> {
    fn key(&self) -> String { self.inner.key() }           // Delegate
    fn event_type(&self) -> EventType { self.inner.event_type() }  // Delegate
    fn value(&self) -> Result<Vec<u8>, KafkaError> {
        serde_json::to_vec(&self)  // Serialize WHOLE wrapper!
    }
}
```

**Result:** JSON output includes `sign_flag`, `tenant_id`, `clickhouse_database` flattened with inner fields:
```json
{
    "sign_flag": 1,
    "tenant_id": "tenant_123",
    "clickhouse_database": "analytics",
    "payment_id": "p_123",       // from inner
    "status": "captured",        // from inner
    ...
}
```

---

### 7. Refactoring Considerations

Based on analysis, key constraints for any refactoring:

| Constraint | Impact |
|------------|--------|
| `sign_flag` required | Must keep KafkaEvent wrapper, old/new pattern |
| `CollapsingMergeTree` | Cannot change message structure without ClickHouse migration |
| Two message types | May unify if timestamp precision is not critical |
| consolidated_events_topic | Verify usage before removing |
| `KafkaMessage` in router | Extract to shared crate for cross-service use |

**Potential improvements:**
1. Extract `KafkaMessage` trait to `events` crate
2. Replace `EventType` enum with topic strings or move enum to shared crate
3. Dedupe `log_*` methods using generics
4. Unify message types if nanosecond precision not needed

---

### 8. Summary: Who Does What

| Component | Role | Location |
|-----------|------|----------|
| **EventsConfig** | Config enum - chooses Kafka vs Logs backend | `router/src/events.rs` |
| **EventsHandler** | Dispatcher - routes to KafkaProducer or EventLogger | `router/src/events.rs` |
| **KafkaProducer** | Actual Kafka client - sends messages to topics | `router/src/services/kafka.rs` |
| **KafkaStore** | DB wrapper - intercepts writes, triggers Kafka events | `router/src/db/kafka_store.rs` |
| **KafkaMessage** | Trait - defines key, event_type, value for serialization | `router/src/services/kafka.rs` |
| **KafkaEvent** | Wrapper - adds sign_flag, tenant_id for entity events | `router/src/services/kafka.rs` |
| **EventType** | Enum - maps to topic names | `router/src/events.rs` |

**Two Event Flows:**

| Flow | Trigger | Path | Wrapper | sign_flag | Consolidated |
|------|---------|------|---------|-----------|--------------|
| Entity Events | KafkaStore DB ops | `log_payment_intent()` | KafkaEvent | Yes (+1/-1) | Yes |
| Log Events | Core logic direct | `log_event()` | None | No | No |

---

## Entry 5: Events Crate Usage Flow

### 1. What the events Crate Is Used For

**Only used for AuditEvent.** All other events (PaymentIntent, Refund, ApiLogs, etc.) bypass this crate entirely.

| System | Trait Path | Used For |
|--------|------------|----------|
| `events` crate | `Event` + `MessagingInterface` | AuditEvent only (9 emission points) |
| `router` crate | `KafkaMessage` directly | All entity events, log events |

---

### 2. Step-by-Step Flow (POST /payments Example)

**Step 1: Metadata Accumulation (server_wrap)**
```rust
// crates/router/src/services/api.rs:241-257
pub async fn server_wrap(..., request_state: &mut ReqState, ...) {
    let mut event_context = request_state.event_context.clone();
    
    // Accumulate metadata throughout request
    event_context.record_info(request_id);         // { "request_id": "req_123" }
    event_context.record_info(("flow", flow));     // { ..., "flow": "payments_create" }
    event_context.record_info(tenant_id);          // { ..., "tenant_id": "tenant_abc" }
    event_context.record_info(auth_type);          // { ..., "auth_type": "MerchantIdAuth" }
}
```

**Step 2: Emit AuditEvent (core logic)**
```rust
// crates/router/src/core/payments/operations/payment_create.rs:1068-1072
pub async fn update_trackers(...) {
    req_state
        .event_context
        .event(AuditEvent::new(AuditEventType::PaymentCreate))
        .with(payment_data.to_event())
        .emit();
}
```

**Step 3: EventBuilder Assembles Final Event**
```rust
// crates/events/src/lib.rs
pub struct EventBuilder<T, A, E, D> {
    message_sink: Arc<A>,              // EventsHandler
    metadata: HashMap<String, String>, // Accumulated metadata
    event: E,                          // AuditEvent
}

impl EventBuilder {
    pub fn emit(self) {
        let flat_event = FlatMapEvent {
            metadata: self.metadata,
            inner: self.event,
        };
        self.message_sink.send_message(flat_event, ..., timestamp);
    }
}
```

**Step 4: FlatMapEvent Serialization**
```rust
// crates/events/src/lib.rs
struct FlatMapEvent<E> {
    metadata: HashMap<String, String>,
    inner: E,
}

// Serialize merges everything into ONE JSON object
// Result: { ...metadata fields..., ...inner event fields... } all flattened
```

**Step 5: EventsHandler Dispatches**
```rust
// crates/router/src/events.rs
impl MessagingInterface for EventsHandler {
    fn send_message<T: Message>(&self, data, metadata, timestamp) {
        match self {
            EventsHandler::Kafka(producer) => producer.send_message(...),
            EventsHandler::Logs(logger) => logger.send_message(...),
        }
    }
}
```

**Step 6: KafkaProducer Sends to Kafka**
```rust
// crates/router/src/services/kafka.rs:688-744
impl MessagingInterface for KafkaProducer {
    fn send_message<T: Message>(&self, data, metadata, timestamp) {
        let topic = self.get_topic(data.get_message_class()); // → "audit_events_topic"
        let payload = serde_json::to_vec(&data);
        let record = BaseRecord::to(topic)
            .key(&data.identifier())
            .payload(&payload)
            .timestamp(timestamp_ms);
        self.producer.send(record);
    }
}
```

**Final Kafka Message:**
```json
{
    "event_type": "payment_create",
    "created_at": "2024-01-15T10:30:00.000Z",
    "request_id": "req_123",
    "flow": "payments_create",
    "tenant_id": "tenant_abc",
    "auth_type": "MerchantIdAuth",
    "payment": {
        "payment_intent": { "payment_id": "p_123", "status": "requires_capture" },
        "payment_attempt": { "attempt_id": "a_123", "connector": "stripe" }
    }
}
```
Sent to: `audit_events_topic` with key: `payment_create-1705315800000000000`

---

### 3. Key Components in events Crate

| Component | Purpose |
|-----------|---------|
| `EventContext<T, A>` | Holds `message_sink` + accumulated `metadata` HashMap |
| `record_info()` | Adds key-value to metadata HashMap |
| `EventBuilder` | Builder pattern with `.with()` and `.emit()` |
| `FlatMapEvent` | Internal wrapper that flattens metadata + event into single JSON |
| `MessagingInterface` | Transport trait with `send_message()` |
| `Event` | Defines `timestamp()`, `identifier()`, `class()` |
| `EventInfo` | Defines `data()` and `key()` for metadata pieces |

---

### 4. Comparison: events Crate Path vs KafkaMessage Path

| Aspect | `events` crate path | `KafkaMessage` path |
|--------|---------------------|---------------------|
| Entry point | `event_context.emit()` | `kafka_producer.log_event()` |
| Metadata | Accumulated via `record_info()` | None |
| Serialization | `FlatMapEvent` flattens metadata + event | Direct `serde_json::to_vec()` |
| sign_flag | Not supported | Supported via `KafkaEvent` wrapper |
| Dual publish | No | Yes (specific + consolidated) |
| Used for | AuditEvent only | All entity events, log events |
| Message count per emit | 1 | 1-3 (depending on entity/update) |

---

### 5. Usage Locations

**Implementations of `Event` trait:**
- `crates/router/src/events/audit_events.rs:61` - `AuditEvent` (ONLY implementation)

**Implementations of `MessagingInterface`:**
- `crates/router/src/services/kafka.rs:688` - `KafkaProducer`
- `crates/router/src/events.rs:110` - `EventsHandler`
- `crates/router/src/events/event_logger.rs:20` - `EventLogger`

**Implementations of `EventInfo` trait:**
- `crates/router/src/events/audit_events.rs:98` - `AuditEvent`
- `crates/router/src/core/payments.rs:8322` - `PaymentEvent`
- `crates/router/src/services/authentication.rs:214` - `AuthenticationType`
- `crates/events/src/lib.rs:226` - `(String, String)` tuple

**AuditEvent emit() calls:**
| File | Line | Event Type |
|------|------|------------|
| `core/payments/operations/payment_create.rs` | 1072 | PaymentCreate |
| `core/payments/operations/payment_confirm.rs` | 2616 | PaymentConfirm |
| `core/payments/operations/payment_cancel.rs` | 302 | PaymentCancelled |
| `core/payments/operations/payment_capture.rs` | 341 | PaymentCapture |
| `core/payments/operations/payment_update.rs` | 1079 | PaymentUpdate |
| `core/payments/operations/payment_approve.rs` | 275 | PaymentApprove |
| `core/payments/operations/payment_status.rs` | 182, 211 | PaymentStatus |
| `core/payments/operations/payment_complete_authorize.rs` | 545 | PaymentCompleteAuthorize |
| `core/payments/operations/payment_reject.rs` | 288 | PaymentReject |

**EventContext usage:**
| File | Line | Usage |
|------|------|-------|
| `routes/app.rs` | 119 | `ReqState.event_context` field |
| `routes/app.rs` | 171 | `EventContext::new(event_handler)` |
| `services/api.rs` | 241 | `record_info(request_id)` |
| `services/api.rs` | 246 | `record_info(("flow", flow))` |
| `services/api.rs` | 257 | `record_info(auth_type)` |

---

### 6. Dead Code in events Crate

**Unused `AuditEventType` variants (defined but never emitted):**
- `Error`, `PaymentCreated`, `ConnectorDecided`, `ConnectorCalled`
- `RefundCreated`, `RefundSuccess`, `RefundFail`

**Underutilized trait:**
- `Message` trait is only implemented for `FlatMapEvent` internally, never by users directly
- Could be made private or documented as internal-only

---

### 7. Dependency Chain

```
Only crates/router/Cargo.toml depends on events crate:
    events = { version = "0.1.0", path = "../events" }

No other microservices use this crate.
```

---

### 8. Implications for Refactoring

The `events` crate is **underutilized** - it has infrastructure for generic event handling with metadata accumulation, but is only used for AuditEvent. Potential directions:

1. **Extend** - Add support for sign_flag, dual publish to make it viable for entity events
2. **Simplify** - Remove unused code since it's only for AuditEvent
3. **Consolidate** - Merge `KafkaMessage` trait into events crate as the canonical interface
4. **Share** - Enable other microservices to use the same event framework

---

## Entry 6: KeyManagerState and Event Handler Dependencies

### 1. KeyManagerState request_id Propagation Bug

**Problem:** `request_id` is NOT reliably populated in `KeyManagerState`.

**Root Cause - Timing Issue:**
```
Timeline:
─────────────────────────────────────────────────────────────────────────
1. get_session_state() called
   └─► KeyManagerState created from AppState (request_id = None)
   └─► store.set_key_manager_state(key_manager_state)
       └─► Store's KeyManagerState.request_id = None

2. add_request_id(request_id) called  
   └─► SessionState.request_id = Some(request_id) ✓
   └─► Store.request_id = Some(request_id.to_string())  // Different field!
   └─► KeyManagerState.request_id remains None! ✗

3. Code paths diverge:
   ├─► Payment flows: &state.into() → Gets request_id from SessionState ✓
   └─► Store operations: get_keymanager_state() → Gets None ✗
─────────────────────────────────────────────────────────────────────────
```

**KeyManagerState Fields** (`common_utils/src/types/keymanager.rs`):
```rust
pub struct KeyManagerState {
    pub tenant_id: id_type::TenantId,
    pub global_tenant_id: id_type::TenantId,
    pub enabled: bool,
    pub url: String,
    pub client_idle_timeout: Option<u64>,
    
    #[cfg(feature = "km_forward_x_request_id")]
    pub request_id: Option<RequestId>,  // EXISTS but not reliably populated
    
    #[cfg(feature = "keymanager_mtls")]
    pub ca: Secret<String>,
    #[cfg(feature = "keymanager_mtls")]
    pub cert: Secret<String>,
    
    pub infra_values: Option<serde_json::Value>,
    pub use_legacy_key_store_decryption: bool,
}
```

**Call Site Analysis:**

| Call Site | How KeyManagerState is Obtained | request_id Available? |
|-----------|--------------------------------|----------------------|
| Payment flows (`payment_create`, etc.) | `&state.into()` from SessionState | **YES** ✓ |
| `transfer_key_to_key_manager` | `&state.into()` from SessionState | **YES** ✓ |
| Store operations (`merchant_key_store`, etc.) | `self.get_keymanager_state()` from store | **NO** ✗ |

---

### 2. Why Three Event Traits Exist

| Trait | Location | Purpose |
|-------|----------|---------|
| `KafkaMessage` | `router/src/services/kafka.rs` | For events that live **inside router** - defines serialization to Kafka |
| `MessagingInterface` | `events/src/lib.rs` | For **AuditEvent** pattern with metadata accumulation via EventContext |
| `EventHandlerInterface` | `hyperswitch_interfaces/src/events.rs` | For crates **outside router** that need to emit events |

**Why EventHandlerInterface was created:**
```rust
// crates/subscriptions/src/state.rs
pub event_handler: Box<dyn hyperswitch_interfaces::events::EventHandlerInterface>,
```

The `subscriptions` crate is a separate microservice that cannot depend on `router`, but still needs to emit events. `EventHandlerInterface` in `hyperswitch_interfaces` solves this cross-crate problem.

---

### 3. Crate Dependency Graph

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

**Key constraint:** `common_utils` cannot import from `router` or `hyperswitch_interfaces` (would create cycles or reverse dependencies).

---

### 4. request_id in Kafka Messages

| Event Type | request_id Location | Emission Path |
|------------|---------------------|---------------|
| ConnectorEvent | In payload | `log_event()` |
| ApiEvent | In payload | `log_event()` |
| AuditEvent | In payload + metadata HashMap | `send_message()` (MessagingInterface) |

**Kafka Headers:** Only the `MessagingInterface` path (`send_message()`) supports Kafka headers. The `log_event()` path does NOT send headers - request_id is always in the JSON payload.

**Headers flow** (MessagingInterface only):
```rust
// api_client.rs:330
state.event_handler().log_connector_event(&connector_event);

// kafka.rs:712-729
let mut headers = OwnedHeaders::new();
for (k, v) in metadata.iter() {
    headers = headers.insert(Header { key: k.as_str(), value: Some(v) });
}
BaseRecord::to(topic)
    .key(&data.identifier())
    .payload(&json_data)
    .headers(headers)
```

---

### 5. Implications for External Service Call Instrumentation

**Challenge:** `call_encryption_service()` is in `common_utils`, but event emission requires access to `EventsHandler` in `router`.

**Possible solutions:**
1. Define trait in `common_utils`, implement in `router`
2. Pass handler explicitly as function parameter
3. Use global channel for fire-and-forget emission

**Required fields for correlation:**
- `request_id` - Essential for correlating events to requests
- `event_handler` - Needed to emit the event to Kafka