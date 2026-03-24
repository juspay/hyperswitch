# Work Plan: Fix /payouts/profile/list total_count Bug (REGENERATED)

## TL;DR

> **Quick Summary**: The `/payouts/profile/list` endpoint returns incorrect `total_count` because the COUNT query does not apply `profile_id` filtering while the SELECT query does. Two bugs exist: `payouts_list_core` hardcodes `total_count: None`, and `payouts_filtered_list_core` calls a count function without passing profile_id.
> 
> **Deliverables**:
> - Updated `PayoutsInterface` trait with `profile_id_list` parameter
> - Fixed Diesel count query with profile_id filtering
> - Fixed `payouts_list_core` to return actual count instead of None
> - Updated all storage implementations (RouterStore, KVRouterStore, KafkaStore, MockDb)
> - Updated core logic call sites to pass profile_id_list
> - Comprehensive test coverage
> 
> **Estimated Effort**: Medium (5-7 hours with testing)
> **Parallel Execution**: YES - 3 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 6
> **Scope**: Fix BOTH endpoints (`/payouts/profile/list` and `/payouts/list`)
> **Git Strategy**: Feature branch from main, local commits only (no push)
> **Testing**: Includes Rust unit/integration tests + Cypress E2E tests

---

## Context

### Original Request
Fix bug where `/payouts/profile/list` returns non-zero `total_count` when the data array is empty. This causes the frontend to render pagination controls instead of "No results found" when a profile with no payouts is selected.

### Fresh Research Findings

**Bug Location #1** - `payouts_list_core` (Affects both merchant and profile endpoints):
- File: `/home/potter/Documents/hyperswitch/crates/router/src/core/payouts.rs`
- Line: **836**
- Issue: Hardcodes `total_count: None`

**Bug Location #2** - `payouts_filtered_list_core` (Profile endpoint only):
- File: `/home/potter/Documents/hyperswitch/crates/router/src/core/payouts.rs`
- Lines: **930-946**
- Issue: Calls `get_total_count_of_filtered_payouts` without `profile_id_list` parameter

**Parameter Flow Analysis**:
1. Handler `payouts_list_by_filter_profile` extracts `profile_id` from auth (routes/payouts.rs:394-399)
2. Passes as `profile_id_list` to `payouts_filtered_list_core`
3. Core calls `filter_active_payout_ids_by_constraints` (line 922) which DOES filter by profile_id
4. Gets `active_payout_ids` (already filtered by profile)
5. Calls `get_total_count_of_filtered_payouts` with `active_payout_ids` but **NO** `profile_id_list`
6. Count query never receives profile_id, counts ALL merchant payouts

**Root Cause**: 
- SELECT query filters results in-memory AFTER database query (line 865: `filter_objects_based_on_profile_id_list`)
- COUNT query runs with only `active_payout_ids` constraint, missing profile filter
- The trait `get_total_count_of_filtered_payouts` lacks `profile_id_list` parameter entirely

### Files Requiring Changes (Verified with Exact Lines)

1. **Trait Definition**: `crates/hyperswitch_domain_models/src/payouts/payouts.rs:76-84`
2. **Diesel Query**: `crates/diesel_models/src/query/payouts.rs:96-133`
3. **RouterStore**: `crates/storage_impl/src/payouts/payouts.rs:780-815`
4. **KVRouterStore**: `crates/storage_impl/src/payouts/payouts.rs:357`
5. **KafkaStore**: `crates/router/src/db/kafka_store.rs:2561`
6. **MockDb**: `crates/storage_impl/src/mock_db/payouts.rs:95`
7. **Core Logic (None bug)**: `crates/router/src/core/payouts.rs:836`
8. **Core Logic (Filtered bug)**: `crates/router/src/core/payouts.rs:930-946`

---

## Work Objectives

### Core Objective
Fix the `total_count` field to correctly reflect the count of payouts matching the filters:
- For `/payouts/profile/list`: count should reflect profile-specific payouts
- For `/payouts/list`: count should reflect merchant-level payouts (no longer `None`)

### Concrete Deliverables
- Updated `PayoutsInterface` trait with `profile_id_list` parameter
- Modified Diesel count query with profile_id filtering logic
- Fixed `payouts_list_core` to calculate and return actual count
- Updated 4 storage implementations
- Updated core call sites
- Unit and integration tests

