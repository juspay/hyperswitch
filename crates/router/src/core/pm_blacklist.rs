pub mod utils;
use std::collections::HashSet;

use api_models::pm_blacklist;
use common_utils::errors::CustomResult;
use diesel_models::pm_blocklist;
use error_stack::{IntoReport, Report, ResultExt};
use router_env::logger;
use storage_impl::errors::StorageError;

use crate::{core::errors, routes::AppState, services, types::domain};

pub async fn block_payment_method(
    state: AppState,
    _req: &actix_web::HttpRequest,
    body: pm_blacklist::BlacklistPmRequest,
    merchant_account: domain::MerchantAccount,
) -> CustomResult<
    services::ApplicationResponse<pm_blacklist::BlacklistPmResponse>,
    errors::ApiErrorResponse,
> {
    let mut fingerprints_blocked = Vec::new();
    let res =
        insert_to_db_non_duplicates(&state, merchant_account.merchant_id, &body.pm_to_block).await;
    let _ = res.into_iter().for_each(|res| match res {
        Ok(block_pm) => {
            fingerprints_blocked.push(block_pm.pm_hash);
        }
        Err(e) => {
            logger::error!("Pm Blocklist entry insertion failed {e:?}");
        }
    });

    Ok(services::api::ApplicationResponse::Json(
        pm_blacklist::BlacklistPmResponse {
            fingerprints_blocked,
        },
    ))
}

pub async fn unblock_payment_method(
    state: AppState,
    _req: &actix_web::HttpRequest,
    body: pm_blacklist::UnblockPmRequest,
    merchant_account: domain::MerchantAccount,
) -> CustomResult<
    services::ApplicationResponse<pm_blacklist::UnblockPmResponse>,
    errors::ApiErrorResponse,
> {
    let entries = body
        .data
        .iter()
        .map(|hash| {
            state.store.delete_pm_blocklist_entry_by_merchant_id_hash(
                merchant_account.merchant_id.clone(),
                hash.to_string(),
            )
        })
        .collect::<Vec<_>>();
    let new_futures = futures::future::join_all(entries).await;
    let mut fingerprints_unblocked = Vec::new();
    let _ = new_futures
        .iter()
        .for_each(|unblocked_pm| match unblocked_pm {
            Ok(res) => {
                if *res {
                    fingerprints_unblocked.extend(body.data.clone().drain(..));
                }
            }
            Err(e) => {
                logger::error!("Unblocking pm failed {e:?}");
            }
        });

    Ok(services::api::ApplicationResponse::Json(
        pm_blacklist::UnblockPmResponse {
            data: fingerprints_unblocked,
        },
    ))
}

pub async fn list_blocked_payment_methods(
    state: AppState,
    _req: &actix_web::HttpRequest,
    merchant_account: domain::MerchantAccount,
) -> CustomResult<
    services::ApplicationResponse<pm_blacklist::ListBlockedPmResponse>,
    errors::ApiErrorResponse,
> {
    let blocked_pms = state
        .store
        .list_all_blocked_pm_for_merchant(merchant_account.merchant_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let mut fingerprints_blocked = Vec::new();
    blocked_pms
        .iter()
        .for_each(|pm| fingerprints_blocked.push(pm.pm_hash.clone()));

    Ok(services::api::ApplicationResponse::Json(
        pm_blacklist::ListBlockedPmResponse {
            fingerprints_blocked,
        },
    ))
}
pub async fn insert_to_db_non_duplicates(
    state: &AppState,
    merchant_id: String,
    pm_hashes: &Vec<String>,
) -> Vec<Result<pm_blocklist::PmBlocklist, Report<StorageError>>> {
    let pm_hashes = remove_duplicates(pm_hashes);
    let mut new_entries = Vec::new();

    for pm_hash in pm_hashes {
        let merchant_id = merchant_id.clone();
        let pm_hash = pm_hash.clone();
        let result = async move {
            if state
                .store
                .find_pm_blocklist_entry_by_merchant_id_hash(merchant_id.clone(), pm_hash.clone())
                .await
                .is_ok()
            {
                Err(StorageError::DuplicateValue {
                    entity: "blocklist_entry",
                    key: None,
                })
                .into_report()
            } else {
                state
                    .store
                    .insert_pm_blocklist_item(pm_blocklist::PmBlocklistNew {
                        merchant_id: merchant_id.clone(),
                        pm_hash: pm_hash.clone().to_string(),
                    })
                    .await
            }
        };
        new_entries.push(result);
    }

    let res = futures::future::join_all(new_entries).await;
    res
}

fn remove_duplicates<T: Eq + std::hash::Hash + Clone>(vec: &Vec<T>) -> Vec<T> {
    let mut set = HashSet::new();

    vec.iter()
        .filter_map(|item| {
            if set.insert(item.clone()) {
                Some(item.clone())
            } else {
                None
            }
        })
        .collect()
}
