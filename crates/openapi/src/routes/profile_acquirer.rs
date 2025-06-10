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
/// Profile Acquirer - List
///
/// List all Profile Acquirers for a specific Profile.
#[utoipa::path(
    get,
    path = "/profile_acquirers/{profile_id}",
    responses(
        (status = 200, description = "Profile Acquirers listed", body = Vec<ProfileAcquirerResponse>),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile Acquirer",
    operation_id = "List Profile Acquirers",
    security(("api_key" = []))
)]
pub async fn profile_acquirer_list() { /* … */
}

#[cfg(feature = "v1")]
/// Profile Acquirer - Update
///
/// Update a Profile Acquirer for accessing our APIs from your servers.
#[utoipa::path(
    post,
    path = "/profile_acquirers/{profile_id}/{profile_acquirer_id}",
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
