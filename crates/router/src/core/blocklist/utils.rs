use api_models::blocklist as api_blocklist;
use common_utils::crypto::{self, SignMessage};
use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "kms")]
use external_services::kms;

use super::{errors, AppState};
use crate::{
    consts,
    core::errors::{RouterResult, StorageErrorExt},
    types::{storage, transformers::ForeignInto},
    utils,
};

pub async fn delete_entry_from_blocklist(
    state: &AppState,
    merchant_id: String,
    request: api_blocklist::DeleteFromBlocklistRequest,
) -> RouterResult<api_blocklist::DeleteFromBlocklistResponse> {
    let blocklist_entry = match request {
        api_blocklist::DeleteFromBlocklistRequest::CardBin(bin) => {
            delete_card_bin_blocklist_entry(state, &bin, &merchant_id).await?
        }

        api_blocklist::DeleteFromBlocklistRequest::ExtendedCardBin(xbin) => {
            delete_card_bin_blocklist_entry(state, &xbin, &merchant_id).await?
        }

        api_blocklist::DeleteFromBlocklistRequest::Fingerprint(fingerprint_id) => {
            let blocklist_fingerprint = state
                .store
                .find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
                    &merchant_id,
                    &fingerprint_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "blocklist record with given fingerprint id not found".to_string(),
                })?;

            #[cfg(feature = "kms")]
            let decrypted_fingerprint = kms::get_kms_client(&state.conf.kms)
                .await
                .decrypt(blocklist_fingerprint.encrypted_fingerprint)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to kms decrypt fingerprint")?;

            #[cfg(not(feature = "kms"))]
            let decrypted_fingerprint = blocklist_fingerprint.encrypted_fingerprint;

            let blocklist_entry = state
                .store
                .delete_blocklist_entry_by_merchant_id_fingerprint_id(&merchant_id, &fingerprint_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "no blocklist record for the given fingerprint id was found"
                        .to_string(),
                })?;

            state
                .store
                .delete_blocklist_lookup_entry_by_merchant_id_fingerprint(
                    &merchant_id,
                    &decrypted_fingerprint,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "no blocklist record for the given fingerprint id was found"
                        .to_string(),
                })?;

            blocklist_entry
        }
    };

    Ok(blocklist_entry.foreign_into())
}

pub async fn list_blocklist_entries_for_merchant(
    state: &AppState,
    merchant_id: String,
    query: api_blocklist::ListBlocklistQuery,
) -> RouterResult<Vec<api_blocklist::BlocklistResponse>> {
    state
        .store
        .list_blocklist_entries_by_merchant_id_data_kind(
            &merchant_id,
            query.data_kind,
            query.limit.into(),
            query.offset.into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "no blocklist records found".to_string(),
        })
        .map(|v| v.into_iter().map(ForeignInto::foreign_into).collect())
}

fn validate_card_bin(bin: &str) -> RouterResult<()> {
    if bin.len() == 6 && bin.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "data".to_string(),
            expected_format: "a 6 digit number".to_string(),
        })
        .into_report()
    }
}

fn validate_extended_card_bin(bin: &str) -> RouterResult<()> {
    if bin.len() == 8 && bin.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "data".to_string(),
            expected_format: "an 8 digit number".to_string(),
        })
        .into_report()
    }
}

pub async fn insert_entry_into_blocklist(
    state: &AppState,
    merchant_id: String,
    to_block: api_blocklist::AddToBlocklistRequest,
) -> RouterResult<api_blocklist::AddToBlocklistResponse> {
    let blocklist_entry = match &to_block {
        api_blocklist::AddToBlocklistRequest::CardBin(bin) => {
            validate_card_bin(bin)?;
            duplicate_check_insert_bin(
                bin,
                state,
                &merchant_id,
                common_enums::BlocklistDataKind::CardBin,
            )
            .await?
        }

        api_blocklist::AddToBlocklistRequest::ExtendedCardBin(bin) => {
            validate_extended_card_bin(bin)?;
            duplicate_check_insert_bin(
                bin,
                state,
                &merchant_id,
                common_enums::BlocklistDataKind::ExtendedCardBin,
            )
            .await?
        }

        api_blocklist::AddToBlocklistRequest::Fingerprint(fingerprint_id) => {
            let blocklist_entry_result = state
                .store
                .find_blocklist_entry_by_merchant_id_fingerprint_id(&merchant_id, fingerprint_id)
                .await;

            match blocklist_entry_result {
                Ok(_) => {
                    return Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "data associated with the given fingerprint is already blocked"
                            .to_string(),
                    })
                    .into_report();
                }

                // if it is a db not found error, we can proceed as normal
                Err(inner) if inner.current_context().is_db_not_found() => {}

                err @ Err(_) => {
                    err.change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("error fetching blocklist entry from table")?;
                }
            }

            let blocklist_fingerprint = state
                .store
                .find_blocklist_fingerprint_by_merchant_id_fingerprint_id(
                    &merchant_id,
                    fingerprint_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "fingerprint not found".to_string(),
                })?;

            #[cfg(feature = "kms")]
            let decrypted_fingerprint = kms::get_kms_client(&state.conf.kms)
                .await
                .decrypt(blocklist_fingerprint.encrypted_fingerprint)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to kms decrypt encrypted fingerprint")?;

            #[cfg(not(feature = "kms"))]
            let decrypted_fingerprint = blocklist_fingerprint.encrypted_fingerprint;

            state
                .store
                .insert_blocklist_lookup_entry(
                    diesel_models::blocklist_lookup::BlocklistLookupNew {
                        merchant_id: merchant_id.clone(),
                        fingerprint: decrypted_fingerprint,
                    },
                )
                .await
                .to_duplicate_response(errors::ApiErrorResponse::PreconditionFailed {
                    message: "the payment instrument associated with the given fingerprint is already in the blocklist".to_string(),
                })
                .attach_printable("failed to add fingerprint to blocklist lookup")?;

            state
                .store
                .insert_blocklist_entry(storage::BlocklistNew {
                    merchant_id: merchant_id.clone(),
                    fingerprint_id: fingerprint_id.clone(),
                    data_kind: blocklist_fingerprint.data_kind,
                    metadata: None,
                    created_at: common_utils::date_time::now(),
                })
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to add fingerprint to pm blocklist")?
        }
    };

    Ok(blocklist_entry.foreign_into())
}

