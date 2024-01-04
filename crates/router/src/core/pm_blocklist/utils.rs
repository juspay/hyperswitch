use std::collections::HashSet;

use api_models::pm_blocklist as blacklist_pm;
use base64::Engine;
use common_utils::{
    crypto::{self, SignMessage},
    errors::CustomResult,
    // ext_traits::Encode,
};
use diesel_models::pm_blocklist;
use error_stack::{IntoReport, ResultExt};
use router_env::logger;
use storage_impl::errors::StorageError;

use super::{errors, AppState};
use crate::{consts, utils};

pub async fn delete_from_blocklist_lookup_db(
    state: &AppState,
    merchant_id: String,
    pm_hashes: &[String],
) -> CustomResult<blacklist_pm::UnblockPmResponse, StorageError> {
    let pm_hashes = remove_duplicates(pm_hashes);
    let blocklist_entries = pm_hashes
        .iter()
        .map(|hash| {
            state.store.delete_pm_blocklist_entry_by_merchant_id_hash(
                merchant_id.clone(),
                hash.to_string(),
            )
        })
        .collect::<Vec<_>>();

    let fingerprint_lookup = pm_hashes
        .iter()
        .map(|entry| state.store.find_pm_fingerprint_entry(entry.to_string()))
        .collect::<Vec<_>>();

    let blacklist_futures = futures::future::join_all(blocklist_entries);
    let fingerprint_lookup_futures = futures::future::join_all(fingerprint_lookup);
    let (unblocked_from_blocklist, fingerprint_lookup) =
        tokio::join!(blacklist_futures, fingerprint_lookup_futures);

    // Delete entries from lookup based on fingerprint results
    let mut lookup_entries = Vec::new();

    for (fingerprint_result, pm_hash) in fingerprint_lookup.into_iter().zip(pm_hashes.iter()) {
        match fingerprint_result {
            Ok(fingerprint) => {
                let query_future = state
                    .store
                    .delete_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash(
                        merchant_id.clone(),
                        fingerprint.kms_hash,
                    );
                lookup_entries.push((query_future, pm_hash.clone()));
            }
            Err(e) => {
                logger::error!("Unblocking pm failed: {e:?}");
            }
        }
    }

    let unblocked_from_lookup =
        futures::future::join_all(lookup_entries.into_iter().map(|(future, _)| future)).await;

    let unblocked_from_blocklist = unblocked_from_blocklist
        .into_iter()
        .map(|result| {
            result.unwrap_or_else(|e| {
                logger::error!("Unblocking pm failed {e:?}");
                false // Assume it's not unblocked in case of an error
            })
        })
        .collect::<Vec<_>>();

    let unblocked_from_lookup = unblocked_from_lookup
        .into_iter()
        .map(|result| {
            result.unwrap_or_else(|e| {
                logger::error!("Unblocking pm failed {e:?}");
                false // Assume it's not unblocked in case of an error
            })
        })
        .collect::<Vec<_>>();

    let mut unblocked_pm = Vec::new();
    let mut not_unblocked_pm = Vec::new();
    for ((unblocked_from_lookup, unblocked_from_blocklist), data) in unblocked_from_lookup
        .into_iter()
        .zip(unblocked_from_blocklist.into_iter())
        .zip(pm_hashes.into_iter())
    {
        if (unblocked_from_lookup, unblocked_from_blocklist) == (true, true) {
            unblocked_pm.push(data);
        } else {
            not_unblocked_pm.push(data);
        }
    }

    if !not_unblocked_pm.is_empty() {
        logger::error!("Unblocking pm failed for: {:?}", not_unblocked_pm);
    }

    Ok(blacklist_pm::UnblockPmResponse { unblocked_pm })
}