### Definition of Done
- [ ] `/payouts/profile/list` returns `total_count: 0` when profile has no payouts
- [ ] `/payouts/profile/list` returns correct count when profile has payouts
- [ ] `/payouts/list` returns actual count instead of `null`/`None`
- [ ] All existing tests pass
- [ ] New tests cover profile-scoped count behavior
- [ ] MockDb implementation functional for tests

### Must Have
- Profile_id filtering in count query matches SELECT query behavior
- Backward compatibility maintained (None = no filter)
- All storage implementations updated consistently
- Tests verify empty profile, single profile, and multiple profiles scenarios
- Fix applies to both profile-level AND merchant-level endpoints

### Must NOT Have (Guardrails)
- Changes to pagination logic
- Changes to response structure (other than total_count accuracy)
- Database schema changes
- Performance degradation (profile_id is already indexed)
- Breaking changes to existing API contracts
- Skipping MockDb implementation

---

## Git Workflow (MANDATORY)

> All changes must be made on a feature branch with local commits only. DO NOT push to remote.

### Pre-Implementation Setup

```bash
# 1. Ensure you're on main branch
git checkout main

# 2. Pull latest changes from origin
git pull origin main

# 3. Create feature branch
git checkout -b fix/payout-profile-list-count

# 4. Verify clean working state
git status
```

### Commit Strategy (Local Only)

**IMPORTANT**: All commits should remain LOCAL. Do NOT push to remote repository.

```bash
# After each task completion:
git add <modified-files>
git commit -m "type(scope): description"

# Verify commits are local only:
git log --oneline -5
# Should show: fix/payout-profile-list-count branch, not origin/fix/payout-profile-list-count

# DO NOT RUN:
# git push origin fix/payout-profile-list-count  # <-- SKIP THIS
```

### Post-Implementation

```bash
# Verify all commits are local
git status

# Optional: Create patch file for review
git format-patch main --stdout > payout-count-fix.patch

# Keep branch locally until ready to push
# When ready (not now), you'll run:
# git push origin fix/payout-profile-list-count
```

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (Cargo test framework)
- **Automated tests**: YES (TDD approach - tests written before fix)
- **Framework**: `cargo test` with in-memory database for integration tests
- **Coverage**: 80%+ on modified lines using `cargo llvm-cov`

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Backend/Database**: Use Bash (cargo test) — Run unit tests, assert pass/fail
- **Integration**: Use Bash (cargo test --test) — Run integration tests

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — trait + Diesel):
├── Task 1: Update PayoutsInterface trait with profile_id_list [quick]
└── Task 2: Add profile_id filter to Diesel count query [quick]

Wave 2 (After Wave 1 — storage implementations, MAX PARALLEL):
├── Task 3: Update RouterStore implementation [quick]
├── Task 4: Update KVRouterStore implementation [quick]
├── Task 5: Update KafkaStore implementation [quick]
└── Task 6: Update MockDb implementation [quick]

Wave 3 (After Wave 2 — core logic + tests):
├── Task 7: Fix payouts_list_core hardcoded None [quick]
├── Task 8: Update payouts_filtered_list_core call site [quick]
├── Task 9: Add unit tests for count query [unspecified-high]
├── Task 10: Add integration tests [unspecified-high]
└── Task 11: Add Cypress E2E tests [unspecified-high]

Wave FINAL (After ALL tasks — verification, 4 parallel):
├── Task F1: Full test suite verification [deep]
├── Task F2: Code quality check [unspecified-high]
├── Task F3: Regression test both endpoints [unspecified-high]
└── Task F4: Cypress E2E verification [unspecified-high]

