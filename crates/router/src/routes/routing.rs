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
#[cfg(feature = "olap")]
#[instrument(skip_all)]
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
#[cfg(feature = "olap")]
#[instrument(skip_all)]
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
#[cfg(feature = "olap")]
#[instrument(skip_all)]
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

#[utoipa::path(
    get,
    path = "/routing",
    responses(
        (status = 200, description = "Successfully fetched routing dictionary", body = RoutingKind),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing")
    ),
   tag = "Routing",
   operation_id = "Retrieve routing dictionary",
   security(("api_key" = []), ("jwt_key" = []))
)]
#[cfg(feature = "olap")]
#[instrument(skip_all)]
pub async fn routing_retrieve_dictionary(
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
#[cfg(feature = "olap")]
#[instrument(skip_all)]
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
   operation_id = "Update default config",
   security(("api_key" = []), ("jwt_key" = []))
)]
#[cfg(feature = "olap")]
#[instrument(skip_all)]
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

#[utoipa::path(
    get,
    path = "/routing/default",
    responses(
        (status = 200, description = "Successfully retrieved default config", body = Vec<RoutableConnectorChoice>),
        (status = 500, description = "Internal server error")
    ),
   tag = "Routing",
   operation_id = "Retrieve default config",
   security(("api_key" = []), ("jwt_key" = []))
)]
#[cfg(feature = "olap")]
#[instrument(skip_all)]
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

#[utoipa::path(
    get,
    path = "/routing/active",
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
#[cfg(feature = "olap")]
#[instrument(skip_all)]
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
#[cfg(feature = "olap")]
#[instrument(skip_all)]
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
#[cfg(feature = "olap")]
#[instrument(skip_all)]
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
