# Refactoring Guide: Moving Database Interfaces in Hyperswitch

This guide outlines the general steps and considerations for refactoring database interaction traits and their implementations from the `router` crate to `hyperswitch_domain_models` (for the trait definition) and `storage_impl` (for store-specific implementations). This pattern helps in decoupling database logic and improving code organization.

## 1. Objective

The primary goal is to separate the definition of a database interaction interface (trait) from its concrete implementations.
- The **trait** (defining *what* operations are possible) moves to `hyperswitch_domain_models`.
- The **implementations** (defining *how* these operations are done for different stores) move to `storage_impl`.
- Implementations specific to `router` utilities (like `KafkaStore`) remain in `router` but use the new trait.

## 2. Core Idea

-   **`hyperswitch_domain_models`**: Hosts the abstract interface (trait). This crate should not depend on `storage_impl` or `router` specific details beyond what's necessary for type definitions (often `diesel_models` or domain-specific types).
-   **`storage_impl`**: Provides implementations for various storage backends/wrappers (`MockDb`, `RouterStore`, `KVRouterStore`).
-   **`router`**: Consumes the interface and may provide implementations for its own specific store types (e.g., `KafkaStore`).

## 3. Detailed Steps

Let's assume we are refactoring `MyInterface` from `crates/router/src/db/my_interface.rs`.

### Step 1: Identify the Trait and Implementations

-   Locate the trait definition (e.g., `MyInterface`).
-   Identify all structs that implement this trait (e.g., `Store`, `MockDb`, `KafkaStore` in the `router` crate). Note that `Store` in `router` is often an alias for `KVRouterStore<DieselStore>`.

### Step 2: Relocate the Trait to `hyperswitch_domain_models`

1.  **Create Trait File**:
    *   Create `crates/hyperswitch_domain_models/src/db/my_interface.rs`.
2.  **Define Trait**:
    *   Move the trait definition into this new file.
    *   Update type paths. Method signatures will often use types from:
        *   `diesel_models` (e.g., `diesel_models::my_model::MyModelNew`).
        *   `common_utils` (e.g., `common_utils::errors::CustomResult`, `common_utils::id_type::MerchantId`).
        *   Types defined within `hyperswitch_domain_models` itself.
        *   `hyperswitch_interfaces::errors` for `StorageError` type in `CustomResult`.
    *   Example:
        ```rust
        // crates/hyperswitch_domain_models/src/db/my_interface.rs
        use common_utils::errors::CustomResult;
        use diesel_models::some_model; // Assuming types are from diesel_models
        // hyperswitch_interfaces::errors might not be needed here if Self::Error is used abstractly

        #[async_trait::async_trait]
        pub trait MyInterface {
            type Error; // Associated type for error

            async fn operation_one(&self, param: some_model::ParamType) -> CustomResult<some_model::ReturnType, Self::Error>;
            // ... other methods
        }
        ```
3.  **Make Module Public**:
    *   In `crates/hyperswitch_domain_models/src/db.rs` (or `src/db/mod.rs` if it exists):
        ```rust
        pub mod my_interface;
        // ... other db modules
        ```
    *   Ensure `pub mod db;` is present in `crates/hyperswitch_domain_models/src/lib.rs`.

### Step 3: Relocate Implementations to `storage_impl`

1.  **Create Implementation File**:
    *   Create `crates/storage_impl/src/my_interface.rs`.
