use actix_web::{web, HttpRequest, Responder};
use api_models::verifications;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, errors, verification},
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::Verification))]
pub async fn apple_pay_merchant_registration(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<verifications::ApplepayMerchantVerificationRequest>,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> impl Responder {
    let flow = Flow::Verification;
    let merchant_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, body, _| {
            let merchant_id = merchant_id.clone();
            async move {
                match auth.platform.get_processor().get_account().get_id() {
                    authenticated_merchant_id if authenticated_merchant_id == &merchant_id => {
                        verification::verify_merchant_creds_for_applepay(
                            state.clone(),
                            body,
                            merchant_id,
                            auth.profile.map(|profile| profile.get_id().clone()),
                        )
                        .await
                    }
                    _ => Err(errors::ApiErrorResponse::Unauthorized.into()),
                }
            }
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileAccountWrite,
                allow_connected: true,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v2"))]
#[instrument(skip_all, fields(flow = ?Flow::Verification))]
pub async fn apple_pay_merchant_registration(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<verifications::ApplepayMerchantVerificationRequest>,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> impl Responder {
    let flow = Flow::Verification;
    let merchant_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, body, _| {
            let merchant_id = merchant_id.clone();
            async move {
                match auth.platform.get_processor().get_account().get_id() {
                    authenticated_merchant_id if authenticated_merchant_id == &merchant_id => {
                        verification::verify_merchant_creds_for_applepay(
                            state.clone(),
                            body,
                            merchant_id,
                            Some(auth.profile.get_id().clone()),
                        )
                        .await
                    }
                    _ => Err(errors::ApiErrorResponse::Unauthorized.into()),
                }
            }
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            },
            &auth::JWTAuth {
                permission: Permission::ProfileAccountWrite,
                allow_connected: true,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::Verification))]
pub async fn retrieve_apple_pay_verified_domains(
    state: web::Data<AppState>,
    req: HttpRequest,
    params: web::Query<verifications::ApplepayGetVerifiedDomainsParam>,
) -> impl Responder {
    let flow = Flow::Verification;
    let merchant_id = &params.merchant_id;
    let mca_id = &params.merchant_connector_account_id;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.clone(),
        |state, auth: auth::AuthenticationData, _, _| {
            let merchant_id = merchant_id.clone();
            let mca_id = mca_id.clone();
            async move {
                match auth.platform.get_processor().get_account().get_id() {
                    authenticated_merchant_id if authenticated_merchant_id == &merchant_id => {
                        #[cfg(feature = "v1")]
                        let profile_id = auth.profile.map(|profile| profile.get_id().clone());
                        #[cfg(feature = "v2")]
                        let profile_id = Some(auth.profile.get_id().clone());

                        verification::get_verified_apple_domains_with_mid_mca_id(
                            state,
                            merchant_id,
                            profile_id,
                            mca_id,
                        )
                        .await
                    }
                    _ => Err(errors::ApiErrorResponse::Unauthorized.into()),
                }
            }
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                allow_connected_scope_operation: false,
                allow_platform_self_operation: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountRead,
                allow_connected: true,
                allow_platform: false,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
