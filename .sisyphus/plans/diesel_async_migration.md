# Migration Plan: async-bb8-diesel to diesel_async

## Overview

Migrate Hyperswitch's database layer from `async-bb8-diesel 0.2.1` (which wraps sync Diesel in `spawn_blocking`) to `diesel_async 0.9.x` (native async Diesel). Stay with **bb8** as the pool backend to minimize config/feature changes.

### Target Versions
- `diesel-async = { version = "0.9", features = ["postgres", "bb8"] }`
- `bb8 = "0.9"` (upgrade from 0.8 — diesel_async requires 0.9)
- `diesel = "2.2.10"` (unchanged)

### Migration Phases (dependency order)
1. **Phase 1**: `diesel_models` (leaf dependency — no internal deps)
2. **Phase 2**: `storage_impl` (depends on diesel_models)
3. **Phase 3**: `router` (depends on storage_impl + diesel_models)
4. **Phase 4**: `drainer` (depends on diesel_models)
5. **Phase 5**: Verification (`cargo check --workspace`)

---

## API Mapping Reference

| async-bb8-diesel | diesel_async |
|---|---|
| `async_bb8_diesel::Connection<PgConnection>` | `diesel_async::AsyncPgConnection` |
| `async_bb8_diesel::ConnectionManager<PgConnection>` | `diesel_async::pooled_connection::AsyncDieselConnectionManager<AsyncPgConnection>` |
| `bb8::Pool<ConnectionManager<PgConnection>>` | `diesel_async::pooled_connection::bb8::Pool<AsyncPgConnection>` |
| `bb8::PooledConnection<'_, ConnectionManager<PgConnection>>` | `diesel_async::pooled_connection::bb8::PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>` |
| `AsyncRunQueryDsl` (trait) | `diesel_async::RunQueryDsl` (trait) |
| `AsyncConnection` (trait) | `diesel_async::AsyncConnection` (trait) |
| `.execute_async(conn)` | `.execute(conn)` |
| `.load_async::<T>(conn)` | `.load::<T>(conn)` |
| `.get_result_async::<T>(conn)` | `.get_result::<T>(conn)` |
| `.get_results_async::<T>(conn)` | `.get_results::<T>(conn)` |
| `.first_async::<T>(conn)` | `.first::<T>(conn)` |
| `conn.transaction_async(\|conn\| async move { ... })` | `conn.transaction(async \|conn\| { ... }).await` |
| `conn.run(\|sync_conn\| ...)` | Not needed — connection is natively async |
| `Connection::as_async_conn(&conn)` | Not needed — pooled conn derefs to `&mut AsyncPgConnection` |
| `&PgPooledConn` (immutable ref to conn) | `&mut PgPooledConn` (mutable ref — diesel_async requires `&mut Conn`) |

### Critical Difference: `&` -> `&mut`
diesel_async's `RunQueryDsl` methods all take `&'conn mut Conn`. Every function signature currently taking `conn: &PgPooledConn` must change to `conn: &mut PgPooledConn`. At call sites, `let conn = ...` becomes `let mut conn = ...` and `&conn` becomes `&mut conn`.

### Import Strategy
```rust
// Keep diesel prelude for query building DSL
use diesel::prelude::*;
// Explicit diesel_async imports shadow diesel's sync RunQueryDsl
use diesel_async::{RunQueryDsl, AsyncConnection, AsyncPgConnection};
```

---

## Phase 1: diesel_models (Foundation Crate)

### 1.1 Cargo.toml

**File**: `crates/diesel_models/Cargo.toml`

```toml
# REMOVE:
async-bb8-diesel = "0.2.1"

# ADD:
diesel-async = { version = "0.9", features = ["postgres", "bb8"] }
```

Note: `diesel_models` does NOT directly depend on `bb8` — it only uses the connection type. The `bb8` feature in `diesel-async` enables the bb8 pool integration types.

### 1.2 Type Alias: `src/lib.rs`

**File**: `crates/diesel_models/src/lib.rs` (line 73)

```rust
// BEFORE:
pub type PgPooledConn = async_bb8_diesel::Connection<diesel::PgConnection>;