Critical Path: Task 1 → Task 2 → (Tasks 3-6 parallel) → (Tasks 7-8) → (Tasks 9-11) → F1-F4
Parallel Speedup: ~50% faster than sequential
Max Concurrent: 4 (Wave 2)
```

### Dependency Matrix

- **1-2**: — — 3,4,5,6
- **3-6**: 1,2 — 7,8
- **7-8**: 3,4,5,6 — 9,10,11
- **9-11**: 7,8 — F1,F2,F3,F4

### Agent Dispatch Summary

- **Wave 1**: **2** — T1 → `quick`, T2 → `quick`
- **Wave 2**: **4** — T3-T6 → `quick`
- **Wave 3**: **5** — T7-T8 → `quick`, T9-T11 → `unspecified-high`
- **Wave FINAL**: **4** — F1 → `deep`, F2-F4 → `unspecified-high`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task MUST have: Recommended Agent Profile + Parallelization info + QA Scenarios.

- [x] 1. Update PayoutsInterface Trait Definition

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/hyperswitch_domain_models/src/payouts/payouts.rs`
  - Find `PayoutsInterface` trait at line 11
  - Locate `get_total_count_of_filtered_payouts` method at lines 76-84
  - Add `profile_id_list: Option<Vec<ProfileId>>` parameter after `active_payout_ids`
  - Current signature lacks profile_id_list entirely
  - New signature should be: `async fn get_total_count_of_filtered_payouts(&self, merchant_id: &id_type::MerchantId, active_payout_ids: &[id_type::PayoutId], profile_id_list: Option<Vec<ProfileId>>, connector: Option<Vec<api_models::enums::Connector>>, currency: Option<Vec<storage_enums::Currency>>, status: Option<Vec<storage_enums::PayoutStatus>>, payout_method: Option<Vec<storage_enums::PayoutType>>) -> error_stack::Result<i64, Self::Error>`
  - Update documentation comments if present

  **Must NOT do**:
  - Do NOT change return type (must remain `Result<i64, Self::Error>`)
  - Do NOT change other method signatures
  - Do NOT remove existing parameters

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple trait signature change, no complex logic
  - **Skills**: [`rust-patterns`]
    - `rust-patterns`: Needed for idiomatic Rust trait modifications and Option<Vec<T>> patterns

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 2)
  - **Blocks**: Task 2, Task 3, Task 4, Task 5, Task 6
  - **Blocked By**: None (can start immediately)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References**:
  - `/home/potter/Documents/hyperswitch/crates/hyperswitch_domain_models/src/payouts/payouts.rs:76-84` - Current trait definition
  - Look at `filter_payouts_and_attempts` signature (same file, line ~60) for parameter pattern reference (uses `profile_id_list: Option<Vec<ProfileId>>`)

  **API/Type References**:
  - `ProfileId` type - in `common_utils::id_type` module
  - `MerchantId` type - for parameter context
  - `PayoutId` type - for active_payout_ids parameter

  **WHY Each Reference Matters**:
  - Trait definition file: Shows exact current signature
  - filter_payouts_and_attempts: Demonstrates the existing convention for profile_id_list parameter

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Verify trait compiles with new signature
    Tool: Bash (cargo check)
    Preconditions: None
    Steps:
      1. Run: cargo check -p hyperswitch_domain_models
      2. Verify: Compilation succeeds with no errors
    Expected Result: Command exits with code 0, no compilation errors
    Failure Indicators: Compilation errors about trait definition
    Evidence: .sisyphus/evidence/task-1-trait-compiles.txt

  Scenario: Verify trait has correct signature
    Tool: Bash (grep)
    Preconditions: Task 1 implementation complete
    Steps:
      1. Run: grep -n "get_total_count_of_filtered_payouts" crates/hyperswitch_domain_models/src/payouts/payouts.rs
      2. Verify: Output contains "profile_id_list: Option<Vec<ProfileId>>"
    Expected Result: grep finds the method with new parameter
    Failure Indicators: Parameter missing or wrong type
    Evidence: .sisyphus/evidence/task-1-signature-verified.txt
  ```

  **Evidence to Capture**:
  - [ ] Screenshot or text of cargo check output
  - [ ] grep output showing signature

  **Commit**: YES (Commit 1)
  - Message: `refactor(payouts): add profile_id_list to PayoutsInterface count method`
  - Files: `crates/hyperswitch_domain_models/src/payouts/payouts.rs`
  - Pre-commit: `cargo check -p hyperswitch_domain_models`

---

- [x] 2. Implement Profile ID Filter in Diesel Count Query

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/diesel_models/src/query/payouts.rs`
  - Find `get_total_count_of_payouts` function at lines 96-133
  - Add `profile_id_list: Option<&[common_utils::id_type::ProfileId]>` parameter to function signature
  - Add profile_id filter logic after line 123 (where payout_type filter ends)
  - Use Diesel pattern: `if let Some(profile_ids) = profile_id_list { query = query.filter(payout_attempts_dsl::profile_id.eq_any(profile_ids.iter().map(|p| p.get_string_repr()))); }`
  - Note: May need to convert ProfileId to String representation for Diesel
  - Update function documentation if present

  **Must NOT do**:
  - Do NOT change the return type (must remain `StorageResult<i64>`)
  - Do NOT remove existing filters
  - Do NOT apply filter when profile_id_list is None or empty (backward compatibility)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Adding filter condition to existing query builder, straightforward Diesel usage
  - **Skills**: [`rust-patterns`]
    - `rust-patterns`: Needed for idiomatic Diesel query builder patterns with Option

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 1)
  - **Blocks**: Task 3, Task 4, Task 5
  - **Blocked By**: Task 1 (needs trait signature, though can work in parallel with careful coordination)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References**:
  - `/home/potter/Documents/hyperswitch/crates/diesel_models/src/query/payouts.rs:96-133` - Current count query implementation
  - Lines 115-123 show connector/currency/status/payout_type filter patterns - copy this pattern
  - `/home/potter/Documents/hyperswitch/crates/storage_impl/src/payouts/payouts.rs:849-850` - Shows how profile_id filtering works in SELECT queries

  **API/Type References**:
  - Diesel `eq_any` method for filtering by multiple values
  - `payout_attempts_dsl::profile_id` column reference
  - `ProfileId::get_string_repr()` method for converting to String

  **WHY Each Reference Matters**:
  - Current count query: Shows exact structure and where to insert new filter
  - SELECT query filters: Demonstrates correct column name and filter pattern
  - Lines 115-123: Shows the conditional filter pattern used throughout

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Verify query compiles with profile filter
    Tool: Bash (cargo check)
    Preconditions: Task 1 complete
    Steps:
      1. Run: cargo check -p diesel_models
      2. Verify: Compilation succeeds
    Expected Result: Command exits with code 0
    Failure Indicators: Type mismatch or column not found errors
    Evidence: .sisyphus/evidence/task-2-query-compiles.txt

  Scenario: Verify filter logic present
    Tool: Bash (grep)
    Preconditions: Task 2 implementation complete
    Steps:
      1. Run: grep -A 3 "profile_id_list" crates/diesel_models/src/query/payouts.rs
      2. Verify: Output shows filter logic with eq_any
    Expected Result: Filter condition present in query builder
    Evidence: .sisyphus/evidence/task-2-filter-logic.txt
  ```

  **Evidence to Capture**:
  - [ ] cargo check output
  - [ ] grep output showing filter logic

  **Commit**: YES (Commit 2)
  - Message: `fix(diesel_models): add profile_id filter to payout count query`
  - Files: `crates/diesel_models/src/query/payouts.rs`
  - Pre-commit: `cargo check -p diesel_models`

---

- [x] 3. Update RouterStore Implementation

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/storage_impl/src/payouts/payouts.rs`
  - Find `get_total_count_of_filtered_payouts` for `RouterStore<T>` at lines 780-815
  - Add `profile_id_list: Option<Vec<ProfileId>>` parameter to function signature
  - Pass `profile_id_list` to the Diesel query call at line 801
  - Convert `Vec<ProfileId>` to `&[ProfileId]` for Diesel function if needed
  - Update error handling to maintain consistency

  **Must NOT do**:
  - Do NOT implement new logic (just pass through to Diesel query)
  - Do NOT change error handling patterns
  - Do NOT forget to pass all other existing parameters

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple parameter forwarding to underlying implementation
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 4, Task 5, Task 6)
  - **Blocks**: Task 7, Task 8
  - **Blocked By**: Task 1, Task 2

  **References**:
  - `/home/potter/Documents/hyperswitch/crates/storage_impl/src/payouts/payouts.rs:780-815` - RouterStore implementation
  - Line 801 shows current call to Diesel query

  **Acceptance Criteria**:

  **QA Scenarios**:

  ```
  Scenario: Verify RouterStore compiles
    Tool: Bash (cargo check)
    Preconditions: Tasks 1-2 complete
    Steps:
      1. Run: cargo check -p storage_impl
      2. Verify: No compilation errors
    Expected Result: Command exits with code 0
    Evidence: .sisyphus/evidence/task-3-routerstore-compiles.txt
  ```

  **Commit**: YES (Commit 3)

