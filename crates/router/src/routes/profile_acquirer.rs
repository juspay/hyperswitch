use actix_web::{web, HttpRequest, HttpResponse};
use api_models::profile_acquirer::ProfileAcquirerCreate;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::api_locking,
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::domain,
};

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::ProfileAcquirerCreate))]
pub async fn create_profile_acquirer(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<ProfileAcquirerCreate>,
) -> HttpResponse {
    let flow = Flow::ProfileAcquirerCreate;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state: super::SessionState, auth_data, req, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth_data.merchant_account, auth_data.key_store),
            ));
            crate::core::profile_acquirer::create_profile_acquirer(
                state,
                req,
                merchant_context.clone(),
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: true,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileAccountWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
