use std::sync::Arc;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::Serialize;
use actix_web::{web, HttpRequest, HttpResponse};
use crate::{
    core::tokenization,
    core::api_locking,
    core::errors::{self, RouterResult},
    services::{self, api as api_service, authentication as auth},
    types::{
        api,
        domain,
        payment_methods as pm_types,
    },
    routes::{app::StorageInterface, SessionState, AppState}
};
use router_env::{instrument, tracing, Flow};
use hyperswitch_domain_models;
use api_models;
use common_utils::ext_traits::{BytesExt, Encode};


#[instrument(skip_all, fields(flow = ?Flow::TokenizationCreate))]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn create_token_vault_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<serde_json::Value>,
) -> HttpResponse {
    let flow = Flow::TokenizationCreate;
    let payload = json_payload.into_inner();

    Box::pin(api_service::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, request, _| async move {
            tokenization::create_vault_token_core(
                state,
                &auth.merchant_account,
                &auth.key_store,
                request,
            )
            .await
        },
        &auth::V2ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
