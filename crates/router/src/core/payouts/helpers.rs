use common_utils::ext_traits::ByteSliceExt;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use storage_models::encryption::Encryption;

use crate::{
    core::{
        errors::{self, RouterResult},
        payment_methods::{cards, vault},
        payments::CustomerDetails,
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    types::{
        api::{self, enums as api_enums},
        domain::{
            self,
            types::{self as domain_types, AsyncLift},
        },
        storage,
    },
    utils,
};

pub async fn make_payout_method_data<'a>(
    state: &'a AppState,
    payout_method_data: &Option<api::PayoutMethodData>,
    payout_attempt: &storage::PayoutAttempt,
) -> RouterResult<Option<api::PayoutMethodData>> {
    let db = &*state.store;
    match (
        payout_method_data.to_owned(),
        payout_attempt.payout_token.to_owned(),
    ) {
        // Get operation
        (None, Some(payout_token)) => {
            let pm = vault::Vault::get_payout_method_data_from_locker(state, &payout_token)
                .await
                .attach_printable(
                    "Payout method for given token not found or there was a problem fetching it",
                );
            match pm {
                Ok((pm, supplementary_data)) => {
                    utils::when(
                        supplementary_data
                            .customer_id
                            .ne(&Some(payout_attempt.customer_id.to_owned())),
                        || {
                            Err(errors::ApiErrorResponse::PreconditionFailed { message: "customer associated with payout method and customer passed in payout are not same".into() })
                        },
                    )?;
                    Ok(pm)
                }
                Err(_e) => {
                    let pm = cards::get_payment_method_from_hs_locker(
                        state,
                        &payout_attempt.customer_id,
                        &payout_attempt.merchant_id,
                        &payout_token,
                    )
                    .await
                    .attach_printable("Failed to fetch payout method details from basilisk")
                    .change_context(errors::ApiErrorResponse::PayoutNotFound)?;
                    let pm_parsed: api::PayoutMethodData = pm
                        .peek()
                        .as_bytes()
                        .to_vec()
                        .parse_struct("PayoutMethodData")
                        .change_context(errors::ApiErrorResponse::InternalServerError)?;

                    Ok(Some(pm_parsed))
                }
            }
        }

        // Create / Update operation
        (Some(payout_method), payout_token) => {
            let lookup_key = vault::Vault::store_payout_method_data_in_locker(
                state,
                payout_token.to_owned(),
                &payout_method,
                Some(payout_attempt.customer_id.to_owned()),
            )
            .await?;

            // Update payout_token in payout_attempt table
            if payout_token.is_none() {
                let payout_update = storage::PayoutAttemptUpdate::PayoutTokenUpdate {
                    payout_token: lookup_key,
                    status: payout_attempt.status,
                };
                db.update_payout_attempt_by_merchant_id_payout_id(
                    &payout_attempt.merchant_id,
                    &payout_attempt.payout_id,
                    payout_update,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error updating token in payout attempt")?;
            }
            Ok(Some(payout_method))
        }

        // Ignore if nothing is passed
        _ => Ok(None),
    }
}

pub async fn save_payout_data_to_locker(
    state: &AppState,
    payout_attempt: &storage::payout_attempt::PayoutAttempt,
    payout_method_data: &api::PayoutMethodData,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<()> {
    match payout_method_data {
        api_models::payouts::PayoutMethodData::Card(card) => {
            let card_details = api::CardDetail {
                card_number: card.card_number.to_owned(),
                card_exp_month: card.expiry_month.to_owned(),
                card_exp_year: card.expiry_year.to_owned(),
                card_holder_name: Some(card.card_holder_name.to_owned()),
            };
            let stored_card_resp = cards::call_to_card_hs(
                state,
                &card_details,
                &payout_attempt.customer_id,
                merchant_account,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
            match stored_card_resp.duplicate {
                Some(false) => {
                    let db = &*state.store;
                    let update_payout = storage::PayoutsUpdate::PayoutMethodIdUpdate {
                        payout_method_id: Some(stored_card_resp.card_reference),
                    };
                    db.update_payout_by_merchant_id_payout_id(
                        &merchant_account.merchant_id.clone(),
                        &payout_attempt.payout_id,
                        update_payout,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error updating payouts in saved payout method")?;
                    Ok(())
                }
                _ => Ok(()),
            }
        }
        api_models::payouts::PayoutMethodData::Bank(_) => {
            let db = &*state.store;
            let key = domain_types::get_merchant_enc_key(db, merchant_account.merchant_id.clone())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to get key from merchant key store")?;
            let enc_str = async {
                serde_json::to_value(payout_method_data.to_owned())
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encode payout method data")
                    .ok()
                    .map(|v| {
                        let secret: Secret<String> = Secret::new(v.to_string());
                        secret
                    })
                    .async_lift(|inner| domain_types::encrypt_optional(inner, &key))
                    .await
            }
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encrypt payout method data")?
            .map(Encryption::from)
            .map(|e| e.into_inner())
            .map_or(Err(errors::ApiErrorResponse::InternalServerError), |e| {
                Ok(hex::encode(e))
            })?;
            let stored_resp = cards::call_to_generic_hs(
                state,
                &enc_str,
                &payout_attempt.customer_id,
                merchant_account,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
            let update_payout = storage::PayoutsUpdate::PayoutMethodIdUpdate {
                payout_method_id: Some(stored_resp.card_reference),
            };
            db.update_payout_by_merchant_id_payout_id(
                &merchant_account.merchant_id,
                &payout_attempt.payout_id,
                update_payout,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payouts in saved payout method")?;
            Ok(())
        }
    }
}

pub async fn get_or_create_customer_details(
    state: &AppState,
    customer_details: &CustomerDetails,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<Option<domain::Customer>> {
    let db: &dyn StorageInterface = &*state.store;
    // Create customer_id if not passed in request
    let customer_id =
        core_utils::get_or_generate_id("customer_id", &customer_details.customer_id, "cust")?;
    let merchant_id = &merchant_account.merchant_id;

    let key = domain_types::get_merchant_enc_key(db, merchant_id.to_string())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    match db
        .find_customer_optional_by_customer_id_merchant_id(&customer_id, merchant_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
    {
        Some(customer) => Ok(Some(customer)),
        None => {
            let customer = domain::Customer {
                customer_id,
                merchant_id: merchant_id.to_string(),
                name: domain_types::encrypt_optional(customer_details.name.to_owned(), &key)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
                email: domain_types::encrypt_optional(
                    customer_details.email.to_owned().map(|e| e.expose()),
                    &key,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
                phone: domain_types::encrypt_optional(customer_details.phone.to_owned(), &key)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
                description: None,
                phone_country_code: customer_details.phone_country_code.to_owned(),
                metadata: None,
                connector_customer: None,
                id: None,
                created_at: common_utils::date_time::now(),
                modified_at: common_utils::date_time::now(),
            };

            Ok(Some(db.insert_customer(customer).await.change_context(
                errors::ApiErrorResponse::InternalServerError,
            )?))
        }
    }
}

pub fn is_payout_initiated(status: api_enums::PayoutStatus) -> bool {
    matches!(
        status,
        api_enums::PayoutStatus::Pending | api_enums::PayoutStatus::RequiresFulfillment
    )
}

pub fn is_payout_terminal_state(status: api_enums::PayoutStatus) -> bool {
    !matches!(
        status,
        api_enums::PayoutStatus::Pending
            | api_enums::PayoutStatus::RequiresCreation
            | api_enums::PayoutStatus::RequiresFulfillment
            | api_enums::PayoutStatus::RequiresPayoutMethodData
    )
}

pub fn is_payout_err_state(status: api_enums::PayoutStatus) -> bool {
    matches!(
        status,
        api_enums::PayoutStatus::Cancelled
            | api_enums::PayoutStatus::Failed
            | api_enums::PayoutStatus::Ineligible
    )
}

pub fn is_eligible_for_local_payout_cancellation(status: api_enums::PayoutStatus) -> bool {
    matches!(
        status,
        api_enums::PayoutStatus::RequiresCreation
            | api_enums::PayoutStatus::RequiresPayoutMethodData,
    )
}
