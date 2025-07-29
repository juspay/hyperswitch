#[cfg(feature = "v1")]
/// Platform - Create
///
/// Create a new platform account
#[utoipa::path(
    post,
    path = "/create_platform",
    request_body(
        content = PlatformAccountCreateRequest,
        examples(
            (
                "Create a platform account with organization_name" = (
                    value = json!({"organization_name": "organization_abc"})
                )
            ),
        )
    ),
    responses(
        (status = 200, description = "Platform Account Created", body = PlatformAccountCreateResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Platform",
    operation_id = "Create a Platform Account",
    security(("jwt_key" = []))
)]
pub async fn create_platform_account() {}