pub async fn list_blocked_pm_from_db(
    state: &AppState,
    merchant_id: String,
) -> CustomResult<blacklist_pm::ListBlockedPmResponse, errors::ApiErrorResponse> {
    let blocked_cardbins = state
        .store
        .list_all_blocked_pm_for_merchant_by_type(merchant_id.clone(), "cardbin".to_string());

    let blocked_extended_bins = state.store.list_all_blocked_pm_for_merchant_by_type(
        merchant_id.clone(),
        "extended_cardbin".to_string(),
    );

    let blocked_fingerprints = state
        .store
        .list_all_blocked_pm_for_merchant_by_type(merchant_id, "fingerprint".to_string());
    let (blocked_cardbins, blocked_fingerprints, blocked_extended_bins) = tokio::join!(
        blocked_cardbins,
        blocked_fingerprints,
        blocked_extended_bins
    );

    match (
        blocked_fingerprints,
        blocked_cardbins,
        blocked_extended_bins,
    ) {
        (Ok(fingerprint), Ok(cardbin), Ok(extended_bin)) => {
            Ok(blacklist_pm::ListBlockedPmResponse {
                blocked_fingerprints: fingerprint
                    .iter()
                    .map(|fingerprint| fingerprint.pm_hash.clone())
                    .collect::<Vec<_>>(),
                blocked_cardbins: cardbin
                    .iter()
                    .map(|cardbin| (cardbin.pm_hash.clone(), cardbin.metadata.clone()))
                    .collect::<Vec<_>>(),
                blocked_extended_cardbins: extended_bin
                    .iter()
                    .map(|extended_bin| {
                        (extended_bin.pm_hash.clone(), extended_bin.metadata.clone())
                    })
                    .collect::<Vec<_>>(),
            })
        }
        (_, _, _) => Err(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Unable to retrieve Blocklist".to_string(),
        }
        .into()),
    }
}

pub async fn insert_to_blocklist_lookup_db(
    state: &AppState,
    merchant_id: String,
    pm_hashes: &[String],
    pm_type: &str,
) -> CustomResult<blacklist_pm::BlacklistPmResponse, StorageError> {
    let pm_hashes = remove_duplicates(pm_hashes);
    let mut new_entries = Vec::new();
    let mut fingerprints_blocked = Vec::new();
    let merchant_secret = state
        .store
        .find_config_by_key(format!("secret_{}", merchant_id.clone()).as_str())
        .await
        .change_context(StorageError::EncryptionError)
        .attach_printable("Merchant Secret not found")?
        .config;

    for pm_hash in pm_hashes {
        let merchant_id = merchant_id.clone();
        let pm_hash = pm_hash.clone();
        let merchant_secret = merchant_secret.clone();

        let result = async move {
            match pm_type {
                "cardbin" => {
                    if pm_hash.len() < 6 {
                        return Err(StorageError::EncryptionError.into());
                    }
                    let card_bin = &pm_hash[..6];
                    duplicate_check_insert_bin(
                        card_bin,
                        state,
                        merchant_id.clone(),
                        merchant_secret.clone(),
                        pm_type.clone(),
                    )
                    .await
                }
                "extended_cardbin" => {
                    if pm_hash.len() < 8 {
                        return Err(StorageError::EncryptionError.into());
                    }
                    let extended_bin = &pm_hash[..8];
                    duplicate_check_insert_bin(
                        extended_bin,
                        state,
                        merchant_id.clone(),
                        merchant_secret.clone(),
                        pm_type.clone(),
                    )
                    .await
                }
                _ => {
                    // For fingerprint we are getting the fingerprint id already
                    //TODO Decrypt this KMS encryption to get hash
                    let kms_decrypted_hash = state
                        .store
                        .find_pm_fingerprint_entry(pm_hash.clone())
                        .await
                        .change_context(errors::StorageError::ValueNotFound(
                            pm_hash.clone().to_string(),
                        ))?
                        .kms_hash;

                    if state
                        .store
                        .find_pm_blocklist_entry_by_merchant_id_fingerprint(
                            merchant_id.clone(),
                            pm_hash.clone(),
                        )
                        .await
                        .is_ok()
                        || state
                            .store
                            .find_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash(
                                merchant_id.clone(),
                                kms_decrypted_hash.clone(),
                            )
                            .await
                            .is_ok()
                    {
                        Err(StorageError::DuplicateValue {
                            entity: "blocklist_entry",
                            key: Some(pm_hash.clone()),
                        })
                        .into_report()
                    } else {
                        let _ = state
                            .store
                            .insert_blocklist_lookup_entry(
                                diesel_models::blocklist_lookup::BlocklistLookupNew {
                                    merchant_id: merchant_id.clone(),
                                    kms_decrypted_hash,
                                },
                            )
                            .await;
                        state
                            .store
                            .insert_pm_blocklist_item(pm_blocklist::PmBlocklistNew {
                                merchant_id: merchant_id.clone(),
                                pm_hash: pm_hash.clone().to_string(),
                                pm_type: pm_type.to_string().clone(),
                                metadata: None,
                            })
                            .await
                    }
                }
            }
        };
        new_entries.push(result);
    }

    let mut all_requested_fingerprints_blocked = true;
    let blocked_pm_futures = futures::future::join_all(new_entries).await;
    blocked_pm_futures.into_iter().for_each(|res| match res {
        Ok(blocked_pm) => fingerprints_blocked.push(blocked_pm.pm_hash),
        Err(e) => {
            all_requested_fingerprints_blocked = false;
            logger::error!("Pm Blocklist entry insertion failed {e:?}");
        }
    });

    if all_requested_fingerprints_blocked {
        let response = match pm_type {
            "cardbin" => blacklist_pm::BlacklistPmResponse {
                blocked: blacklist_pm::BlocklistType::Cardbin(fingerprints_blocked),
            },
            "extended_cardbin" => blacklist_pm::BlacklistPmResponse {
                blocked: blacklist_pm::BlocklistType::ExtendedCardbin(fingerprints_blocked),
            },
            _ => blacklist_pm::BlacklistPmResponse {
                blocked: blacklist_pm::BlocklistType::Fingerprint(fingerprints_blocked),
            },
        };
        Ok(response)
    } else {
        Err(errors::StorageError::ValueNotFound("fingerprint".to_string()).into())
    }
}

