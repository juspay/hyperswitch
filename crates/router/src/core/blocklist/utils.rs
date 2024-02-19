use api_models::blocklist as api_blocklist;
use common_enums::MerchantDecision;
use common_utils::errors::CustomResult;
use diesel_models::configs;
use error_stack::{IntoReport, ResultExt};
use masking::StrongSecret;

use super::{errors, transformers::generate_fingerprint, AppState};
use crate::{
    consts,
    core::{
        errors::{RouterResult, StorageErrorExt},
        payments::PaymentData,
    },
    logger,
    types::{domain, storage, transformers::ForeignInto},
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

        api_blocklist::DeleteFromBlocklistRequest::Fingerprint(fingerprint_id) => state
            .store
            .delete_blocklist_entry_by_merchant_id_fingerprint_id(&merchant_id, &fingerprint_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
                message: "no blocklist record for the given fingerprint id was found".to_string(),
            })?,
    };

    Ok(blocklist_entry.foreign_into())
}

pub async fn toggle_blocklist_guard_for_merchant(
    state: &AppState,
    merchant_id: String,
    query: api_blocklist::ToggleBlocklistQuery,
) -> CustomResult<api_blocklist::ToggleBlocklistResponse, errors::ApiErrorResponse> {
    let key = get_blocklist_guard_key(merchant_id.as_str());
    let maybe_guard = state.store.find_config_by_key(&key).await;
    let new_config = configs::ConfigNew {
        key: key.clone(),
        config: query.status.to_string(),
    };
    match maybe_guard {
        Ok(_config) => {
            let updated_config = configs::ConfigUpdate::Update {
                config: Some(query.status.to_string()),
            };
            state
                .store
                .update_config_by_key(&key, updated_config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error enabling the blocklist guard")?;
        }
        Err(e) if e.current_context().is_db_not_found() => {
            state
                .store
                .insert_config(new_config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error enabling the blocklist guard")?;
        }
        Err(e) => {
            logger::error!(error=?e);
            Err(e)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error enabling the blocklist guard")?;
        }
    };
    let guard_status = if query.status { "enabled" } else { "disabled" };
    Ok(api_blocklist::ToggleBlocklistResponse {
        blocklist_guard_status: guard_status.to_string(),
    })
}

/// Provides the identifier for the specific merchant's blocklist guard config
#[inline(always)]
pub fn get_blocklist_guard_key(merchant_id: &str) -> String {
    format!("guard_blocklist_for_{merchant_id}")
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

            state
                .store
                .insert_blocklist_entry(storage::BlocklistNew {
                    merchant_id: merchant_id.clone(),
                    fingerprint_id: fingerprint_id.clone(),
                    data_kind: api_models::enums::enums::BlocklistDataKind::PaymentMethod,
                    metadata: None,
                    created_at: common_utils::date_time::now(),
                })
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to add fingerprint to blocklist")?
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

fn get_merchant_fingerprint_secret_key(merchant_id: &str) -> String {
    format!("fingerprint_secret_{merchant_id}")
}

async fn duplicate_check_insert_bin(
    bin: &str,
    state: &AppState,
    merchant_id: &str,
    data_kind: common_enums::BlocklistDataKind,
) -> RouterResult<storage::Blocklist> {
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
    state
        .store
        .delete_blocklist_entry_by_merchant_id_fingerprint_id(merchant_id, bin)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "could not find a blocklist entry for the given bin".to_string(),
        })
}

pub async fn validate_data_for_blocklist<F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut PaymentData<F>,
) -> CustomResult<bool, errors::ApiErrorResponse>
where
    F: Send + Clone,
{
    let db = &state.store;
    let merchant_id = &merchant_account.merchant_id;
    let merchant_fingerprint_secret =
        get_merchant_fingerprint_secret(state, merchant_id.as_str()).await?;

    // Hashed Fingerprint to check whether or not this payment should be blocked.
    let card_number_fingerprint = if let Some(api_models::payments::PaymentMethodData::Card(card)) = payment_data.payment_method_data.as_ref() {
            generate_fingerprint(
                state,
                StrongSecret::new(card.card_number.clone().get_card_no()),
                StrongSecret::new(merchant_fingerprint_secret.clone()),
                api_models::enums::LockerChoice::Tartarus,
            )
            .await
            .attach_printable("error in pm fingerprint creation")
            .map_or_else(
                |err| {
                    logger::error!(error=?err);
                    None
                },
                Some,
            )
            .map(|payload| payload.card_fingerprint)
    } else {
        None
    };

    // Hashed Cardbin to check whether or not this payment should be blocked.
    let card_bin_fingerprint = payment_data
        .payment_method_data
        .as_ref()
        .and_then(|pm_data| match pm_data {
            api_models::payments::PaymentMethodData::Card(card) => {
                Some(card.card_number.clone().get_card_isin())
            }
            _ => None,
        });

    // Hashed Extended Cardbin to check whether or not this payment should be blocked.
    let extended_card_bin_fingerprint =
        payment_data
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                api_models::payments::PaymentMethodData::Card(card) => {
                    Some(card.card_number.clone().get_extended_card_bin())
                }
                _ => None,
            });

    //validating the payment method.
    let mut blocklist_futures = Vec::new();
    if let Some(card_number_fingerprint) = card_number_fingerprint.as_ref() {
        blocklist_futures.push(db.find_blocklist_entry_by_merchant_id_fingerprint_id(
            merchant_id,
            card_number_fingerprint,
        ));
    }

    if let Some(card_bin_fingerprint) = card_bin_fingerprint.as_ref() {
        blocklist_futures.push(
            db.find_blocklist_entry_by_merchant_id_fingerprint_id(
                merchant_id,
                card_bin_fingerprint,
            ),
        );
    }

    if let Some(extended_card_bin_fingerprint) = extended_card_bin_fingerprint.as_ref() {
        blocklist_futures.push(db.find_blocklist_entry_by_merchant_id_fingerprint_id(
            merchant_id,
            extended_card_bin_fingerprint,
        ));
    }

    let blocklist_lookups = futures::future::join_all(blocklist_futures).await;

    let mut should_payment_be_blocked = false;
    for lookup in blocklist_lookups {
        match lookup {
            Ok(_) => {
                should_payment_be_blocked = true;
            }
            Err(e) => {
                logger::error!(blocklist_db_error=?e, "failed db operations for blocklist");
            }
        }
    }
    if should_payment_be_blocked {
        // Update db for attempt and intent status.
        db.update_payment_intent(
            payment_data.payment_intent.clone(),
            storage::PaymentIntentUpdate::RejectUpdate {
                status: common_enums::IntentStatus::Failed,
                merchant_decision: Some(MerchantDecision::Rejected.to_string()),
                updated_by: merchant_account.storage_scheme.to_string(),
            },
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable(
            "Failed to update status in Payment Intent to failed due to it being blocklisted",
        )?;

        // If payment is blocked not showing connector details
        let attempt_update = storage::PaymentAttemptUpdate::BlocklistUpdate {
            status: common_enums::AttemptStatus::Failure,
            error_code: Some(Some("HE-03".to_string())),
            error_message: Some(Some("This payment method is blocked".to_string())),
            updated_by: merchant_account.storage_scheme.to_string(),
        };
        db.update_payment_attempt_with_attempt_id(
            payment_data.payment_attempt.clone(),
            attempt_update,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable(
            "Failed to update status in Payment Attempt to failed, due to it being blocklisted",
        )?;

        Err(errors::ApiErrorResponse::PaymentBlockedError {
            code: 200,
            message: "This payment method is blocked".to_string(),
            status: "Failed".to_string(),
            reason: "Blocked".to_string(),
        }
        .into())
    } else {
        payment_data.payment_attempt.fingerprint_id  = generate_payment_fingerprint(
            state,
            payment_data.payment_attempt.merchant_id.clone(),
            payment_data.payment_method_data.clone(),
        )
        .await?;
        Ok(false)
    }
}

pub async fn generate_payment_fingerprint(
    state: &AppState,
    merchant_id: String,
    payment_method_data: Option<crate::types::api::PaymentMethodData>,
) -> CustomResult<Option<String>, errors::ApiErrorResponse> {
    let merchant_fingerprint_secret = get_merchant_fingerprint_secret(state, &merchant_id).await?;

    Ok(
        if let Some(api_models::payments::PaymentMethodData::Card(card)) =
            payment_method_data.as_ref()
        {
            generate_fingerprint(
                state,
                StrongSecret::new(card.card_number.clone().get_card_no()),
                StrongSecret::new(merchant_fingerprint_secret),
                api_models::enums::LockerChoice::Tartarus,
            )
            .await
            .attach_printable("error in pm fingerprint creation")
            .map_or_else(
                |err| {
                    logger::error!(error=?err);
                    None
                },
                Some,
            )
            .map(|payload| payload.card_fingerprint)
        } else {
            logger::error!("failed to retrieve card fingerprint");
            None
        },
    )
}
