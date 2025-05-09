use actix_web::{web, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    self as app,
    core::{api_locking, three_ds_decision_rule as three_ds_decision_rule_core},
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::ThreeDsDecisionRuleCreate))]
#[cfg(feature = "olap")]
pub async fn create_three_ds_decision_rule(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<api_models::three_ds_decision_rule::ThreeDsDecisionRuleRecord>,
) -> impl Responder {
    let flow = Flow::ThreeDsDecisionRuleCreate;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let merchant_context = crate::types::domain::MerchantContext::NormalMerchant(Box::new(
                crate::types::domain::Context(auth.merchant_account, auth.key_store),
            ));
            three_ds_decision_rule_core::create_three_ds_decision_rule(state, merchant_context, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ThreeDsDecisionRuleRetrieve))]
#[cfg(feature = "olap")]
pub async fn retrieve_decision_rule(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<common_utils::id_type::ThreeDSDecisionRuleId>,
) -> impl Responder {
    let flow = Flow::ThreeDsDecisionRuleRetrieve;
    let rule_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            let merchant_context = crate::types::domain::MerchantContext::NormalMerchant(Box::new(
                crate::types::domain::Context(auth.merchant_account, auth.key_store),
            ));
            three_ds_decision_rule_core::retrieve_three_ds_decision_rule(
                state,
                merchant_context,
                &rule_id,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ThreeDsDecisionRuleUpdate))]
#[cfg(feature = "olap")]
pub async fn update_decision_rule(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<common_utils::id_type::ThreeDSDecisionRuleId>,
    payload: web::Json<api_models::three_ds_decision_rule::ThreeDsDecisionRuleUpdateRequest>,
) -> impl Responder {
    let flow = Flow::ThreeDsDecisionRuleUpdate;
    let rule_id = path.into_inner();
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let merchant_context = crate::types::domain::MerchantContext::NormalMerchant(Box::new(
                crate::types::domain::Context(auth.merchant_account, auth.key_store),
            ));
            three_ds_decision_rule_core::update_three_ds_decision_rule(
                state,
                merchant_context,
                &rule_id,
                req,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ThreeDsDecisionRuleDelete))]
#[cfg(feature = "olap")]
pub async fn delete_decision_rule(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<common_utils::id_type::ThreeDSDecisionRuleId>,
) -> impl Responder {
    let flow = Flow::ThreeDsDecisionRuleDelete;
    let rule_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            let merchant_context = crate::types::domain::MerchantContext::NormalMerchant(Box::new(
                crate::types::domain::Context(auth.merchant_account, auth.key_store),
            ));
            three_ds_decision_rule_core::delete_three_ds_decision_rule(
                state,
                merchant_context,
                &rule_id,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ThreeDsDecisionRuleExecute))]
#[cfg(feature = "olap")]
pub async fn execute_decision_rule(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<common_utils::id_type::ThreeDSDecisionRuleId>,
    payload: web::Json<api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteRequest>,
) -> impl Responder {
    let flow = Flow::ThreeDsDecisionRuleExecute;
    let rule_id = path.into_inner();
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let merchant_context = crate::types::domain::MerchantContext::NormalMerchant(Box::new(
                crate::types::domain::Context(auth.merchant_account, auth.key_store),
            ));
            three_ds_decision_rule_core::execute_three_ds_decision_rule(
                state,
                merchant_context,
                &rule_id,
                req,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
