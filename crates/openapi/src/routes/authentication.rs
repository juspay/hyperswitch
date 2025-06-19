/// Authentication - Create
///
/// Create a new authentication for accessing our APIs from your servers.
///
#[utoipa::path(
    post,
    path = "/authentication",
    request_body = AuthenticationCreateRequest,
    responses(
        (status = 200, description = "Authentication created", body = AuthenticationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Create an Authentication",
    security(("api_key" = []))
)]
pub async fn authentication_create() {}
