use actix_web::{web, HttpRequest, HttpResponse};
use api_models::merchant_acquirer::{MerchantAcquirerCreate, MerchantAcquirerUpdate};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::api_locking,
    services::{api, authentication as auth, authorization::permissions},
    types::domain,
};

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::MerchantAcquirerCreate))]
pub async fn create_merchant_acquirer(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<MerchantAcquirerCreate>,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> HttpResponse {
    let flow = Flow::MerchantAcquirerCreate;
    let payload = json_payload.into_inner();
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state: super::SessionState, auth_data, req, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth_data.merchant_account, auth_data.key_store),
            ));
            crate::core::merchant_acquirer::create_merchant_acquirer(
                state,
                req,
                merchant_context.clone(),
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuthWithMerchantIdFromRoute(merchant_id.clone())),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: permissions::Permission::MerchantAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::MerchantAcquirerUpdate))]
pub async fn update_merchant_acquirer(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<MerchantAcquirerUpdate>,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::MerchantAcquirerId,
    )>,
) -> HttpResponse {
    let flow = Flow::MerchantAcquirerUpdate;
    let payload = json_payload.into_inner();
    let (merchant_id, merchant_acquirer_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state: super::SessionState, _auth_data, req, _| {
            crate::core::merchant_acquirer::update_merchant_acquirer(
                state,
                req,
                merchant_acquirer_id.clone(),
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuthWithMerchantIdFromRoute(merchant_id.clone())),
            &auth::JWTAuthMerchantFromRoute {
                merchant_id,
                required_permission: permissions::Permission::MerchantAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
