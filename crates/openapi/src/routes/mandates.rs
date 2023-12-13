/// Mandates - Retrieve Mandate
///
/// Retrieves a mandate created using the Payments/Create API
#[utoipa::path(
    get,
    path = "/mandates/{mandate_id}",
    params(
        ("mandate_id" = String, Path, description = "The identifier for mandate")
    ),
    responses(
        (status = 200, description = "The mandate was retrieved successfully", body = MandateResponse),
        (status = 404, description = "Mandate does not exist in our records")
    ),
    tag = "Mandates",
    operation_id = "Retrieve a Mandate",
    security(("api_key" = []))
)]
pub async fn get_mandate() {}

/// Mandates - Revoke Mandate
///
/// Revokes a mandate created using the Payments/Create API
#[utoipa::path(
    post,
    path = "/mandates/revoke/{mandate_id}",
    params(
        ("mandate_id" = String, Path, description = "The identifier for a mandate")
    ),
    responses(
        (status = 200, description = "The mandate was revoked successfully", body = MandateRevokedResponse),
        (status = 400, description = "Mandate does not exist in our records")
    ),
    tag = "Mandates",
    operation_id = "Revoke a Mandate",
    security(("api_key" = []))
)]
pub async fn revoke_mandate() {}

/// Mandates - List Mandates
#[utoipa::path(
    get,
    path = "/mandates/list",
    params(
        ("limit" = Option<i64>, Query, description = "The maximum number of Mandate Objects to include in the response"),
        ("mandate_status" = Option<MandateStatus>, Query, description = "The status of mandate"),
        ("connector" = Option<String>, Query, description = "The connector linked to mandate"),
        ("created_time" = Option<PrimitiveDateTime>, Query, description = "The time at which mandate is created"),
        ("created_time.lt" = Option<PrimitiveDateTime>, Query, description = "Time less than the mandate created time"),
        ("created_time.gt" = Option<PrimitiveDateTime>, Query, description = "Time greater than the mandate created time"),
        ("created_time.lte" = Option<PrimitiveDateTime>, Query, description = "Time less than or equals to the mandate created time"),
        ("created_time.gte" = Option<PrimitiveDateTime>, Query, description = "Time greater than or equals to the mandate created time"),
    ),
    responses(
        (status = 200, description = "The mandate list was retrieved successfully", body = Vec<MandateResponse>),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Mandates",
    operation_id = "List Mandates",
    security(("api_key" = []))
)]
pub async fn retrieve_mandates_list() {}
