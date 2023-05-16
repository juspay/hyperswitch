use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, RouterResult},
        utils as core_utils,
    },
    db::StorageInterface,
    logger,
    routes::AppState,
    types::{api::payouts, storage},
    utils::{self},
};

#[instrument(skip(db))]
pub async fn validate_uniqueness_of_payout_id_against_merchant_id(
    db: &dyn StorageInterface,
    payout_id: &str,
    merchant_id: &str,
) -> RouterResult<Option<storage::Payouts>> {
    let payout = db
        .find_payout_by_merchant_id_payout_id(merchant_id, payout_id)
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

/// Validates the request on below checks
/// - merchant_id passed is same as the one in merchant_account table
/// - payout_id is unique against merchant_id
pub async fn validate_create_request(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
) -> RouterResult<String> {
    let merchant_id = &merchant_account.merchant_id;

    let predicate = req.merchant_id.as_ref().map(|mid| mid != merchant_id);
    utils::when(predicate.unwrap_or(false), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "merchant_id".to_string(),
            expected_format: "merchant_id from merchant account".to_string(),
        })
        .attach_printable("invalid merchant_id in request"))
    })?;

    let db: &dyn StorageInterface = &*state.store;
    let payout_id = core_utils::get_or_generate_id("payout_id", &req.payout_id, "payout")?;
    match validate_uniqueness_of_payout_id_against_merchant_id(db, &payout_id, merchant_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Unique violation while checking payout_id: {} against merchant_id: {}",
                payout_id.to_owned(),
                merchant_id
            )
        })? {
        Some(_) => Err(report!(errors::ApiErrorResponse::DuplicatePayout {
            payout_id
        })),
        None => Ok(payout_id),
    }
}