---

- [x] 4. Update KVRouterStore Implementation

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/storage_impl/src/payouts/payouts.rs`
  - Find `get_total_count_of_filtered_payouts` for `KVRouterStore<T>` at line 357
  - Add `profile_id_list: Option<Vec<ProfileId>>` parameter
  - Pass `profile_id_list` to `self.router_store.get_total_count_of_filtered_payouts()` call
  - This is a delegation wrapper - just forward the parameter

  **Must NOT do**:
  - Do NOT add KV-specific logic (just pass through)
  - Do NOT change delegation pattern

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 3, Task 5, Task 6)
  - **Blocks**: Task 7, Task 8
  - **Blocked By**: Task 1, Task 2

  **References**:
  - `/home/potter/Documents/hyperswitch/crates/storage_impl/src/payouts/payouts.rs:357` - KVRouterStore implementation

  **Commit**: YES (Commit 3)

---

- [x] 5. Update KafkaStore Implementation

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/router/src/db/kafka_store.rs`
  - Find `get_total_count_of_filtered_payouts` at line 2561
  - Add `profile_id_list: Option<Vec<ProfileId>>` parameter
  - Pass `profile_id_list` to `self.diesel_store.get_total_count_of_filtered_payouts()` call
  - Follow existing delegation pattern in KafkaStore

  **Must NOT do**:
  - Do NOT add Kafka-specific logic (just pass through)
  - Do NOT change error handling

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 3, Task 4, Task 6)
  - **Blocks**: Task 7, Task 8
  - **Blocked By**: Task 1, Task 2

  **References**:
  - `/home/potter/Documents/hyperswitch/crates/router/src/db/kafka_store.rs:2561` - KafkaStore implementation

  **Commit**: YES (Commit 3)

