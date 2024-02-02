//! Analysis for usage of Routing in Payment flows
//!
//! Functions that are used to perform the api level configuration, retrieval, updation
//! of Routing configs.
use actix_web::{web, HttpRequest, Responder};
use api_models::routing as routing_types;
#[cfg(feature = "business_profile_routing")]
use api_models::routing::{RoutingRetrieveLinkQuery, RoutingRetrieveQuery};
use router_env::{
    tracing::{self, instrument},
    Flow,
};

use crate::{
    core::{api_locking, conditional_config, routing, surcharge_decision_config},
    routes::AppState,
    services::{api as oss_api, authentication as auth, authorization::permissions::Permission},
};

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// This method handles the creation of a routing configuration by extracting the necessary data from the request, authenticating the user, and then calling the `create_routing_config` method in the `routing` module to create the routing configuration. The method returns a responder that wraps the result of the operation.
pub async fn routing_create_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<routing_types::RoutingConfigRequest>,
) -> impl Responder {
    let flow = Flow::RoutingCreateConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload| {
            routing::create_routing_config(state, auth.merchant_account, auth.key_store, payload)
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RoutingWrite),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::RoutingWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// This method handles the routing link configuration by calling the link_routing_config function from the routing module. It takes the Appstate, HttpRequest, and RoutingAlgorithmId as input parameters and returns an implementation of Responder. The method uses server_wrap function from oss_api to wrap the flow, state, request, path, and other parameters for processing. It also performs authentication checks based on feature flags and permissions before processing the request. The method is asynchronous and uses the async keyword for the function definition.
pub async fn routing_link_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<routing_types::RoutingAlgorithmId>,
) -> impl Responder {
    let flow = Flow::RoutingLinkConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        path.into_inner(),
        |state, auth: auth::AuthenticationData, algorithm_id| {
            routing::link_routing_config(
                state,
                auth.merchant_account,
                #[cfg(not(feature = "business_profile_routing"))]
                auth.key_store,
                algorithm_id.0,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RoutingWrite),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::RoutingWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Asynchronously retrieves the routing configuration for a given routing algorithm ID. This method takes in the web app state, HTTP request, and the routing algorithm ID as input, and returns a Responder. It uses the oss_api::server_wrap function to handle the server logic for retrieving the routing configuration and authenticating the request. The method also performs API locking if applicable.
pub async fn routing_retrieve_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<routing_types::RoutingAlgorithmId>,
) -> impl Responder {
    let algorithm_id = path.into_inner();
    let flow = Flow::RoutingRetrieveConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        algorithm_id,
        |state, auth: auth::AuthenticationData, algorithm_id| {
            routing::retrieve_routing_config(state, auth.merchant_account, algorithm_id)
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RoutingRead),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::RoutingRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// This method is used to list the routing configurations based on the given query parameters. If the feature "business_profile_routing" is enabled, it retrieves the merchant routing dictionary using the provided query parameters. If the feature is not enabled, it retrieves the merchant routing dictionary without any query parameters.
pub async fn list_routing_configs(
    state: web::Data<AppState>,
    req: HttpRequest,
    #[cfg(feature = "business_profile_routing")] query: web::Query<RoutingRetrieveQuery>,
) -> impl Responder {
    #[cfg(feature = "business_profile_routing")]
    {
        let flow = Flow::RoutingRetrieveDictionary;
        Box::pin(oss_api::server_wrap(
            flow,
            state,
            &req,
            query.into_inner(),
            |state, auth: auth::AuthenticationData, query_params| {
                routing::retrieve_merchant_routing_dictionary(
                    state,
                    auth.merchant_account,
                    query_params,
                )
            },
            #[cfg(not(feature = "release"))]
            auth::auth_type(
                &auth::ApiKeyAuth,
                &auth::JWTAuth(Permission::RoutingRead),
                req.headers(),
            ),
            #[cfg(feature = "release")]
            &auth::JWTAuth(Permission::RoutingRead),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(not(feature = "business_profile_routing"))]
    {
        let flow = Flow::RoutingRetrieveDictionary;
        Box::pin(oss_api::server_wrap(
            flow,
            state,
            &req,
            (),
            |state, auth: auth::AuthenticationData, _| {
                routing::retrieve_merchant_routing_dictionary(state, auth.merchant_account)
            },
            #[cfg(not(feature = "release"))]
            auth::auth_type(
                &auth::ApiKeyAuth,
                &auth::JWTAuth(Permission::RoutingRead),
                req.headers(),
            ),
            #[cfg(feature = "release")]
            &auth::JWTAuth(Permission::RoutingRead),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Handles the routing unlink configuration API endpoint. If the 'business_profile_routing' feature is enabled, it takes in the routing configuration payload and passes it to the 'unlink_routing_config' function from the 'routing' module. Otherwise, it calls the same function with the authentication data from the request. It then wraps the result in the OSS API server and returns the response.
pub async fn routing_unlink_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    #[cfg(feature = "business_profile_routing")] payload: web::Json<
        routing_types::RoutingConfigRequest,
    >,
) -> impl Responder {
    #[cfg(feature = "business_profile_routing")]
    {
        let flow = Flow::RoutingUnlinkConfig;
        Box::pin(oss_api::server_wrap(
            flow,
            state,
            &req,
            payload.into_inner(),
            |state, auth: auth::AuthenticationData, payload_req| {
                routing::unlink_routing_config(state, auth.merchant_account, payload_req)
            },
            #[cfg(not(feature = "release"))]
            auth::auth_type(
                &auth::ApiKeyAuth,
                &auth::JWTAuth(Permission::RoutingWrite),
                req.headers(),
            ),
            #[cfg(feature = "release")]
            &auth::JWTAuth(Permission::RoutingWrite),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(not(feature = "business_profile_routing"))]
    {
        let flow = Flow::RoutingUnlinkConfig;
        Box::pin(oss_api::server_wrap(
            flow,
            state,
            &req,
            (),
            |state, auth: auth::AuthenticationData, _| {
                routing::unlink_routing_config(state, auth.merchant_account, auth.key_store)
            },
            #[cfg(not(feature = "release"))]
            auth::auth_type(
                &auth::ApiKeyAuth,
                &auth::JWTAuth(Permission::RoutingWrite),
                req.headers(),
            ),
            #[cfg(feature = "release")]
            &auth::JWTAuth(Permission::RoutingWrite),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Update the default configuration for routing with the given JSON payload of RoutableConnectorChoice. This method wraps the call to update_default_routing_config with the necessary authentication and locking checks.
pub async fn routing_update_default_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<Vec<routing_types::RoutableConnectorChoice>>,
) -> impl Responder {
    oss_api::server_wrap(
        Flow::RoutingUpdateDefaultConfig,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, updated_config| {
            routing::update_default_routing_config(state, auth.merchant_account, updated_config)
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RoutingWrite),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::RoutingWrite),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// This method handles the retrieval of the default routing configuration. It uses the oss_api::server_wrap function to wrap the logic for retrieving the default routing configuration. It takes the Appstate, HttpRequest, and an authentication data as input parameters and returns a Responder.
pub async fn routing_retrieve_default_config(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    oss_api::server_wrap(
        Flow::RoutingRetrieveDefaultConfig,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _| {
            routing::retrieve_default_routing_config(state, auth.merchant_account)
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RoutingRead),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::RoutingRead),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// This method is used to upsert a surcharge decision manager configuration. It takes in the App
/// State, HttpRequest, and a JSON payload containing the surcharge decision configuration request.
/// The method then uses the OSS API to wrap the flow, state, request, JSON payload, and authentication
/// data, and calls the upsert_surcharge_decision_config method to perform the upsert operation.
pub async fn upsert_surcharge_decision_manager_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_models::surcharge_decision_configs::SurchargeDecisionConfigReq>,
) -> impl Responder {
    let flow = Flow::DecisionManagerUpsertConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, update_decision| {
            surcharge_decision_config::upsert_surcharge_decision_config(
                state,
                auth.key_store,
                auth.merchant_account,
                update_decision,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::SurchargeDecisionManagerWrite),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::SurchargeDecisionManagerWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Deletes a surcharge decision manager configuration.
pub async fn delete_surcharge_decision_manager_config(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let flow = Flow::DecisionManagerDeleteConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, ()| {
            surcharge_decision_config::delete_surcharge_decision_config(
                state,
                auth.key_store,
                auth.merchant_account,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::SurchargeDecisionManagerWrite),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::SurchargeDecisionManagerWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Asynchronously retrieves the surcharge decision manager configuration. It wraps the retrieval process with server_wrap, which handles authorization, locking, and error handling.
pub async fn retrieve_surcharge_decision_manager_config(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let flow = Flow::DecisionManagerRetrieveConfig;
    oss_api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _| {
            surcharge_decision_config::retrieve_surcharge_decision_config(
                state,
                auth.merchant_account,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::SurchargeDecisionManagerRead),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::SurchargeDecisionManagerRead),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Asynchronously upserts a decision manager configuration using the provided JSON payload and authentication data.
pub async fn upsert_decision_manager_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_models::conditional_configs::DecisionManager>,
) -> impl Responder {
    let flow = Flow::DecisionManagerUpsertConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, update_decision| {
            conditional_config::upsert_conditional_config(
                state,
                auth.key_store,
                auth.merchant_account,
                update_decision,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::SurchargeDecisionManagerRead),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::SurchargeDecisionManagerRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Asynchronously deletes the decision manager configuration using the provided state and request.
pub async fn delete_decision_manager_config(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let flow = Flow::DecisionManagerDeleteConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, ()| {
            conditional_config::delete_conditional_config(
                state,
                auth.key_store,
                auth.merchant_account,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::SurchargeDecisionManagerWrite),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::SurchargeDecisionManagerWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Asynchronously retrieves the decision manager configuration. This method wraps the server_wrap function from the oss_api module to handle the retrieval of the conditional configuration based on the provided state and authentication data. It uses the Flow::DecisionManagerRetrieveConfig enum variant to identify the flow of the operation. Depending on the feature flag "release", it applies the appropriate authentication type and API locking action. The method returns an implementation of the Responder trait.
pub async fn retrieve_decision_manager_config(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let flow = Flow::DecisionManagerRetrieveConfig;
    oss_api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _| {
            conditional_config::retrieve_conditional_config(state, auth.merchant_account)
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::SurchargeDecisionManagerRead),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::SurchargeDecisionManagerRead),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Retrieves the linked routing configuration based on the feature flag "business_profile_routing".
/// If the feature flag is enabled, it uses the provided query parameters to retrieve the routing configuration with authentication.
/// If the feature flag is not enabled, it retrieves the routing configuration without any query parameters and authentication.
pub async fn routing_retrieve_linked_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    #[cfg(feature = "business_profile_routing")] query: web::Query<RoutingRetrieveLinkQuery>,
) -> impl Responder {
    #[cfg(feature = "business_profile_routing")]
    {
        use crate::services::authentication::AuthenticationData;
        let flow = Flow::RoutingRetrieveActiveConfig;
        Box::pin(oss_api::server_wrap(
            flow,
            state,
            &req,
            query.into_inner(),
            |state, auth: AuthenticationData, query_params| {
                routing::retrieve_linked_routing_config(state, auth.merchant_account, query_params)
            },
            #[cfg(not(feature = "release"))]
            auth::auth_type(
                &auth::ApiKeyAuth,
                &auth::JWTAuth(Permission::RoutingRead),
                req.headers(),
            ),
            #[cfg(feature = "release")]
            &auth::JWTAuth(Permission::RoutingRead),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }

    #[cfg(not(feature = "business_profile_routing"))]
    {
        let flow = Flow::RoutingRetrieveActiveConfig;
        Box::pin(oss_api::server_wrap(
            flow,
            state,
            &req,
            (),
            |state, auth: auth::AuthenticationData, _| {
                routing::retrieve_linked_routing_config(state, auth.merchant_account)
            },
            #[cfg(not(feature = "release"))]
            auth::auth_type(
                &auth::ApiKeyAuth,
                &auth::JWTAuth(Permission::RoutingRead),
                req.headers(),
            ),
            #[cfg(feature = "release")]
            &auth::JWTAuth(Permission::RoutingRead),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Retrieves the default routing configuration for profiles. This method handles the request asynchronously
/// and uses server_wrap to wrap the actual logic. Depending on the release feature, it applies different
/// authentication types using the auth_type method. The method returns a Responder.
pub async fn routing_retrieve_default_config_for_profiles(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    oss_api::server_wrap(
        Flow::RoutingRetrieveDefaultConfig,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _| {
            routing::retrieve_default_routing_config_for_profiles(state, auth.merchant_account)
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RoutingRead),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RoutingRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
/// Updates the default routing configuration for a specific profile using the provided JSON payload. This method requires the AppState, HttpRequest, profile ID, and JSON payload as input parameters. It then constructs a RoutingPayloadWrapper and passes it to the oss_api::server_wrap function, which handles authentication and calls the routing::update_default_routing_config_for_profile function to update the default routing configuration. Finally, it awaits the result and returns it as a Responder.
pub async fn routing_update_default_config_for_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<Vec<routing_types::RoutableConnectorChoice>>,
) -> impl Responder {
    let routing_payload_wrapper = routing_types::RoutingPayloadWrapper {
        updated_config: json_payload.into_inner(),
        profile_id: path.into_inner(),
    };
    oss_api::server_wrap(
        Flow::RoutingUpdateDefaultConfig,
        state,
        &req,
        routing_payload_wrapper,
        |state, auth: auth::AuthenticationData, wrapper| {
            routing::update_default_routing_config_for_profile(
                state,
                auth.merchant_account,
                wrapper.updated_config,
                wrapper.profile_id,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RoutingWrite),
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth(Permission::RoutingWrite),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
