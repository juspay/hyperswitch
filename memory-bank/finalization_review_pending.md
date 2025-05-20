# Memory Bank Finalization - Pending Items & Clarifications

This document tracks pending items, suggestions, and points requiring clarification identified during the review and finalization of the Hyperswitch Memory Bank.

## Core Documents

### 1. `projectbrief.md`
-   **Suggestion**: Under "Links to Related Resources," the comment `*(Consider adding direct links to Android and iOS SDK repositories if they are separate and public, e.g., github.com/juspay/hyperswitch-sdk-android)*` is still present.
    -   **Action**: Either add these links if they exist and are public, or remove the comment.

### 2. `productContext.md`
-   **Status**: Appears comprehensive and up-to-date. No specific pending items noted during this review.

### 3. `systemPatterns.md`
-   **Minor Point (Mermaid Diagram - Locker)**: Text clarifies Locker's internal nature well. Diagram depiction is acceptable but could be reinforced if desired. (Low priority)
-   **Suggestion (Connector Trait)**: Consider linking `trait Connector` to its actual definition or a more specific thematic document. (Low priority)
-   **Action**: Ensure all "Links to Detailed Documentation" at the end are valid and point to existing/planned documents.

### 4. `techContext.md`
-   **Verification**: Confirm the exact and current name of the Helm chart repository/directory (mentioned as `hyperswitch-helm`).
-   **Verification (Crate List)**: Ensure the crate list under "Workspace Organization" is reasonably aligned with the actual `crates/` directory for an overview. (The "Key Crates" section helps focus).
-   **Action**: Ensure all "Links to Detailed Documentation" at the end are valid.

### 5. `crateIndex.md`
-   **Action (Completeness)**: Cross-reference the crate list with the actual `crates/` directory. Add any missing crates.
-   **Action (TBD Sections)**: Fill in "TBD" sections for Purpose, Key Components, Links, and Dependencies for crates like `config_importer`, `connector_configs`, `hsdev`, `euclid_macros`, `euclid_wasm`, `hyperswitch_constraint_graph`.
-   **Action (Detailed Documentation Links)**: For every crate, ensure an `overview.md` exists (or is created) in `memory-bank/thematic/crates/<crate_name>/overview.md` and that `crateIndex.md` links to it. This is a major pending item.
    -   Specifically note missing links for: `common_types`, `pm_auth`, `currency_conversion`, `events`, `external_services`, `hyperswitch_interfaces`, `openapi`, `test_utils`, `analytics`, `euclid`, `kgraph_utils`.
    -   ✓ Completed documentation for: `hyperswitch_domain_models` (2025-05-20), `common_enums` (2025-05-20), `router_derive` (2025-05-20), `cards` (2025-05-20), `payment_methods` (2025-05-20)
-   **Stylistic Choice (Dependencies)**: Decide on the level of detail for listing dependencies (exhaustive vs. key internal).
-   **Suggestion (Mermaid Diagram)**: Review and potentially enhance the dependency graph for clarity and completeness of key relationships.

## Thematic Crate Overviews (Pending Review / Points from Reviewed Files)

### 1. `router/overview.md`
-   **Clarification (DB Module)**: Confirm current status of migration of logic from `router/src/db.rs` to `storage_impl`.
-   **Clarification (`lib.rs`)**: Confirm if `router` crate is intended/commonly used as a library. If not, the mention of `lib.rs` might be slightly confusing.
-   **Verification (Feature Flags)**: Ensure example feature flags are still representative.

### 2. `scheduler/overview.md`
-   **Action (Metadata)**: Update "Last Reviewed" date to `2025-05-20`. Fill or remove `[List maintainers if known]`.
-   **Clarification (Consumer Scaling)**: Confirm current scaling capabilities of the consumer component.