// AFTER:
pub type PgPooledConn = diesel_async::AsyncPgConnection;
```

### 1.3 Generic Query Helpers: `src/query/generics.rs`

**File**: `crates/diesel_models/src/query/generics.rs`

This is the central hub — all generic CRUD helpers. Changes:

#### Import change (line 3):
```rust
// BEFORE:
use async_bb8_diesel::AsyncRunQueryDsl;

// AFTER:
use diesel_async::RunQueryDsl;
```

#### Connection parameter: `&PgPooledConn` -> `&mut PgPooledConn`
ALL functions in this file take `conn: &PgPooledConn`. Change every occurrence to `conn: &mut PgPooledConn`.

Functions to update (non-exhaustive list — update ALL):
- `generic_insert` (line 66)
- `generic_update` (line 95)
- `generic_update_with_results` (line 121)
- `generic_update_with_unique_predicate_get_result` (line 169)
- `generic_update_by_id` (line 205)
- `generic_delete` (line 256)
- `generic_delete_one_with_result` (line 284)
- `generic_find_by_id_core` (line 315)
- `generic_find_by_id` (line 339)
- `generic_find_by_id_optional` (line 350)
- `generic_find_one_core` (line 365)
- `generic_find_one` (line 383)
- `generic_find_one_optional` (line 392)
- `generic_filter` (line 404)
- `generic_count` (line 448)

#### Method name changes:
```
get_result_async(conn)  ->  get_result(conn)
execute_async(conn)     ->  execute(conn)
get_results_async(conn) ->  get_results(conn)
first_async(conn)       ->  first(conn)
```

#### Trait bound changes:
The `LoadQuery<'static, PgConnection, R>` bounds need to change. `diesel_async::RunQueryDsl` methods don't use `LoadQuery` — they use individual bounds on `AsQuery`, `QueryFragment<Pg>`, and `Queryable`.

For each function with `LoadQuery<'static, PgConnection, R>`:
```rust
// BEFORE:
InsertStatement<T, <V as Insertable<T>>::Values>:
    AsQuery + LoadQuery<'static, PgConnection, R> + Send,

// AFTER (approximate — exact bounds may need adjustment during implementation):
InsertStatement<T, <V as Insertable<T>>::Values>:
    AsQuery + Send + 'static,
<InsertStatement<T, <V as Insertable<T>>::Values> as AsQuery>::Query:
    QueryFragment<Pg> + Send + 'static,
R: diesel::deserialize::Queryable<
    <InsertStatement<T, <V as Insertable<T>>::Values> as AsQuery>::SqlType, Pg
> + Send + 'static,
```

**IMPORTANT**: The `PgConnection` import in the `use diesel::pg::{Pg, PgConnection}` statement can be removed if `PgConnection` is no longer referenced anywhere in the file after the trait bound changes. Check all `PgConnection` references.

#### `RunQueryDsl<PgConnection>` bounds:
Some functions have `RunQueryDsl<PgConnection>` bounds (e.g., line 213, 318, 342). These are diesel's sync trait. With diesel_async, these may need to change or be removed. Check if they're still needed.

### 1.4 KV Module: `src/kv.rs`

**File**: `crates/diesel_models/src/kv.rs`

**HIGH RISK** — This file has two `conn.run()` calls that bridge async to sync diesel. There is no direct diesel_async equivalent.

#### Import change (line 5):
```rust
// BEFORE:
use async_bb8_diesel::AsyncConnection;

// AFTER:
use diesel_async::AsyncConnection;
// May also need:
use diesel_async::RunQueryDsl;
```

