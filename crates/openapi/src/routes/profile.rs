// ******************************************** V1 profile routes ******************************************** //

#[cfg(feature = "v1")]
/// Profile - Create
///
/// Creates a new *profile* for a merchant
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile",
    params (
        ("account_id" = String, Path, description = "The unique identifier for the merchant account")
    ),
    request_body(
        content = ProfileCreate,
        examples(
            (
                "Create a profile with minimal fields" = (
                    value = json!({})
                )
            ),
            (
                "Create a profile with profile name" = (
                    value = json!({
                        "profile_name": "shoe_business"
                    })
                )
            )
        )
    ),
    responses(
        (status = 200, description = "Profile Created", body = ProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile",
    operation_id = "Create A Profile",
    security(("api_key" = []))
)]
pub async fn profile_create() {}

#[cfg(feature = "v1")]
/// Profile - Update
///
/// Update the *profile*
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile/{profile_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("profile_id" = String, Path, description = "The unique identifier for the profile")
    ),
    request_body(
        content = ProfileCreate,
        examples(
            (
                "Update profile with profile name fields" = (
                    value = json!({
                        "profile_name" : "shoe_business"
                    })
                )
            )
    )),
    responses(
        (status = 200, description = "Profile Updated", body = ProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile",
    operation_id = "Update a Profile",
    security(("api_key" = []))
)]
pub async fn profile_update() {}

#[cfg(feature = "v1")]
/// Profile - Retrieve
///
/// Retrieve existing *profile*
#[utoipa::path(
    get,
    path = "/account/{account_id}/business_profile/{profile_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("profile_id" = String, Path, description = "The unique identifier for the profile")
    ),
    responses(
        (status = 200, description = "Profile Updated", body = ProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile",
    operation_id = "Retrieve a Profile",
    security(("api_key" = []))
)]
pub async fn profile_retrieve() {}

// ******************************************** Common profile routes ******************************************** //

/// Profile - Delete
///
/// Delete the *profile*
#[utoipa::path(
    delete,
    path = "/account/{account_id}/business_profile/{profile_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("profile_id" = String, Path, description = "The unique identifier for the profile")
    ),
    responses(
        (status = 200, description = "Profiles Deleted", body = bool),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile",
    operation_id = "Delete the Profile",
    security(("admin_api_key" = []))
)]
pub async fn profile_delete() {}

/// Profile - List
///
/// Lists all the *profiles* under a merchant
#[utoipa::path(
    get,
    path = "/account/{account_id}/business_profile",
    params (
        ("account_id" = String, Path, description = "Merchant Identifier"),
    ),
    responses(
        (status = 200, description = "Profiles Retrieved", body = Vec<ProfileResponse>)
    ),
    tag = "Profile",
    operation_id = "List Profiles",
    security(("api_key" = []))
)]
pub async fn profile_list() {}

// ******************************************** V2 profile routes ******************************************** //

#[cfg(feature = "v2")]
/// Profile - Create
///
/// Creates a new *profile* for a merchant
#[utoipa::path(
    post,
    path = "/v2/profiles",
    params(
        (
            "X-Merchant-Id" = String, Header,
            description = "Merchant ID of the profile.",
            example = json!({"X-Merchant-Id": "abc_iG5VNjsN9xuCg7Xx0uWh"})
        ),
    ),
    request_body(
        content = ProfileCreate,
        examples(
            (
                "Create a profile with profile name" = (
                    value = json!({
                        "profile_name": "shoe_business"
                    })
                )
            )
        )
    ),
    responses(
        (status = 200, description = "Account Created", body = ProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile",
    operation_id = "Create A Profile",
    security(("admin_api_key" = []))
)]
pub async fn profile_create() {}

#[cfg(feature = "v2")]
/// Profile - Update
///
/// Update the *profile*
#[utoipa::path(
    put,
    path = "/v2/profiles/{id}",
    params(
        ("id" = String, Path, description = "The unique identifier for the profile"),
        (
            "X-Merchant-Id" = String, Header,
            description = "Merchant ID of the profile.",
            example = json!({"X-Merchant-Id": "abc_iG5VNjsN9xuCg7Xx0uWh"})
        ),
    ),
    request_body(
        content = ProfileCreate,
        examples(
            (
                "Update profile with profile name fields" = (
                    value = json!({
                        "profile_name" : "shoe_business"
                    })
                )
            )
    )),
    responses(
        (status = 200, description = "Profile Updated", body = ProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile",
    operation_id = "Update a Profile",
    security(("admin_api_key" = []))
)]
pub async fn profile_update() {}

