#[cfg(any(feature = "v1", feature = "v2"))]
/// Organization - Create
///
/// Create a new organization
#[utoipa::path(
    post,
    path = "/organization",
    request_body(
        content = OrganizationRequest,
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

#[cfg(any(feature = "v1", feature = "v2"))]
/// Organization - Retrieve
///
/// Retrieve an existing organization
#[utoipa::path(
    get,
    path = "/organization/{organization_id}",
    params (("organization_id" = String, Path, description = "The unique identifier for the Organization")),
    responses(
        (status = 200, description = "Organization Created", body =OrganizationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "Retrieve an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_retrieve() {}

#[cfg(any(feature = "v1", feature = "v2"))]
/// Organization - Update
///
/// Create a new organization for .
#[utoipa::path(
    put,
    path = "/organization/{organization_id}",
    request_body(
        content = OrganizationRequest,
        examples(
            (
                "Update organization_name of the organization" = (
                    value = json!({"organization_name": "organization_abcd"})
                )
            ),
        )
    ),
    params (("organization_id" = String, Path, description = "The unique identifier for the Organization")),
    responses(
        (status = 200, description = "Organization Created", body =OrganizationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "Update an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_update() {}
