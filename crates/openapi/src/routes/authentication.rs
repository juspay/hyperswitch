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

/// Authentication - POST Eligibility Check
///
#[utoipa::path(
    post,
    path = "/authentication/{authentication_id}/eligibility-check",
    request_body = AuthenticationEligibilityCheckRequest,
    responses(
        (status = 200, description = "Eligibility Performed for the Authentication", body = AuthenticationEligibilityCheckResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Submit Eligibility for an Authentication",
    security(("publishable_key" = []))
)]
pub async fn authentication_eligibility_check() {}

/// Authentication - GET Eligibility Check
///
#[utoipa::path(
    get,
    path = "/authentication/{authentication_id}/eligibility-check",
    request_body = AuthenticationRetrieveEligibilityCheckRequest,
    responses(
        (status = 200, description = "Retrieved Eligibility check data for the Authentication", body = AuthenticationRetrieveEligibilityCheckResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Retrieve Eligibility Check data for an Authentication",
    security(("api_key" = []))
)]
pub async fn authentication_retrieve_eligibility_check() {}
