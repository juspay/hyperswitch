#[cfg(feature = "v1")]
/// Business Profile - Create
///
/// Creates a new *business profile* for a merchant
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile",
    params (
        ("account_id" = String, Path, description = "The unique identifier for the merchant account")
    ),
    request_body(
        content = BusinessProfileCreate,
        examples(
            (
                "Create a business profile with minimal fields" = (
                    value = json!({})
                )
            ),
            (
                "Create a business profile with profile name" = (
                    value = json!({
                        "profile_name": "shoe_business"
                    })
                )
            )
        )
    ),
    responses(
        (status = 200, description = "Business Account Created", body = BusinessProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Create A Business Profile",
    security(("admin_api_key" = []))
)]
pub async fn business_profile_create() {}

#[cfg(feature = "v2")]
/// Business Profile - Create
///
/// Creates a new *business profile* for a merchant
#[utoipa::path(
    post,
    path = "/v2/profiles",
    request_body(
        content = BusinessProfileCreate,
        examples(
            (
                "Create a business profile with profile name" = (
                    value = json!({
                        "profile_name": "shoe_business"
                    })
                )
            )
        )
    ),
    responses(
        (status = 200, description = "Business Account Created", body = BusinessProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Create A Business Profile",
    security(("admin_api_key" = []))
)]
pub async fn business_profile_create() {}

/// Business Profile - List
///
/// Lists all the *business profiles* under a merchant
#[utoipa::path(
    get,
    path = "/account/{account_id}/business_profile",
    params (
        ("account_id" = String, Path, description = "Merchant Identifier"),
    ),
    responses(
        (status = 200, description = "Business profiles Retrieved", body = Vec<BusinessProfileResponse>)
    ),
    tag = "Business Profile",
    operation_id = "List Business Profiles",
    security(("api_key" = []))
)]
pub async fn business_profile_list() {}

#[cfg(feature = "v1")]
/// Business Profile - Update
///
/// Update the *business profile*
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile/{profile_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("profile_id" = String, Path, description = "The unique identifier for the business profile")
    ),
    request_body(
        content = BusinessProfileCreate,
        examples(
            (
                "Update business profile with profile name fields" = (
                    value = json!({
                        "profile_name" : "shoe_business"
                    })
                )
            )
    )),
    responses(
        (status = 200, description = "Business Profile Updated", body = BusinessProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Update a Business Profile",
    security(("admin_api_key" = []))
)]
pub async fn business_profile_update() {}

#[cfg(feature = "v2")]
/// Business Profile - Update
///
/// Update the *business profile*
#[utoipa::path(
    put,
    path = "/v2/profiles/{profile_id}",
    params(
        ("profile_id" = String, Path, description = "The unique identifier for the business profile")
    ),
    request_body(
        content = BusinessProfileCreate,
        examples(
            (
                "Update business profile with profile name fields" = (
                    value = json!({
                        "profile_name" : "shoe_business"
                    })
                )
            )
    )),
    responses(
        (status = 200, description = "Business Profile Updated", body = BusinessProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Update a Business Profile",
    security(("admin_api_key" = []))
)]
pub async fn business_profile_update() {}

/// Business Profile - Delete
///
/// Delete the *business profile*
#[utoipa::path(
    delete,
    path = "/account/{account_id}/business_profile/{profile_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("profile_id" = String, Path, description = "The unique identifier for the business profile")
    ),
    responses(
        (status = 200, description = "Business profiles Deleted", body = bool),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Delete the Business Profile",
    security(("api_key" = []))
)]
pub async fn business_profile_delete() {}

#[cfg(feature = "v1")]
/// Business Profile - Retrieve
///
/// Retrieve existing *business profile*
#[utoipa::path(
    get,
    path = "/account/{account_id}/business_profile/{profile_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("profile_id" = String, Path, description = "The unique identifier for the business profile")
    ),
    responses(
        (status = 200, description = "Business Profile Updated", body = BusinessProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Retrieve a Business Profile",
    security(("admin_api_key" = []))
)]
pub async fn business_profile_retrieve() {}

#[cfg(feature = "v2")]
/// Business Profile - Retrieve
///
/// Retrieve existing *business profile*
#[utoipa::path(
    get,
    path = "/v2/profiles/{profile_id}",
    params(
        ("profile_id" = String, Path, description = "The unique identifier for the business profile")
    ),
    responses(
        (status = 200, description = "Business Profile Updated", body = BusinessProfileResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Business Profile",
    operation_id = "Retrieve a Business Profile",
    security(("admin_api_key" = []))
)]
pub async fn business_profile_retrieve() {}

