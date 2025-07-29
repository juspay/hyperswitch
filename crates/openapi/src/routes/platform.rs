#[cfg(feature = "v1")]
/// Platform - Create
///
/// Create a new platform account
#[utoipa::path(
    post,
    path = "/user/create_platform",
    request_body(
        content = PlatformAccountCreateRequest,
        description = "Create a platform account with organization_name",
        examples(
            (
                "Create a platform account with organization_name" = (
                    value = json!({"organization_name": "organization_abc"})
                )
            )
        )
    ),
    responses(
        (
            status = 200,
            description = "Platform Account Created",
            body = PlatformAccountCreateResponse,
            examples(
                (
                    "Successful Platform Account Creation" = (
                        description = "Return values for a successfully created platform account",
                        value = json!({
                            "org_id": "org_abc",
                            "org_name": "organization_abc",
                            "org_type": "platform",
                            "merchant_id": "merchant_abc",
                            "merchant_account_type": "platform"
                        })
                    )
                )
            )
        ),
        (status = 400, description = "Invalid data")
    ),
    tag = "Platform",
    operation_id = "Create a Platform Account",
    security(("jwt_key" = []))
)]
pub async fn create_platform_account() {}
