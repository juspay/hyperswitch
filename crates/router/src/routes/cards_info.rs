use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use api_models::cards_info as cards_info_api_types;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, cards_info},
    services::{api, authentication as auth},
};

#[cfg(feature = "v1")]
/// Cards Info - Retrieve
///
/// Retrieve the card information given the card bin
#[utoipa::path(
    get,
    path = "/cards/{bin}",
    params(("bin" = String, Path, description = "The first 6 or 9 digits of card")),
    responses(
        (status = 200, description = "Card iin data found", body = CardInfoResponse),
        (status = 404, description = "Card iin data not found")
    ),
    operation_id = "Retrieve card information",
    security(("api_key" = []), ("publishable_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::CardsInfo))]
pub async fn card_iin_info(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    payload: web::Query<api_models::cards_info::CardsInfoRequestParams>,
) -> impl Responder {
    let card_iin = path.into_inner();
    let request_params = payload.into_inner();

    let payload = api_models::cards_info::CardsInfoRequest {
        client_secret: request_params.client_secret,
        card_iin,
    };

    let api_auth = auth::ApiKeyAuth::default();

    let (auth, _) = match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth)
    {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    Box::pin(api::server_wrap(
        Flow::CardsInfo,
        state,
        &req,
        payload,
        |state, auth, req, _| {
            let platform = auth.into();
            cards_info::retrieve_card_info(state, platform, req)
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CardsInfoCreate))]
pub async fn create_cards_info(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<cards_info_api_types::CardInfoCreateRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();
    let flow = Flow::CardsInfoCreate;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload,
        |state, _, payload, _| cards_info::create_card_info(state, payload),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CardsInfoUpdate))]
pub async fn update_cards_info(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<cards_info_api_types::CardInfoUpdateRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();
    let flow = Flow::CardsInfoUpdate;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload,
        |state, _, payload, _| cards_info::update_card_info(state, payload),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", any(feature = "olap", feature = "oltp")))]
#[instrument(skip_all, fields(flow = ?Flow::CardsInfoMigrate))]
pub async fn migrate_cards_info(
    state: web::Data<AppState>,
    req: HttpRequest,
    MultipartForm(form): MultipartForm<cards_info::CardsInfoUpdateForm>,
) -> HttpResponse {
    let flow = Flow::CardsInfoMigrate;
    let records = match cards_info::get_cards_bin_records(form) {
        Ok(records) => records,
        Err(e) => return api::log_and_return_error_response(e.into()),
    };
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        records,
        |state, _, payload, _| cards_info::migrate_cards_info(state, payload),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