#### `collect_binds` call (lines 95-101):
```rust
// BEFORE:
let bind_collector = conn
    .run(move |c| {
        let mut bc = RawBytesBindCollector::<Pg>::new();
        query.collect_binds(&mut bc, c, &Pg)?;
        Ok::<RawBytesBindCollector<Pg>, diesel::result::Error>(bc)
    })
    .await

// AFTER (Option A — if collect_binds works without a live connection):
// collect_binds only needs &Pg backend, not a PgConnection.
// Check if `query.collect_binds(&mut bc, &Pg)` works directly.
let mut bc = RawBytesBindCollector::<Pg>::new();
query.collect_binds(&mut bc, &Pg)
    .change_context(errors::DatabaseError::QueryGenerationFailed)
    .attach_printable("Failed to construct bind parameters")?;
// If the signature requires a connection reference, use Option B.

// AFTER (Option B — if a connection is truly needed, use spawn_blocking):
// Keep a sync PgConnection for bind collection, or use spawn_blocking.
```

**NOTE**: The exact `collect_binds` signature in their diesel version needs verification. If it takes `&mut PgConnection`, we may need `spawn_blocking` or a `SyncConnectionWrapper`. The implementation agent should check diesel 2.2.x docs for `QueryFragment::collect_binds`.

#### `ExecuteDsl::execute` call (lines 140-151):
```rust
// BEFORE:
pub async fn execute(self, conn: &mut crate::PgPooledConn) -> crate::StorageResult<usize> {
    let query = self.to_collected_query();
    conn.run(move |c| ExecuteDsl::execute(query, c))
        .await
        .attach_printable("Failed to execute drainer query")
        .switch()
}

// AFTER (approach TBD — implementation agent should investigate):
// Option A: If CollectedQuery implements diesel_async's RunQueryDsl:
//   query.execute(&mut conn).await
// Option B: Use AsyncPgConnection's raw SQL execution:
//   conn.execute_sql(&query.sql, &query.binds).await
// Option C: Use spawn_blocking with a sync PgConnection:
//   tokio::task::spawn_blocking(move || { ... })
```

**IMPORTANT**: This is the highest-risk area. The `CollectedQuery` type is a diesel-internal type that pre-builds SQL + bind values. It may not directly implement diesel_async's `RunQueryDsl`. The implementation agent must investigate how to execute a `CollectedQuery` on an `AsyncPgConnection`.

#### Connection parameter type:
Functions like `from_query`, `generate_insert_query`, `generate_update_query_by_id`, `generate_update_query_with_predicate` already take `conn: &mut crate::PgPooledConn`. With the new alias, `PgPooledConn = AsyncPgConnection`, so `&mut PgPooledConn` is `&mut AsyncPgConnection`. This should work directly.

### 1.5 Query Module Imports

All files in `crates/diesel_models/src/query/` that import `AsyncRunQueryDsl`:

**Files** (non-exhaustive — check ALL files in `src/query/`):
- `src/query/role.rs`
- `src/query/events.rs`
- `src/query/user_role.rs`
- `src/query/payout_attempt.rs`
- `src/query/blocklist.rs`
- `src/query/routing_algorithm.rs`
- `src/query/customers.rs`
- `src/query/payouts.rs`
- `src/query/payment_method.rs`
- `src/query/payment_attempt.rs`
- `src/query/user/sample_data.rs`
- `src/query/user/theme.rs`

**Change in each file**:
```rust
// BEFORE:
use async_bb8_diesel::AsyncRunQueryDsl;

// AFTER:
use diesel_async::RunQueryDsl;
```

Also check for any `*_async` method calls in these files and replace with unsuffixed versions.

### 1.6 Phase 1 Verification
```bash
cargo check -p diesel_models --all-features
```
Fix all compilation errors before proceeding to Phase 2.

---

## Phase 2: storage_impl (Middle Layer)

### 2.1 Cargo.toml

**File**: `crates/storage_impl/Cargo.toml`

```toml
# REMOVE:
async-bb8-diesel = "0.2.1"

# CHANGE:
bb8 = "0.8.6"  ->  bb8 = "0.9"

# ADD:
diesel-async = { version = "0.9", features = ["postgres", "bb8"] }
```

### 2.2 Connection Module: `src/connection.rs`

**File**: `crates/storage_impl/src/connection.rs`