2.  **Implement for Store Types**:
    *   Move or recreate implementations for `MockDb`, `RouterStore<T: DatabaseStore + Sync>`, and `KVRouterStore<T: DatabaseStore + Sync>` into this file.
    *   **Import necessary items**:
        ```rust
        // crates/storage_impl/src/my_interface.rs
        use common_utils::errors::CustomResult;
        use diesel_models::some_model;
        use error_stack::report;
        use hyperswitch_domain_models::db::my_interface::MyInterface; // The new trait path
        use router_env::{instrument, tracing}; // For logging/tracing macros

        use crate::{ // Relative to storage_impl
            connection, // For RouterStore to get DB connections
            errors, // storage_impl's error_handling
            kv_router_store::KVRouterStore,
            mock_db::MockDb,
            DatabaseStore, // Trait for RouterStore generic constraint
            RouterStore,
        };

        // Note: DieselStore itself does not directly implement MyInterface.
        // The core DB logic resides in the RouterStore<T> impl where T is typically DieselStore.
        ```
    *   **`MockDb` Implementation**:
        *   Define `type Error = crate::errors::StorageError;`.
        *   Return `CustomResult<..., Self::Error>`.
        *   Usually returns `Err(report!(Self::Error::MockDbError).attach_printable("...not implemented"))`.
    *   **`RouterStore<T: DatabaseStore + Sync>` Implementation**:
        *   Define `type Error = crate::errors::StorageError;`.
        *   Return `CustomResult<..., Self::Error>`.
        *   This implementation contains the core database logic.
        *   It uses `crate::connection::pg_connection_write(self).await?` or `pg_connection_read(self).await?` to obtain a DB connection. `self` here is `&RouterStore<T>`.
        *   Example:
            ```rust
            #[async_trait::async_trait]
            impl<T: DatabaseStore + Sync> MyInterface for RouterStore<T> {
                type Error = crate::errors::StorageError;

                async fn operation_one(&self, param: some_model::ParamType) -> CustomResult<some_model::ReturnType, Self::Error> {
                    let conn = crate::connection::pg_connection_write(self).await?;
                    // ... logic using conn and diesel_models ...
                    // e.g., some_model::SomeDieselStruct::insert(&conn)...
                    // map_err(|e| report!(errors::StorageError::from(e)))
                }
            }
            ```
    *   **`KVRouterStore<T: DatabaseStore + Sync>` Implementation**:
        *   Define `type Error = crate::errors::StorageError;`.
        *   Return `CustomResult<..., Self::Error>`.
        *   Delegates calls to `self.router_store` (which is a `RouterStore<T>`).
        *   Example: `self.router_store.operation_one(param).await`
3.  **Make Module Public**:
    *   In `crates/storage_impl/src/lib.rs`:
        ```rust
        pub mod my_interface;
        // ... other storage_impl modules
        ```

### Step 4: Update the Original Crate (e.g., `router`)

1.  **Clean Up Original File** (`crates/router/src/db/my_interface.rs`):
    *   Remove the `MyInterface` trait definition.
    *   Remove the implementations for `Store` and `MockDb`.
2.  **Update Remaining Implementations** (e.g., for `KafkaStore`):
    *   Import the trait from its new location: `use hyperswitch_domain_models::db::my_interface::MyInterface;`.
    *   Ensure method signatures in the `impl MyInterface for KafkaStore` block match the trait definition.
    *   Define `type Error = crate::core::errors::StorageError;` within the impl block.
    *   Return `CustomResult<..., Self::Error>`.
    *   The `KafkaStore` implementation typically delegates to its `diesel_store` field (which is a `RouterStore<DieselStore>`).
    *   Example:
        ```rust
        // crates/router/src/db/my_interface.rs
        use diesel_models::some_model; // For types in KafkaStore impl
        use hyperswitch_domain_models::db::my_interface::MyInterface;
        use router_env::{instrument, tracing};

        use crate::{
            core::errors::{self, CustomResult}, // router's errors
            db::kafka_store::KafkaStore,
        };

        #[async_trait::async_trait]
        impl MyInterface for KafkaStore {
            type Error = errors::StorageError; // errors refers to crate::core::errors

            async fn operation_one(&self, param: some_model::ParamType) -> CustomResult<some_model::ReturnType, Self::Error> {
                self.diesel_store.operation_one(param).await
            }
            // ...
        }
        ```
3.  **Adjust Imports**: Remove unused imports and add new ones as needed. If the trait needs to be accessible via the current module's path (e.g., `crate_name::db::module_name::MyInterface`), make the import public: `pub use hyperswitch_domain_models::db::my_interface::MyInterface;`.

