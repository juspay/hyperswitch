/// Payouts - Create
#[utoipa::path(
    post,
    path = "/payouts/create",
    request_body=PayoutsCreateRequest,
    responses(
        (status = 200, description = "Payout created", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Create a Payout",
    security(("api_key" = []))
)]
pub async fn payouts_create() {}

/// Payouts - Retrieve
#[utoipa::path(
    get,
    path = "/payouts/{payout_id}",
    params(
        ("payout_id" = String, Path, description = "The identifier for payout"),
        ("force_sync" = Option<bool>, Query, description = "Sync with the connector to get the payout details (defaults to false)")
    ),
    responses(
        (status = 200, description = "Payout retrieved", body = PayoutCreateResponse),
        (status = 404, description = "Payout does not exist in our records")
    ),
    tag = "Payouts",
    operation_id = "Retrieve a Payout",
    security(("api_key" = []))
)]
pub async fn payouts_retrieve() {}

/// Payouts - Update
#[utoipa::path(
    post,
    path = "/payouts/{payout_id}",
    params(
        ("payout_id" = String, Path, description = "The identifier for payout")
    ),
    request_body=PayoutUpdateRequest,
    responses(
        (status = 200, description = "Payout updated", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Update a Payout",
    security(("api_key" = []))
)]
pub async fn payouts_update() {}

/// Payouts - Cancel
#[utoipa::path(
    post,
    path = "/payouts/{payout_id}/cancel",
    params(
        ("payout_id" = String, Path, description = "The identifier for payout")
    ),
    request_body=PayoutCancelRequest,
    responses(
        (status = 200, description = "Payout cancelled", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Cancel a Payout",
    security(("api_key" = []))
)]
pub async fn payouts_cancel() {}

/// Payouts - Fulfill
#[utoipa::path(
    post,
    path = "/payouts/{payout_id}/fulfill",
    params(
        ("payout_id" = String, Path, description = "The identifier for payout")
    ),
    request_body=PayoutFulfillRequest,
    responses(
        (status = 200, description = "Payout fulfilled", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Fulfill a Payout",
    security(("api_key" = []))
)]
pub async fn payouts_fulfill() {}

/// Payouts - List
#[utoipa::path(
    get,
    path = "/payouts/list",
    params(
        ("customer_id" = String, Query, description = "The identifier for customer"),
        ("starting_after" = String, Query, description = "A cursor for use in pagination, fetch the next list after some object"),
        ("ending_before" = String, Query, description = "A cursor for use in pagination, fetch the previous list before some object"),
        ("limit" = String, Query, description = "limit on the number of objects to return"),
        ("created" = String, Query, description = "The time at which payout is created"),
        ("time_range" = String, Query, description = "The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc).")
    ),
    responses(
        (status = 200, description = "Payouts listed", body = PayoutListResponse),
        (status = 404, description = "Payout not found")
    ),
    tag = "Payouts",
    operation_id = "List payouts using generic constraints",
    security(("api_key" = []))
)]
pub async fn payouts_list() {}

/// Payouts - List for the Given Profiles
#[utoipa::path(
    get,
    path = "/payouts/profile/list",
    params(
        ("customer_id" = String, Query, description = "The identifier for customer"),
        ("starting_after" = String, Query, description = "A cursor for use in pagination, fetch the next list after some object"),
        ("ending_before" = String, Query, description = "A cursor for use in pagination, fetch the previous list before some object"),
        ("limit" = String, Query, description = "limit on the number of objects to return"),
        ("created" = String, Query, description = "The time at which payout is created"),
        ("time_range" = String, Query, description = "The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc).")
    ),
    responses(
        (status = 200, description = "Payouts listed", body = PayoutListResponse),
        (status = 404, description = "Payout not found")
    ),
    tag = "Payouts",
    operation_id = "List payouts using generic constraints for the given Profiles",
    security(("api_key" = []))
)]
pub async fn payouts_list_profile() {}

/// Payouts - List available filters
#[utoipa::path(
    post,
    path = "/payouts/filter",
    request_body=TimeRange,
    responses(
        (status = 200, description = "Filters listed", body = PayoutListFilters)
    ),
    tag = "Payouts",
    operation_id = "List available payout filters",
    security(("api_key" = []))
)]
pub async fn payouts_list_filters() {}

/// Payouts - List using filters
#[utoipa::path(
    post,
    path = "/payouts/list",
    request_body=PayoutListFilterConstraints,
    responses(
        (status = 200, description = "Payouts filtered", body = PayoutListResponse),
        (status = 404, description = "Payout not found")
    ),
    tag = "Payouts",
    operation_id = "Filter payouts using specific constraints",
    security(("api_key" = []))
)]
pub async fn payouts_list_by_filter() {}

/// Payouts - List using filters for the given Profiles
#[utoipa::path(
    post,
    path = "/payouts/list",
    request_body=PayoutListFilterConstraints,
    responses(
        (status = 200, description = "Payouts filtered", body = PayoutListResponse),
        (status = 404, description = "Payout not found")
    ),
    tag = "Payouts",
    operation_id = "Filter payouts using specific constraints for the given Profiles",
    security(("api_key" = []))
)]
pub async fn payouts_list_by_filter_profile() {}

/// Payouts - Confirm
#[utoipa::path(
    post,
    path = "/payouts/{payout_id}/confirm",
    params(
        ("payout_id" = String, Path, description = "The identifier for payout")
    ),
    request_body=PayoutConfirmRequest,
    responses(
        (status = 200, description = "Payout updated", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Confirm a Payout",
    security(("api_key" = []))
)]
pub async fn payouts_confirm() {}
