/// Routing - Create
///
/// Create a routing config
#[utoipa::path(
    post,
    path = "/routing",
    request_body = RoutingConfigRequest,
    responses(
        (status = 200, description = "Routing config created", body = RoutingDictionaryRecord),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Create a routing config",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously creates a routing configuration.
pub async fn routing_create_config() {
    // implementation goes here
}

/// Routing - Activate config
///
/// Activate a routing config
#[utoipa::path(
    post,
    path = "/routing/{algorithm_id}/activate",
    params(
        ("algorithm_id" = String, Path, description = "The unique identifier for a config"),
    ),
    responses(
        (status = 200, description = "Routing config activated", body = RoutingDictionaryRecord),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 400, description = "Bad request")
    ),
   tag = "Routing",
   operation_id = "Activate a routing config",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously retrieves the routing link configuration. This method is responsible for fetching the routing link configuration data, typically from a database or an external service, and returning it to the caller. 
pub async fn routing_link_config() {
    // method implementation here
}

/// Routing - Retrieve
///
/// Retrieve a routing algorithm

#[utoipa::path(
    get,
    path = "/routing/{algorithm_id}",
    params(
        ("algorithm_id" = String, Path, description = "The unique identifier for a config"),
    ),
    responses(
        (status = 200, description = "Successfully fetched routing config", body = MerchantRoutingAlgorithm),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 403, description = "Forbidden")
    ),
   tag = "Routing",
   operation_id = "Retrieve a routing config",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously retrieves the configuration for routing.
pub async fn routing_retrieve_config() {
    // method implementation goes here
}

/// Routing - List
///
/// List all routing configs
#[utoipa::path(
    get,
    path = "/routing",
    params(
        ("limit" = Option<u16>, Query, description = "The number of records to be returned"),
        ("offset" = Option<u8>, Query, description = "The record offset from which to start gathering of results"),
        ("profile_id" = Option<String>, Query, description = "The unique identifier for a merchant profile"),
    ),
    responses(
        (status = 200, description = "Successfully fetched routing configs", body = RoutingKind),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing")
    ),
   tag = "Routing",
   operation_id = "List routing configs",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously retrieves a list of routing configurations.
pub async fn list_routing_configs() {}

/// Routing - Deactivate
///
/// Deactivates a routing config
#[utoipa::path(
    post,
    path = "/routing/deactivate",
    request_body = RoutingConfigRequest,
    responses(
        (status = 200, description = "Successfully deactivated routing config", body = RoutingDictionaryRecord),
        (status = 500, description = "Internal server error"),
        (status = 400, description = "Malformed request"),
        (status = 403, description = "Malformed request"),
        (status = 422, description = "Unprocessable request")
    ),
   tag = "Routing",
   operation_id = "Deactivate a routing config",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously handles the routing for unlinking a configuration. This method is responsible for processing the request to unlink a configuration and updating the appropriate data in the system.
pub async fn routing_unlink_config() {}

/// Routing - Update Default Config
///
/// Update default fallback config
#[utoipa::path(
    post,
    path = "/routing/default",
    request_body = Vec<RoutableConnectorChoice>,
    responses(
        (status = 200, description = "Successfully updated default config", body = Vec<RoutableConnectorChoice>),
        (status = 500, description = "Internal server error"),
        (status = 400, description = "Malformed request"),
        (status = 422, description = "Unprocessable request")
    ),
   tag = "Routing",
   operation_id = "Update default fallback config",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously updates the default configuration for routing.
pub async fn routing_update_default_config() {
    // method implementation goes here
}

/// Routing - Retrieve Default Config
///
/// Retrieve default fallback config
#[utoipa::path(
    get,
    path = "/routing/default",
    responses(
        (status = 200, description = "Successfully retrieved default config", body = Vec<RoutableConnectorChoice>),
        (status = 500, description = "Internal server error")
    ),
   tag = "Routing",
   operation_id = "Retrieve default fallback config",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously retrieves the default configuration for routing.
pub async fn routing_retrieve_default_config() {
    // method implementation goes here
}

/// Routing - Retrieve Config
///
/// Retrieve active config
#[utoipa::path(
    get,
    path = "/routing/active",
    params(
        ("profile_id" = Option<String>, Query, description = "The unique identifier for a merchant profile"),
    ),
    responses(
        (status = 200, description = "Successfully retrieved active config", body = LinkedRoutingConfigRetrieveResponse),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 403, description = "Forbidden")
    ),
   tag = "Routing",
   operation_id = "Retrieve active config",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously retrieves the linked configuration for routing.
pub async fn routing_retrieve_linked_config() {
    // method implementation here
}

/// Routing - Retrieve Default For Profile
///
/// Retrieve default config for profiles
#[utoipa::path(
    get,
    path = "/routing/default/profile",
    responses(
        (status = 200, description = "Successfully retrieved default config", body = ProfileDefaultRoutingConfig),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing")
    ),
   tag = "Routing",
   operation_id = "Retrieve default configs for all profiles",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously retrieves the default configuration for profiles to determine the routing.
pub async fn routing_retrieve_default_config_for_profiles() {
    // method implementation here
}

/// Routing - Update Default For Profile
///
/// Update default config for profiles
#[utoipa::path(
    post,
    path = "/routing/default/profile/{profile_id}",
    request_body = Vec<RoutableConnectorChoice>,
    params(
        ("profile_id" = String, Path, description = "The unique identifier for a profile"),
    ),
    responses(
        (status = 200, description = "Successfully updated default config for profile", body = ProfileDefaultRoutingConfig),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 400, description = "Malformed request"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Update default configs for all profiles",
   security(("api_key" = []), ("jwt_key" = []))
)]
/// Asynchronously updates the default configuration for a profile's routing.
pub async fn routing_update_default_config_for_profile() {
    // method implementation goes here
}
