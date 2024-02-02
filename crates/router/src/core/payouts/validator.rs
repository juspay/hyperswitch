use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::helpers;
use crate::{
    core::{
        errors::{self, RouterResult},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    types::{api::payouts, domain, storage},
    utils,
};

#[cfg(feature = "payouts")]
#[instrument(skip(db))]
/// Validates the uniqueness of a payout ID against a merchant ID in the database.
///
/// # Arguments
///
/// * `db` - A reference to a database storage interface
/// * `payout_id` - A string representing the payout ID to validate
/// * `merchant_id` - A string representing the merchant ID to validate against
///
/// # Returns
///
/// An asynchronous result containing an option of `storage::Payouts` if the validation is successful, or an error if the validation fails.
///
pub async fn validate_uniqueness_of_payout_id_against_merchant_id(
    db: &dyn StorageInterface,
    payout_id: &str,
    merchant_id: &str,
) -> RouterResult<Option<storage::Payouts>> {
    let payout = db
        .find_payout_by_merchant_id_payout_id(merchant_id, payout_id)
        .await;
    match payout {
        Err(err) => {
            if err.current_context().is_db_not_found() {
                // Empty vec should be returned by query in case of no results, this check exists just
                // to be on the safer side. Fixed this, now vector is not returned but should check the flow in detail later.
                Ok(None)
            } else {
                Err(err
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while finding payout_attempt, database error"))
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
/// - payout_token provided is legitimate
#[cfg(feature = "payouts")]
/// Validates the create request for a payout, checking the merchant ID, payout ID, payout token, and profile ID, and returning a tuple containing the validated payout ID, optional payout method data, and profile ID.
pub async fn validate_create_request(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    merchant_key_store: &domain::MerchantKeyStore,
) -> RouterResult<(String, Option<payouts::PayoutMethodData>, String)> {
    let merchant_id = &merchant_account.merchant_id;

    // Merchant ID
    let predicate = req.merchant_id.as_ref().map(|mid| mid != merchant_id);
    utils::when(predicate.unwrap_or(false), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "merchant_id".to_string(),
            expected_format: "merchant_id from merchant account".to_string(),
        })
        .attach_printable("invalid merchant_id in request"))
    })?;

    // Payout ID
    let db: &dyn StorageInterface = &*state.store;
    let payout_id = core_utils::get_or_generate_uuid("payout_id", req.payout_id.as_ref())?;
    match validate_uniqueness_of_payout_id_against_merchant_id(db, &payout_id, merchant_id)
        .await
        .change_context(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.to_owned(),
        })
        .attach_printable_lazy(|| {
            format!(
                "Unique violation while checking payout_id: {} against merchant_id: {}",
                payout_id.to_owned(),
                merchant_id
            )
        })? {
        Some(_) => Err(report!(errors::ApiErrorResponse::DuplicatePayout {
            payout_id: payout_id.to_owned()
        })),
        None => Ok(()),
    }?;

    // Payout token
    let payout_method_data = match req.payout_token.to_owned() {
        Some(payout_token) => {
            let customer_id = req.customer_id.to_owned().map_or("".to_string(), |c| c);
            helpers::make_payout_method_data(
                state,
                req.payout_method_data.as_ref(),
                Some(&payout_token),
                &customer_id,
                &merchant_account.merchant_id,
                payout_id.as_ref(),
                req.payout_type.as_ref(),
                merchant_key_store,
            )
            .await?
        }
        None => None,
    };

    // Profile ID
    let profile_id = core_utils::get_profile_id_from_business_details(
        req.business_country,
        req.business_label.as_ref(),
        merchant_account,
        req.profile_id.as_ref(),
        &*state.store,
        false,
    )
    .await?;

    Ok((payout_id, payout_method_data, profile_id))
}