#### Type aliases (lines 6-8):
```rust
// BEFORE:
pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

// AFTER:
use diesel_async::pooled_connection::{bb8::Pool, AsyncDieselConnectionManager};
use diesel_async::AsyncPgConnection;

pub type PgPool = Pool<AsyncPgConnection>;
pub type PgPooledConn = AsyncPgConnection;
```

#### Connection helper return types (lines 23-61):
```rust
// BEFORE:
pub async fn pg_connection_read<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    crate::errors::StorageError,
>

// AFTER:
pub async fn pg_connection_read<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    diesel_async::pooled_connection::bb8::PooledConnection<
        '_, AsyncDieselConnectionManager<AsyncPgConnection>
    >,
    crate::errors::StorageError,
>
```

Apply the same return type change to `pg_connection_write` (line 49).

Remove `use bb8::PooledConnection;` (line 1) and `use diesel::PgConnection;` (line 3) — no longer needed.

### 2.3 Database Store: `src/database/store.rs`

**File**: `crates/storage_impl/src/database/store.rs`

#### Imports (lines 1-2):
```rust
// BEFORE:
use async_bb8_diesel::{AsyncConnection, ConnectionError};
use bb8::CustomizeConnection;

// AFTER:
use diesel_async::pooled_connection::{bb8, AsyncDieselConnectionManager, PoolError};
use diesel_async::{AsyncConnection, AsyncPgConnection};
use bb8::CustomizeConnection;  // Keep — still bb8
```

#### Type aliases (lines 15-16):
```rust
// BEFORE:
pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

// AFTER:
pub type PgPool = diesel_async::pooled_connection::bb8::Pool<AsyncPgConnection>;
pub type PgPooledConn = AsyncPgConnection;
```

#### Pool creation `diesel_make_pg_pool` (lines 146-168):
```rust
// BEFORE:
let manager = async_bb8_diesel::ConnectionManager::<PgConnection>::new(database_url);
let mut pool = bb8::Pool::builder()
    .max_size(database.pool_size)
    .min_idle(database.min_idle)
    .queue_strategy(database.queue_strategy.into())
    .connection_timeout(std::time::Duration::from_secs(database.connection_timeout))
    .max_lifetime(database.max_lifetime.map(std::time::Duration::from_secs));
if test_transaction {
    pool = pool.connection_customizer(Box::new(TestTransaction));
}
pool.build(manager).await

// AFTER:
let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
let mut pool = bb8::Pool::builder()
    .max_size(database.pool_size)
    .min_idle(database.min_idle)
    .queue_strategy(database.queue_strategy.into())
    .connection_timeout(std::time::Duration::from_secs(database.connection_timeout))
    .max_lifetime(database.max_lifetime.map(std::time::Duration::from_secs));
if test_transaction {
    pool = pool.connection_customizer(Box::new(TestTransaction));
}
pool.build(manager).await
```

#### TestTransaction customizer (lines 170-185):
```rust
// BEFORE:
#[derive(Debug)]
struct TestTransaction;

#[async_trait::async_trait]
impl CustomizeConnection<PgPooledConn, ConnectionError> for TestTransaction {
    #[allow(clippy::unwrap_used)]
    async fn on_acquire(&self, conn: &mut PgPooledConn) -> Result<(), ConnectionError> {
        use diesel::Connection;
        conn.run(|conn| {
            conn.begin_test_transaction().unwrap();
            Ok(())
        })
        .await
    }
}

// AFTER:
#[derive(Debug)]
struct TestTransaction;

#[async_trait::async_trait]
impl CustomizeConnection<AsyncPgConnection, PoolError> for TestTransaction {
    #[allow(clippy::unwrap_used)]
    async fn on_acquire(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> Result<(), PoolError> {
        conn.begin_test_transaction()
            .await
            .map_err(PoolError::ConnectionError)?;
        Ok(())
    }
}
```

**NOTE**: Verify that `bb8 0.9`'s `CustomizeConnection` trait signature matches. The error type for `AsyncDieselConnectionManager` is `PoolError` (from diesel_async). Check if `begin_test_transaction()` returns `diesel::result::Error` and needs wrapping in `PoolError`.

### 2.4 Config: `src/config.rs`

