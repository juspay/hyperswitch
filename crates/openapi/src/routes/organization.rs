#[cfg(feature = "v1")]
/// Organization - Create
///
/// Create a new organization
#[utoipa::path(
    post,
    path = "/organization",
    request_body(
        content = OrganizationCreateRequest,
        examples(
            (
                "Create an organization with organization_name" = (
                    value = json!({"organization_name": "organization_abc"})
                )
            ),
        )
    ),
    responses(
        (status = 200, description = "Organization Created", body =OrganizationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "Create an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_create() {}

#[cfg(feature = "v1")]
/// Organization - Retrieve
///
/// Retrieve an existing organization
#[utoipa::path(
    get,
    path = "/organization/{id}",
    params (("id" = String, Path, description = "The unique identifier for the Organization")),
    responses(
        (status = 200, description = "Organization Created", body =OrganizationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "Retrieve an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_retrieve() {}

#[cfg(feature = "v1")]
/// Organization - Update
///
/// Create a new organization for .
#[utoipa::path(
    put,
    path = "/organization/{id}",
    request_body(
        content = OrganizationUpdateRequest,
        examples(
            (
                "Update organization_name of the organization" = (
                    value = json!({"organization_name": "organization_abcd"})
                )
            ),
        )
    ),
    params (("id" = String, Path, description = "The unique identifier for the Organization")),
    responses(
        (status = 200, description = "Organization Created", body =OrganizationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "Update an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_update() {}

#[cfg(feature = "v2")]
/// Organization - Create
///
/// Create a new organization
#[utoipa::path(
    post,
    path = "/v2/organizations",
    request_body(
        content = OrganizationCreateRequest,
        examples(
            (
                "Create an organization with organization_name" = (
                    value = json!({"organization_name": "organization_abc"})
                )
            ),
        )
    ),
    responses(
        (status = 200, description = "Organization Created", body =OrganizationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "Create an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_create() {}

#[cfg(feature = "v2")]
/// Organization - Retrieve
///
/// Retrieve an existing organization
#[utoipa::path(
    get,
    path = "/v2/organizations/{id}",
    params (("id" = String, Path, description = "The unique identifier for the Organization")),
    responses(
        (status = 200, description = "Organization Created", body =OrganizationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "Retrieve an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_retrieve() {}

#[cfg(feature = "v2")]
/// Organization - Update
///
/// Create a new organization for .
#[utoipa::path(
    put,
    path = "/v2/organizations/{id}",
    request_body(
        content = OrganizationUpdateRequest,
        examples(
            (
                "Update organization_name of the organization" = (
                    value = json!({"organization_name": "organization_abcd"})
                )
            ),
        )
    ),
    params (("id" = String, Path, description = "The unique identifier for the Organization")),
    responses(
        (status = 200, description = "Organization Created", body =OrganizationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "Update an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_update() {}

#[cfg(feature = "v2")]
/// Organization - Merchant Account - List
///
/// List merchant accounts for an Organization
#[utoipa::path(
    get,
    path = "/v2/organizations/{id}/merchant-accounts",
    params (("id" = String, Path, description = "The unique identifier for the Organization")),
    responses(
        (status = 200, description = "Merchant Account list retrieved successfully", body = Vec<MerchantAccountResponse>),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "List Merchant Accounts",
    security(("admin_api_key" = []))
)]
pub async fn merchant_account_list() {}