#[cfg(feature = "v2")]
/// Business Profile - Retrieve Active Routing Algorithm
///
/// Retrieve active routing algorithm under the business profile
#[utoipa::path(
    get,
    path = "/v2/profiles/{profile_id}/routing_algorithm",
    params(
        ("profile_id" = String, Path, description = "The unique identifier for the business profile"),
        ("limit" = Option<u16>, Query, description = "The number of records of the algorithms to be returned"),
        ("offset" = Option<u8>, Query, description = "The record offset of the algorithm from which to start gathering the results")),
    responses(
        (status = 200, description = "Successfully retrieved active config", body = LinkedRoutingConfigRetrieveResponse),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 403, description = "Forbidden")
    ),
   tag = "Business Profile",
   operation_id = "Retrieve the active routing algorithm under the business profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_retrieve_linked_config() {}
#[cfg(feature = "v2")]
/// Business Profile - Activate routing algorithm
///
/// Activates a routing algorithm under a business profile
#[utoipa::path(
    patch,
    path = "/v2/profiles/{profile_id}/activate_routing_algorithm",
    request_body ( content = RoutingAlgorithmId,
      examples(  (
            "Activate a routing algorithm" = (
                value = json!({
                    "routing_algorithm_id": "routing_algorithm_123"
                })
            )
            ))),
    params(
        ("profile_id" = String, Path, description = "The unique identifier for the business profile"),
    ),
    responses(
        (status = 200, description = "Routing Algorithm is activated", body = RoutingDictionaryRecord),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 400, description = "Bad request")
    ),
   tag = "Business Profile",
   operation_id = "Activates a routing algorithm under a business profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_link_config() {}

#[cfg(feature = "v2")]
/// Business Profile - Deactivate routing algorithm
///
/// Deactivates a routing algorithm under a business profile
#[utoipa::path(
    patch,
    path = "/v2/profiles/{profile_id}/deactivate_routing_algorithm",
    params(
        ("profile_id" = String, Path, description = "The unique identifier for the business profile"),
    ),
    responses(
        (status = 200, description = "Successfully deactivated routing config", body = RoutingDictionaryRecord),
        (status = 500, description = "Internal server error"),
        (status = 400, description = "Malformed request"),
        (status = 403, description = "Malformed request"),
        (status = 422, description = "Unprocessable request")
    ),
   tag = "Business Profile",
   operation_id = " Deactivates a routing algorithm under a business profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_unlink_config() {}

#[cfg(feature = "v2")]
/// Business Profile - Update Default Fallback Routing Algorithm
///
/// Update the default fallback routing algorithm for the business profile
#[utoipa::path(
    post,
    path = "/v2/profiles/{profile_id}/fallback_routing",
    request_body = Vec<RoutableConnectorChoice>,
    params(
        ("profile_id" = String, Path, description = "The unique identifier for the business profile"),
    ),
    responses(
        (status = 200, description = "Successfully updated the default fallback routing algorithm", body = Vec<RoutableConnectorChoice>),
        (status = 500, description = "Internal server error"),
        (status = 400, description = "Malformed request"),
        (status = 422, description = "Unprocessable request")
    ),
   tag = "Business Profile",
   operation_id = "Update the default fallback routing algorithm for the business profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_update_default_config() {}

#[cfg(feature = "v2")]
/// Business Profile - Retrieve Default Fallback Routing Algorithm
///
/// Retrieve the default fallback routing algorithm for the business profile
#[utoipa::path(
    get,
    path = "/v2/profiles/{profile_id}/fallback_routing",
    params(
        ("profile_id" = String, Path, description = "The unique identifier for the business profile"),
    ),
    responses(
        (status = 200, description = "Successfully retrieved default fallback routing algorithm", body = Vec<RoutableConnectorChoice>),
        (status = 500, description = "Internal server error")
    ),
   tag = "Business Profile",
   operation_id = "Retrieve the default fallback routing algorithm for the business profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_retrieve_default_config() {}

/// Merchant Connector - List
///
/// List Merchant Connector Details for the business profile
#[utoipa::path(
    get,
    path = "/v2/profiles/{profile_id}/connector_accounts",
    params(
        ("profile_id" = String, Path, description = "The unique identifier for the business profile"),
    ),
    responses(
        (status = 200, description = "Merchant Connector list retrieved successfully", body = Vec<MerchantConnectorResponse>),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Business Profile",
    operation_id = "List all Merchant Connectors",
    security(("admin_api_key" = []))
)]
#[cfg(feature = "v2")]
pub async fn connector_list() {}
