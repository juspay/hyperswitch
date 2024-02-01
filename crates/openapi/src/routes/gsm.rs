/// Gsm - Create
///
/// Creates a GSM (Global Status Mapping) Rule. A GSM rule is used to map a connector's error message/error code combination during a particular payments flow/sub-flow to Hyperswitch's unified status/error code/error message combination. It is also used to decide the next action in the flow - retry/requeue/do_default
#[utoipa::path(
    post,
    path = "/gsm",
    request_body(
        content = GsmCreateRequest,
    ),
    responses(
        (status = 200, description = "Gsm created", body = GsmResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Gsm",
    operation_id = "Create Gsm Rule",
    security(("admin_api_key" = [])),
)]
/// Asynchronously creates a GSM (Global System for Mobile Communications) rule.
pub async fn create_gsm_rule() {
    // method implementation goes here
}

/// Gsm - Get
///
/// Retrieves a Gsm Rule
#[utoipa::path(
    post,
    path = "/gsm/get",
    request_body(
        content = GsmRetrieveRequest,
    ),
    responses(
        (status = 200, description = "Gsm retrieved", body = GsmResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Gsm",
    operation_id = "Retrieve Gsm Rule",
    security(("admin_api_key" = [])),
)]
/// Asynchronously retrieves the GSM (Global System for Mobile communications) rule.
pub async fn get_gsm_rule() {
    // method implementation goes here
}

/// Gsm - Update
///
/// Updates a Gsm Rule
#[utoipa::path(
    post,
    path = "/gsm/update",
    request_body(
        content = GsmUpdateRequest,
    ),
    responses(
        (status = 200, description = "Gsm updated", body = GsmResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Gsm",
    operation_id = "Update Gsm Rule",
    security(("admin_api_key" = [])),
)]
/// Asynchronously updates the GSM rule.
pub async fn update_gsm_rule() {
    // method implementation here
}

/// Gsm - Delete
///
/// Deletes a Gsm Rule
#[utoipa::path(
    post,
    path = "/gsm/delete",
    request_body(
        content = GsmDeleteRequest,
    ),
    responses(
        (status = 200, description = "Gsm deleted", body = GsmDeleteResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Gsm",
    operation_id = "Delete Gsm Rule",
    security(("admin_api_key" = [])),
)]
/// Asynchronously deletes a GSM rule.
pub async fn delete_gsm_rule() {
    // implementation goes here
}