**File**: `crates/storage_impl/src/config.rs`

The `QueueStrategy` enum and `From<QueueStrategy> for bb8::QueueStrategy` impl (lines 39-54) should remain unchanged — bb8 0.9 still has the same `QueueStrategy` enum.

Verify: `bb8::QueueStrategy` still exists in bb8 0.9 with `Fifo` and `Lifo` variants. If so, no changes needed.

### 2.5 Utils: `src/utils.rs`

**File**: `crates/storage_impl/src/utils.rs`

#### Import (line 1):
```rust
// BEFORE:
use bb8::PooledConnection;

// AFTER: Remove this line — use diesel_async's re-exported type
// Or keep as: use diesel_async::pooled_connection::bb8::PooledConnection;
```

#### Return types (lines 10-88):
Change all 4 helper functions (`pg_connection_read`, `pg_connection_write`, `pg_accounts_connection_read`, `pg_accounts_connection_write`):

```rust
// BEFORE:
) -> error_stack::Result<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    StorageError,
>

// AFTER:
) -> error_stack::Result<
    diesel_async::pooled_connection::bb8::PooledConnection<
        '_, AsyncDieselConnectionManager<AsyncPgConnection>
    >,
    StorageError,
>
```

Remove `use diesel::PgConnection;` (line 2) if no longer referenced.

### 2.6 Merchant Connector Account: `src/merchant_connector_account.rs`

**File**: `crates/storage_impl/src/merchant_connector_account.rs`

#### Import (line 1):
```rust
// BEFORE:
use async_bb8_diesel::AsyncConnection;

// AFTER:
use diesel_async::AsyncConnection;
```

#### Transaction (lines 630-710):
```rust
// BEFORE:
conn.transaction_async(|connection_pool| async move {
    for (merchant_connector_account, update_merchant_connector_account) in
        merchant_connector_accounts
    {
        ...
        let update = update_call(&connection_pool, ...);
        ...
        update.await.map_err(...)?;
        ...
    }
    Ok::<_, Self::Error>(())
})
.await?;

// AFTER (diesel_async 0.9.x with native async closures):
conn.transaction(async |connection_pool| {
    for (merchant_connector_account, update_merchant_connector_account) in
        merchant_connector_accounts
    {
        ...
        let update = update_call(&connection_pool, ...);
        ...
        update.await.map_err(...)?;
        ...
    }
    Ok::<_, Self::Error>(())
})
.await?;
```

**NOTE**: The `conn` variable needs to be `mut` — `let mut conn = ...` instead of `let conn = ...`. Check the variable declaration above this code block.

**NOTE**: diesel_async 0.9.x supports native `async |conn| { ... }` closures. If using an older version, you'd need `.boxed()` or `Box::pin(async move { ... })`. But 0.9.x should support native async closures.

### 2.7 Payments: `src/payments/payment_intent.rs`

**File**: `crates/storage_impl/src/payments/payment_intent.rs`

#### Import (line 4):
```rust
// BEFORE:
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};

// AFTER:
use diesel_async::{AsyncConnection, RunQueryDsl};
```

#### `as_async_conn` removal (lines 484-495, 927-928):
```rust
// BEFORE (line 484-489):
let conn: bb8::PooledConnection<
    '_,
    async_bb8_diesel::ConnectionManager<diesel::PgConnection>,
> = pg_connection_read(self).await?;

DieselPaymentIntent::find_by_global_id(&conn, id).await...

// AFTER:
let mut conn = pg_connection_read(self).await?;

DieselPaymentIntent::find_by_global_id(&mut conn, id).await...
```

```rust
// BEFORE (line 927-928):
let conn = connection::pg_connection_read(self).await?;
let conn = async_bb8_diesel::Connection::as_async_conn(&conn);

// AFTER:
let mut conn = connection::pg_connection_read(self).await?;
// Use &mut conn directly — no as_async_conn needed
```

### 2.8 Payouts: `src/payouts/payouts.rs`

**File**: `crates/storage_impl/src/payouts/payouts.rs`

