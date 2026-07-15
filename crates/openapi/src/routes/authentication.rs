/// Authentication - Create
///
/// Create a new authentication for accessing our APIs from your servers.
///
#[utoipa::path(
    post,
    path = "/authentication",
    request_body = api_models::authentication::AuthenticationCreateRequest,
    responses(
        (status = 200, description = "Authentication created", body = api_models::authentication::AuthenticationResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Create an Authentication",
    security(("api_key" = []))
)]
pub async fn authentication_create() {}

/// Authentication - Eligibility
///
/// Check if an authentication is eligible for a specific merchant.
///
#[utoipa::path(
    post,
    path = "/authentication/{authentication_id}/eligibility",
    request_body = api_models::authentication::AuthenticationEligibilityRequest,
    responses(
        (status = 200, description = "Authentication eligibility checked", body = api_models::authentication::AuthenticationEligibilityResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Check Authentication Eligibility",
    security(("api_key" = []), ("publishable_key" = []))
)]
#[cfg(feature = "v1")]
pub async fn authentication_eligibility() {}

/// Authentication - Authenticate
///
/// Authenticate an authentication for accessing our APIs from your servers.
///
#[utoipa::path(
    post,
    path = "/authentication/{authentication_id}/authenticate",
    request_body = api_models::authentication::AuthenticationAuthenticateRequest,
    responses(
        (status = 200, description = "Authentication authenticated", body = api_models::authentication::AuthenticationAuthenticateResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Authenticate an Authentication",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub async fn authentication_authenticate() {}

/// Authentication - Redirect
///
/// Redirect an authentication for accessing our APIs from your servers.
///
#[utoipa::path(
    post,
    path = "/authentication/{authentication_id}/redirect",
    request_body = api_models::authentication::AuthenticationSyncPostUpdateRequest,
    responses(
        (status = 200, description = "Authentication redirect"),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Redirect an Authentication",
    security()
)]
pub async fn authentication_redirect() {}

/// Authentication - Sync
///
/// Sync an authentication for accessing our APIs from your servers.
///
#[utoipa::path(
    post,
    path = "/authentication/{authentication_id}/sync",
    request_body = api_models::authentication::AuthenticationSyncRequest,
    responses(
        (status = 200, description = "Authentication sync", body = api_models::authentication::AuthenticationSyncResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Sync an Authentication",
    security(("api_key" = []), ("publishable_key" = []))
)]
#[cfg(feature = "v1")]
pub async fn authentication_sync() {}

/// Authentication - Enable Authn Methods Token
///
/// Enable authn methods token for an authentication.
///
#[utoipa::path(
    post,
    path = "/authentication/{authentication_id}/enabled_authn_methods_token",
    request_body = api_models::authentication::AuthenticationSessionTokenRequest,
    responses(
        (status = 200, description = "Authentication enabled authn methods token", body = api_models::authentication::AuthenticationSessionResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Enable Authentication Authn Methods Token",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub async fn authentication_enabled_authn_methods_token() {}

/// Authentication - POST Eligibility Check
///
#[utoipa::path(
    post,
    path = "/authentication/{authentication_id}/eligibility-check",
    request_body = api_models::authentication::AuthenticationEligibilityCheckRequest,
    responses(
        (status = 200, description = "Eligibility Performed for the Authentication", body = api_models::authentication::AuthenticationEligibilityCheckResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Submit Eligibility for an Authentication",
    security(("publishable_key" = []))
)]
#[cfg(feature = "v1")]
pub async fn authentication_eligibility_check() {}

/// Authentication - GET Eligibility Check
///
#[utoipa::path(
    get,
    path = "/authentication/{authentication_id}/eligibility-check",
    request_body = api_models::authentication::AuthenticationRetrieveEligibilityCheckRequest,
    responses(
        (status = 200, description = "Retrieved Eligibility check data for the Authentication", body = api_models::authentication::AuthenticationRetrieveEligibilityCheckResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Authentication",
    operation_id = "Retrieve Eligibility Check data for an Authentication",
    security(("api_key" = []))
)]
#[cfg(feature = "v1")]
pub async fn authentication_retrieve_eligibility_check() {}