## 4. Key Considerations

-   **Type Consistency**: Ensure that the types used in the trait definition in `hyperswitch_domain_models` are consistently used in all implementations across `storage_impl` and `router`. This often means using `diesel_models` structs directly in method signatures.
-   **Error Handling**: The trait defines `type Error;`. Implementers define this as their specific `StorageError` type (e.g., `crate::errors::StorageError` in `storage_impl`, `crate::core::errors::StorageError` in `router`). Return types become `CustomResult<T, Self::Error>`.
-   **Store Hierarchy & Logic**:
    -   `DieselStore`: Provides basic DB connection capabilities (e.g., `get_conn()`). It does *not* directly implement the high-level domain interface.
    -   `RouterStore<T: DatabaseStore + Sync>`: This is where the core database interaction logic for the interface methods resides (when `T` is `DieselStore`). It uses `crate::connection::pg_connection_xxx(self)` to get connections.
    -   `KVRouterStore<T: DatabaseStore + Sync>`: Wraps a `RouterStore<T>`, adds KV caching. Implements interfaces by delegating to `self.router_store`.
    -   `MockDb`: Implements the interface, typically returning mock errors or data.
    -   `KafkaStore`: Wraps a `RouterStore<DieselStore>` (via `diesel_store` field), implements the interface by delegating.
-   **Module System**: Correctly declare `pub mod` in the respective `lib.rs` and `db.rs` (or `db/mod.rs`) files to make the new modules and their contents accessible.
-   **Aggregate Traits**: If your refactored interface (e.g., `MyInterface`) is included as a supertrait in a larger aggregate trait (e.g., `StorageInterface` in `router::db`), you must specify the concrete error type there as well. For example:
    ```rust
    // In router/src/db.rs, if StorageInterface aggregates MyInterface
    use storage_impl::errors::StorageError; // Assuming this is the concrete error for implementers
    // ... other imports ...
    use crate::db::my_interface::MyInterface; // Path to the re-exported MyInterface

    pub trait StorageInterface:
        // ... other traits ...
        + MyInterface<Error = StorageError> // Specify the concrete error type
        // ... other traits ...
    {
        // ...
    }
    ```
    This ensures that implementers of `StorageInterface` (like `Store` and `MockDb` in `router::db`) satisfy `MyInterface` with the expected error type. The `StorageError` here would typically be `storage_impl::errors::StorageError` as `Store` (which is `KVRouterStore<DieselStore>`) implements `MyInterface` with that error type.
-   **Incremental Compilation**: After each major step (e.g., moving the trait, moving one implementation), try to compile to catch errors early.

## 5. Example File Structure Change

**Before**:
```
hyperswitch/
├── crates/
│   ├── router/
│   │   ├── src/
│   │   │   ├── db/
│   │   │   │   ├── my_interface.rs (contains trait MyInterface and impls for Store, MockDb, KafkaStore)
│   │   │   │   └── ...
│   │   │   └── lib.rs
...
```

**After**:
```
hyperswitch/
├── crates/
│   ├── hyperswitch_domain_models/
│   │   ├── src/
│   │   │   ├── db/
│   │   │   │   ├── my_interface.rs (contains trait MyInterface)
│   │   │   │   └── mod.rs or db.rs (declares pub mod my_interface)
│   │   │   └── lib.rs (declares pub mod db)
│   ├── storage_impl/
│   │   ├── src/
│   │   │   ├── my_interface.rs (contains impls for MockDb, RouterStore, KVRouterStore)
│   │   │   └── lib.rs (declares pub mod my_interface)
│   ├── router/
│   │   ├── src/
│   │   │   ├── db/
│   │   │   │   ├── my_interface.rs (contains impl for KafkaStore, imports trait)
│   │   │   │   └── ...
│   │   │   └── lib.rs
...
```

By following these steps and considerations, refactoring database interfaces can be done systematically, leading to a cleaner and more maintainable codebase.
