use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResult},
    db::StorageInterface,
    logger,
    types::storage,
};

#[instrument(skip(db))]
pub async fn validate_uniqueness_of_payout_id_against_merchant_id(
    db: &dyn StorageInterface,
    payout_id: &str,
    merchant_id: &str,
) -> RouterResult<Option<storage::PayoutCreate>> {
    let payout = db
        .find_payout_create_by_merchant_id_payout_id(merchant_id, payout_id)
        .await;

    logger::debug!(?payout);
    match payout {
        Err(err) => {
            if err.current_context().is_db_not_found() {
                // Empty vec should be returned by query in case of no results, this check exists just
                // to be on the safer side. Fixed this, now vector is not returned but should check the flow in detail later.
                Ok(None)
            } else {
                Err(err
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while finding payout_create, database error"))
            }
        }
        Ok(payout) => {
            if payout.payout_id == payout_id {
                Ok(Some(payout))
            } else {
                Ok(None)
            }
        }
    }
}
