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

/// Organization - Retrieve
///
/// Retrieve an existing organization
#[utoipa::path(
    post,
    path = "/organization/{organization_id}",
    params (("organization_id" = String, Path, description = "The unique identifier for the Organization")),
    responses(
        (status = 200, description = "Organization Created", body =OrganizationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Organization",
    operation_id = "Create an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_retrieve() {}

/// Organization - Update
///
/// Create a new organization for .
#[utoipa::path(
    post,
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
    operation_id = "Create an Organization",
    security(("admin_api_key" = []))
)]
pub async fn organization_update() {}