#### Import (line 4):
```rust
// BEFORE:
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};

// AFTER:
use diesel_async::{AsyncConnection, RunQueryDsl};
```

#### `as_async_conn` removal (lines 490-491, 597-598):
```rust
// BEFORE (line 490-491):
let conn = connection::pg_connection_read(self).await?;
let conn = async_bb8_diesel::Connection::as_async_conn(&conn);

// AFTER:
let mut conn = connection::pg_connection_read(self).await?;
// Use &mut conn directly
```

Same pattern at line 597-598.

### 2.9 Other files in storage_impl

Check these files for any `async_bb8_diesel` or `bb8` references:
- `src/lib.rs` — re-exports `PgPool`, `DatabaseStore` impl
- `src/kv_router_store.rs` — `DatabaseStore` impl

These files reference `PgPool` from `store.rs`, which will now be the new type. No direct `async_bb8_diesel` imports expected, but verify.

### 2.10 Phase 2 Verification
```bash
cargo check -p storage_impl --all-features
cargo check -p diesel_models --all-features  # regression check
```

---

## Phase 3: router (Top Layer)

### 3.1 Cargo.toml

**File**: `crates/router/Cargo.toml`

```toml
# REMOVE:
async-bb8-diesel = "0.2.1"

# CHANGE:
bb8 = "0.8"  ->  bb8 = "0.9"

# ADD:
diesel-async = { version = "0.9", features = ["postgres", "bb8"] }
```

### 3.2 Connection Module: `src/connection.rs`

**File**: `crates/router/src/connection.rs`

#### Type aliases (lines 8-10):
```rust
// BEFORE:
pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;
pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

// AFTER:
pub type PgPool = diesel_async::pooled_connection::bb8::Pool<AsyncPgConnection>;
pub type PgPooledConn = AsyncPgConnection;
```

Remove `use diesel::PgConnection;` (line 2).

#### Helper return types (lines 25-103):
All 4 helper functions (`pg_connection_read`, `pg_accounts_connection_read`, `pg_connection_write`, `pg_accounts_connection_write`):

```rust
// BEFORE:
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    storage_errors::StorageError,
>

// AFTER:
) -> errors::CustomResult<
    diesel_async::pooled_connection::bb8::PooledConnection<
        '_, diesel_async::pooled_connection::AsyncDieselConnectionManager<
            diesel_async::AsyncPgConnection
        >
    >,
    storage_errors::StorageError,
>
```

Remove `use bb8::PooledConnection;` (line 1).

### 3.3 Health Check: `src/db/health_check.rs`

**File**: `crates/router/src/db/health_check.rs`

#### Import (line 1):
```rust
// BEFORE:
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};

// AFTER:
use diesel_async::{AsyncConnection, RunQueryDsl};
```

#### Transaction (lines 22-58):
```rust
// BEFORE:
let conn = connection::pg_connection_write(self)
    .await
    .change_context(errors::HealthCheckDBError::DBError)?;

conn.transaction_async(|conn| async move {
    let query = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 1"));
    let _x: i32 = query.get_result_async(&conn).await.map_err(|err| {
        ...
    })?;

    ...
    config.insert(&conn).await.map_err(...)?;
    storage::Config::delete_by_key(&conn, "test_key").await.map_err(...)?;
    Ok::<_, errors::HealthCheckDBError>(())
})
.await?;

// AFTER:
let mut conn = connection::pg_connection_write(self)
    .await
    .change_context(errors::HealthCheckDBError::DBError)?;

conn.transaction(async |conn| {
    let query = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 1"));
    let _x: i32 = query.get_result::<i32>(conn).await.map_err(|err| {
        ...
    })?;

    ...
    config.insert(conn).await.map_err(...)?;
    storage::Config::delete_by_key(conn, "test_key").await.map_err(...)?;
    Ok::<_, errors::HealthCheckDBError>(())
})
.await?;
```

Key changes:
1. `let conn` -> `let mut conn`
2. `transaction_async(|conn| async move { ... })` -> `transaction(async |conn| { ... })`
3. `get_result_async(&conn)` -> `get_result::<i32>(conn)` (pass `conn` not `&conn`)
4. `config.insert(&conn)` -> `config.insert(conn)` (pass `conn` not `&conn`)
5. `Config::delete_by_key(&conn, ...)` -> `Config::delete_by_key(conn, ...)`

