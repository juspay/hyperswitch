use common_utils::fp_utils::when;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResponse},
    db::StorageInterface,
    services::ApplicationResponse,
    types::transformers::ForeignFrom,
};

#[instrument(skip_all)]
pub async fn retrieve_card_info(
    store: &dyn StorageInterface,
    card_iin: String,
) -> RouterResponse<api_models::cards_info::CardInfoResponse> {
    when(card_iin.len() != 6, || {
        Err(errors::ApiErrorResponse::InvalidCardIinLength)
    })?;

    let card_info = store
        .get_card_info(&card_iin)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve card information")?
        .ok_or(report!(errors::ApiErrorResponse::InvalidCardIin))?;

    Ok(ApplicationResponse::Json(
        api_models::cards_info::CardInfoResponse::foreign_from(card_info),
    ))
}
