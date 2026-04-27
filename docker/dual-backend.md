# RFC: Dual Redis Backend — `redis-rs` and `fred-rs` Feature Flags

**Branch:** `redis-rs-library`
**PR:** [#11793](https://github.com/juspay/hyperswitch/pull/11793)
**Author:** Shivansh Mathur

---

## Overview

This document describes the full implementation plan for supporting both the `redis` crate (`redis-rs`) and the `fred` crate (`fred-rs`) as compile-time selectable backends within the `redis_interface` crate.

The current branch migrates from `fred` to `redis`. The goal is to keep **both implementations** available under Cargo feature flags:

- `redis-rs` — uses the `redis` crate (current implementation, **default**)
- `fred-rs` — uses the `fred` crate (original implementation)

Only one backend compiles at a time. All consumer crates (`storage_impl`, `router`, `drainer`, `scheduler`) remain unchanged at the source level.

---

## Verified against

All crate API references in this RFC were cross-checked against:

- **In-repo fred-era source** at commit `ec6595fd70^` (`crates/redis_interface/src/{lib,commands,types}.rs` as it compiled against consumers on this branch).
- **In-repo redis-rs source** at the current branch tip.
- **docs.rs for fred `8.0.6`** — `RedisError::new<T: Into<Cow<'static, str>>>(kind, details)`, `RedisErrorKind::Unknown` present (1 of 17 variants), `fred::types::Message` fields `channel: Str / value: RedisValue / kind: MessageKind / server: Server`, `RedisValue` variants `Boolean / Integer(i64) / Double / String(Str) / Bytes(Bytes) / Null / Queued / Map / Array`, `XID` variants `Auto / Manual(Str) / Max / NewInGroup`.
- **docs.rs for redis `1.2.0`** — `redis::ParsingError` is a top-level export; `redis::Value` enum exists; `FromRedisValue::from_redis_value` returns `Result<Self, ParsingError>` (changed from `RedisError` in redis-rs 1.0).

**Version-lock caveats**: `fred = 8.0.6` and `redis = 1.2.0` are pinned. Any crate bump may shift the API surface — re-verify if versions change.

---

## Progress since draft

| Phase | Item | Status | Notes |
|---|---|---|---|
| 1.1 | Remove `router_env`, fix `tracing` import | ✅ DONE | `router_env` absent from `Cargo.toml`; `use tracing::instrument;` in place |
| 1.2 | Fix `unwrap_or_default()` on retries | ✅ DONE (differently) | Code uses `warn! + saturate to usize::MAX` at `lib.rs:187,213,393`, not `change_context` propagation — see revised 1.2 below |
| 1.3 | Remove unused `_is_subscriber_handler_spawned` | ✅ DONE | grep returns no hits in `lib.rs` |
| 1.4 | Remove redundant `drop(pubsub)` | ✅ DONE | grep returns no hits |
| 1.5 | Remove explanatory comment block | ✅ DONE | grep returns no hits |
| 2.1 | `stream_read_entries` return type | ⏳ PENDING | Still returns `redis::streams::StreamReadReply` (`commands.rs:947`). Helper `stream_read_grouped` (`commands.rs:975`) already contains the conversion logic |
| 2.2 | `stream_read_with_options` return type | ⏳ PENDING | Still returns `redis::streams::StreamReadReply` (`commands.rs:1020`) |
| 2.3 | `drainer/src/health_check.rs` caller | ⏳ PENDING | Still reads `.keys`/`.ids` at `health_check.rs:205–217` |
| 2.4 | `scheduler/src/utils.rs` caller | ⏳ PENDING | Still reads `.keys`/`.ids`/`.map` at `utils.rs:228–246`; stale comment at line 237 |
| 3.x | All dual-backend work | ⏳ PENDING | `RedisValue` wrapper exists at `types.rs:10–83` but is not yet cfg-gated; `pub use redis::Value` still present at `types.rs:5` |
| 4 | Test organisation | ⏳ PENDING | New phase — see below |

---

## Current State (as of branch `redis-rs-library`)

| File | Lines | Status |
|---|---|---|
| `crates/redis_interface/src/lib.rs` | 709 | redis-rs implementation; `PubSubMessage.value: redis::Value` still leaks |
| `crates/redis_interface/src/commands.rs` | 3969 | redis-rs implementation; `mod tests` ≈ 2638 lines (~65 tests) — most backend-neutral |
| `crates/redis_interface/src/types.rs` | 868 | `RedisValue` wrapper present (`:10–83`); still re-exports `pub use redis::Value` (`:5`); ~56 tests |
| `crates/redis_interface/src/errors.rs` | 81 | shared-compatible |
| `crates/redis_interface/src/constant.rs` | 17 | redis-rs specific constants |

**Fred-era source** is recoverable from git commit `ec6595fd70^` (the commit immediately before the migration).

---

## Architecture Decision

The `redis_interface` crate is the sole abstraction boundary. Every consumer imports only from `redis_interface` — never from `redis` or `fred` directly. This means the **entire dual-backend implementation stays inside `crates/redis_interface/`**, with no `#[cfg]` changes in any consumer source file.

### Target Directory Structure

```
crates/redis_interface/
  Cargo.toml               ← updated: optional redis/fred deps, new features
  build.rs                 ← NEW: mutual exclusion enforcement
  src/
    lib.rs                 ← REWRITTEN: thin dispatcher + re-exports
    errors.rs              ← SHARED (one additive change)
    constant.rs            ← SHARED (pub/sub constants only)
    types.rs               ← SHARED (6 backend-gated blocks)
    backends/
      redis_rs/
        mod.rs             ← MOVED from current lib.rs
        commands.rs        ← MOVED from current commands.rs
      fred/
        mod.rs             ← RESTORED from ec6595fd70^ + adapters
        commands.rs        ← RESTORED from ec6595fd70^ + return type fixes
```

---

## Phase 1 — Resolve Open PR Review Comments

These changes address reviewer feedback on PR #11793 and must land before the backend split.

> **Status:** 1.1, 1.3, 1.4, 1.5 are ✅ already in the branch. 1.2 landed but in a different shape than first proposed (see below). Snippets below are kept for provenance.

### 1.1 — Remove `router_env`, fix tracing import ✅ DONE

**Reviewer:** SanchithHegde (`Cargo.toml:25`)

`commands.rs` imports `use router_env::tracing` when `tracing` is already a direct workspace dependency. `router_env` is not needed in `redis_interface`.

**`crates/redis_interface/Cargo.toml`**
```toml
# REMOVE this line:
router_env = { version = "0.1.0", path = "../router_env", features = [...] }
```

**`crates/redis_interface/src/commands.rs:19`**
```rust
// BEFORE
use router_env::tracing;
use tracing::instrument;

// AFTER
use tracing::instrument;
```

> **Note:** `tracing = { workspace = true }` is already added as a direct dependency (done in the current branch state).

---

### 1.2 — Fix `unwrap_or_default()` silently zeroing retries ✅ DONE (warn+saturate)

**Reviewer:** SanchithHegde (`lib.rs:144` in PR diff)

The original RFC proposed `change_context(InvalidConfiguration(...))?` to fail startup on oversized values. The landed fix instead **logs a warning and saturates to `usize::MAX`**, so the service still boots. Current code at `lib.rs:187, 213, 393`:

```rust
// LANDED — warn + saturate
let limit = usize::try_from(conf.max_in_flight_commands).unwrap_or_else(|_| {
    tracing::warn!(
        "max_in_flight_commands ({}) exceeds usize, using usize::MAX",
        conf.max_in_flight_commands
    );
    usize::MAX
});
```

**Rationale for keeping warn+saturate:** the only realistic trigger is a 32-bit build with a very large config value; saturating to `usize::MAX` preserves the operator's intent (essentially unbounded), while a startup failure would be disproportionate. Revisit only if we start targeting constrained platforms where the distinction matters.

**Out of scope here:** the remaining `.unwrap_or_default()` at `lib.rs:317` is a pubsub channel-name fallback on a malformed `PushInfo` — unrelated to retry-count handling. Leave as-is.

---

### 1.3 — Remove unused `_is_subscriber_handler_spawned` parameter ✅ DONE

**Reviewer:** SanchithHegde (`lib.rs:276`)

Both `manage_standalone` and `manage_cluster` accept this parameter but never use it (prefixed with `_`).

**`crates/redis_interface/src/lib.rs`**
```rust
// BEFORE
async fn manage_standalone(
    pubsub_connection: &tokio::sync::Mutex<redis::aio::PubSub>,
    redis_client: &redis::Client,
    subscriptions: &tokio::sync::RwLock<std::collections::HashSet<String>>,
    broadcast_sender: &tokio::sync::broadcast::Sender<PubSubMessage>,
    _is_subscriber_handler_spawned: &Arc<atomic::AtomicBool>,  // unused
) { ... }

// AFTER
async fn manage_standalone(
    pubsub_connection: &tokio::sync::Mutex<redis::aio::PubSub>,
    redis_client: &redis::Client,
    subscriptions: &tokio::sync::RwLock<std::collections::HashSet<String>>,
    broadcast_sender: &tokio::sync::broadcast::Sender<PubSubMessage>,
) { ... }
```

Apply the same removal to `manage_cluster`. Update both call sites in `manage_subscriptions()` to drop the argument.

---

### 1.4 — Remove unnecessary explicit `drop(pubsub)` ✅ DONE

**Reviewer:** SanchithHegde (`lib.rs:292`)

The inner block's closing `}` already releases the lock guard. The explicit `drop` is redundant.

**`crates/redis_interface/src/lib.rs:275–280`**
```rust
// BEFORE
let result = {
    let mut pubsub = pubsub_connection.lock().await;
    let msg = pubsub.on_message().next().await;
    drop(pubsub);   // redundant
    msg
};

// AFTER
let result = {
    let mut pubsub = pubsub_connection.lock().await;
    pubsub.on_message().next().await
};
```

---

### 1.5 — Remove explanatory comment block ✅ DONE

**Reviewer:** SanchithHegde (`lib.rs:269`)

```rust
// REMOVE these lines from manage_standalone:
// Note: We hold the Mutex across `.await` here because
// `pubsub.on_message()` borrows the PubSub. This is safe because
// this is the only task that reads messages — subscribe/unsubscribe
// are the only other operations that acquire this lock, and they
// complete quickly. The lock is released explicitly after reading.
```

---

## Phase 2 — Fix Stream Return Type Leakage ⏳ PENDING

`stream_read_entries` (`commands.rs:947`) and `stream_read_with_options` (`commands.rs:1020`) still return `redis::streams::StreamReadReply` — a type from the `redis` crate — in the public API. Two consumer files access redis-crate-specific struct fields on this type directly.

This **must be fixed before the backend split** — the fred backend returns `XReadResponse<...>` (a fred-specific type), not `StreamReadReply`. The shared return type is `StreamReadResult`.

> **Note:** `stream_read_grouped` at `commands.rs:975` already implements the `StreamReadReply → StreamReadResult` conversion. The 2.1 change is effectively "promote that logic into `stream_read_entries` and delete `stream_read_grouped` as redundant" — the transformation is already written and tested.

```
StreamReadResult = HashMap<String, StreamEntries>
StreamEntries    = Vec<(String, HashMap<String, String>)>
                        ↑ entry_id   ↑ field → value
```

### 2.1 — Change `stream_read_entries` return type

**`crates/redis_interface/src/commands.rs`** (current line ~928):

```rust
// BEFORE
pub async fn stream_read_entries(
    &self,
    streams: &[RedisKey],
    ids: &[String],
    read_count: Option<u64>,
) -> CustomResult<redis::streams::StreamReadReply, errors::RedisError>

// AFTER
pub async fn stream_read_entries(
    &self,
    streams: &[RedisKey],
    ids: &[String],
    read_count: Option<u64>,
) -> CustomResult<StreamReadResult, errors::RedisError> {
    let mut conn = self.pool.clone();
    let stream_keys: Vec<String> = streams.iter().map(|s| s.tenant_aware_key(self)).collect();
    let count = read_count.unwrap_or(self.config.default_stream_read_count);

    let options = StreamReadOptions::default()
        .count(usize::try_from(count).change_context(errors::RedisError::StreamReadFailed)?);

    let reply: redis::streams::StreamReadReply = conn
        .xread_options(&stream_keys, ids, &options)
        .await
        .map_err(|err| match err.kind() {
            redis::ErrorKind::UnexpectedReturnType | redis::ErrorKind::Parse =>
                report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable),
            _ =>
                report!(err).change_context(errors::RedisError::StreamReadFailed),
        })?;

    Ok(reply.keys.into_iter().map(|stream_key| {
        let entries: StreamEntries = stream_key.ids.into_iter().map(|id| {
            let fields = id.map.into_iter()
                .filter_map(|(k, v)| redis_value_to_option_string(&v).map(|s| (k, s)))
                .collect();
            (id.id, fields)
        }).collect();
        (stream_key.key, entries)
    }).collect())
}
```

`stream_read_grouped` is now equivalent — remove it or keep as a thin alias.

---

### 2.2 — Change `stream_read_with_options` return type

**`crates/redis_interface/src/commands.rs`** (current line ~996):

Same pattern: keep `redis::streams::StreamReadReply` as a local `let reply` variable, convert to `StreamReadResult` before returning.

```rust
// BEFORE
pub async fn stream_read_with_options(
    ...
) -> CustomResult<redis::streams::StreamReadReply, errors::RedisError>

// AFTER
pub async fn stream_read_with_options(
    ...
) -> CustomResult<StreamReadResult, errors::RedisError> {
    // ... build options, call xread_options ...
    let reply: redis::streams::StreamReadReply = conn.xread_options(...).await.map_err(...)?;

    Ok(reply.keys.into_iter().map(|stream_key| {
        let entries: StreamEntries = stream_key.ids.into_iter().map(|id| {
            let fields = id.map.into_iter()
                .filter_map(|(k, v)| redis_value_to_option_string(&v).map(|s| (k, s)))
                .collect();
            (id.id, fields)
        }).collect();
        (stream_key.key, entries)
    }).collect())
}
```

---

### 2.3 — Update `drainer/src/health_check.rs`

**Current lines 199–217** access `redis::streams::StreamReadReply` fields directly:

```rust
// BEFORE
let output = redis_conn
    .stream_read_entries(&[TEST_STREAM_NAME.into()], &["0-0".to_string()], Some(10))
    .await
    .change_context(HealthCheckRedisError::StreamReadFailed)?;

let (_, id_to_trim) = output
    .keys                                            // redis::streams::StreamKey
    .iter()
    .find(|key| key.key == redis_conn.add_prefix(TEST_STREAM_NAME))
    .and_then(|stream_key| {
        stream_key.ids.last()                        // Vec<redis::streams::StreamId>
            .map(|last_entry| (&stream_key.ids, last_entry.id.clone()))
    })
    .ok_or(error_stack::report!(HealthCheckRedisError::StreamReadFailed))?;

// AFTER — StreamReadResult = HashMap<String, Vec<(String, HashMap<String, String>)>>
let output = redis_conn
    .stream_read_entries(&[TEST_STREAM_NAME.into()], &["0-0".to_string()], Some(10))
    .await
    .change_context(HealthCheckRedisError::StreamReadFailed)?;

let id_to_trim = output
    .get(&redis_conn.add_prefix(TEST_STREAM_NAME))
    .and_then(|entries| entries.last())
    .map(|(entry_id, _fields)| entry_id.clone())
    .ok_or(error_stack::report!(HealthCheckRedisError::StreamReadFailed))?;
```

---

### 2.4 — Update `scheduler/src/utils.rs`

**Current lines 228–246** access `redis::streams::StreamReadReply` fields:

```rust
// BEFORE (actual current code at utils.rs:227–246, including stale comment at :237)
// Convert StreamReadReply to the expected format
let (batches, entry_ids): (Vec<Vec<ProcessTrackerBatch>>, Vec<Vec<String>>) = response
    .keys
    .into_iter()
    .map(|stream_key| {
        stream_key.ids.into_iter().try_fold(
            (Vec::new(), Vec::new()),
            |(mut batches, mut entry_ids), id| {
                // Redis entry ID
                entry_ids.push(id.id);
                // Convert redis::Value map to HashMap<String, Option<String>>
                let fields = redis_interface::stream_fields_to_option_strings(id.map);
                batches.push(ProcessTrackerBatch::from_redis_stream_entry(fields)?);
                Ok((batches, entry_ids))
            },
        )
    })
    .collect::<CustomResult<Vec<_>, errors::ProcessTrackerError>>()?
    .into_iter()
    .unzip();

// AFTER — fields are already String in StreamReadResult, no redis::Value conversion needed.
// Remove the stale "Convert redis::Value map..." comment.
let (batches, entry_ids): (Vec<Vec<ProcessTrackerBatch>>, Vec<Vec<String>>) = response
    .into_values()
    .map(|entries| {
        entries.into_iter().try_fold(
            (Vec::new(), Vec::new()),
            |(mut batches, mut entry_ids), (entry_id, fields)| {
                entry_ids.push(entry_id);
                let fields: HashMap<String, Option<String>> =
                    fields.into_iter().map(|(k, v)| (k, Some(v))).collect();
                batches.push(ProcessTrackerBatch::from_redis_stream_entry(fields)?);
                Ok((batches, entry_ids))
            },
        )
    })
    .collect::<CustomResult<Vec<_>, errors::ProcessTrackerError>>()?
    .into_iter()
    .unzip();
```

**Also:** once `stream_fields_to_option_strings` is no longer needed here (the only caller), it becomes a redis-rs-backend-only helper — see Phase 3.5(e).

> **Note:** This also satisfies `jagan-jaya`'s review request for a stream iteration utility in drainer and scheduler — callers no longer need to parse backend-internal stream reply types manually.

---

## Phase 3 — Dual Backend Feature Flag

### 3.1 — Update `crates/redis_interface/Cargo.toml`

```toml
[package]
name = "redis_interface"
description = "A user-friendly interface to Redis"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
readme = "README.md"
license.workspace = true

[features]
default = ["redis-rs"]       # redis-rs is the default; cargo build uses it with no flags
redis-rs = ["dep:redis"]
fred-rs  = ["dep:fred"]
multitenancy_fallback = []

[dependencies]
error-stack  = "0.4.1"
futures      = "0.3"
serde        = { version = "1.0.219", features = ["derive"] }
thiserror    = "1.0.69"
tokio        = "1.48.0"
tokio-stream = { version = "0.1.17", features = ["sync"] }
tracing      = { workspace = true }

# Backend deps — exactly one will be compiled depending on the active feature
redis = { version = "1.2.0", optional = true, features = [
    "tokio-comp", "cluster-async", "streams", "script", "connection-manager"
]}
fred = { version = "8.0.6", optional = true, features = [
    "metrics", "partial-tracing", "subscriber-client"
]}

# First party crates
common_utils = { version = "0.1.0", path = "../common_utils", features = ["async_ext"] }

[dev-dependencies]
tokio = { version = "1.48.0", features = ["macros", "rt-multi-thread"] }

[lints]
workspace = true
```

**Key changes from current:**
- `router_env` removed entirely (Phase 1.1)
- `redis` made optional under `dep:redis`
- `fred` added as optional under `dep:fred`
- `redis-rs` and `fred-rs` feature gates added
- `default = ["redis-rs"]` so `cargo build` without flags uses redis-rs

---

### 3.2 — Create `build.rs` for Mutual Exclusion

**`crates/redis_interface/build.rs`** (new file):

```rust
fn main() {
    let redis_rs = std::env::var("CARGO_FEATURE_REDIS_RS").is_ok();
    let fred_rs  = std::env::var("CARGO_FEATURE_FRED_RS").is_ok();

    match (redis_rs, fred_rs) {
        (true, true) => panic!(
            "\n\nFeatures `redis-rs` and `fred-rs` are mutually exclusive.\n\
             Enable exactly one:\n\
             --features redis-rs   (default)\n\
             --features fred-rs\n"
        ),
        (false, false) => panic!(
            "\n\nExactly one of `redis-rs` or `fred-rs` must be enabled.\n\
             Neither is currently active.\n"
        ),
        _ => {}
    }
}
```

This produces a clear compile-time error if both are active simultaneously or if neither is active.

---

### 3.3 — Rewrite `src/lib.rs` as Thin Dispatcher

```rust
//! Redis interface — compile-time backend selection via Cargo features.
//! Enable exactly one of: `redis-rs` (default) or `fred-rs`.
//!
//! # Examples
//! ```
//! use redis_interface::{types::RedisSettings, RedisConnectionPool};
//!
//! #[tokio::main]
//! async fn main() {
//!     let redis_conn = RedisConnectionPool::new(&RedisSettings::default()).await;
//! }
//! ```

pub mod errors;
pub mod types;
pub mod constant;

#[cfg(all(feature = "redis-rs", not(feature = "fred-rs")))]
mod backends { pub mod redis_rs; }

#[cfg(all(feature = "fred-rs", not(feature = "redis-rs")))]
mod backends { pub mod fred; }

// Re-export the active backend's public types under unified names.
// All external code imports `redis_interface::RedisConnectionPool` etc.
// and is never aware of which backend is active.

#[cfg(feature = "redis-rs")]
pub use backends::redis_rs::{
    PubSubMessage, RedisClient, RedisConn, RedisConfig, RedisConnectionPool, SubscriberClient,
};

#[cfg(feature = "fred-rs")]
pub use backends::fred::{
    PubSubMessage, RedisClient, RedisConfig, RedisConnectionPool, SubscriberClient,
};

pub use self::types::*;
```

---

### 3.4 — Move redis-rs Code into `backends/redis_rs/`

**`backends/redis_rs/mod.rs`** = current `src/lib.rs` content (with Phase 1 fixes applied).

**`backends/redis_rs/commands.rs`** = current `src/commands.rs` (with Phase 2 fixes applied).

No logical changes — just file relocation. The redis-specific command string constants (`REDIS_COMMAND_SET`, `REDIS_COMMAND_HSCAN`, etc.) move from `constant.rs` into `backends/redis_rs/commands.rs` since they are redis-rs specific. `constant.rs` retains only the pub/sub backoff constants shared by both backends:

```rust
// constant.rs after cleanup — shared by both backends
pub const PUBSUB_INITIAL_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(1);
pub const PUBSUB_MAX_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(30);
pub const PUBSUB_RETRY_BACKOFF_FACTOR: u32 = 2;
```

---

### 3.5 — Gate `types.rs` for Backend Neutrality

`types.rs` must compile cleanly with either feature active. Six sets of `#[cfg]` blocks are needed.

#### a) `RedisValue` inner type

> **Current state:** `RedisValue` already exists at `types.rs:10–83` with methods `new / from_bytes / from_string / as_bytes / as_string / into_inner`, `Deref`, `From<RedisValue> for redis::Value`, and `ToRedisArgs / ToSingleRedisArg`. The work here is **adding `#[cfg]` gates** to the existing wrapper, plus removing two leaks at `types.rs:5–6`:
>
> ```rust
> pub use redis::Value;                   // REMOVE — consumers must not see redis crate
> use redis::Value as RedisCrateValue;    // REMOVE — internal alias no longer useful
> ```

```rust
pub struct RedisValue {
    #[cfg(feature = "redis-rs")]
    inner: redis::Value,
    #[cfg(feature = "fred-rs")]
    inner: fred::types::RedisValue,
}

impl RedisValue {
    #[cfg(feature = "redis-rs")]
    pub fn new(value: redis::Value) -> Self { Self { inner: value } }
    #[cfg(feature = "fred-rs")]
    pub fn new(value: fred::types::RedisValue) -> Self { Self { inner: value } }

    #[cfg(feature = "redis-rs")]
    pub fn from_bytes(val: Vec<u8>) -> Self {
        Self { inner: redis::Value::BulkString(val) }
    }
    #[cfg(feature = "fred-rs")]
    pub fn from_bytes(val: Vec<u8>) -> Self {
        Self { inner: fred::types::RedisValue::Bytes(val.into()) }
    }

    #[cfg(feature = "redis-rs")]
    pub fn from_string(value: String) -> Self {
        Self { inner: redis::Value::SimpleString(value) }
    }
    #[cfg(feature = "fred-rs")]
    pub fn from_string(value: String) -> Self {
        Self { inner: fred::types::RedisValue::String(value.into()) }
    }

    #[cfg(feature = "redis-rs")]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            redis::Value::BulkString(b) => Some(b.as_slice()),
            redis::Value::SimpleString(s) => Some(s.as_bytes()),
            _ => None,
        }
    }
    #[cfg(feature = "fred-rs")]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            fred::types::RedisValue::Bytes(b) => Some(b.as_ref()),
            fred::types::RedisValue::String(s) => Some(s.as_bytes()),
            _ => None,
        }
    }

    #[cfg(feature = "redis-rs")]
    pub fn as_string(&self) -> Option<String> {
        match &self.inner {
            redis::Value::BulkString(b) => String::from_utf8(b.clone()).ok(),
            redis::Value::SimpleString(s) => Some(s.clone()),
            _ => None,
        }
    }
    #[cfg(feature = "fred-rs")]
    pub fn as_string(&self) -> Option<String> {
        match &self.inner {
            fred::types::RedisValue::String(s) => Some(s.to_string()),
            fred::types::RedisValue::Bytes(b) => String::from_utf8(b.to_vec()).ok(),
            _ => None,
        }
    }

    #[cfg(feature = "redis-rs")]
    pub fn into_inner(self) -> redis::Value { self.inner }
    #[cfg(feature = "fred-rs")]
    pub fn into_inner(self) -> fred::types::RedisValue { self.inner }
}

// Deref
#[cfg(feature = "redis-rs")]
impl std::ops::Deref for RedisValue {
    type Target = redis::Value;
    fn deref(&self) -> &Self::Target { &self.inner }
}
#[cfg(feature = "fred-rs")]
impl std::ops::Deref for RedisValue {
    type Target = fred::types::RedisValue;
    fn deref(&self) -> &Self::Target { &self.inner }
}

// From<RedisValue>
#[cfg(feature = "redis-rs")]
impl From<RedisValue> for redis::Value {
    fn from(v: RedisValue) -> Self { v.inner }
}
#[cfg(feature = "fred-rs")]
impl From<RedisValue> for fred::types::RedisValue {
    fn from(v: RedisValue) -> Self { v.inner }
}

// ToRedisArgs — redis-rs only
#[cfg(feature = "redis-rs")]
impl redis::ToRedisArgs for RedisValue {
    fn write_redis_args<W>(&self, out: &mut W) where W: ?Sized + redis::RedisWrite {
        match &self.inner {
            redis::Value::BulkString(bytes) => bytes.write_redis_args(out),
            redis::Value::SimpleString(s)   => s.write_redis_args(out),
            _ => Vec::<u8>::new().write_redis_args(out),
        }
    }
}
#[cfg(feature = "redis-rs")]
impl redis::ToSingleRedisArg for RedisValue {}
```

#### b) Reply type trait implementations

```rust
// SetnxReply
#[cfg(feature = "redis-rs")]
impl redis::FromRedisValue for SetnxReply {
    fn from_redis_value(v: redis::Value) -> Result<Self, redis::ParsingError> {
        match v {
            redis::Value::Okay => Ok(Self::KeySet),
            redis::Value::SimpleString(ref s) if s == "OK" => Ok(Self::KeySet),
            redis::Value::BulkString(ref s) if s == b"OK" => Ok(Self::KeySet),
            redis::Value::Nil => Ok(Self::KeyNotSet),
            _ => {
                tracing::error!(received = ?v, "Unexpected SETNX reply");
                Err(redis::ParsingError::from(format!("Unexpected SETNX reply: {:?}", v)))
            }
        }
    }
}

#[cfg(feature = "fred-rs")]
impl fred::types::FromRedis for SetnxReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            fred::types::RedisValue::String(_) => Ok(Self::KeySet),
            fred::types::RedisValue::Null      => Ok(Self::KeyNotSet),
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown, "Unexpected SETNX reply",
            )),
        }
    }
}

// Apply the same #[cfg] pattern to: HsetnxReply, MsetnxReply, DelReply, SaddReply
```

#### c) `RedisEntryId` trait implementations

```rust
// redis-rs: convert to string for ToRedisArgs
#[cfg(feature = "redis-rs")]
impl redis::ToRedisArgs for RedisEntryId {
    fn write_redis_args<W>(&self, out: &mut W) where W: ?Sized + redis::RedisWrite {
        self.to_stream_id().write_redis_args(out)
    }
}

// fred-rs: convert to XID
#[cfg(feature = "fred-rs")]
impl From<RedisEntryId> for fred::types::XID {
    fn from(id: RedisEntryId) -> Self {
        match id {
            RedisEntryId::UserSpecifiedID { milliseconds, sequence_number } =>
                Self::Manual(fred::bytes_utils::format_bytes!("{milliseconds}-{sequence_number}")),
            RedisEntryId::AutoGeneratedID   => Self::Auto,
            RedisEntryId::AfterLastID       => Self::Max,
            RedisEntryId::UndeliveredEntryID => Self::NewInGroup,
        }
    }
}

#[cfg(feature = "fred-rs")]
impl From<&RedisEntryId> for fred::types::XID { /* same match */ }
```

> **Note on `XID::Manual`:** per docs.rs fred 8.0.6, the variant is `Manual(Str)` where `Str = bytes_utils::Str`, not `Manual(Bytes)`. The `format_bytes!` macro produces a `Bytes` value that is converted into `Str` via `From`/`Into`. This is exactly what the fred-era source at `ec6595fd70^` used, so the snippet is ported verbatim — no change required, but a reader seeing `format_bytes!` passed to a `Str`-typed variant should not be alarmed.

#### d) `StreamCapKind` / `StreamCapTrim` conversions (fred-rs only)

```rust
#[cfg(feature = "fred-rs")]
impl From<StreamCapKind> for fred::types::XCapKind {
    fn from(item: StreamCapKind) -> Self {
        match item {
            StreamCapKind::MaxLen => Self::MaxLen,
            StreamCapKind::MinID  => Self::MinID,
        }
    }
}

#[cfg(feature = "fred-rs")]
impl From<StreamCapTrim> for fred::types::XCapTrim {
    fn from(item: StreamCapTrim) -> Self {
        match item {
            StreamCapTrim::Exact        => Self::Exact,
            StreamCapTrim::AlmostExact  => Self::AlmostExact,
        }
    }
}
```

#### e) `redis_value_to_option_string` and `stream_fields_to_option_strings` (redis-rs only)

These helpers take `&redis::Value` as a parameter. Under fred-rs they are not needed (stream entries are already `String` after the Phase 2 return type fix).

```rust
#[cfg(feature = "redis-rs")]
pub fn redis_value_to_option_string(v: &redis::Value) -> Option<String> { ... }

#[cfg(feature = "redis-rs")]
pub fn stream_fields_to_option_strings(
    fields: std::collections::HashMap<String, redis::Value>,
) -> std::collections::HashMap<String, Option<String>> { ... }
```

#### f) `PubSubMessage.value` — change from `redis::Value` to `RedisValue`

`PubSubMessage` is currently defined in `lib.rs:96–99` with `value: Value` (where `Value` comes from `pub use redis::Value`). In the dual-backend world it moves to each backend's `mod.rs` and its `value` field uses `RedisValue` (the crate-owned wrapper) so consumers are backend-agnostic:

```rust
// In backends/redis_rs/mod.rs AND backends/fred/mod.rs
pub struct PubSubMessage {
    pub channel: String,
    pub value: crate::types::RedisValue,   // NOT redis::Value
}
```

**Consumer impact — this change simplifies the only current caller.** At `storage_impl/src/redis/pub_sub.rs:88`:

```rust
// BEFORE — consumer wraps a leaked redis::Value into RedisValue manually
let message = match CacheRedact::try_from(RedisValue::new(message.value))

// AFTER — message.value is already RedisValue
let message = match CacheRedact::try_from(message.value)
```

No other consumer touches `PubSubMessage.value`.

---

### 3.6 — Fix `errors.rs`: Add Missing Variant

Fred's `errors.rs` was identical to redis-rs except it was missing `ScriptExecutionFailed`. Add it unconditionally so the file is fully shared:

```rust
// crates/redis_interface/src/errors.rs — add:
#[error("Failed to evaluate Lua script in Redis")]
ScriptExecutionFailed,
```

---

### 3.7 — Restore `backends/fred/mod.rs`

**Base:** `git show ec6595fd70^:crates/redis_interface/src/lib.rs`

Apply these changes on top of the restored content:

#### a) Remove fred trait re-exports
```rust
// REMOVE — consumers must not depend on fred traits directly:
pub use fred::interfaces::{EventInterface, PubsubInterface};
```

#### b) Add `PubSubMessage` struct
```rust
// Fred's original code had no PubSubMessage — callers used Deref to fred::types::Message.
// Add for API parity with redis-rs backend:
pub struct PubSubMessage {
    pub channel: String,
    pub value: crate::types::RedisValue,
}
```

#### c) Rework `SubscriberClient` to expose `message_rx()`

The original fred `SubscriberClient` used `Deref` to expose `fred::clients::SubscriberClient`, which callers called `.message_rx()` on — returning `BroadcastReceiver<fred::types::Message>`. Replace this with a proper `broadcast::Receiver<PubSubMessage>` for API parity.

```rust
pub struct SubscriberClient {
    inner: fred::clients::SubscriberClient,
    pub is_subscriber_handler_spawned: Arc<atomic::AtomicBool>,
    broadcast_sender: tokio::sync::broadcast::Sender<PubSubMessage>,
}

impl SubscriberClient {
    pub async fn new(
        config: fred::types::RedisConfig,
        reconnect_policy: fred::types::ReconnectPolicy,
        perf: fred::types::PerformanceConfig,
        broadcast_capacity: usize,
    ) -> CustomResult<Self, errors::RedisError> {
        let client = fred::clients::SubscriberClient::new(
            config, Some(perf), None, Some(reconnect_policy),
        );
        client.connect();
        client.wait_for_connect().await
            .change_context(errors::RedisError::RedisConnectionError)?;
        let (broadcast_sender, _) = tokio::sync::broadcast::channel(broadcast_capacity);
        Ok(Self {
            inner: client,
            is_subscriber_handler_spawned: Arc::new(atomic::AtomicBool::new(false)),
            broadcast_sender,
        })
    }

    pub fn message_rx(&self) -> tokio::sync::broadcast::Receiver<PubSubMessage> {
        self.broadcast_sender.subscribe()
    }

    pub fn is_subscriber_handler_spawned(&self) -> &Arc<atomic::AtomicBool> {
        &self.is_subscriber_handler_spawned
    }

    pub async fn subscribe(&self, channel: &str) -> CustomResult<(), errors::RedisError> {
        use fred::interfaces::PubsubInterface;
        self.inner.subscribe(channel).await
            .change_context(errors::RedisError::SubscribeError)
    }

    pub async fn unsubscribe(&self, channel: &str) -> CustomResult<(), errors::RedisError> {
        use fred::interfaces::PubsubInterface;
        self.inner.unsubscribe(channel).await
            .change_context(errors::RedisError::SubscribeError)
    }

    pub async fn manage_subscriptions(&self) {
        use fred::interfaces::PubsubInterface;
        let mut rx = self.inner.message_rx();
        let sender = self.broadcast_sender.clone();
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    let channel = msg.channel.to_string();
                    let value = crate::types::RedisValue::from_bytes(
                        msg.value.as_bytes().map(|b| b.to_vec()).unwrap_or_default(),
                    );
                    let _ = sender.send(PubSubMessage { channel, value });
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("fred pub/sub receiver lagged, {} messages dropped", n);
                }
            }
        }
    }
}
// Remove the Deref impl — consumers no longer access fred traits through SubscriberClient
```

#### d) `RedisClient.publish()` — explicit method for API parity

```rust
impl RedisClient {
    pub async fn new(
        config: fred::types::RedisConfig,
        reconnect_policy: fred::types::ReconnectPolicy,
        perf: fred::types::PerformanceConfig,
    ) -> CustomResult<Self, errors::RedisError> {
        let client = fred::prelude::RedisClient::new(
            config, Some(perf), None, Some(reconnect_policy),
        );
        client.connect();
        client.wait_for_connect().await
            .change_context(errors::RedisError::RedisConnectionError)?;
        Ok(Self { inner: client })
    }

    pub async fn publish(
        &self,
        channel: &str,
        message: crate::types::RedisValue,
    ) -> CustomResult<usize, errors::RedisError> {
        use fred::interfaces::PubsubInterface;
        self.inner.publish(channel, message.into_inner()).await
            .change_context(errors::RedisError::PublishError)
    }
}
// Remove the Deref impl — consumers use .publish() directly
```

#### e) `RedisConfig` — simplified 4-field version

```rust
pub struct RedisConfig {
    pub(crate) default_ttl: u32,
    pub(crate) default_stream_read_count: u64,
    pub(crate) default_hash_ttl: u32,
    pub(crate) cluster_enabled: bool,
    // max_failure_threshold is present in RedisSettings but not used by fred-rs
}

impl From<&RedisSettings> for RedisConfig {
    fn from(config: &RedisSettings) -> Self {
        Self {
            default_ttl: config.default_ttl,
            default_stream_read_count: config.stream_read_count,
            default_hash_ttl: config.default_hash_ttl,
            cluster_enabled: config.cluster_enabled,
        }
    }
}
```

---

### 3.8 — Restore `backends/fred/commands.rs`

**Base:** `git show ec6595fd70^:crates/redis_interface/src/commands.rs`

Apply these changes on top:

#### a) Change `stream_read_entries` return type

```rust
// BEFORE (fred era)
pub async fn stream_read_entries<K, Ids>(...) 
    -> CustomResult<XReadResponse<String, String, String, String>, errors::RedisError>

// AFTER — convert XReadResponse to StreamReadResult before returning
pub async fn stream_read_entries(
    &self,
    streams: &[RedisKey],
    ids: &[String],
    read_count: Option<u64>,
) -> CustomResult<StreamReadResult, errors::RedisError> {
    let strms = self.get_keys_with_prefix(streams);
    let reply: XReadResponse<String, String, String, String> = self.pool
        .xread_map(
            Some(read_count.unwrap_or(self.config.default_stream_read_count)),
            None,
            strms,
            ids,
        )
        .await
        .map_err(|err| match err.kind() {
            RedisErrorKind::NotFound | RedisErrorKind::Parse =>
                report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable),
            _ =>
                report!(err).change_context(errors::RedisError::StreamReadFailed),
        })?;

    Ok(reply.into_iter().map(|(key, entries)| {
        let parsed: StreamEntries = entries.into_iter()
            .map(|(entry_id, fields)| (entry_id, fields.into_iter().collect()))
            .collect();
        (key, parsed)
    }).collect())
}
```

#### b) Change `stream_read_with_options` return type

Same pattern — convert `XReadResponse<String, String, String, Option<String>>` to `StreamReadResult`. Drop `None` values when building the field map:

```rust
pub async fn stream_read_with_options(
    &self,
    streams: &[RedisKey],
    ids: &[String],
    count: Option<u64>,
    block: Option<u64>,
    group: Option<(&str, &str)>,
) -> CustomResult<StreamReadResult, errors::RedisError> {
    // ... existing dispatch to xread_map / xreadgroup_map ...
    // let reply: XReadResponse<String, String, String, Option<String>> = ...;

    Ok(reply.into_iter().map(|(key, entries)| {
        let parsed: StreamEntries = entries.into_iter()
            .map(|(entry_id, fields)| {
                let fields = fields.into_iter()
                    .filter_map(|(k, v)| v.map(|s| (k, s)))
                    .collect();
                (entry_id, fields)
            })
            .collect();
        (key, parsed)
    }).collect())
}
```

#### c) Normalise `get_keys_with_prefix` signature

Change from generic `K: Into<MultipleKeys>` to concrete `&[RedisKey]` to match the redis-rs backend's convention:

```rust
fn get_keys_with_prefix(&self, streams: &[RedisKey]) -> Vec<String> {
    streams.iter().map(|k| k.tenant_aware_key(self)).collect()
}
```

---

### 3.9 — Consumer Crate `Cargo.toml` Feature Forwarding

No source code changes in consumer crates. Only `Cargo.toml` additions.

Add to **`crates/storage_impl/Cargo.toml`**, **`crates/router/Cargo.toml`**, **`crates/drainer/Cargo.toml`**, **`crates/scheduler/Cargo.toml`**:

```toml
[features]
# Append to existing features section:
redis-rs = ["redis_interface/redis-rs"]
fred-rs  = ["redis_interface/fred-rs"]
```

Because `redis_interface` defaults to `redis-rs`, a plain `cargo build` with no flags works without any change.

---

### 3.10 — CI Build Matrix

Add to `.github/workflows/CI-pr.yml`:

```yaml
- name: Check redis_interface with redis-rs (default)
  run: cargo check -p redis_interface

- name: Check redis_interface with fred-rs
  run: cargo check -p redis_interface --no-default-features --features fred-rs

- name: Check full workspace with redis-rs (default)
  run: cargo check --features "release"

- name: Check full workspace with fred-rs
  run: cargo check --no-default-features --features "release,fred-rs"

- name: Verify mutual exclusion is enforced
  run: |
    cargo check -p redis_interface --features "redis-rs,fred-rs" 2>&1 \
      | grep -q "mutually exclusive" \
      && echo "OK: mutual exclusion enforced" \
      || (echo "FAIL: mutual exclusion not enforced" && exit 1)

- name: Test redis_interface with redis-rs
  run: cargo test -p redis_interface
  services:
    redis:
      image: "public.ecr.aws/docker/library/redis:alpine"
      ports: ["6379:6379"]

- name: Test redis_interface with fred-rs
  run: cargo test -p redis_interface --no-default-features --features fred-rs
  services:
    redis:
      image: "public.ecr.aws/docker/library/redis:alpine"
      ports: ["6379:6379"]
```

---

## Phase 4 — Test Organisation

The existing test suite (~121 tests: ~65 in `commands.rs` + ~56 in `types.rs`) is mostly already backend-neutral: only 12 grep-hits for `redis::Value::` / `redis::FromRedisValue` / `StreamReadReply` across the two files, and 6 of those 12 are in `types.rs` testing `FromRedisValue` trait impls that are inherently redis-rs-specific. The remaining ~95% of test bodies call only `RedisConnectionPool::*` public APIs and will run unchanged against either backend.

**The current RFC plan (moving tests wholesale into `backends/redis_rs/`) would leave the fred-rs backend with zero shared coverage.** This phase prevents that by partitioning tests by what they actually exercise.

### 4.1 — Three test buckets

| Bucket | What it tests | Lives in | Runs under |
|---|---|---|---|
| 1. Shared integration | Public `RedisConnectionPool` API against a real Redis server | `crates/redis_interface/tests/integration.rs` (or `src/shared_tests.rs`) | Both features — proves behavioural parity |
| 2. Backend unit | Trait impls (`FromRedisValue`, `FromRedis`), value-variant matching on the underlying crate's types | `backends/redis_rs/types_tests.rs` + `backends/fred/types_tests.rs` (mirror pair) | Each under its own feature only |
| 3. Parity | *Falls out of Bucket 1 automatically* — same test body passing under both features is the parity assertion | — | — |

### 4.2 — File moves

- **Most of `mod tests` in `commands.rs`** (~2638 lines, the ~65 tests that use only `RedisConnectionPool::*`) → `crates/redis_interface/tests/integration.rs`. Backend-neutral after Phase 2 flips the stream return types.
- **6 tests in `types.rs`** that assert against `redis::Value::{Int, BulkString, SimpleString, ...}` → split into two backend-gated mirror files:
  - `backends/redis_rs/types_tests.rs` (keeps existing `redis::Value` assertions)
  - `backends/fred/types_tests.rs` (mirrors the six tests with `fred::types::RedisValue::{Integer, Bytes, String, ...}` assertions — same inputs, same expected semantics)

### 4.3 — Live-Redis gating

Tests that require a running server must be `#[ignore]`-gated or detect an env var (e.g. `REDIS_URL`), so `cargo test` with no Redis still compiles both backends cleanly. This is also a prerequisite for the CI matrix in 3.10 — the CI job that starts a `redis:alpine` service runs them by un-ignoring explicitly.

### 4.4 — Authoring guidance (to add to RFC sign-off checklist)

Any new test must pick a bucket:

- **Reaches for `redis::Value::*` or `fred::types::RedisValue::*` directly?** → Bucket 2 (backend-specific).
- **Calls only `RedisConnectionPool::*` / `RedisValue::as_bytes()` / the unified public API?** → Bucket 1 (shared).
- **Asserts behavioural equivalence between the backends?** → already covered by running Bucket 1 twice; no separate file needed.

### 4.5 — Duplication budget

Bucket 2 carries roughly 6 redis-rs + 6 fred mirror tests. That duplication is intentional — each backend must prove it maps its own crate's value variants correctly. Beyond those six pairs, duplication should be flagged in review and moved up to Bucket 1.

---

## Complete File Change Table

| File | Action | Phase | Status |
|---|---|---|---|
| `redis_interface/Cargo.toml` | Remove `router_env` | 1.1 | ✅ DONE |
| `redis_interface/Cargo.toml` | Make `redis`/`fred` optional; add feature gates | 3.1 | ⏳ PENDING |
| `redis_interface/build.rs` | **Create** — mutual exclusion enforcement at compile time | 3.2 | ⏳ PENDING |
| `redis_interface/src/lib.rs` | **Rewrite** as thin dispatcher | 3.3 | ⏳ PENDING |
| `redis_interface/src/lib.rs` | Fix `unwrap_or_default` (warn+saturate), remove unused param, remove `drop`, remove comment | 1.2–1.5 | ✅ DONE |
| `redis_interface/src/lib.rs` | `PubSubMessage.value: redis::Value → RedisValue` | 3.5(f) | ⏳ PENDING |
| `redis_interface/src/commands.rs` | Fix `use router_env::tracing` → `use tracing` | 1.1 | ✅ DONE |
| `redis_interface/src/commands.rs` | Fix `stream_read_entries` / `stream_read_with_options` return types (conversion logic already in `stream_read_grouped`) | 2.1–2.2 | ⏳ PENDING |
| `redis_interface/src/types.rs` | Remove `pub use redis::Value` / `RedisCrateValue` alias (:5–6); cfg-gate existing `RedisValue` wrapper, reply impls, `RedisEntryId`, `StreamCap*`, helpers | 3.5 | ⏳ PENDING |
| `redis_interface/src/errors.rs` | Add `ScriptExecutionFailed` variant | 3.6 | ⏳ PENDING |
| `redis_interface/src/constant.rs` | Remove cmd string constants (→ `redis_rs/commands.rs`); keep pub/sub backoff | 3.4 | ⏳ PENDING |
| `redis_interface/src/backends/redis_rs/mod.rs` | **Create** (moved from `src/lib.rs`) | 3.4 | ⏳ PENDING |
| `redis_interface/src/backends/redis_rs/commands.rs` | **Create** (moved from `src/commands.rs` minus shared tests) | 3.4 | ⏳ PENDING |
| `redis_interface/src/backends/redis_rs/types_tests.rs` | **Create** — 6 backend-specific value-variant tests | 4.2 | ⏳ PENDING |
| `redis_interface/src/backends/fred/mod.rs` | **Create** (restored from `ec6595fd70^` + adapters) | 3.7 | ⏳ PENDING |
| `redis_interface/src/backends/fred/commands.rs` | **Create** (restored + return type fixes) | 3.8 | ⏳ PENDING |
| `redis_interface/src/backends/fred/types_tests.rs` | **Create** — mirror 6 tests against `fred::types::RedisValue` | 4.2 | ⏳ PENDING |
| `redis_interface/tests/integration.rs` | **Create** — ~65 shared command tests moved from `commands.rs` | 4.2 | ⏳ PENDING |
| `drainer/src/health_check.rs` | Update `stream_read_entries` caller (read `.keys`/`.ids` → map lookup) | 2.3 | ⏳ PENDING |
| `scheduler/src/utils.rs` | Update `stream_read_with_options` caller; remove stale `// Convert redis::Value map...` comment at :237 | 2.4 | ⏳ PENDING |
| `storage_impl/src/redis/pub_sub.rs` | Drop `RedisValue::new(message.value)` wrap at :88 (consumer simplification) | 3.5(f) | ⏳ PENDING |
| `storage_impl/Cargo.toml` | Add feature forwarding | 3.9 | ⏳ PENDING |
| `router/Cargo.toml` | Add feature forwarding | 3.9 | ⏳ PENDING |
| `drainer/Cargo.toml` | Add feature forwarding | 3.9 | ⏳ PENDING |
| `scheduler/Cargo.toml` | Add feature forwarding | 3.9 | ⏳ PENDING |
| `.github/workflows/CI-pr.yml` | Add dual-backend build matrix | 3.10 | ⏳ PENDING |

---

## Zero-Change Consumer Guarantee

The following files require **no source changes**. The abstraction holds completely:

| File | Why unchanged |
|---|---|
| `storage_impl/src/redis/pub_sub.rs` | Calls `subscriber.message_rx()`, `subscribe()`, `is_subscriber_handler_spawned()` — same API in both backends |
| `storage_impl/src/redis/kv_store.rs` | Uses `RedisError`, `HsetnxReply`, `SetnxReply` — all in shared `errors.rs` / `types.rs` |
| `storage_impl/src/redis/cache.rs` | Calls `msg.value.as_bytes()` — works via `RedisValue::as_bytes()` in both backends |
| `router/src/connection.rs` | Calls `RedisConnectionPool::new(&conf.redis)` — same signature both backends |
| `drainer/src/connection.rs` | Same as above |

---

## Configuration Field Mapping

`RedisSettings` is a **single shared struct** in `types.rs`. Fields that only one backend uses are simply ignored by the other backend's `new()` implementation — no `#[cfg]` needed in the struct itself.

| Field | redis-rs | fred-rs | Notes |
|---|---|---|---|
| `host`, `port`, `cluster_enabled`, `cluster_urls` | Used | Used | Core connectivity |
| `use_legacy_version` | Used (RESP2/3 toggle) | Used (RESP3 via config) | Both |
| `pool_size` | Ignored (single `ConnectionManager`) | Used (`RedisPool::new`) | fred-rs only |
| `reconnect_max_attempts` | Used (`set_number_of_retries`) | Used (`ReconnectPolicy`) | Both |
| `reconnect_delay` | Used (`set_min_delay`) | Used (`ReconnectPolicy`) | Both |
| `default_ttl`, `default_hash_ttl`, `stream_read_count` | Used | Used | Both |
| `auto_pipeline` | Ignored (ConnectionManager auto-pipelines) | Used (`PerformanceConfig`) | fred-rs only |
| `disable_auto_backpressure` | Ignored | Used (`BackpressureConfig`) | fred-rs only |
| `max_in_flight_commands` | Used (`set_pipeline_buffer_size`) | Used (`BackpressureConfig`) | Both |
| `default_command_timeout` | Used (`set_response_timeout`) | Used (`PerformanceConfig`) | Both |
| `max_feed_count` | Ignored | Used (`PerformanceConfig`) | fred-rs only |
| `broadcast_channel_capacity` | Used (broadcast channel size) | Used (broadcast channel size) | Both |
| `unresponsive_timeout` | Used (PING timeout) | Used (`ConnectionConfig::unresponsive`) | Both |
| `unresponsive_check_interval` | Used (PING interval) | Used (`ConnectionConfig::unresponsive`) | Both |
| `max_failure_threshold` | Used (shutdown threshold) | Ignored | redis-rs only |

---

## Usage

```bash
# Default (redis-rs)
cargo build
cargo test -p redis_interface

# Fred backend
cargo build --no-default-features --features fred-rs
cargo test -p redis_interface --no-default-features --features fred-rs

# Verify mutual exclusion (should fail with clear error)
cargo check -p redis_interface --features "redis-rs,fred-rs"
```