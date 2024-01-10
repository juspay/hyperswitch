#[cfg(feature = "oltp")]
pub mod utils;

use api_models::pm_blocklist;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;

use crate::{core::errors, routes::AppState, services, types::domain};

pub async fn block_payment_method(
    state: AppState,
    _req: &actix_web::HttpRequest,
    body: pm_blocklist::BlocklistType,
    merchant_account: domain::MerchantAccount,
) -> CustomResult<
    services::ApplicationResponse<pm_blocklist::BlocklistPmResponse>,
    errors::ApiErrorResponse,
> {
    let blocklist_type: &str;
    match &body {
        pm_blocklist::BlocklistType::CardBin(cards) => {
            blocklist_type = "cardbin";
            Ok(services::api::ApplicationResponse::Json(
                utils::insert_to_blocklist_lookup_db(
                    &state,
                    merchant_account.merchant_id,
                    cards,
                    blocklist_type,
                )
                .await
                .change_context(
                    errors::ApiErrorResponse::GenericNotFoundError {
                        message: "Unable to block cardbins".to_string(),
                    },
                )?,
            ))
        }
        pm_blocklist::BlocklistType::ExtendedBin(extended_cardbins) => {
            blocklist_type = "extended_cardbin";
            Ok(services::api::ApplicationResponse::Json(
                utils::insert_to_blocklist_lookup_db(
                    &state,
                    merchant_account.merchant_id,
                    &extended_cardbins,
                    blocklist_type,
                )
                .await
                .change_context(
                    errors::ApiErrorResponse::GenericNotFoundError {
                        message: "Unable to block extended cardbins".to_string(),
                    },
                )?,
            ))
        }
        pm_blocklist::BlocklistType::Fingerprint(fingerprints) => {
            blocklist_type = "fingerprint";
            Ok(services::api::ApplicationResponse::Json(
                utils::insert_to_blocklist_lookup_db(
                    &state,
                    merchant_account.merchant_id,
                    &fingerprints,
                    blocklist_type,
                )
                .await
                .change_context(
                    errors::ApiErrorResponse::GenericNotFoundError {
                        message: "Unable to block fingerprints".to_string(),
                    },
                )?,
            ))
        }
    }
}

pub async fn unblock_payment_method(
    state: AppState,
    _req: &actix_web::HttpRequest,
    body: pm_blocklist::UnblockPmRequest,
    merchant_account: domain::MerchantAccount,
) -> CustomResult<
    services::ApplicationResponse<pm_blocklist::UnblockPmResponse>,
    errors::ApiErrorResponse,
> {
    Ok(services::api::ApplicationResponse::Json(
        utils::delete_from_blocklist_lookup_db(&state, merchant_account.merchant_id, &body.data)
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: "Unable to Unblock payment methods".to_string(),
            })?,
    ))
}

pub async fn list_blocked_payment_methods(
    state: AppState,
    _req: &actix_web::HttpRequest,
    merchant_account: domain::MerchantAccount,
) -> CustomResult<
    services::ApplicationResponse<pm_blocklist::ListBlockedPmResponse>,
    errors::ApiErrorResponse,
> {
    Ok(services::api::ApplicationResponse::Json(
        utils::list_blocked_pm_from_db(&state, merchant_account.merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: "Unable to list Blocked payment methods".to_string(),
            })?,
    ))
}
