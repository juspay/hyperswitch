/// Payouts - Create
#[utoipa::path(
    post,
    path = "/payouts/create",
    request_body=PayoutCreateRequest,
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
        ("payout_id" = String, Path, description = "The identifier for payout]")
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
        ("payout_id" = String, Path, description = "The identifier for payout]")
    ),
    request_body=PayoutCreateRequest,
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
    request_body=PayoutActionRequest,
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
    request_body=PayoutActionRequest,
    responses(
        (status = 200, description = "Payout fulfilled", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Fulfill a Payout",
    security(("api_key" = []))
)]
pub async fn payouts_fulfill() {}
