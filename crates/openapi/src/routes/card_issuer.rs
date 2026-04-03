/// Card Issuer - Create
///
/// Creates a new card issuer entry
#[utoipa::path(
    post,
    path = "/card_issuers",
    request_body(
        content = CardIssuerRequest,
    ),
    responses(
        (status = 200, description = "Card issuer created", body = CardIssuerResponse),
        (status = 400, description = "Missing or invalid fields"),
        (status = 409, description = "Card issuer already exists"),
    ),
    tag = "Card Issuer",
    operation_id = "Create Card Issuer",
    security(("admin_api_key" = [])),
)]
pub async fn add_card_issuer() {}

/// Card Issuer - Update
///
/// Updates an existing card issuer entry
#[utoipa::path(
    put,
    path = "/card_issuers/{id}",
    params(
        ("id" = String, Path, description = "The unique identifier for the card issuer"),
    ),
    request_body(
        content = CardIssuerUpdateRequest,
    ),
    responses(
        (status = 200, description = "Card issuer updated", body = CardIssuerResponse),
        (status = 404, description = "Card issuer not found"),
        (status = 409, description = "Card issuer with this name already exists"),
    ),
    tag = "Card Issuer",
    operation_id = "Update Card Issuer",
    security(("admin_api_key" = [])),
)]
pub async fn update_card_issuer() {}

/// Card Issuer - List
///
/// Lists card issuers with optional search filter
#[utoipa::path(
    get,
    path = "/card_issuers",
    params(
        ("query" = Option<String>, Query, description = "Optional search term to filter issuers by name"),
        ("limit" = Option<u8>, Query, description = "Maximum number of results to return"),
    ),
    responses(
        (status = 200, description = "Card issuers listed", body = CardIssuerListResponse),
    ),
    tag = "Card Issuer",
    operation_id = "List Card Issuers",
    security(("api_key" = []), ("jwt_key" = [])),
)]
pub async fn list_card_issuers() {}
