/// API Key - Create
///
/// Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
/// displayed only once on creation, so ensure you store it securely.
#[utoipa::path(
    post,
    path = "/api_keys/{merchant_id)",
    params(("merchant_id" = String, Path, description = "The unique identifier for the merchant account")),
    request_body= CreateApiKeyRequest,
    responses(
        (status = 200, description = "API Key created", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "API Key",
    operation_id = "Create an API Key",
    security(("admin_api_key" = []))
)]
/// Asynchronously creates a new API key for accessing the system. 
pub async fn api_key_create() {}

/// API Key - Retrieve
///
/// Retrieve information about the specified API Key.
#[utoipa::path(
    get,
    path = "/api_keys/{merchant_id}/{key_id}",
    params (
        ("merchant_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("key_id" = String, Path, description = "The unique identifier for the API Key")
    ),
    responses(
        (status = 200, description = "API Key retrieved", body = RetrieveApiKeyResponse),
        (status = 404, description = "API Key not found")
    ),
    tag = "API Key",
    operation_id = "Retrieve an API Key",
    security(("admin_api_key" = []))
)]
/// Asynchronously retrieves an API key from the server. 
pub async fn api_key_retrieve() {
    // implementation details
}

/// API Key - Update
///
/// Update information for the specified API Key.
#[utoipa::path(
    post,
    path = "/api_keys/{merchant_id}/{key_id}",
    request_body = UpdateApiKeyRequest,
    params (
        ("merchant_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("key_id" = String, Path, description = "The unique identifier for the API Key")
    ),
    responses(
        (status = 200, description = "API Key updated", body = RetrieveApiKeyResponse),
        (status = 404, description = "API Key not found")
    ),
    tag = "API Key",
    operation_id = "Update an API Key",
    security(("admin_api_key" = []))
)]
/// Asynchronous function to update the API key.
pub async fn api_key_update() {
    // implementation details
}

/// API Key - Revoke
///
/// Revoke the specified API Key. Once revoked, the API Key can no longer be used for
/// authenticating with our APIs.
#[utoipa::path(
    delete,
    path = "/api_keys/{merchant_id)/{key_id}",
    params (
        ("merchant_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("key_id" = String, Path, description = "The unique identifier for the API Key")
    ),
    responses(
        (status = 200, description = "API Key revoked", body = RevokeApiKeyResponse),
        (status = 404, description = "API Key not found")
    ),
    tag = "API Key",
    operation_id = "Revoke an API Key",
    security(("admin_api_key" = []))
)]
/// Asynchronously revokes the API key for the current user.
pub async fn api_key_revoke() {
    // implementation goes here
}
