#[cfg(feature = "v1")]
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
pub async fn routing_create_config() {}

#[cfg(feature = "v2")]
/// Routing - Create
///
/// Create a routing algorithm
#[utoipa::path(
    post,
    path = "/v2/routing-algorithms",
    request_body = RoutingConfigRequest,
    responses(
        (status = 200, description = "Routing Algorithm created", body = RoutingDictionaryRecord),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Create a routing algorithm",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_create_config() {}

#[cfg(feature = "v1")]
/// Routing - Activate config
///
/// Activate a routing config
#[utoipa::path(
    post,
    path = "/routing/{routing_algorithm_id}/activate",
    params(
        ("routing_algorithm_id" = String, Path, description = "The unique identifier for a config"),
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

#[cfg(feature = "v1")]
/// Routing - Retrieve
///
/// Retrieve a routing algorithm
#[utoipa::path(
    get,
    path = "/routing/{routing_algorithm_id}",
    params(
        ("routing_algorithm_id" = String, Path, description = "The unique identifier for a config"),
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
pub async fn routing_retrieve_config() {}

#[cfg(feature = "v2")]
/// Routing - Retrieve
///
/// Retrieve a routing algorithm with its algorithm id
#[utoipa::path(
    get,
    path = "/v2/routing-algorithms/{id}",
    params(
        ("id" = String, Path, description = "The unique identifier for a routing algorithm"),
    ),
    responses(
        (status = 200, description = "Successfully fetched routing algorithm", body = MerchantRoutingAlgorithm),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 403, description = "Forbidden")
    ),
   tag = "Routing",
   operation_id = "Retrieve a routing algorithm with its algorithm id",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn routing_retrieve_config() {}

#[cfg(feature = "v1")]
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
pub async fn list_routing_configs() {}

#[cfg(feature = "v1")]
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
pub async fn routing_unlink_config() {}

#[cfg(feature = "v1")]
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
pub async fn routing_update_default_config() {}

#[cfg(feature = "v1")]
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
pub async fn routing_retrieve_default_config() {}

#[cfg(feature = "v1")]
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
pub async fn routing_retrieve_linked_config() {}

#[cfg(feature = "v1")]
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
pub async fn routing_retrieve_default_config_for_profiles() {}

#[cfg(feature = "v1")]
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
pub async fn routing_update_default_config_for_profile() {}

#[cfg(feature = "v1")]
/// Routing - Toggle success based dynamic routing for profile
///
/// Create a success based dynamic routing algorithm
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/success_based/toggle",
    params(
        ("account_id" = String, Path, description = "Merchant id"),
        ("profile_id" = String, Path, description = "Profile id under which Dynamic routing needs to be toggled"),
        ("enable" = DynamicRoutingFeatures, Query, description = "Feature to enable for success based routing"),
    ),
    responses(
        (status = 200, description = "Routing Algorithm created", body = RoutingDictionaryRecord),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Toggle success based dynamic routing algorithm",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn toggle_success_based_routing() {}

#[cfg(feature = "v1")]
/// Routing - Auth Rate Based
///
/// Create a success based dynamic routing algorithm
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/success_based/create",
    request_body = SuccessBasedRoutingConfig,
    params(
        ("account_id" = String, Path, description = "Merchant id"),
        ("profile_id" = String, Path, description = "Profile id under which Dynamic routing needs to be created"),
        ("enable" = DynamicRoutingFeatures, Query, description = "Feature to enable for success based routing"),
    ),
    responses(
        (status = 200, description = "Routing Algorithm created", body = RoutingDictionaryRecord),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Create success based dynamic routing algorithm",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn create_success_based_routing() {}

#[cfg(feature = "v1")]
/// Routing - Update success based dynamic routing config for profile
///
/// Update success based dynamic routing algorithm
#[utoipa::path(
    patch,
    path = "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/success_based/config/{algorithm_id}",
    params(
        ("account_id" = String, Path, description = "Merchant id"),
        ("profile_id" = String, Path, description = "Profile id under which Dynamic routing needs to be toggled"),
        ("algorithm_id" = String, Path, description = "Success based routing algorithm id which was last activated to update the config"),
    ),
    request_body = SuccessBasedRoutingConfig,
    responses(
        (status = 200, description = "Routing Algorithm updated", body = RoutingDictionaryRecord),
        (status = 400, description = "Update body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Update success based dynamic routing configs",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn success_based_routing_update_configs() {}

#[cfg(feature = "v1")]
/// Routing - Toggle elimination routing for profile
///
/// Create a elimination based dynamic routing algorithm
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/elimination/toggle",
    params(
        ("account_id" = String, Path, description = "Merchant id"),
        ("profile_id" = String, Path, description = "Profile id under which Dynamic routing needs to be toggled"),
        ("enable" = DynamicRoutingFeatures, Query, description = "Feature to enable for elimination based routing"),
    ),
    responses(
        (status = 200, description = "Routing Algorithm created", body = RoutingDictionaryRecord),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Toggle elimination routing algorithm",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn toggle_elimination_routing() {}

#[cfg(feature = "v1")]
/// Routing - Elimination
///
/// Create a elimination based dynamic routing algorithm
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/elimination/create",
    request_body = EliminationRoutingConfig,
    params(
        ("account_id" = String, Path, description = "Merchant id"),
        ("profile_id" = String, Path, description = "Profile id under which Dynamic routing needs to be created"),
        ("enable" = DynamicRoutingFeatures, Query, description = "Feature to enable for elimination based routing"),
    ),
    responses(
        (status = 200, description = "Routing Algorithm created", body = RoutingDictionaryRecord),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Create elimination routing algorithm",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn create_elimination_routing() {}

#[cfg(feature = "v1")]
/// Routing - Toggle Contract routing for profile
///
/// Create a Contract based dynamic routing algorithm
#[utoipa::path(
    post,
    path = "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/contracts/toggle",
    params(
        ("account_id" = String, Path, description = "Merchant id"),
        ("profile_id" = String, Path, description = "Profile id under which Dynamic routing needs to be toggled"),
        ("enable" = DynamicRoutingFeatures, Query, description = "Feature to enable for contract based routing"),
    ),
    request_body = ContractBasedRoutingConfig,
    responses(
        (status = 200, description = "Routing Algorithm created", body = RoutingDictionaryRecord),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Toggle contract routing algorithm",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn contract_based_routing_setup_config() {}

#[cfg(feature = "v1")]
/// Routing - Update contract based dynamic routing config for profile
///
/// Update contract based dynamic routing algorithm
#[utoipa::path(
    patch,
    path = "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/contracts/config/{algorithm_id}",
    params(
        ("account_id" = String, Path, description = "Merchant id"),
        ("profile_id" = String, Path, description = "Profile id under which Dynamic routing needs to be toggled"),
        ("algorithm_id" = String, Path, description = "Contract based routing algorithm id which was last activated to update the config"),
    ),
    request_body = ContractBasedRoutingConfig,
    responses(
        (status = 200, description = "Routing Algorithm updated", body = RoutingDictionaryRecord),
        (status = 400, description = "Update body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Update contract based dynamic routing configs",
   security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn contract_based_routing_update_configs() {}

#[cfg(feature = "v1")]
/// Routing - Evaluate
///
/// Evaluate routing rules
#[utoipa::path(
    post,
    path = "/routing/evaluate",
    request_body = OpenRouterDecideGatewayRequest,
    responses(
        (status = 200, description = "Routing rules evaluated successfully", body = DecideGatewayResponse),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Evaluate routing rules",
   security(("api_key" = []))
)]
pub async fn call_decide_gateway_open_router() {}

#[cfg(feature = "v1")]
/// Routing - Feedback
///
/// Update gateway scores for dynamic routing
#[utoipa::path(
    post,
    path = "/routing/feedback",
    request_body = UpdateScorePayload,
    responses(
        (status = 200, description = "Gateway score updated successfully", body = UpdateScoreResponse),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Update gateway scores",
   security(("api_key" = []))
)]
pub async fn call_update_gateway_score_open_router() {}

#[cfg(feature = "v1")]
/// Routing - Rule Evaluate
///
/// Evaluate routing rules
#[utoipa::path(
    post,
    path = "/routing/rule/evaluate",
    request_body = RoutingEvaluateRequest,
    responses(
        (status = 200, description = "Routing rules evaluated successfully", body = RoutingEvaluateResponse),
        (status = 400, description = "Request body is malformed"),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Routing",
   operation_id = "Evaluate routing rules (alternative)",
   security(("api_key" = []))
)]
pub async fn evaluate_routing_rule() {}
