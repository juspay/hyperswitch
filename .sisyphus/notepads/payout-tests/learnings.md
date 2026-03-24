# Payout Integration Tests - Learnings

## Test Structure
- Integration tests located in `crates/router/tests/payouts.rs`
- Uses existing test utilities in `crates/router/tests/utils.rs`
- Pattern follows other integration tests (refunds.rs, customers.rs)

## Endpoints Tested
- GET /payouts/profile/list - Profile-level payout list
- GET /payouts/list - Merchant-level payout list

## Response Structure
- `PayoutListResponse` defined in `api_models/src/payouts.rs` lines 896-905
- Contains: `size` (usize), `data` (Vec<PayoutCreateResponse>), `total_count` (Option<i64>)
- `total_count` field is serialized conditionally with `#[serde(skip_serializing_if = "Option::is_none")]`

## Key Implementation Details
- `payouts_list_core` in `router/src/core/payouts.rs` (lines 721-860) handles list logic
- Profile-level endpoint filters by `profile_id_list` using `filter_objects_based_on_profile_id_list`
- `total_count` is calculated via `get_total_count_of_filtered_payouts` database query
- Both simple list and filtered list endpoints return `total_count: Some(total_count)`

## Test Coverage
1. Empty profile returns total_count: 0
2. Profile with payouts returns correct total_count >= size
3. Merchant-level endpoint returns actual total_count
4. Response structure validation (size, data, total_count fields)
5. Consistency check between profile-level and merchant-level counts