pub async fn get_merchant_fingerprint_secret(
    state: &AppState,
    merchant_id: &str,
) -> RouterResult<String> {
    let key = get_merchant_fingerprint_secret_key(merchant_id);
    let config_fetch_result = state.store.find_config_by_key(&key).await;

    match config_fetch_result {
        Ok(config) => Ok(config.config),

        Err(e) if e.current_context().is_db_not_found() => {
            let new_fingerprint_secret =
                utils::generate_id(consts::FINGERPRINT_SECRET_LENGTH, "fs");
            let new_config = storage::ConfigNew {
                key,
                config: new_fingerprint_secret.clone(),
            };

            state
                .store
                .insert_config(new_config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("unable to create new fingerprint secret for merchant")?;

            Ok(new_fingerprint_secret)
        }

        Err(e) => Err(e)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("error fetching merchant fingerprint secret"),
    }
}

pub fn get_merchant_fingerprint_secret_key(merchant_id: &str) -> String {
    format!("fingerprint_secret_{merchant_id}")
}

async fn duplicate_check_insert_bin(
    bin: &str,
    state: &AppState,
    merchant_id: &str,
    data_kind: common_enums::BlocklistDataKind,
) -> RouterResult<storage::Blocklist> {
    let merchant_secret = get_merchant_fingerprint_secret(state, merchant_id).await?;
    let bin_fingerprint = crypto::HmacSha512::sign_message(
        &crypto::HmacSha512,
        merchant_secret.clone().as_bytes(),
        bin.as_bytes(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("error in bin hash creation")?;

    let encoded_fingerprint = hex::encode(bin_fingerprint.clone());

    let blocklist_entry_result = state
        .store
        .find_blocklist_entry_by_merchant_id_fingerprint_id(merchant_id, bin)
        .await;

    match blocklist_entry_result {
        Ok(_) => {
            return Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "provided bin is already blocked".to_string(),
            })
            .into_report();
        }

        Err(e) if e.current_context().is_db_not_found() => {}

        err @ Err(_) => {
            return err
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("unable to fetch blocklist entry");
        }
    }

    // Checking for duplicacy
    state
        .store
        .insert_blocklist_lookup_entry(diesel_models::blocklist_lookup::BlocklistLookupNew {
            merchant_id: merchant_id.to_string(),
            fingerprint: encoded_fingerprint.clone(),
        })
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error inserting blocklist lookup entry")?;

    state
        .store
        .insert_blocklist_entry(storage::BlocklistNew {
            merchant_id: merchant_id.to_string(),
            fingerprint_id: bin.to_string(),
            data_kind,
            metadata: None,
            created_at: common_utils::date_time::now(),
        })
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error inserting pm blocklist item")
}

async fn delete_card_bin_blocklist_entry(
    state: &AppState,
    bin: &str,
    merchant_id: &str,
) -> RouterResult<storage::Blocklist> {
    let merchant_secret = get_merchant_fingerprint_secret(state, merchant_id).await?;
    let bin_fingerprint = crypto::HmacSha512
        .sign_message(merchant_secret.as_bytes(), bin.as_bytes())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error when hashing card bin")?;
    let encoded_fingerprint = hex::encode(bin_fingerprint);

    state
        .store
        .delete_blocklist_lookup_entry_by_merchant_id_fingerprint(merchant_id, &encoded_fingerprint)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "could not find a blocklist entry for the given bin".to_string(),
        })?;

    state
        .store
        .delete_blocklist_entry_by_merchant_id_fingerprint_id(merchant_id, bin)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "could not find a blocklist entry for the given bin".to_string(),
        })
}