---

- [x] 6. Update MockDb Implementation

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/storage_impl/src/mock_db/payouts.rs`
  - Find `get_total_count_of_filtered_payouts` at line 95
  - Add `profile_id_list: Option<Vec<ProfileId>>` parameter
  - Implement proper count logic (currently returns `Err(StorageError::MockDbError)`)
  - Filter mock data by profile_id_list when provided
  - Count matching records and return as i64
  - Handle None case (no filter) - count all

  **Must NOT do**:
  - Do NOT leave as MockDbError stub (must implement for tests)
  - Do NOT ignore profile_id_list parameter (must filter)
  - Do NOT panic or return error

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Implementing mock logic, requires understanding of mock data structure
  - **Skills**: [`rust-patterns`, `rust-testing`]
    - `rust-testing`: For understanding mock implementation patterns

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 3, Task 4, Task 5)
  - **Blocks**: Task 9, Task 10 (tests depend on mock)
  - **Blocked By**: Task 1, Task 2

  **References**:
  - `/home/potter/Documents/hyperswitch/crates/storage_impl/src/mock_db/payouts.rs:95` - MockDb implementation
  - Look at how other methods filter mock data for pattern

  **Acceptance Criteria**:

  **QA Scenarios**:

  ```
  Scenario: Verify MockDb returns count not error
    Tool: Bash (cargo test)
    Preconditions: Tasks 1-5 complete
    Steps:
      1. Run: cargo test -p storage_impl mock_db 2>&1 | head -50
      2. Verify: No MockDbError in output
    Expected Result: Tests can use MockDb without errors
    Evidence: .sisyphus/evidence/task-6-mock-works.txt
  ```

  **Commit**: YES (Commit 3)

---

- [x] 7. Fix payouts_list_core Hardcoded None

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/router/src/core/payouts.rs`
  - Find `payouts_list_core` function at line 722
  - Find `total_count: None` at line 836
  - Replace with actual count calculation similar to payouts_filtered_list_core pattern
  - Call `get_total_count_of_filtered_payouts` with:
    - merchant_id
    - active_payout_ids (get from filter_payout_list_constraints)
    - profile_id_list (passed to function)
    - connector, currency, status, payout_type (from constraints)
  - Store result and pass to PayoutListResponse

  **Must NOT do**:
  - Do NOT change response structure
  - Do NOT change pagination logic
  - Do NOT skip error handling

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: NO (sequential with Wave 2)
  - **Parallel Group**: Wave 3 (with Tasks 8-10)
  - **Blocks**: Task 9, Task 10
  - **Blocked By**: Task 3, Task 4, Task 5, Task 6

  **References**:
  - `/home/potter/Documents/hyperswitch/crates/router/src/core/payouts.rs:722-839` - payouts_list_core
  - `/home/potter/Documents/hyperswitch/crates/router/src/core/payouts.rs:930-946` - Example pattern in payouts_filtered_list_core

  **Acceptance Criteria**:

  **QA Scenarios**:

  ```
  Scenario: Verify payouts_list_core returns count
    Tool: Bash (grep)
    Preconditions: Task 7 complete
    Steps:
      1. Run: grep -A 5 "total_count" crates/router/src/core/payouts.rs | grep -A 3 "payouts_list_core"
      2. Verify: Shows get_total_count_of_filtered_payouts call, not None
    Expected Result: total_count is calculated, not hardcoded None
    Evidence: .sisyphus/evidence/task-7-none-fixed.txt
  ```

  **Commit**: YES (Commit 4)
  - Message: `fix(router): fix total_count in payouts_list_core`
  - Files: `crates/router/src/core/payouts.rs`