### 3.4 Storage Extension Traits

**Files**:
- `src/types/storage/refund.rs`
- `src/types/storage/payment_link.rs`
- `src/types/storage/dispute.rs`
- `src/types/storage/mandate.rs`

Each file imports `use async_bb8_diesel::AsyncRunQueryDsl;`:

```rust
// BEFORE:
use async_bb8_diesel::AsyncRunQueryDsl;

// AFTER:
use diesel_async::RunQueryDsl;
```

Also check for `*_async` method calls and replace with unsuffixed versions. And check if function signatures take `&PgPooledConn` and change to `&mut PgPooledConn`.

### 3.5 Other files

Search the entire `router` crate for any remaining:
- `async_bb8_diesel` imports
- `bb8::Pool` or `bb8::PooledConnection` with old type params
- `*_async` method calls on diesel queries
- `transaction_async` calls
- `as_async_conn` calls
- `Connection::as_async_conn` calls

### 3.6 Phase 3 Verification
```bash
cargo check -p router --all-features
```

---

## Phase 4: drainer

### 4.1 Cargo.toml

**File**: `crates/drainer/Cargo.toml`

```toml
# REMOVE:
async-bb8-diesel = "0.2.1"

# CHANGE:
bb8 = "0.8"  ->  bb8 = "0.9"

# ADD:
diesel-async = { version = "0.9", features = ["postgres", "bb8"] }
```

### 4.2 Connection Module: `src/connection.rs`

**File**: `crates/drainer/src/connection.rs`

#### Type alias (line 7):
```rust
// BEFORE:
pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

// AFTER:
pub type PgPool = diesel_async::pooled_connection::bb8::Pool<AsyncPgConnection>;
```

#### Pool creation `diesel_make_pg_pool` (lines 21-35):
```rust
// BEFORE:
let manager = async_bb8_diesel::ConnectionManager::<PgConnection>::new(database_url);
let pool = bb8::Pool::builder()
    .max_size(database.pool_size)
    .connection_timeout(std::time::Duration::from_secs(database.connection_timeout));
pool.build(manager).await

// AFTER:
let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
let pool = bb8::Pool::builder()
    .max_size(database.pool_size)
    .connection_timeout(std::time::Duration::from_secs(database.connection_timeout));
pool.build(manager).await
```

#### `pg_connection` helper (lines 37-44):
```rust
// BEFORE:
pub async fn pg_connection(
    pool: &PgPool,
) -> PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>> {
    pool.get()
        .await
        .expect("Couldn't retrieve PostgreSQL connection")
}

// AFTER:
pub async fn pg_connection(
    pool: &PgPool,
) -> diesel_async::pooled_connection::bb8::PooledConnection<
    '_, AsyncDieselConnectionManager<AsyncPgConnection>,
> {
    pool.get()
        .await
        .expect("Couldn't retrieve PostgreSQL connection")
}
```

Remove `use bb8::PooledConnection;` (line 1) and `use diesel::PgConnection;` (line 3).

### 4.3 Health Check: `src/health_check.rs`

**File**: `crates/drainer/src/health_check.rs`

#### Import (line 4):
```rust
// BEFORE:
use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};

// AFTER:
use diesel_async::{AsyncConnection, RunQueryDsl};
```

#### Transaction (lines 119-157):
```rust
// BEFORE:
let conn = pg_connection(&self.master_pool).await;

conn
    .transaction_async(|conn| {
        Box::pin(async move {
            let query = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 1"));
            let _x: i32 = query.get_result_async(&conn).await.map_err(|err| {
                ...
            })?;

            ...
            config.insert(&conn).await.map_err(...)?;
            Config::delete_by_key(&conn, "test_key").await.map_err(...)?;
            Ok::<_, HealthCheckDBError>(())
        })
    })
    .await?;

// AFTER:
let mut conn = pg_connection(&self.master_pool).await;

conn
    .transaction(async |conn| {
        let query = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 1"));
        let _x: i32 = query.get_result::<i32>(conn).await.map_err(|err| {
            ...
        })?;

        ...
        config.insert(conn).await.map_err(...)?;
        Config::delete_by_key(conn, "test_key").await.map_err(...)?;
        Ok::<_, HealthCheckDBError>(())
    })
    .await?;
```

