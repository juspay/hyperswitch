//! Analysis for usage of Routing in Payment flows
//!
//! Functions that are used to perform the api level configuration, retrieval, updation
//! of Routing configs.
use actix_web::{web, HttpRequest, Responder};
#[cfg(feature = "business_profile_routing")]
use api_models::routing::{RoutingRetrieveLinkQuery, RoutingRetrieveQuery};
use api_models::{enums, routing as routing_types};
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
pub async fn routing_create_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<routing_types::RoutingConfigRequest>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    let flow = Flow::RoutingCreateConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload| {
            routing::create_routing_config(
                state,
                auth.merchant_account,
                auth.key_store,
                payload,
                transaction_type,
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
pub async fn routing_link_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<routing_types::RoutingAlgorithmId>,
    transaction_type: &enums::TransactionType,
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
                transaction_type,
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
pub async fn list_routing_configs(
    state: web::Data<AppState>,
    req: HttpRequest,
    #[cfg(feature = "business_profile_routing")] query: web::Query<RoutingRetrieveQuery>,
    transaction_type: &enums::TransactionType,
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
                    transaction_type,
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
pub async fn routing_unlink_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    #[cfg(feature = "business_profile_routing")] payload: web::Json<
        routing_types::RoutingConfigRequest,
    >,
    transaction_type: &enums::TransactionType,
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
                routing::unlink_routing_config(
                    state,
                    auth.merchant_account,
                    payload_req,
                    transaction_type,
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

    #[cfg(not(feature = "business_profile_routing"))]
    {
        let flow = Flow::RoutingUnlinkConfig;
        Box::pin(oss_api::server_wrap(
            flow,
            state,
            &req,
            (),
            |state, auth: auth::AuthenticationData, _| {
                routing::unlink_routing_config(
                    state,
                    auth.merchant_account,
                    auth.key_store,
                    transaction_type,
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
}

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