---

- [x] 8. Update payouts_filtered_list_core Call Site

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/router/src/core/payouts.rs`
  - Find `payouts_filtered_list_core` at line 842
  - Find call to `get_total_count_of_filtered_payouts` at lines 930-946
  - Currently calls without profile_id_list parameter
  - Add profile_id_list parameter to the call
  - Extract profile_id_list from constraints if available
  - Pass through to count function

  **Must NOT do**:
  - Do NOT change the filtering logic
  - Do NOT change how active_payout_ids is calculated
  - Do NOT remove other parameters

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 7)
  - **Parallel Group**: Wave 3 (with Tasks 7, 9, 10)
  - **Blocks**: Task 9, Task 10
  - **Blocked By**: Task 3, Task 4, Task 5, Task 6

  **References**:
  - `/home/potter/Documents/hyperswitch/crates/router/src/core/payouts.rs:930-946` - Current call site
  - `/home/potter/Documents/hyperswitch/crates/router/src/core/payouts.rs:922` - Where active_payout_ids is calculated with profile filter

  **Acceptance Criteria**:

  **QA Scenarios**:

  ```
  Scenario: Verify profile_id_list passed to count
    Tool: Bash (grep)
    Preconditions: Task 8 complete
    Steps:
      1. Run: grep -B2 -A2 "get_total_count_of_filtered_payouts" crates/router/src/core/payouts.rs | grep -A 4 "payouts_filtered_list_core"
      2. Verify: Shows profile_id_list being passed as parameter
    Expected Result: Call includes profile_id_list argument
    Evidence: .sisyphus/evidence/task-8-profile-passed.txt
  ```

  **Commit**: YES (Commit 4)
  - Message: `fix(router): pass profile_id_list to count in filtered list`

---

- [x] 9. Add Unit Tests for Count Query

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/diesel_models/src/query/payouts.rs`
  - Find or create test module
  - Add tests for `get_total_count_of_payouts` with:
    - `profile_id_list: None` (backward compatibility)
    - `profile_id_list: Some(vec![profile_id])` (single profile)
    - `profile_id_list: Some(vec![profile1, profile2])` (multiple profiles)
    - Empty result case (profile with no payouts → count = 0)
  - Use in-memory test database or mocks
  - Verify count matches expected number of payouts per profile

  **Must NOT do**:
  - Do NOT test without database context
  - Do NOT skip the empty profile case (the bug being fixed)
  - Do NOT use production database

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: [`rust-testing`, `rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 10)
  - **Parallel Group**: Wave 3 (with Tasks 7-8, 10)
  - **Blocks**: Task F1, Task F2, Task F3
  - **Blocked By**: Task 6, Task 7, Task 8

  **References**:
  - Existing tests in `diesel_models/src/query/` for patterns
  - Test database setup in the crate

  **Acceptance Criteria**:

  **QA Scenarios**:

  ```
  Scenario: Run unit tests
    Tool: Bash (cargo test)
    Preconditions: Tasks 1-8 complete
    Steps:
      1. Run: cargo test -p diesel_models get_total_count -- --nocapture
      2. Verify: All tests pass
    Expected Result: Test exit code 0
    Evidence: .sisyphus/evidence/task-9-unit-tests.txt

  Scenario: Verify empty profile count is zero
    Tool: Bash (cargo test)
    Steps:
      1. Run: cargo test -p diesel_models test_count_empty_profile -- --nocapture
      2. Verify: Test passes
    Expected Result: Count returns 0 for profile with no payouts
    Evidence: .sisyphus/evidence/task-9-empty-test.txt
  ```

  **Commit**: YES (Commit 5)

---

- [x] 10. Add Integration Tests

  **What to do**:
  - Open `/home/potter/Documents/hyperswitch/crates/router/tests/payouts.rs`
  - Add tests for `/payouts/profile/list` endpoint:
    - Test empty profile returns total_count: 0
    - Test profile with payouts returns correct count
    - Test multiple profiles combined count
  - Add tests for `/payouts/list` endpoint:
    - Test returns actual count instead of null
  - Call endpoints via HTTP and verify response
  - Assert both `data.len()` and `total_count` match

  **Must NOT do**:
  - Do NOT skip HTTP-level testing
  - Do NOT skip empty profile case
  - Do NOT mock the database

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: [`rust-testing`, `rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 9)
  - **Parallel Group**: Wave 3 (with Tasks 7-9)
  - **Blocks**: Task F1, Task F2, Task F3
  - **Blocked By**: Task 6, Task 7, Task 8

  **References**:
  - `/home/potter/Documents/hyperswitch/crates/router/tests/payouts.rs` - Existing tests
  - Other integration tests in `crates/router/tests/`

  **Acceptance Criteria**:

  **QA Scenarios**:

  ```
  Scenario: Run integration tests
    Tool: Bash (cargo test)
    Steps:
      1. Run: cargo test --test payouts -- --nocapture
      2. Verify: All tests pass
    Expected Result: Exit code 0
    Evidence: .sisyphus/evidence/task-10-integration.txt

  Scenario: Verify empty profile returns zero
    Tool: Bash (cargo test)
    Steps:
      1. Run: cargo test --test payouts test_profile_empty -- --nocapture
      2. Verify: Response.total_count == 0
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-10-empty-integration.txt
  ```

  **Commit**: YES (Commit 5)

