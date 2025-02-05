//! Analysis for usage of Routing in Payment flows
//!
//! Functions that are used to perform the api level configuration, retrieval, updation
//! of Routing configs.

use actix_web::{web, HttpRequest, Responder};
use api_models::{enums, routing as routing_types, routing::RoutingRetrieveQuery};
use router_env::{
    tracing::{self, instrument},
    Flow,
};

use crate::{
    core::{api_locking, conditional_config, routing, surcharge_decision_config},
    routes::AppState,
    services::{api as oss_api, authentication as auth, authorization::permissions::Permission},
};
#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn routing_create_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<routing_types::RoutingConfigRequest>,
    transaction_type: enums::TransactionType,
) -> impl Responder {
    let flow = Flow::RoutingCreateConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload, _| {
            routing::create_routing_algorithm_under_profile(
                state,
                auth.merchant_account,
                auth.key_store,
                auth.profile_id,
                payload,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileRoutingWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all)]
pub async fn routing_create_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<routing_types::RoutingConfigRequest>,
    transaction_type: enums::TransactionType,
) -> impl Responder {
    let flow = Flow::RoutingCreateConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload, _| {
            routing::create_routing_algorithm_under_profile(
                state,
                auth.merchant_account,
                auth.key_store,
                Some(auth.profile.get_id().clone()),
                payload,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileRoutingWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn routing_link_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::RoutingId>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    let flow = Flow::RoutingLinkConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        path.into_inner(),
        |state, auth: auth::AuthenticationData, algorithm, _| {
            routing::link_routing_config(
                state,
                auth.merchant_account,
                auth.key_store,
                auth.profile_id,
                algorithm,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileRoutingWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all)]
pub async fn routing_link_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
    json_payload: web::Json<routing_types::RoutingAlgorithmId>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    let flow = Flow::RoutingLinkConfig;
    let wrapper = routing_types::RoutingLinkWrapper {
        profile_id: path.into_inner(),
        algorithm_id: json_payload.into_inner(),
    };

    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        wrapper.clone(),
        |state, auth: auth::AuthenticationData, wrapper, _| {
            routing::link_routing_config_under_profile(
                state,
                auth.merchant_account,
                auth.key_store,
                wrapper.profile_id,
                wrapper.algorithm_id.routing_algorithm_id,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuthProfileFromRoute {
                profile_id: wrapper.profile_id,
                required_permission: Permission::MerchantRoutingWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuthProfileFromRoute {
            profile_id: wrapper.profile_id,
            required_permission: Permission::MerchantRoutingWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn routing_retrieve_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::RoutingId>,
) -> impl Responder {
    let algorithm_id = path.into_inner();
    let flow = Flow::RoutingRetrieveConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        algorithm_id,
        |state, auth: auth::AuthenticationData, algorithm_id, _| {
            routing::retrieve_routing_algorithm_from_algorithm_id(
                state,
                auth.merchant_account,
                auth.key_store,
                auth.profile_id,
                algorithm_id,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRoutingRead,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileRoutingRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all)]
pub async fn routing_retrieve_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::RoutingId>,
) -> impl Responder {
    let algorithm_id = path.into_inner();
    let flow = Flow::RoutingRetrieveConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        algorithm_id,
        |state, auth: auth::AuthenticationData, algorithm_id, _| {
            routing::retrieve_routing_algorithm_from_algorithm_id(
                state,
                auth.merchant_account,
                auth.key_store,
                Some(auth.profile.get_id().clone()),
                algorithm_id,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRoutingRead,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileRoutingRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
pub async fn list_routing_configs(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<RoutingRetrieveQuery>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    let flow = Flow::RoutingRetrieveDictionary;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        query.into_inner(),
        |state, auth: auth::AuthenticationData, query_params, _| {
            routing::retrieve_merchant_routing_dictionary(
                state,
                auth.merchant_account,
                None,
                query_params,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantRoutingRead,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::MerchantRoutingRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn list_routing_configs_for_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<RoutingRetrieveQuery>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    let flow = Flow::RoutingRetrieveDictionary;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        query.into_inner(),
        |state, auth: auth::AuthenticationData, query_params, _| {
            routing::retrieve_merchant_routing_dictionary(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                query_params,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRoutingRead,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileRoutingRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all)]
pub async fn routing_unlink_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    let flow = Flow::RoutingUnlinkConfig;
    let path = path.into_inner();
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        path.clone(),
        |state, auth: auth::AuthenticationData, path, _| {
            routing::unlink_routing_config_under_profile(
                state,
                auth.merchant_account,
                auth.key_store,
                path,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuthProfileFromRoute {
                profile_id: path,
                required_permission: Permission::MerchantRoutingWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuthProfileFromRoute {
            profile_id: path,
            required_permission: Permission::MerchantRoutingWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn routing_unlink_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<routing_types::RoutingConfigRequest>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    let flow = Flow::RoutingUnlinkConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload_req, _| {
            routing::unlink_routing_config(
                state,
                auth.merchant_account,
                auth.key_store,
                payload_req,
                auth.profile_id,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileRoutingWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all)]
pub async fn routing_update_default_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
    json_payload: web::Json<Vec<routing_types::RoutableConnectorChoice>>,
) -> impl Responder {
    let wrapper = routing_types::ProfileDefaultRoutingConfig {
        profile_id: path.into_inner(),
        connectors: json_payload.into_inner(),
    };
    Box::pin(oss_api::server_wrap(
        Flow::RoutingUpdateDefaultConfig,
        state,
        &req,
        wrapper,
        |state, auth: auth::AuthenticationData, wrapper, _| {
            routing::update_default_fallback_routing(
                state,
                auth.merchant_account,
                auth.key_store,
                wrapper.profile_id,
                wrapper.connectors,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantRoutingWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::MerchantRoutingWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn routing_update_default_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<Vec<routing_types::RoutableConnectorChoice>>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    Box::pin(oss_api::server_wrap(
        Flow::RoutingUpdateDefaultConfig,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, updated_config, _| {
            routing::update_default_routing_config(
                state,
                auth.merchant_account,
                updated_config,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantRoutingWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::MerchantRoutingWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all)]
pub async fn routing_retrieve_default_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
) -> impl Responder {
    let path = path.into_inner();
    Box::pin(oss_api::server_wrap(
        Flow::RoutingRetrieveDefaultConfig,
        state,
        &req,
        path.clone(),
        |state, auth: auth::AuthenticationData, profile_id, _| {
            routing::retrieve_default_fallback_algorithm_for_profile(
                state,
                auth.merchant_account,
                auth.key_store,
                profile_id,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuthProfileFromRoute {
                profile_id: path,
                required_permission: Permission::MerchantRoutingRead,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuthProfileFromRoute {
            profile_id: path,
            required_permission: Permission::MerchantRoutingRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn routing_retrieve_default_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    Box::pin(oss_api::server_wrap(
        Flow::RoutingRetrieveDefaultConfig,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            routing::retrieve_default_routing_config(
                state,
                auth.profile_id,
                auth.merchant_account,
                transaction_type,
            )
        },
        &auth::JWTAuth {
            permission: Permission::ProfileRoutingRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
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
        |state, auth: auth::AuthenticationData, update_decision, _| {
            surcharge_decision_config::upsert_surcharge_decision_config(
                state,
                auth.key_store,
                auth.merchant_account,
                update_decision,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantSurchargeDecisionManagerWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::MerchantSurchargeDecisionManagerWrite,
        },
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
        |state, auth: auth::AuthenticationData, (), _| {
            surcharge_decision_config::delete_surcharge_decision_config(
                state,
                auth.key_store,
                auth.merchant_account,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantSurchargeDecisionManagerWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::MerchantSurchargeDecisionManagerWrite,
        },
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
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            surcharge_decision_config::retrieve_surcharge_decision_config(
                state,
                auth.merchant_account,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantSurchargeDecisionManagerRead,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::MerchantSurchargeDecisionManagerRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
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
        |state, auth: auth::AuthenticationData, update_decision, _| {
            conditional_config::upsert_conditional_config(
                state,
                auth.key_store,
                auth.merchant_account,
                update_decision,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantThreeDsDecisionManagerWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::MerchantThreeDsDecisionManagerWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all)]
pub async fn upsert_decision_manager_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_models::conditional_configs::DecisionManagerRequest>,
) -> impl Responder {
    let flow = Flow::DecisionManagerUpsertConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, update_decision, _| {
            conditional_config::upsert_conditional_config(
                state,
                auth.key_store,
                update_decision,
                auth.profile,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileThreeDsDecisionManagerWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileThreeDsDecisionManagerWrite,
        },
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
        |state, auth: auth::AuthenticationData, (), _| {
            conditional_config::delete_conditional_config(
                state,
                auth.key_store,
                auth.merchant_account,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantThreeDsDecisionManagerWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::MerchantThreeDsDecisionManagerWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[cfg(feature = "olap")]
#[instrument(skip_all)]
pub async fn retrieve_decision_manager_config(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let flow = Flow::DecisionManagerRetrieveConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            conditional_config::retrieve_conditional_config(state, auth.key_store, auth.profile)
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileThreeDsDecisionManagerWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::ProfileThreeDsDecisionManagerWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[cfg(feature = "olap")]
#[instrument(skip_all)]
pub async fn retrieve_decision_manager_config(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let flow = Flow::DecisionManagerRetrieveConfig;
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            conditional_config::retrieve_conditional_config(state, auth.merchant_account)
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantThreeDsDecisionManagerRead,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuth {
            permission: Permission::MerchantThreeDsDecisionManagerRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn routing_retrieve_linked_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<routing_types::RoutingRetrieveLinkQuery>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    use crate::services::authentication::AuthenticationData;
    let flow = Flow::RoutingRetrieveActiveConfig;
    let query = query.into_inner();
    if let Some(profile_id) = query.profile_id.clone() {
        Box::pin(oss_api::server_wrap(
            flow,
            state,
            &req,
            query.clone(),
            |state, auth: AuthenticationData, query_params, _| {
                routing::retrieve_linked_routing_config(
                    state,
                    auth.merchant_account,
                    auth.key_store,
                    auth.profile_id,
                    query_params,
                    transaction_type,
                )
            },
            #[cfg(not(feature = "release"))]
            auth::auth_type(
                &auth::HeaderAuth(auth::ApiKeyAuth),
                &auth::JWTAuthProfileFromRoute {
                    profile_id,
                    required_permission: Permission::ProfileRoutingRead,
                },
                req.headers(),
            ),
            #[cfg(feature = "release")]
            &auth::JWTAuthProfileFromRoute {
                profile_id,
                required_permission: Permission::ProfileRoutingRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    } else {
        Box::pin(oss_api::server_wrap(
            flow,
            state,
            &req,
            query.clone(),
            |state, auth: AuthenticationData, query_params, _| {
                routing::retrieve_linked_routing_config(
                    state,
                    auth.merchant_account,
                    auth.key_store,
                    auth.profile_id,
                    query_params,
                    transaction_type,
                )
            },
            #[cfg(not(feature = "release"))]
            auth::auth_type(
                &auth::HeaderAuth(auth::ApiKeyAuth),
                &auth::JWTAuth {
                    permission: Permission::ProfileRoutingRead,
                },
                req.headers(),
            ),
            #[cfg(feature = "release")]
            &auth::JWTAuth {
                permission: Permission::ProfileRoutingRead,
            },
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all)]
pub async fn routing_retrieve_linked_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<RoutingRetrieveQuery>,
    path: web::Path<common_utils::id_type::ProfileId>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    use crate::services::authentication::AuthenticationData;
    let flow = Flow::RoutingRetrieveActiveConfig;
    let wrapper = routing_types::RoutingRetrieveLinkQueryWrapper {
        routing_query: query.into_inner(),
        profile_id: path.into_inner(),
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        wrapper.clone(),
        |state, auth: AuthenticationData, wrapper, _| {
            routing::retrieve_routing_config_under_profile(
                state,
                auth.merchant_account,
                auth.key_store,
                wrapper.routing_query,
                wrapper.profile_id,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuthProfileFromRoute {
                profile_id: wrapper.profile_id,
                required_permission: Permission::ProfileRoutingRead,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuthProfileFromRoute {
            profile_id: wrapper.profile_id,
            required_permission: Permission::ProfileRoutingRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
pub async fn routing_retrieve_default_config_for_profiles(
    state: web::Data<AppState>,
    req: HttpRequest,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    Box::pin(oss_api::server_wrap(
        Flow::RoutingRetrieveDefaultConfig,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            routing::retrieve_default_routing_config_for_profiles(
                state,
                auth.merchant_account,
                auth.key_store,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantRoutingRead,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantRoutingRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "olap")]
#[instrument(skip_all)]
pub async fn routing_update_default_config_for_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::ProfileId>,
    json_payload: web::Json<Vec<routing_types::RoutableConnectorChoice>>,
    transaction_type: &enums::TransactionType,
) -> impl Responder {
    let routing_payload_wrapper = routing_types::RoutingPayloadWrapper {
        updated_config: json_payload.into_inner(),
        profile_id: path.into_inner(),
    };
    Box::pin(oss_api::server_wrap(
        Flow::RoutingUpdateDefaultConfig,
        state,
        &req,
        routing_payload_wrapper.clone(),
        |state, auth: auth::AuthenticationData, wrapper, _| {
            routing::update_default_routing_config_for_profile(
                state,
                auth.merchant_account,
                auth.key_store,
                wrapper.updated_config,
                wrapper.profile_id,
                transaction_type,
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuthProfileFromRoute {
                profile_id: routing_payload_wrapper.profile_id,
                required_permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        #[cfg(feature = "release")]
        &auth::JWTAuthProfileFromRoute {
            profile_id: routing_payload_wrapper.profile_id,
            required_permission: Permission::ProfileRoutingWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn toggle_success_based_routing(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<api_models::routing::ToggleDynamicRoutingQuery>,
    path: web::Path<routing_types::ToggleDynamicRoutingPath>,
) -> impl Responder {
    let flow = Flow::ToggleDynamicRouting;
    let wrapper = routing_types::ToggleDynamicRoutingWrapper {
        feature_to_enable: query.into_inner().enable,
        profile_id: path.into_inner().profile_id,
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        wrapper.clone(),
        |state,
         auth: auth::AuthenticationData,
         wrapper: routing_types::ToggleDynamicRoutingWrapper,
         _| {
            routing::toggle_specific_dynamic_routing(
                state,
                auth.merchant_account,
                auth.key_store,
                wrapper.feature_to_enable,
                wrapper.profile_id,
                api_models::routing::DynamicRoutingType::SuccessRateBasedRouting,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuthProfileFromRoute {
                profile_id: wrapper.profile_id,
                required_permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn success_based_routing_update_configs(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<routing_types::DynamicRoutingUpdateConfigQuery>,
    json_payload: web::Json<routing_types::SuccessBasedRoutingConfig>,
) -> impl Responder {
    let flow = Flow::UpdateDynamicRoutingConfigs;
    let routing_payload_wrapper = routing_types::SuccessBasedRoutingPayloadWrapper {
        updated_config: json_payload.into_inner(),
        algorithm_id: path.clone().algorithm_id,
        profile_id: path.clone().profile_id,
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        routing_payload_wrapper.clone(),
        |state, _, wrapper: routing_types::SuccessBasedRoutingPayloadWrapper, _| async {
            Box::pin(routing::success_based_routing_update_configs(
                state,
                wrapper.updated_config,
                wrapper.algorithm_id,
                wrapper.profile_id,
            ))
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuthProfileFromRoute {
                profile_id: routing_payload_wrapper.profile_id,
                required_permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn contract_based_routing_setup_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<routing_types::ToggleDynamicRoutingPath>,
    query: web::Query<api_models::routing::ToggleDynamicRoutingQuery>,
    json_payload: Option<web::Json<routing_types::ContractBasedRoutingConfig>>,
) -> impl Responder {
    let flow = Flow::ToggleDynamicRouting;
    let routing_payload_wrapper = routing_types::ContractBasedRoutingSetupPayloadWrapper {
        config: json_payload.map(|json| json.into_inner()),
        profile_id: path.into_inner().profile_id,
        features_to_enable: query.into_inner().enable,
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        routing_payload_wrapper.clone(),
        |state,
         auth: auth::AuthenticationData,
         wrapper: routing_types::ContractBasedRoutingSetupPayloadWrapper,
         _| async move {
            Box::pin(routing::contract_based_dynamic_routing_setup(
                state,
                auth.key_store,
                auth.merchant_account,
                wrapper.profile_id,
                wrapper.features_to_enable,
                wrapper.config,
            ))
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuthProfileFromRoute {
                profile_id: routing_payload_wrapper.profile_id,
                required_permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn contract_based_routing_update_configs(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<routing_types::DynamicRoutingUpdateConfigQuery>,
    json_payload: web::Json<routing_types::ContractBasedRoutingConfig>,
) -> impl Responder {
    let flow = Flow::UpdateDynamicRoutingConfigs;
    let routing_payload_wrapper = routing_types::ContractBasedRoutingPayloadWrapper {
        updated_config: json_payload.into_inner(),
        algorithm_id: path.algorithm_id.clone(),
        profile_id: path.profile_id.clone(),
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        routing_payload_wrapper.clone(),
        |state,
         auth: auth::AuthenticationData,
         wrapper: routing_types::ContractBasedRoutingPayloadWrapper,
         _| async {
            Box::pin(routing::contract_based_routing_update_configs(
                state,
                wrapper.updated_config,
                auth.merchant_account,
                auth.key_store,
                wrapper.algorithm_id,
                wrapper.profile_id,
            ))
            .await
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuthProfileFromRoute {
                profile_id: routing_payload_wrapper.profile_id,
                required_permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn toggle_elimination_routing(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<api_models::routing::ToggleDynamicRoutingQuery>,
    path: web::Path<routing_types::ToggleDynamicRoutingPath>,
) -> impl Responder {
    let flow = Flow::ToggleDynamicRouting;
    let wrapper = routing_types::ToggleDynamicRoutingWrapper {
        feature_to_enable: query.into_inner().enable,
        profile_id: path.into_inner().profile_id,
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        wrapper.clone(),
        |state,
         auth: auth::AuthenticationData,
         wrapper: routing_types::ToggleDynamicRoutingWrapper,
         _| {
            routing::toggle_specific_dynamic_routing(
                state,
                auth.merchant_account,
                auth.key_store,
                wrapper.feature_to_enable,
                wrapper.profile_id,
                api_models::routing::DynamicRoutingType::EliminationRouting,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuthProfileFromRoute {
                profile_id: wrapper.profile_id,
                required_permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all)]
pub async fn set_dynamic_routing_volume_split(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<api_models::routing::DynamicRoutingVolumeSplitQuery>,
    path: web::Path<routing_types::ToggleDynamicRoutingPath>,
) -> impl Responder {
    let flow = Flow::VolumeSplitOnRoutingType;
    let routing_info = api_models::routing::RoutingVolumeSplit {
        routing_type: api_models::routing::RoutingType::Dynamic,
        split: query.into_inner().split,
    };
    let payload = api_models::routing::RoutingVolumeSplitWrapper {
        routing_info,
        profile_id: path.into_inner().profile_id,
    };

    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state,
         auth: auth::AuthenticationData,
         payload: api_models::routing::RoutingVolumeSplitWrapper,
         _| {
            routing::configure_dynamic_routing_volume_split(
                state,
                auth.merchant_account,
                auth.key_store,
                payload.profile_id,
                payload.routing_info,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuthProfileFromRoute {
                profile_id: payload.profile_id,
                required_permission: Permission::ProfileRoutingWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