#[cfg(feature = "v2")]
/// Profile - Activate routing algorithm
///
/// Activates a routing algorithm under a profile
#[utoipa::path(
    patch,
    path = "/v2/profiles/{id}/activate-routing-algorithm",
    request_body ( content = RoutingAlgorithmId,
      examples(  (
            "Activate a routing algorithm" = (
                value = json!({
                    "routing_algorithm_id": "routing_algorithm_123"
                })
            )
            ))),
    params(
        ("id" = String, Path, description = "The unique identifier for the profile"),
    ),
    responses(
        (status = 200, description = "Routing Algorithm is activated", body = RoutingDictionaryRecord),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 400, description = "Bad request")
    ),
   tag = "Profile",
   operation_id = "Activates a routing algorithm under a profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_link_config() {}

#[cfg(feature = "v2")]
/// Profile - Deactivate routing algorithm
///
/// Deactivates a routing algorithm under a profile
#[utoipa::path(
    patch,
    path = "/v2/profiles/{id}/deactivate-routing-algorithm",
    params(
        ("id" = String, Path, description = "The unique identifier for the profile"),
    ),
    responses(
        (status = 200, description = "Successfully deactivated routing config", body = RoutingDictionaryRecord),
        (status = 500, description = "Internal server error"),
        (status = 400, description = "Malformed request"),
        (status = 403, description = "Malformed request"),
        (status = 422, description = "Unprocessable request")
    ),
   tag = "Profile",
   operation_id = " Deactivates a routing algorithm under a profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_unlink_config() {}

#[cfg(feature = "v2")]
/// Profile - Update Default Fallback Routing Algorithm
///
/// Update the default fallback routing algorithm for the profile
#[utoipa::path(
    patch,
    path = "/v2/profiles/{id}/fallback-routing",
    request_body = Vec<RoutableConnectorChoice>,
    params(
        ("id" = String, Path, description = "The unique identifier for the profile"),
    ),
    responses(
        (status = 200, description = "Successfully updated the default fallback routing algorithm", body = Vec<RoutableConnectorChoice>),
        (status = 500, description = "Internal server error"),
        (status = 400, description = "Malformed request"),
        (status = 422, description = "Unprocessable request")
    ),
   tag = "Profile",
   operation_id = "Update the default fallback routing algorithm for the profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_update_default_config() {}

#[cfg(feature = "v2")]
/// Profile - Retrieve
///
/// Retrieve existing *profile*
#[utoipa::path(
    get,
    path = "/v2/profiles/{id}",
    params(
        ("id" = String, Path, description = "The unique identifier for the profile"),
        (
            "X-Merchant-Id" = String, Header,
            description = "Merchant ID of the profile.",
            example = json!({"X-Merchant-Id": "abc_iG5VNjsN9xuCg7Xx0uWh"})
        ),
    ),
    responses(
        (status = 200, description = "Profile Updated", body = ProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Profile",
    operation_id = "Retrieve a Profile",
    security(("admin_api_key" = []))
)]
pub async fn profile_retrieve() {}

#[cfg(feature = "v2")]
/// Profile - Retrieve Active Routing Algorithm
///_
/// Retrieve active routing algorithm under the profile
#[utoipa::path(
    get,
    path = "/v2/profiles/{id}/routing-algorithm",
    params(
        ("id" = String, Path, description = "The unique identifier for the profile"),
        ("limit" = Option<u16>, Query, description = "The number of records of the algorithms to be returned"),
        ("offset" = Option<u8>, Query, description = "The record offset of the algorithm from which to start gathering the results")),
    responses(
        (status = 200, description = "Successfully retrieved active config", body = LinkedRoutingConfigRetrieveResponse),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 403, description = "Forbidden")
    ),
   tag = "Profile",
   operation_id = "Retrieve the active routing algorithm under the profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_retrieve_linked_config() {}

#[cfg(feature = "v2")]
/// Profile - Retrieve Default Fallback Routing Algorithm
///
/// Retrieve the default fallback routing algorithm for the profile
#[utoipa::path(
    get,
    path = "/v2/profiles/{id}/fallback-routing",
    params(
        ("id" = String, Path, description = "The unique identifier for the profile"),
    ),
    responses(
        (status = 200, description = "Successfully retrieved default fallback routing algorithm", body = Vec<RoutableConnectorChoice>),
        (status = 500, description = "Internal server error")
    ),
   tag = "Profile",
   operation_id = "Retrieve the default fallback routing algorithm for the profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_retrieve_default_config() {}

/// Profile - Connector Accounts List
///
/// List Connector Accounts for the profile
#[utoipa::path(
    get,
    path = "/v2/profiles/{id}/connector-accounts",
    params(
        ("id" = String, Path, description = "The unique identifier for the business profile"),
        (
            "X-Merchant-Id" = String, Header,
            description = "Merchant ID of the profile.",
            example = json!({"X-Merchant-Id": "abc_iG5VNjsN9xuCg7Xx0uWh"})
        ),
    ),
    responses(
        (status = 200, description = "Merchant Connector list retrieved successfully", body = Vec<MerchantConnectorResponse>),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Business Profile",
    operation_id = "List all Merchant Connectors for Profile",
    security(("admin_api_key" = []))
)]
#[cfg(feature = "v2")]
pub async fn connector_list() {}