---

- [x] 11. Add Cypress E2E Tests for Payout List Count

  **What to do**:
  - Create new Cypress test file: `/home/potter/Documents/hyperswitch/cypress-tests/cypress/e2e/spec/Payout/00007-PayoutListCount.cy.js`
  - Add test to verify `total_count` field in `/payouts/profile/list` response
  - Test scenarios:
    1. Profile with no payouts → `total_count` should be 0
    2. Profile with payouts → `total_count` should match actual count
    3. Merchant-level `/payouts/list` → `total_count` should not be null
  - Use existing Cypress patterns from commands.js (lines 1374-1400)
  - Leverage `globalState` for profile_id and apiKey management
  - Use cy.request() to call the API endpoints
  - Validate response structure and total_count accuracy

  **Must NOT do**:
  - Do NOT skip error handling
  - Do NOT hardcode test data that could change
  - Do NOT use production database or real credentials

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: E2E testing requires understanding of Cypress patterns and API testing
  - **Skills**: [`e2e-testing`, `rust-patterns`]
    - `e2e-testing`: For Cypress patterns and API testing best practices

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 9, Task 10)
  - **Parallel Group**: Wave 3 (with Tasks 7-10)
  - **Blocks**: Task F1, Task F2, Task F3
  - **Blocked By**: Task 6, Task 7, Task 8

  **References**:
  - `/home/potter/Documents/hyperswitch/cypress-tests/cypress/support/commands.js:1374-1400` - Example total_count validation
  - `/home/potter/Documents/hyperswitch/cypress-tests/cypress/e2e/spec/Payout/00003-CardTest.cy.js` - Payout test pattern
  - `/home/potter/Documents/hyperswitch/cypress-tests/cypress/utils/State.js` - Global state management

  **Acceptance Criteria**:

  **QA Scenarios**:

  ```
  Scenario: Create Cypress test file
    Tool: Bash
    Preconditions: Tasks 1-8 complete
    Steps:
      1. Verify file exists: test -f cypress-tests/cypress/e2e/spec/Payout/00007-PayoutListCount.cy.js
      2. Verify: File contains "total_count" validation
    Expected Result: File exists with proper test structure
    Evidence: .sisyphus/evidence/task-11-cypress-file.txt

  Scenario: Run Cypress payout tests
    Tool: Bash
    Preconditions: Task 11 complete, server running locally
    Steps:
      1. Run: cd cypress-tests && npm run cypress:payouts
      2. Verify: Tests pass without errors
    Expected Result: All payout tests pass including new list count test
    Evidence: .sisyphus/evidence/task-11-cypress-run.txt

  Scenario: Verify total_count assertion in Cypress
    Tool: Bash (grep)
    Preconditions: Task 11 complete
    Steps:
      1. Run: grep -n "total_count" cypress-tests/cypress/e2e/spec/Payout/00007-PayoutListCount.cy.js
      2. Verify: Shows assertions for total_count field
    Expected Result: File contains total_count validation logic
    Evidence: .sisyphus/evidence/task-11-assertions.txt
  ```

  **Evidence to Capture**:
  - [ ] Cypress test file contents
  - [ ] Test run output showing pass/fail
  - [ ] Screenshot of test results if available

  **Commit**: YES (Commit 5 - grouped with Tasks 9-10)

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 3 review agents run in PARALLEL. ALL must APPROVE. Rejection → fix → re-run.

