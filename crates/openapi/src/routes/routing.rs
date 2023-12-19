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
pub async fn routing_create_config() {}

#[utoipa::path(
    post,
    path = "/routing/{algorithm_id}/activate",
    params(
        ("algorithm_id" = String, Path, description = "The unique identifier for an algorithm"),
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

pub async fn routing_link_config() {}

#[utoipa::path(
    get,
    path = "/routing/{algorithm_id}",
    params(
        ("algorithm_id" = String, Path, description = "The unique identifier for an algorithm"),
    ),
    responses(
        (status = 200, description = "Successfully fetched routing algorithm", body = MerchantRoutingAlgorithm),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 403, description = "Forbidden")
    ),
   tag = "Routing",
   operation_id = "Retrieve a routing algorithm",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_retrieve_config() {}

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
pub async fn list_routing_configs() {}

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
pub async fn routing_unlink_config() {}

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
pub async fn routing_update_default_config() {}

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
pub async fn routing_retrieve_default_config() {}

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
pub async fn routing_retrieve_linked_config() {}

#[utoipa::path(
    get,
    path = "/routing/default/profile",
    responses(
        (status = 200, description = "Successfully retrieved default config", body = ProfileDefaultRoutingConfig),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing")
    ),
   tag = "Routing",
   operation_id = "Retrieve default config for profiles",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_retrieve_default_config_for_profiles() {}

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
   operation_id = "Update default config for profile",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_update_default_config_for_profile() {}
