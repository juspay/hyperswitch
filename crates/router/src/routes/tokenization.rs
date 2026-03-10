#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use std::sync::Arc;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use common_utils::{
    ext_traits::{BytesExt, Encode},
    id_type,
};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use error_stack::ResultExt;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use masking::Secret;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use router_env::{instrument, logger, tracing, Flow};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use serde::Serialize;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use crate::{
    core::{
        api_locking,
        errors::{self, RouterResult},
        tokenization,
    },
    headers::X_CUSTOMER_ID,
    routes::{app::StorageInterface, AppState, SessionState},
    services::{self, api as api_service, authentication as auth},
    types::{api, domain, payment_methods as pm_types},
};

#[instrument(skip_all, fields(flow = ?Flow::TokenizationCreate))]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn create_token_vault_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_models::tokenization::GenericTokenizationRequest>,
) -> HttpResponse {
    let flow = Flow::TokenizationCreate;
    let payload = json_payload.into_inner();
    let customer_id = payload.customer_id.clone();
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
        auth::api_or_client_auth(
            &auth::V2ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            },
            &auth::V2ClientAuth(common_utils::types::authentication::ResourceId::Customer(
                customer_id,
            )),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::TokenizationDelete))]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn delete_tokenized_data_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::GlobalTokenId>,
    json_payload: web::Json<api_models::tokenization::DeleteTokenDataRequest>,
) -> HttpResponse {
    let flow = Flow::TokenizationDelete;
    let payload = json_payload.into_inner();
    let session_id = payload.session_id.clone();
    let token_id = path.into_inner();

    Box::pin(api_service::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            tokenization::delete_tokenized_data_core(state, platform, &token_id, req)
        },
        auth::api_or_client_auth(
            &auth::V2ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            },
            &auth::V2ClientAuth(
                common_utils::types::authentication::ResourceId::PaymentMethodSession(session_id),
            ),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
