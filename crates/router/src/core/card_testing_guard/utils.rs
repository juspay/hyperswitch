use api_models::card_testing_guard;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface, Secret};

use router_env::logger;

use super::{errors, SessionState};
use crate::{
    consts,
    core::errors::{RouterResult, StorageErrorExt},
    types::{domain::{
        self,
        types::{self as domain_types, AsyncLift},
    }, storage,},
    utils::{
        self,
        crypto::{self, SignMessage},
    },
};

pub async fn generate_fingerprint(
    state: &SessionState,
    payment_method_data: Option<&api_models::payments::PaymentMethodData>,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<Secret<String>> {

    let merchant_id = merchant_account.get_id();
    let merchant_fingerprint_secret = get_merchant_fingerprint_secret(state, merchant_id).await?;
    
    let card_number_fingerprint = payment_method_data
    .as_ref()
    .and_then(|pm_data| match pm_data {
        api_models::payments::PaymentMethodData::Card(card) => {
            crypto::HmacSha512::sign_message(
                &crypto::HmacSha512,
                merchant_fingerprint_secret.as_bytes(),
                card.card_number.clone().get_card_no().as_bytes(),
            )
            .attach_printable("error in pm fingerprint creation")
            .map_or_else(
                |err| {
                    logger::error!(error=?err);
                    None
                },
                Some,
            )
        }
        _ => None,
    })
    .map(hex::encode);

    card_number_fingerprint
    .map(|fingerprint| Secret::new(fingerprint))
    .ok_or_else(|| {
        todo!()
    })
}

pub async fn get_merchant_fingerprint_secret(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
) -> RouterResult<String> {
    let db = state.store.as_ref();
    let key_manager_state = &(&*state).into();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(key_manager_state, merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_fingerprint_secret_key = merchant_account.clone().fingerprint_secret_key;

    match merchant_fingerprint_secret_key {
        Some(fingerprint_secret_key) => {
            Ok(fingerprint_secret_key.get_inner().clone().expose())
        },
        None => {
            let new_fingerprint =
                utils::generate_id(consts::FINGERPRINT_SECRET_LENGTH, "fs");
            let fingerprint_secret = Some(Secret::new(new_fingerprint.clone()));
            let _ = db.update_specific_fields_in_merchant(
                key_manager_state, 
                merchant_id, 
                storage::MerchantAccountUpdate::FingerprintSecretKeyUpdate { 
                    fingerprint_secret_key: AsyncLift::async_lift(fingerprint_secret, |inner| async {
                                                    domain_types::crypto_operation(
                                                        &key_manager_state,
                                                        common_utils::type_name!(domain::MerchantAccount),
                                                        domain::types::CryptoOperation::EncryptOptional(inner),
                                                        common_utils::types::keymanager::Identifier::Merchant(key_store.merchant_id.clone()),
                                                        key_store.key.clone().into_inner().peek(),
                                                        )
                                                    .await
                                                    .and_then(|val| val.try_into_optionaloperation())
                                                 })
                                                .await
                                                .unwrap()
                }, 
                &key_store)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("error updating the merchant account when creating payment connector")?;

            Ok(new_fingerprint)
        }
    }
}

pub async fn update_card_testing_guard_for_merchant(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    payload: card_testing_guard::UpdateCardTestingGuardRequest,
) -> CustomResult<card_testing_guard::UpdateCardTestingGuardResponse, errors::ApiErrorResponse> {

    let db = state.store.as_ref();
    let key_manager_state = &(&*state).into();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let _ = db.update_specific_fields_in_merchant(
        key_manager_state, 
        merchant_id, 
        storage::MerchantAccountUpdate::CardTestingGuardUpdate { 
            card_ip_blocking: payload.card_ip_blocking, 
            guest_user_card_blocking: payload.guest_user_card_blocking,
            customer_id_blocking: payload.customer_id_blocking,
        },
        &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error updating the merchant account when creating payment connector")?;

    Ok(card_testing_guard::UpdateCardTestingGuardResponse {
        card_ip_blocking_status: get_card_testing_guard_status(payload.card_ip_blocking)?,
        guest_user_card_blocking_status: get_card_testing_guard_status(payload.guest_user_card_blocking)?,
        customer_id_blocking_status: get_card_testing_guard_status(payload.customer_id_blocking)?,
    })
}

pub fn get_card_testing_guard_status(
    status: bool
) -> RouterResult<String> {
    match status {
        true => Ok("Enabled".to_owned()),
        false => Ok("Disabled".to_owned()),
    }
}