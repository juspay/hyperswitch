use error_stack::ResultExt;

use crate::{
    core::{
        errors::{self, RouterResult},
        payment_methods::vault,
    },
    routes::AppState,
    types::{api, storage},
    utils,
};

pub async fn make_payout_data<'a>(
    state: &'a AppState,
    request: &api::PayoutCreateRequest,
    payout_create: &storage::PayoutCreate,
) -> RouterResult<Option<api::PayoutMethodData>> {
    let db = &*state.store;
    match (
        request.payout_method_data.to_owned(),
        payout_create.payout_token.to_owned(),
    ) {
        (None, Some(payout_token)) => {
            let (pm, supplementary_data) = vault::Vault::get_payout_method_data_from_locker(
                state,
                &payout_token,
            )
            .await
            .attach_printable(
                "Payout method for given token not found or there was a problem fetching it",
            )?;
            utils::when(
                supplementary_data
                    .customer_id
                    .ne(&Some(payout_create.customer_id.to_owned())),
                || {
                    Err(errors::ApiErrorResponse::PreconditionFailed { message: "customer associated with payout method and customer passed in payout are not same".into() })
                },
            )?;
            Ok(pm)
        }
        (Some(payout_method), None) => {
            let payout_token = vault::Vault::store_payout_method_data_in_locker(
                state,
                None,
                &payout_method,
                Some(payout_create.customer_id.to_owned()),
                payout_create.payout_type,
            )
            .await?;
            //FIXME: we should have Status field in payout_create and update status from require_payout_method_data to require_fulfillment
            let payout_update = storage::PayoutCreateUpdate::PayoutTokenUpdate { payout_token };
            db.update_payout_create_by_merchant_id_payout_id(
                &payout_create.merchant_id,
                &payout_create.payout_id,
                payout_update,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating token in payout create")?;
            Ok(Some(payout_method))
        }
        _ => Ok(None),
    }
}
