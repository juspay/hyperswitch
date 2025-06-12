#[cfg(feature = "v1")]
/// Profile Acquirer - Create
///
/// Create a new Profile Acquirer for accessing our APIs from your servers.
#[utoipa::path(
    post,
    path = "/profile_acquirers",
    request_body = ProfileAcquirerCreate,
    responses(
        (status = 200, description = "Profile Acquirer created", body = ProfileAcquirerResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile Acquirer",
    operation_id = "Create a Profile Acquirer",
    security(("api_key" = []))
)]
pub async fn profile_acquirer_create() { /* … */
}

#[cfg(feature = "v1")]
/// Profile Acquirer - Update
///
/// Update a Profile Acquirer for accessing our APIs from your servers.
#[utoipa::path(
    post,
    path = "/profile_acquirers/{profile_id}/{profile_acquirer_id}",
    params (
        ("profile_id" = String, Path, description = "The unique identifier for the Profile"),
        ("profile_acquirer_id" = String, Path, description = "The unique identifier for the Profile Acquirer")
    ),
    request_body = ProfileAcquirerUpdate,
    responses(
        (status = 200, description = "Profile Acquirer updated", body = ProfileAcquirerResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile Acquirer",
    operation_id = "Update a Profile Acquirer",
    security(("api_key" = []))
)]
pub async fn profile_acquirer_update() { /* … */
}
