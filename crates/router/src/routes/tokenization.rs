#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use api_models;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use router_env::{instrument, tracing, Flow};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use crate::{
    core::{api_locking, tokenization},
    routes::AppState,
    services::{api as api_service, authentication as auth},
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
