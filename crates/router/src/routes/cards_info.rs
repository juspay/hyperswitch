use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::cards_info,
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
)]
#[instrument(skip_all, fields(flow = ?Flow::CardsInfo))]
pub async fn card_iin_info(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let card_iin = path.into_inner();

    api::server_wrap(
        state.get_ref(),
        &req,
        card_iin,
        |state, _, card_iin| async {
            cards_info::retrieve_card_info(&*state.store, card_iin).await
        },
        &auth::NoAuth,
    )
    .await
}