Key changes:
1. `let conn` -> `let mut conn`
2. `Box::pin(async move { ... })` no longer needed — native async closure
3. `get_result_async(&conn)` -> `get_result::<i32>(conn)`
4. `&conn` -> `conn` in model calls

### 4.4 Services: `src/services.rs`

**File**: `crates/drainer/src/services.rs`

Check for `PgPool` field type — it uses the alias from `connection.rs`, which will now be the new type. No direct `async_bb8_diesel` import expected, but verify.

### 4.5 Phase 4 Verification
```bash
cargo check -p drainer --all-features
```

---

## Phase 5: Full Workspace Verification

```bash
# Full workspace check
cargo check --workspace --all-features

# Run any DB-related tests
cargo test -p diesel_models
cargo test -p storage_impl
cargo test -p drainer

# Clippy
cargo clippy --workspace --all-features
```

---

## Risk Areas

### 1. `kv.rs` `conn.run()` calls (HIGHEST RISK)
The two `conn.run()` calls in `diesel_models/src/kv.rs` bridge to sync diesel. diesel_async has no sync bridge. Options:
- **`collect_binds`**: May not need a connection at all — just `&Pg` backend. Verify the diesel 2.2.x API.
- **`ExecuteDsl::execute(query, c)`**: The `CollectedQuery` type may not implement diesel_async's `RunQueryDsl`. May need raw SQL execution via `AsyncPgConnection` or `spawn_blocking`.

### 2. Trait bound changes in `generics.rs` (HIGH RISK)
`LoadQuery<'static, PgConnection, R>` bounds need to change to diesel_async-compatible bounds. The exact bounds depend on diesel_async's `RunQueryDsl` implementation. Getting these wrong will cause cascading compilation errors across all query modules.

### 3. `&PgPooledConn` -> `&mut PgPooledConn` (MEDIUM RISK, HIGH VOLUME)
Every function taking `&PgPooledConn` needs `&mut PgPooledConn`. This is mechanical but touches hundreds of call sites across the codebase. Missing one causes a compiler error, so `cargo check` will catch them.

### 4. bb8 0.8 -> 0.9 upgrade (LOW RISK)
The `QueueStrategy` enum and `CustomizeConnection` trait should be compatible. Verify `CustomizeConnection::on_acquire` signature hasn't changed.

### 5. Transaction closure syntax (MEDIUM RISK)
diesel_async 0.9.x uses native `async |conn| { ... }` closures. Verify the exact syntax — older versions required `Box::pin(async move { ... })` or `.scoped_boxed()`. The `merchant_connector_account.rs` transaction has complex control flow inside the closure — ensure it compiles with the new syntax.

### 6. `PgConnection` references in trait bounds
After removing `async_bb8_diesel`, check if `diesel::PgConnection` is still referenced anywhere in trait bounds or type parameters. It should be replaced with `AsyncPgConnection` where it refers to the async connection type. However, `Pg` (the backend type) should remain — it's used in `QueryFragment<Pg>`, `Queryable<..., Pg>`, etc.

---

## Execution Strategy

### Parallelization
- **Phase 1** (diesel_models) MUST complete first — all other crates depend on it.
- **Phase 2** (storage_impl) depends on Phase 1.
- **Phase 3** (router) and **Phase 4** (drainer) both depend on Phase 2 and can run in PARALLEL.
- **Phase 5** (verification) runs after all phases.

### Delegation
- Phase 1: One agent (foundational, high-risk — needs careful attention)
- Phase 2: One agent (depends on Phase 1 results)
- Phase 3 + Phase 4: Two agents in parallel
- Phase 5: Orchestrator verifies
