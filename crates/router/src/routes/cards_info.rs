use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, cards_info},
    services::{api, authentication as auth},
};

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
//#\[instrument\(skip_all, fields(flow = ?Flow::CardsInfo))]
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

    let (auth, _) = match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    api::server_wrap(
        Flow::CardsInfo,
        state,
        &req,
        payload,
        |state, auth, req, _| cards_info::retrieve_card_info(state, auth.merchant_account, req),
        &*auth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