### 3. `hyperswitch_connectors/overview.md`
-   **Verification (Supported Connectors)**: Verify accuracy and up-to-dateness of the supported connectors list.
-   **Clarification (Alternative Payment Methods)**: Clarify if "Digital Wallets", "Bank Transfers", etc., refer to method types enabled by various connectors or specific, dedicated connectors.
-   **Suggestion (Module Structure Example)**: Clarify common naming patterns for main connector logic files within their modules.
-   **Clarification (Connector Trait Location)**: Explicitly state that the main `Connector` trait is defined in `hyperswitch_connectors/src/traits.rs`.
-   **Suggestion (Connector Registry)**: Briefly mention where the connector registry is located or how it's implemented.
-   **Clarification ("Common Utilities")**: Distinguish between utilities local to `hyperswitch_connectors` (in `src/utils/`) and those consumed from the `common_utils` crate.

---

### 4. `diesel_models/overview.md`
-   **Verification (Schema Location)**: Confirm precise location of `schema.rs` (e.g., `src/schema.rs`).
-   **Verification (`query_utils.rs` / `src/query/`)**: Confirm existence and role of `query_utils.rs` or a `src/query/` directory for query helpers.
-   **Verification (Key Entities List)**: Cross-check key entities list against actual schema for major omissions or naming inconsistencies.
-   **Clarification ("Config" Entity)**: Confirm if general application config is stored in the DB and modeled here.
-   **Clarification ("User" Entity)**: Confirm if a `User` model (distinct from `Customer`) exists.

---

### 5. `api_models/overview.md`
-   **Clarification ("Config", "User" Entities)**: Similar to `diesel_models`, confirm the nature of `Config` and `User` models listed under "Key Resources" for consistency.
-   **Clarification (`enums/` directory)**: Verify the role of `api_models/src/enums/` if it exists, especially in relation to the `common_enums` crate.
-   **Note (Security/Performance)**: Points like "Supports rate limiting headers" are good; actual enforcement logic is understood to be elsewhere (e.g., `router`). (No action, just an observation for overall consistency).

---

### 6. `storage_impl/overview.md`
-   **Clarification (Migrations Directory)**: Reconcile the location of migration scripts. The overview's code structure suggests `storage_impl/src/migrations/`, while project standard is likely top-level `/migrations`. Clarify if `storage_impl` *contains* scripts or just *interacts* with the migration process.
-   **Clarification (`redis/` Module)**: Confirm if a dedicated `src/redis/` module exists within `storage_impl` for Redis-specific logic, or if these interactions are embedded elsewhere (e.g., `DatabaseStore`).

---

## Overall Finalization Status & Next Steps (as of 2025-05-20)

**Review Progress:**
- All core Memory Bank files (`projectbrief.md`, `productContext.md`, `activeContext.md`, `systemPatterns.md`, `techContext.md`, `progress.md`) have been reviewed.
- `crateIndex.md` has been reviewed.
- The 11 key thematic crate overviews listed in `progress.md` (`router`, `scheduler`, `hyperswitch_connectors`, `diesel_models`, `api_models`, `storage_impl`, `redis_interface`, `common_utils`, `router_env`, `drainer`, `masking`) have been reviewed.

**Major Pending Tasks for Finalization:**

1.  **Address All Specific Points**: Implement suggestions and clarifications listed above for each reviewed document.
2.  **`crateIndex.md` Completion**:
    *   Verify against `crates/` directory and add any missing crates.
    *   Complete all "TBD" sections (Purpose, Key Components, Links, Dependencies).
    *   **Create Missing Crate Overviews**: This is the most significant task. For every crate listed in `crateIndex.md` that currently has `[Detailed Documentation - MISSING]`, an `overview.md` file needs to be created in the `memory-bank/thematic/crates/<crate_name>/` directory and linked from `crateIndex.md`.
3.  **Structural Review**: Assess if any new thematic subfolders are needed or if any content should be moved to the `archive`.
4.  **Maintenance Cadence**: Define and document a process for regular review and updates to the Memory Bank post-finalization.

---

This list will be updated as the review progresses.
