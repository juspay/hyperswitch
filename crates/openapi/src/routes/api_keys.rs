#[cfg(feature = "v1")]
/// API Key - Create
///
/// Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
/// displayed only once on creation, so ensure you store it securely.
#[utoipa::path(
    post,
    path = "/api_keys/{merchant_id}",
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
pub async fn api_key_create() {}

#[cfg(feature = "v2")]
/// API Key - Create
///
/// Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
/// displayed only once on creation, so ensure you store it securely.
#[utoipa::path(
    post,
    path = "/v2/api-keys",
    request_body= CreateApiKeyRequest,
    responses(
        (status = 200, description = "API Key created", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "API Key",
    operation_id = "Create an API Key",
    security(("admin_api_key" = []))
)]
pub async fn api_key_create() {}

#[cfg(feature = "v1")]
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
pub async fn api_key_retrieve() {}

#[cfg(feature = "v2")]
/// API Key - Retrieve
///
/// Retrieve information about the specified API Key.
#[utoipa::path(
    get,
    path = "/v2/api-keys/{id}",
    params (
        ("id" = String, Path, description = "The unique identifier for the API Key")
    ),
    responses(
        (status = 200, description = "API Key retrieved", body = RetrieveApiKeyResponse),
        (status = 404, description = "API Key not found")
    ),
    tag = "API Key",
    operation_id = "Retrieve an API Key",
    security(("admin_api_key" = []))
)]
pub async fn api_key_retrieve() {}

#[cfg(feature = "v1")]
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
pub async fn api_key_update() {}

#[cfg(feature = "v2")]
/// API Key - Update
///
/// Update information for the specified API Key.
#[utoipa::path(
    put,
    path = "/v2/api-keys/{id}",
    request_body = UpdateApiKeyRequest,
    params (
        ("id" = String, Path, description = "The unique identifier for the API Key")
    ),
    responses(
        (status = 200, description = "API Key updated", body = RetrieveApiKeyResponse),
        (status = 404, description = "API Key not found")
    ),
    tag = "API Key",
    operation_id = "Update an API Key",
    security(("admin_api_key" = []))
)]
pub async fn api_key_update() {}

#[cfg(feature = "v1")]
/// API Key - Revoke
///
/// Revoke the specified API Key. Once revoked, the API Key can no longer be used for
/// authenticating with our APIs.
#[utoipa::path(
    delete,
    path = "/api_keys/{merchant_id}/{key_id}",
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
pub async fn api_key_revoke() {}

#[cfg(feature = "v2")]
/// API Key - Revoke
///
/// Revoke the specified API Key. Once revoked, the API Key can no longer be used for
/// authenticating with our APIs.
#[utoipa::path(
    delete,
    path = "/v2/api-keys/{id}",
    params (
        ("id" = String, Path, description = "The unique identifier for the API Key")
    ),
    responses(
        (status = 200, description = "API Key revoked", body = RevokeApiKeyResponse),
        (status = 404, description = "API Key not found")
    ),
    tag = "API Key",
    operation_id = "Revoke an API Key",
    security(("admin_api_key" = []))
)]
pub async fn api_key_revoke() {}

#[cfg(feature = "v1")]
/// API Key - List
///
/// List all the API Keys associated to a merchant account.
#[utoipa::path(
    get,
    path = "/api_keys/{merchant_id}/list",
    params(
        ("merchant_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("limit" = Option<i64>, Query, description = "The maximum number of API Keys to include in the response"),
        ("skip" = Option<i64>, Query, description = "The number of API Keys to skip when retrieving the list of API keys."),
    ),
    responses(
        (status = 200, description = "List of API Keys retrieved successfully", body = Vec<RetrieveApiKeyResponse>),
    ),
    tag = "API Key",
    operation_id = "List all API Keys associated with a merchant account",
    security(("admin_api_key" = []))
)]
pub async fn api_key_list() {}

#[cfg(feature = "v2")]
/// API Key - List
///
/// List all the API Keys associated to a merchant account.
#[utoipa::path(
    get,
    path = "/v2/api-keys/list",
    params(
        ("limit" = Option<i64>, Query, description = "The maximum number of API Keys to include in the response"),
        ("skip" = Option<i64>, Query, description = "The number of API Keys to skip when retrieving the list of API keys."),
    ),
    responses(
        (status = 200, description = "List of API Keys retrieved successfully", body = Vec<RetrieveApiKeyResponse>),
    ),
    tag = "API Key",
    operation_id = "List all API Keys associated with a merchant account",
    security(("admin_api_key" = []))
)]
pub async fn api_key_list() {}