- [ ] F1. **Full Test Suite Verification** — `deep`
  Run `cargo test --workspace` to verify all tests pass including new ones. Check that payout-related tests specifically pass. Verify no compilation errors across all modified crates.
  Output: `Tests [PASS/FAIL] | Coverage [%] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --workspace` and `cargo fmt --check`. Review modified files for: unused imports, proper error handling, consistent naming. Check AI slop patterns: excessive comments, over-abstraction.
  Output: `Clippy [PASS/FAIL] | Format [PASS/FAIL] | Issues [N] | VERDICT`

- [ ] F3. **Regression Test — Both Endpoints** — `unspecified-high`
  Verify that both `/payouts/list` and `/payouts/profile/list` return accurate total_count. Run existing payout tests to ensure no regression.
  Output: `Profile Tests [N/N pass] | Merchant Tests [N/N pass] | VERDICT`

- [ ] F4. **Cypress E2E Verification** — `unspecified-high`
  Run Cypress payout tests to verify total_count behavior at the API level. Ensure `/payouts/profile/list` returns correct total_count for profiles with and without payouts.
  Output: `Cypress Tests [N/N pass] | VERDICT`

---

## Commit Strategy

**Commit 1: Interface Definition** (Task 1)
```
refactor(payouts): add profile_id_list to PayoutsInterface count method

- Update get_total_count_of_filtered_payouts signature
- Add profile_id_list: Option<Vec<ProfileId>> parameter
- Maintain backward compatibility with None = no filter
```

**Commit 2: Query Implementation** (Task 2)
```
fix(diesel_models): add profile_id filter to payout count query

- Update get_total_count_of_payouts query function
- Add profile_id filter using eq_any pattern
- Follow existing filter patterns (connector, currency, etc.)
```

**Commit 3: Storage Implementations** (Tasks 3-6)
```
fix(storage): implement profile_id filtering in payout count

- Update RouterStore and KVRouterStore implementations
- Update KafkaStore delegation
- Update MockDb with proper implementation
- Pass profile_id_list through to Diesel query
```

**Commit 4: Core Logic Fixes** (Tasks 7-8)
```
fix(router): fix total_count in payout list endpoints

- Fix payouts_list_core to return actual count instead of None
- Update payouts_filtered_list_core to pass profile_id_list
- Ensure profile_id extracted from constraints
```

**Commit 5: Tests** (Tasks 9-11)
```
test(payouts): add tests for profile-scoped payout count

- Unit tests for count query with profile filtering
- Integration tests for both endpoints
- Cypress E2E tests for API validation
- Test empty profile, single profile, multiple profiles
```

---

## Success Criteria

### Verification Commands
```bash
# Compile check
cargo check --workspace

# Run tests
cargo test --workspace -- payouts

# Coverage
cargo llvm-cov --workspace --fail-under-lines 80

# Clippy
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass (cargo test --workspace)
- [ ] Coverage >= 80% on modified lines
- [ ] Clippy clean with no warnings
- [ ] Both endpoints return accurate total_count
- [ ] MockDb implementation functional
- [ ] Cypress E2E tests pass
- [ ] No regression in existing functionality
