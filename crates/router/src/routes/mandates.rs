use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, mandate},
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api::mandates,
};

/// Mandates - Retrieve Mandate
///
/// Retrieves a mandate created using the Payments/Create API
#[instrument(skip_all, fields(flow = ?Flow::MandatesRetrieve))]
// #[get("/{id}")]
pub async fn get_mandate(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::MandatesRetrieve;
    let mandate_id = mandates::MandateId {
        mandate_id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        mandate_id,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            mandate::get_mandate(state, platform, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::MandatesRevoke))]
pub async fn revoke_mandate(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::MandatesRevoke;
    let mandate_id = mandates::MandateId {
        mandate_id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        mandate_id,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            mandate::revoke_mandate(state, platform, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Mandates - List Mandates
#[instrument(skip_all, fields(flow = ?Flow::MandatesList))]
pub async fn retrieve_mandates_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Query<api_models::mandates::MandateListConstraints>,
) -> HttpResponse {
    let flow = Flow::MandatesList;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            mandate::retrieve_mandates_list(state, platform, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantMandateRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