fn remove_duplicates<T: Eq + std::hash::Hash + Clone>(vec: &[T]) -> Vec<T> {
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

async fn duplicate_check_insert_bin(
    bin: &str,
    state: &AppState,
    merchant_id: String,
    merchant_secret: String,
    pm_type: &str,
) -> CustomResult<pm_blocklist::PmBlocklist, StorageError> {
    let hashed_bin = crypto::HmacSha512::sign_message(
        &crypto::HmacSha512,
        merchant_secret.clone().as_bytes(),
        // what if they supply 10 digits instead of say 6 or 8
        bin.as_bytes(),
    )
    .change_context(StorageError::EncryptionError)
    .attach_printable("error in bin hash creation")?;

    let encoded_hash = consts::BASE64_ENGINE.encode(hashed_bin.clone());

    // Checking for duplicacy
    if state
        .store
        .find_blocklist_lookup_entry_by_merchant_id_kms_decrypted_hash(
            merchant_id.clone(),
            encoded_hash.clone(),
        )
        .await
        .is_ok()
    {
        Err(StorageError::DuplicateValue {
            entity: "blocklist_entry",
            key: Some(bin.to_string().clone()),
        })
        .into_report()
    } else {
        // TODO KMS encrypt the encoded hash and then store
        let fingerprint_id = state
            .store
            .insert_pm_fingerprint_entry(diesel_models::pm_fingerprint::PmFingerprintNew {
                fingerprint_id: utils::generate_id(consts::ID_LENGTH, "fingerprint"),
                kms_hash: encoded_hash.clone(),
            })
            .await
            .change_context(errors::StorageError::ValueNotFound(
                bin.to_string().clone().to_string(),
            ))?
            .fingerprint_id;
        let _ = state
            .store
            .insert_blocklist_lookup_entry(diesel_models::blocklist_lookup::BlocklistLookupNew {
                merchant_id: merchant_id.clone(),
                kms_decrypted_hash: encoded_hash.clone(),
            })
            .await;

        state
            .store
            .insert_pm_blocklist_item(pm_blocklist::PmBlocklistNew {
                merchant_id: merchant_id.clone(),
                pm_hash: fingerprint_id.clone().to_string(),
                pm_type: pm_type.to_string().clone(),
                metadata: Some(bin.to_string().clone()),
            })
            .await
    }
}
