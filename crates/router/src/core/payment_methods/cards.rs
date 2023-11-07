use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use api_models::{
    admin::{self, PaymentMethodsEnabled},
    enums::{self as api_enums},
    payment_methods::{
        CardDetailsPaymentMethod, CardNetworkTypes, PaymentExperienceTypes, PaymentMethodsData,
        RequestPaymentMethodTypes, RequiredFieldInfo, ResponsePaymentMethodIntermediate,
        ResponsePaymentMethodTypes, ResponsePaymentMethodsEnabled,
    },
    payments::BankCodeResponse,
};
use common_utils::{
    consts,
    ext_traits::{AsyncExt, StringExt, ValueExt},
    generate_id,
};
use diesel_models::{encryption::Encryption, enums as storage_enums, payment_method};
use error_stack::{report, IntoReport, ResultExt};
use masking::Secret;
use router_env::{instrument, tracing};

use crate::{
    configs::settings,
    core::{
        errors::{self, StorageErrorExt},
        payment_methods::{
            transformers::{self as payment_methods},
            vault,
        },
        payments::{
            helpers,
            routing::{self, SessionFlowRoutingInput},
        },
    },
    db, logger,
    pii::prelude::*,
    routes::{
        self,
        metrics::{self, request},
        payment_methods::ParentPaymentMethodToken,
    },
    services,
    types::{
        api::{self, routing as routing_types, PaymentMethodCreateExt},
        domain::{
            self,
            types::{decrypt, encrypt_optional, AsyncLift},
        },
        storage::{self, enums},
        transformers::ForeignFrom,
    },
    utils::{self, ConnectorResponseExt, OptionExt},
};

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_payment_method(
    db: &dyn db::StorageInterface,
    req: &api::PaymentMethodCreate,
    customer_id: &str,
    payment_method_id: &str,
    merchant_id: &str,
    pm_metadata: Option<serde_json::Value>,
    payment_method_data: Option<Encryption>,
    key_store: &domain::MerchantKeyStore,
) -> errors::CustomResult<storage::PaymentMethod, errors::ApiErrorResponse> {
    db.find_customer_by_customer_id_merchant_id(customer_id, merchant_id, key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let response = db
        .insert_payment_method(storage::PaymentMethodNew {
            customer_id: customer_id.to_string(),
            merchant_id: merchant_id.to_string(),
            payment_method_id: payment_method_id.to_string(),
            payment_method: req.payment_method,
            payment_method_type: req.payment_method_type,
            payment_method_issuer: req.payment_method_issuer.clone(),
            scheme: req.card_network.clone(),
            metadata: pm_metadata.map(masking::Secret::new),
            payment_method_data,
            ..storage::PaymentMethodNew::default()
        })
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in db")?;

    Ok(response)
}

#[instrument(skip_all)]
pub async fn add_payment_method(
    state: routes::AppState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    req.validate()?;
    let merchant_id = &merchant_account.merchant_id;
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;
    let response = match req.card.clone() {
        Some(card) => add_card_to_locker(
            &state,
            req.clone(),
            card,
            customer_id.clone(),
            merchant_account,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Add Card Failed"),
        None => {
            let pm_id = generate_id(consts::ID_LENGTH, "pm");
            let payment_method_response = api::PaymentMethodResponse {
                merchant_id: merchant_id.to_string(),
                customer_id: Some(customer_id.clone()),
                payment_method_id: pm_id,
                payment_method: req.payment_method,
                payment_method_type: req.payment_method_type,
                card: None,
                metadata: req.metadata.clone(),
                created: Some(common_utils::date_time::now()),
                recurring_enabled: false,           //[#219]
                installment_payment_enabled: false, //[#219]
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), //[#219]
            };
            Ok((payment_method_response, false))
        }
    };

    let (resp, is_duplicate) = response?;
    if !is_duplicate {
        let pm_metadata = resp.metadata.as_ref().map(|data| data.peek());

        let pm_card_details = resp
            .card
            .as_ref()
            .map(|card| PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone())));

        let pm_data_encrypted =
            create_encrypted_payment_method_data(key_store, pm_card_details).await;

        create_payment_method(
            &*state.store,
            &req,
            &customer_id,
            &resp.payment_method_id,
            &resp.merchant_id,
            pm_metadata.cloned(),
            pm_data_encrypted,
            key_store,
        )
        .await?;
    }

    Ok(resp).map(services::ApplicationResponse::Json)
}

#[instrument(skip_all)]
pub async fn update_customer_payment_method(
    state: routes::AppState,
    merchant_account: domain::MerchantAccount,
    req: api::PaymentMethodUpdate,
    payment_method_id: &str,
    key_store: domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = state.store.as_ref();
    let pm = db
        .delete_payment_method_by_merchant_id_payment_method_id(
            &merchant_account.merchant_id,
            payment_method_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    if pm.payment_method == enums::PaymentMethod::Card {
        delete_card_from_locker(
            &state,
            &pm.customer_id,
            &pm.merchant_id,
            &pm.payment_method_id,
        )
        .await?;
    };
    let new_pm = api::PaymentMethodCreate {
        payment_method: pm.payment_method,
        payment_method_type: pm.payment_method_type,
        payment_method_issuer: pm.payment_method_issuer,
        payment_method_issuer_code: pm.payment_method_issuer_code,
        card: req.card,
        metadata: req.metadata,
        customer_id: Some(pm.customer_id),
        card_network: req
            .card_network
            .as_ref()
            .map(|card_network| card_network.to_string()),
    };
    add_payment_method(state, new_pm, &merchant_account, &key_store).await
}

// Wrapper function to switch lockers

/// The response will be the tuple of PaymentMethodResponse and the duplication check of payment_method
pub async fn add_card_to_locker(
    state: &routes::AppState,
    req: api::PaymentMethodCreate,
    card: api::CardDetail,
    customer_id: String,
    merchant_account: &domain::MerchantAccount,
) -> errors::CustomResult<(api::PaymentMethodResponse, bool), errors::VaultError> {
    metrics::STORED_TO_LOCKER.add(&metrics::CONTEXT, 1, &[]);
    request::record_operation_time(
        async {
            add_card_hs(state, req, card, customer_id, merchant_account)
                .await
                .map_err(|error| {
                    metrics::CARD_LOCKER_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                    error
                })
        },
        &metrics::CARD_ADD_TIME,
        &[],
    )
    .await
}

pub async fn get_card_from_locker(
    state: &routes::AppState,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &str,
) -> errors::RouterResult<payment_methods::Card> {
    metrics::GET_FROM_LOCKER.add(&metrics::CONTEXT, 1, &[]);

    request::record_operation_time(
        async {
            get_card_from_hs_locker(state, customer_id, merchant_id, card_reference)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while getting card from basilisk_hs")
                .map_err(|error| {
                    metrics::CARD_LOCKER_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                    error
                })
        },
        &metrics::CARD_GET_TIME,
        &[],
    )
    .await
}

pub async fn delete_card_from_locker(
    state: &routes::AppState,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &str,
) -> errors::RouterResult<payment_methods::DeleteCardResp> {
    metrics::DELETE_FROM_LOCKER.add(&metrics::CONTEXT, 1, &[]);

    request::record_operation_time(
        async move {
            delete_card_from_hs_locker(state, customer_id, merchant_id, card_reference)
                .await
                .map_err(|error| {
                    metrics::CARD_LOCKER_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                    error
                })
        },
        &metrics::CARD_DELETE_TIME,
        &[],
    )
    .await
}

#[instrument(skip_all)]
pub async fn add_card_hs(
    state: &routes::AppState,
    req: api::PaymentMethodCreate,
    card: api::CardDetail,
    customer_id: String,
    merchant_account: &domain::MerchantAccount,
) -> errors::CustomResult<(api::PaymentMethodResponse, bool), errors::VaultError> {
    let payload = payment_methods::StoreLockerReq::LockerCard(payment_methods::StoreCardReq {
        merchant_id: &merchant_account.merchant_id,
        merchant_customer_id: customer_id.to_owned(),
        card: payment_methods::Card {
            card_number: card.card_number.to_owned(),
            name_on_card: card.card_holder_name.to_owned(),
            card_exp_month: card.card_exp_month.to_owned(),
            card_exp_year: card.card_exp_year.to_owned(),
            card_brand: None,
            card_isin: None,
            nick_name: card.nick_name.as_ref().map(masking::Secret::peek).cloned(),
        },
    });
    let store_card_payload = call_to_locker_hs(state, &payload, &customer_id).await?;

    let payment_method_resp = payment_methods::mk_add_card_response_hs(
        card,
        store_card_payload.card_reference,
        req,
        &merchant_account.merchant_id,
    );
    Ok((
        payment_method_resp,
        store_card_payload.duplicate.unwrap_or(false),
    ))
}

#[instrument(skip_all)]
pub async fn decode_and_decrypt_locker_data(
    key_store: &domain::MerchantKeyStore,
    enc_card_data: String,
) -> errors::CustomResult<Secret<String>, errors::VaultError> {
    // Fetch key
    let key = key_store.key.get_inner().peek();
    // Decode
    let decoded_bytes = hex::decode(&enc_card_data)
        .into_report()
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to decode hex string into bytes")?;
    // Decrypt
    decrypt(Some(Encryption::new(decoded_bytes.into())), key)
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)?
        .map_or(
            Err(report!(errors::VaultError::FetchPaymentMethodFailed)),
            |d| Ok(d.into_inner()),
        )
}

#[instrument(skip_all)]
pub async fn get_payment_method_from_hs_locker<'a>(
    state: &'a routes::AppState,
    key_store: &domain::MerchantKeyStore,
    customer_id: &str,
    merchant_id: &str,
    payment_method_reference: &'a str,
) -> errors::CustomResult<Secret<String>, errors::VaultError> {
    let locker = &state.conf.locker;
    #[cfg(not(feature = "kms"))]
    let jwekey = &state.conf.jwekey;
    #[cfg(feature = "kms")]
    let jwekey = &state.kms_secrets;

    let payment_method_data = if !locker.mock_locker {
        let request = payment_methods::mk_get_card_request_hs(
            jwekey,
            locker,
            customer_id,
            merchant_id,
            payment_method_reference,
        )
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)
        .attach_printable("Making get payment method request failed")?;
        let response = services::call_connector_api(state, request)
            .await
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable("Failed while executing call_connector_api for get_card");
        let jwe_body: services::JweBody = response
            .get_response_inner("JweBody")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)?;
        let decrypted_payload = payment_methods::get_decrypted_response_payload(jwekey, jwe_body)
            .await
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable("Error getting decrypted response payload for get card")?;
        let get_card_resp: payment_methods::RetrieveCardResp = decrypted_payload
            .parse_struct("RetrieveCardResp")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)?;
        let retrieve_card_resp = get_card_resp
            .payload
            .get_required_value("RetrieveCardRespPayload")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)?;
        let enc_card_data = retrieve_card_resp
            .enc_card_data
            .get_required_value("enc_card_data")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)?;
        decode_and_decrypt_locker_data(key_store, enc_card_data.peek().to_string()).await?
    } else {
        mock_get_payment_method(&*state.store, key_store, payment_method_reference)
            .await?
            .payment_method
            .payment_method_data
    };
    Ok(payment_method_data)
}

#[instrument(skip_all)]
pub async fn call_to_locker_hs<'a>(
    state: &routes::AppState,
    payload: &payment_methods::StoreLockerReq<'a>,
    customer_id: &str,
) -> errors::CustomResult<payment_methods::StoreCardRespPayload, errors::VaultError> {
    let locker = &state.conf.locker;
    #[cfg(not(feature = "kms"))]
    let jwekey = &state.conf.jwekey;
    #[cfg(feature = "kms")]
    let jwekey = &state.kms_secrets;
    let db = &*state.store;
    let stored_card_response = if !locker.mock_locker {
        let request = payment_methods::mk_add_locker_request_hs(jwekey, locker, payload).await?;
        let response = services::call_connector_api(state, request)
            .await
            .change_context(errors::VaultError::SaveCardFailed);

        let jwe_body: services::JweBody = response
            .get_response_inner("JweBody")
            .change_context(errors::VaultError::FetchCardFailed)?;

        let decrypted_payload = payment_methods::get_decrypted_response_payload(jwekey, jwe_body)
            .await
            .change_context(errors::VaultError::SaveCardFailed)
            .attach_printable("Error getting decrypted response payload")?;
        let stored_card_resp: payment_methods::StoreCardResp = decrypted_payload
            .parse_struct("StoreCardResp")
            .change_context(errors::VaultError::ResponseDeserializationFailed)?;
        stored_card_resp
    } else {
        let card_id = generate_id(consts::ID_LENGTH, "card");
        mock_call_to_locker_hs(db, &card_id, payload, None, None, Some(customer_id)).await?
    };

    let stored_card = stored_card_response
        .payload
        .get_required_value("StoreCardRespPayload")
        .change_context(errors::VaultError::SaveCardFailed)?;
    Ok(stored_card)
}

pub async fn update_payment_method(
    db: &dyn db::StorageInterface,
    pm: payment_method::PaymentMethod,
    pm_metadata: serde_json::Value,
) -> errors::CustomResult<(), errors::VaultError> {
    let pm_update = payment_method::PaymentMethodUpdate::MetadataUpdate {
        metadata: Some(pm_metadata),
    };
    db.update_payment_method(pm, pm_update)
        .await
        .change_context(errors::VaultError::UpdateInPaymentMethodDataTableFailed)?;
    Ok(())
}

#[instrument(skip_all)]
pub async fn get_card_from_hs_locker<'a>(
    state: &'a routes::AppState,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &'a str,
) -> errors::CustomResult<payment_methods::Card, errors::VaultError> {
    let locker = &state.conf.locker;
    #[cfg(not(feature = "kms"))]
    let jwekey = &state.conf.jwekey;
    #[cfg(feature = "kms")]
    let jwekey = &state.kms_secrets;

    if !locker.mock_locker {
        let request = payment_methods::mk_get_card_request_hs(
            jwekey,
            locker,
            customer_id,
            merchant_id,
            card_reference,
        )
        .await
        .change_context(errors::VaultError::FetchCardFailed)
        .attach_printable("Making get card request failed")?;
        let response = services::call_connector_api(state, request)
            .await
            .change_context(errors::VaultError::FetchCardFailed)
            .attach_printable("Failed while executing call_connector_api for get_card");
        let jwe_body: services::JweBody = response
            .get_response_inner("JweBody")
            .change_context(errors::VaultError::FetchCardFailed)?;
        let decrypted_payload = payment_methods::get_decrypted_response_payload(jwekey, jwe_body)
            .await
            .change_context(errors::VaultError::FetchCardFailed)
            .attach_printable("Error getting decrypted response payload for get card")?;
        let get_card_resp: payment_methods::RetrieveCardResp = decrypted_payload
            .parse_struct("RetrieveCardResp")
            .change_context(errors::VaultError::FetchCardFailed)?;
        let retrieve_card_resp = get_card_resp
            .payload
            .get_required_value("RetrieveCardRespPayload")
            .change_context(errors::VaultError::FetchCardFailed)?;
        retrieve_card_resp
            .card
            .get_required_value("Card")
            .change_context(errors::VaultError::FetchCardFailed)
    } else {
        let (get_card_resp, _) = mock_get_card(&*state.store, card_reference).await?;
        payment_methods::mk_get_card_response(get_card_resp)
            .change_context(errors::VaultError::ResponseDeserializationFailed)
    }
}

#[instrument(skip_all)]
pub async fn delete_card_from_hs_locker<'a>(
    state: &routes::AppState,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &'a str,
) -> errors::RouterResult<payment_methods::DeleteCardResp> {
    let locker = &state.conf.locker;
    #[cfg(not(feature = "kms"))]
    let jwekey = &state.conf.jwekey;
    #[cfg(feature = "kms")]
    let jwekey = &state.kms_secrets;

    let request = payment_methods::mk_delete_card_request_hs(
        jwekey,
        locker,
        customer_id,
        merchant_id,
        card_reference,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Making delete card request failed")?;

    if !locker.mock_locker {
        let response = services::call_connector_api(state, request)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while executing call_connector_api for delete card");
        let jwe_body: services::JweBody = response.get_response_inner("JweBody")?;
        let decrypted_payload = payment_methods::get_decrypted_response_payload(jwekey, jwe_body)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting decrypted response payload for delete card")?;
        let delete_card_resp: payment_methods::DeleteCardResp = decrypted_payload
            .parse_struct("DeleteCardResp")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Ok(delete_card_resp)
    } else {
        Ok(mock_delete_card_hs(&*state.store, card_reference)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("card_delete_failure_message")?)
    }
}

///Mock api for local testing
pub async fn mock_call_to_locker_hs<'a>(
    db: &dyn db::StorageInterface,
    card_id: &str,
    payload: &payment_methods::StoreLockerReq<'a>,
    card_cvc: Option<String>,
    payment_method_id: Option<String>,
    customer_id: Option<&str>,
) -> errors::CustomResult<payment_methods::StoreCardResp, errors::VaultError> {
    let mut locker_mock_up = storage::LockerMockUpNew {
        card_id: card_id.to_string(),
        external_id: uuid::Uuid::new_v4().to_string(),
        card_fingerprint: uuid::Uuid::new_v4().to_string(),
        card_global_fingerprint: uuid::Uuid::new_v4().to_string(),
        merchant_id: "".to_string(),
        card_number: "4111111111111111".to_string(),
        card_exp_year: "2099".to_string(),
        card_exp_month: "12".to_string(),
        card_cvc,
        payment_method_id,
        customer_id: customer_id.map(str::to_string),
        name_on_card: None,
        nickname: None,
        enc_card_data: None,
    };
    locker_mock_up = match payload {
        payment_methods::StoreLockerReq::LockerCard(store_card_req) => storage::LockerMockUpNew {
            merchant_id: store_card_req.merchant_id.to_string(),
            card_number: store_card_req.card.card_number.peek().to_string(),
            card_exp_year: store_card_req.card.card_exp_year.peek().to_string(),
            card_exp_month: store_card_req.card.card_exp_month.peek().to_string(),
            name_on_card: store_card_req.card.name_on_card.to_owned().expose_option(),
            nickname: store_card_req.card.nick_name.to_owned(),
            ..locker_mock_up
        },
        payment_methods::StoreLockerReq::LockerGeneric(store_generic_req) => {
            storage::LockerMockUpNew {
                merchant_id: store_generic_req.merchant_id.to_string(),
                enc_card_data: Some(store_generic_req.enc_data.to_owned()),
                ..locker_mock_up
            }
        }
    };

    let response = db
        .insert_locker_mock_up(locker_mock_up)
        .await
        .change_context(errors::VaultError::SaveCardFailed)?;
    let payload = payment_methods::StoreCardRespPayload {
        card_reference: response.card_id,
        duplicate: Some(false),
    };
    Ok(payment_methods::StoreCardResp {
        status: "SUCCESS".to_string(),
        error_code: None,
        error_message: None,
        payload: Some(payload),
    })
}

#[instrument(skip_all)]
pub async fn mock_get_card<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<(payment_methods::GetCardResponse, Option<String>), errors::VaultError> {
    let locker_mock_up = db
        .find_locker_by_card_id(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    let add_card_response = payment_methods::AddCardResponse {
        card_id: locker_mock_up
            .payment_method_id
            .unwrap_or(locker_mock_up.card_id),
        external_id: locker_mock_up.external_id,
        card_fingerprint: locker_mock_up.card_fingerprint.into(),
        card_global_fingerprint: locker_mock_up.card_global_fingerprint.into(),
        merchant_id: Some(locker_mock_up.merchant_id),
        card_number: locker_mock_up
            .card_number
            .try_into()
            .into_report()
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Invalid card number format from the mock locker")
            .map(Some)?,
        card_exp_year: Some(locker_mock_up.card_exp_year.into()),
        card_exp_month: Some(locker_mock_up.card_exp_month.into()),
        name_on_card: locker_mock_up.name_on_card.map(|card| card.into()),
        nickname: locker_mock_up.nickname,
        customer_id: locker_mock_up.customer_id,
        duplicate: locker_mock_up.duplicate,
    };
    Ok((
        payment_methods::GetCardResponse {
            card: add_card_response,
        },
        locker_mock_up.card_cvc,
    ))
}

#[instrument(skip_all)]
pub async fn mock_get_payment_method<'a>(
    db: &dyn db::StorageInterface,
    key_store: &domain::MerchantKeyStore,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::GetPaymentMethodResponse, errors::VaultError> {
    let locker_mock_up = db
        .find_locker_by_card_id(card_id)
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)?;
    let dec_data = if let Some(e) = locker_mock_up.enc_card_data {
        decode_and_decrypt_locker_data(key_store, e).await
    } else {
        Err(report!(errors::VaultError::FetchPaymentMethodFailed))
    }?;
    let payment_method_response = payment_methods::AddPaymentMethodResponse {
        payment_method_id: locker_mock_up
            .payment_method_id
            .unwrap_or(locker_mock_up.card_id),
        external_id: locker_mock_up.external_id,
        merchant_id: Some(locker_mock_up.merchant_id),
        nickname: locker_mock_up.nickname,
        customer_id: locker_mock_up.customer_id,
        duplicate: locker_mock_up.duplicate,
        payment_method_data: dec_data,
    };
    Ok(payment_methods::GetPaymentMethodResponse {
        payment_method: payment_method_response,
    })
}

#[instrument(skip_all)]
pub async fn mock_delete_card_hs<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::DeleteCardResp, errors::VaultError> {
    db.delete_locker_mock_up(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(payment_methods::DeleteCardResp {
        status: "SUCCESS".to_string(),
        error_code: None,
        error_message: None,
    })
}

#[instrument(skip_all)]
pub async fn mock_delete_card<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::DeleteCardResponse, errors::VaultError> {
    let locker_mock_up = db
        .delete_locker_mock_up(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(payment_methods::DeleteCardResponse {
        card_id: Some(locker_mock_up.card_id),
        external_id: Some(locker_mock_up.external_id),
        card_isin: None,
        status: "SUCCESS".to_string(),
    })
}
//------------------------------------------------------------------------------
pub fn get_banks(
    state: &routes::AppState,
    pm_type: api_enums::PaymentMethodType,
    connectors: Vec<String>,
) -> Result<Vec<BankCodeResponse>, errors::ApiErrorResponse> {
    let mut bank_names_hm: HashMap<String, HashSet<api_enums::BankNames>> = HashMap::new();

    if matches!(
        pm_type,
        api_enums::PaymentMethodType::Giropay | api_enums::PaymentMethodType::Sofort
    ) {
        Ok(vec![BankCodeResponse {
            bank_name: vec![],
            eligible_connectors: connectors,
        }])
    } else {
        let mut bank_code_responses = vec![];
        for connector in &connectors {
            if let Some(connector_bank_names) = state.conf.bank_config.0.get(&pm_type) {
                if let Some(connector_hash_set) = connector_bank_names.0.get(connector) {
                    bank_names_hm.insert(connector.clone(), connector_hash_set.banks.clone());
                } else {
                    logger::error!("Could not find any configured connectors for payment_method -> {pm_type} for connector -> {connector}");
                }
            } else {
                logger::error!("Could not find any configured banks for payment_method -> {pm_type} for connector -> {connector}");
            }
        }

        let vector_of_hashsets = bank_names_hm
            .values()
            .map(|bank_names_hashset| bank_names_hashset.to_owned())
            .collect::<Vec<_>>();

        let mut common_bank_names = HashSet::new();
        if let Some(first_element) = vector_of_hashsets.first() {
            common_bank_names = vector_of_hashsets
                .iter()
                .skip(1)
                .fold(first_element.to_owned(), |acc, hs| {
                    acc.intersection(hs).cloned().collect()
                });
        }

        if !common_bank_names.is_empty() {
            bank_code_responses.push(BankCodeResponse {
                bank_name: common_bank_names.clone().into_iter().collect(),
                eligible_connectors: connectors.clone(),
            });
        }

        for connector in connectors {
            if let Some(all_bank_codes_for_connector) = bank_names_hm.get(&connector) {
                let remaining_bank_codes: HashSet<_> = all_bank_codes_for_connector
                    .difference(&common_bank_names)
                    .collect();

                if !remaining_bank_codes.is_empty() {
                    bank_code_responses.push(BankCodeResponse {
                        bank_name: remaining_bank_codes
                            .into_iter()
                            .map(|ele| ele.to_owned())
                            .collect(),
                        eligible_connectors: vec![connector],
                    })
                }
            } else {
                logger::error!("Could not find any configured banks for payment_method -> {pm_type} for connector -> {connector}");
            }
        }
        Ok(bank_code_responses)
    }
}

fn get_val(str: String, val: &serde_json::Value) -> Option<String> {
    str.split('.')
        .try_fold(val, |acc, x| acc.get(x))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub async fn list_payment_methods(
    state: routes::AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    mut req: api::PaymentMethodListRequest,
) -> errors::RouterResponse<api::PaymentMethodListResponse> {
    let db = &*state.store;
    let pm_config_mapping = &state.conf.pm_filters;

    let payment_intent = helpers::verify_payment_intent_time_and_client_secret(
        db,
        &merchant_account,
        req.client_secret.clone(),
    )
    .await?;

    let shipping_address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(
                db,
                pi.shipping_address_id.clone(),
                &key_store,
                pi.payment_id.clone(),
                merchant_account.merchant_id.clone(),
                merchant_account.storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten();

    let billing_address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(
                db,
                pi.billing_address_id.clone(),
                &key_store,
                pi.payment_id.clone(),
                merchant_account.merchant_id.clone(),
                merchant_account.storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten();

    let customer = payment_intent
        .as_ref()
        .async_and_then(|pi| async {
            pi.customer_id
                .as_ref()
                .async_and_then(|cust| async {
                    db.find_customer_by_customer_id_merchant_id(
                        cust.as_str(),
                        &pi.merchant_id,
                        &key_store,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
                    .ok()
                })
                .await
        })
        .await;

    let payment_attempt = payment_intent
        .as_ref()
        .async_map(|pi| async {
            db.find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &pi.payment_id,
                &pi.merchant_id,
                &pi.active_attempt.get_id(),
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)
        })
        .await
        .transpose()?;

    let payment_type = payment_attempt.as_ref().map(|pa| {
        let amount = api::Amount::from(pa.amount);
        let mandate_type = if pa.mandate_id.is_some() {
            Some(api::MandateTransactionType::RecurringMandateTransaction)
        } else if pa.mandate_details.is_some() {
            Some(api::MandateTransactionType::NewMandateTransaction)
        } else {
            None
        };

        helpers::infer_payment_type(&amount, mandate_type.as_ref())
    });

    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &merchant_account.merchant_id,
            false,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let profile_id = payment_intent
        .as_ref()
        .async_map(|payment_intent| async {
            crate::core::utils::get_profile_id_from_business_details(
                payment_intent.business_country,
                payment_intent.business_label.as_ref(),
                &merchant_account,
                payment_intent.profile_id.as_ref(),
                db,
                false,
            )
            .await
            .attach_printable("Could not find profile id from business details")
        })
        .await
        .transpose()?;

    // filter out connectors based on the business country
    let filtered_mcas = helpers::filter_mca_based_on_business_profile(all_mcas, profile_id);

    logger::debug!(mca_before_filtering=?filtered_mcas);

    let mut response: Vec<ResponsePaymentMethodIntermediate> = vec![];
    for mca in filtered_mcas {
        let payment_methods = match mca.payment_methods_enabled {
            Some(pm) => pm,
            None => continue,
        };

        filter_payment_methods(
            payment_methods,
            &mut req,
            &mut response,
            payment_intent.as_ref(),
            payment_attempt.as_ref(),
            billing_address.as_ref(),
            mca.connector_name,
            pm_config_mapping,
            &state.conf.mandates.supported_payment_methods,
        )
        .await?;
    }

    if let Some((payment_attempt, payment_intent)) =
        payment_attempt.as_ref().zip(payment_intent.as_ref())
    {
        let routing_enabled_pms = HashSet::from([
            api_enums::PaymentMethod::BankTransfer,
            api_enums::PaymentMethod::BankDebit,
            api_enums::PaymentMethod::BankRedirect,
        ]);

        let routing_enabled_pm_types = HashSet::from([
            api_enums::PaymentMethodType::GooglePay,
            api_enums::PaymentMethodType::ApplePay,
            api_enums::PaymentMethodType::Klarna,
            api_enums::PaymentMethodType::Paypal,
        ]);

        let mut chosen = Vec::<api::SessionConnectorData>::new();
        for intermediate in &response {
            if routing_enabled_pm_types.contains(&intermediate.payment_method_type)
                || routing_enabled_pms.contains(&intermediate.payment_method)
            {
                let connector_data = api::ConnectorData::get_connector_by_name(
                    &state.clone().conf.connectors,
                    &intermediate.connector,
                    api::GetToken::from(intermediate.payment_method_type),
                    None,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("invalid connector name received")?;

                chosen.push(api::SessionConnectorData {
                    payment_method_type: intermediate.payment_method_type,
                    connector: connector_data,
                    business_sub_label: None,
                });
            }
        }
        let sfr = SessionFlowRoutingInput {
            state: &state,
            country: shipping_address.clone().and_then(|ad| ad.country),
            key_store: &key_store,
            merchant_account: &merchant_account,
            payment_attempt,
            payment_intent,
            chosen,
        };
        let result = routing::perform_session_flow_routing(sfr)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("error performing session flow routing")?;

        response.retain(|intermediate| {
            if !routing_enabled_pm_types.contains(&intermediate.payment_method_type)
                && !routing_enabled_pms.contains(&intermediate.payment_method)
            {
                return true;
            }

            if let Some(choice) = result.get(&intermediate.payment_method_type) {
                intermediate.connector == choice.connector.connector_name.to_string()
            } else {
                false
            }
        });

        let mut routing_info: storage::PaymentRoutingInfo = payment_attempt
            .straight_through_algorithm
            .clone()
            .map(|val| val.parse_value("PaymentRoutingInfo"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid PaymentRoutingInfo format found in payment attempt")?
            .unwrap_or_else(|| storage::PaymentRoutingInfo {
                algorithm: None,
                pre_routing_results: None,
            });

        let mut pre_routing_results: HashMap<
            api_enums::PaymentMethodType,
            routing_types::RoutableConnectorChoice,
        > = HashMap::new();

        for (pm_type, choice) in result {
            let routable_choice = routing_types::RoutableConnectorChoice {
                #[cfg(feature = "backwards_compatibility")]
                choice_kind: routing_types::RoutableChoiceKind::FullStruct,
                connector: choice
                    .connector
                    .connector_name
                    .to_string()
                    .parse()
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("")?,
                #[cfg(feature = "connector_choice_mca_id")]
                merchant_connector_id: choice.connector.merchant_connector_id,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                sub_label: choice.sub_label,
            };

            pre_routing_results.insert(pm_type, routable_choice);
        }

        routing_info.pre_routing_results = Some(pre_routing_results);

        let encoded = utils::Encode::<storage::PaymentRoutingInfo>::encode_to_value(&routing_info)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to serialize payment routing info to value")?;

        let attempt_update = storage::PaymentAttemptUpdate::UpdateTrackers {
            payment_token: None,
            connector: None,
            straight_through_algorithm: Some(encoded),
            amount_capturable: None,
            updated_by: merchant_account.storage_scheme.to_string(),
            merchant_connector_id: None,
        };

        state
            .store
            .update_payment_attempt_with_attempt_id(
                payment_attempt.clone(),
                attempt_update,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
    }

    let req = api_models::payments::PaymentsRequest::foreign_from((
        payment_attempt.as_ref(),
        shipping_address.as_ref(),
        billing_address.as_ref(),
        customer.as_ref(),
    ));
    let req_val = serde_json::to_value(req).ok();
    logger::debug!(filtered_payment_methods=?response);

    let mut payment_experiences_consolidated_hm: HashMap<
        api_enums::PaymentMethod,
        HashMap<api_enums::PaymentMethodType, HashMap<api_enums::PaymentExperience, Vec<String>>>,
    > = HashMap::new();

    let mut card_networks_consolidated_hm: HashMap<
        api_enums::PaymentMethod,
        HashMap<api_enums::PaymentMethodType, HashMap<api_enums::CardNetwork, Vec<String>>>,
    > = HashMap::new();

    let mut banks_consolidated_hm: HashMap<api_enums::PaymentMethodType, Vec<String>> =
        HashMap::new();

    let mut bank_debits_consolidated_hm =
        HashMap::<api_enums::PaymentMethodType, Vec<String>>::new();

    let mut bank_transfer_consolidated_hm =
        HashMap::<api_enums::PaymentMethodType, Vec<String>>::new();

    let mut required_fields_hm = HashMap::<
        api_enums::PaymentMethod,
        HashMap<api_enums::PaymentMethodType, HashMap<String, RequiredFieldInfo>>,
    >::new();

    for element in response.clone() {
        let payment_method = element.payment_method;
        let payment_method_type = element.payment_method_type;
        let connector = element.connector.clone();

        let connector_variant = api_enums::Connector::from_str(connector.as_str())
            .into_report()
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector",
            })
            .attach_printable_lazy(|| format!("unable to parse connector name {connector:?}"))?;
        state.conf.required_fields.0.get(&payment_method).map(
            |required_fields_hm_for_each_payment_method_type| {
                required_fields_hm_for_each_payment_method_type
                    .0
                    .get(&payment_method_type)
                    .map(|required_fields_hm_for_each_connector| {
                        required_fields_hm.entry(payment_method).or_default();
                        required_fields_hm_for_each_connector
                            .fields
                            .get(&connector_variant)
                            .map(|required_fields_final| {
                                let mut required_fields_hs = required_fields_final.common.clone();
                                if let Some(pa) = payment_attempt.as_ref() {
                                    if let Some(_mandate) = &pa.mandate_details {
                                        required_fields_hs
                                            .extend(required_fields_final.mandate.clone());
                                    } else {
                                        required_fields_hs
                                            .extend(required_fields_final.non_mandate.clone());
                                    }
                                }

                                {
                                    for (key, val) in &mut required_fields_hs {
                                        let temp = req_val
                                            .as_ref()
                                            .and_then(|r| get_val(key.to_owned(), r));
                                        if let Some(s) = temp {
                                            val.value = Some(s)
                                        };
                                    }
                                }

                                let existing_req_fields_hs = required_fields_hm
                                    .get_mut(&payment_method)
                                    .and_then(|inner_hm| inner_hm.get_mut(&payment_method_type));

                                // If payment_method_type already exist in required_fields_hm, extend the required_fields hs to existing hs.
                                if let Some(inner_hs) = existing_req_fields_hs {
                                    inner_hs.extend(required_fields_hs);
                                } else {
                                    required_fields_hm.get_mut(&payment_method).map(|inner_hm| {
                                        inner_hm.insert(payment_method_type, required_fields_hs)
                                    });
                                }
                            })
                    })
            },
        );

        if let Some(payment_experience) = element.payment_experience {
            if let Some(payment_method_hm) =
                payment_experiences_consolidated_hm.get_mut(&payment_method)
            {
                if let Some(payment_method_type_hm) =
                    payment_method_hm.get_mut(&payment_method_type)
                {
                    if let Some(vector_of_connectors) =
                        payment_method_type_hm.get_mut(&payment_experience)
                    {
                        vector_of_connectors.push(connector);
                    } else {
                        payment_method_type_hm.insert(payment_experience, vec![connector]);
                    }
                } else {
                    payment_method_hm.insert(
                        payment_method_type,
                        HashMap::from([(payment_experience, vec![connector])]),
                    );
                }
            } else {
                let inner_hm = HashMap::from([(payment_experience, vec![connector])]);
                let payment_method_type_hm = HashMap::from([(payment_method_type, inner_hm)]);
                payment_experiences_consolidated_hm.insert(payment_method, payment_method_type_hm);
            }
        }

        if let Some(card_networks) = element.card_networks {
            if let Some(payment_method_hm) = card_networks_consolidated_hm.get_mut(&payment_method)
            {
                if let Some(payment_method_type_hm) =
                    payment_method_hm.get_mut(&payment_method_type)
                {
                    for card_network in card_networks {
                        if let Some(vector_of_connectors) =
                            payment_method_type_hm.get_mut(&card_network)
                        {
                            let connector = element.connector.clone();
                            vector_of_connectors.push(connector);
                        } else {
                            let connector = element.connector.clone();
                            payment_method_type_hm.insert(card_network, vec![connector]);
                        }
                    }
                } else {
                    let mut inner_hashmap: HashMap<api_enums::CardNetwork, Vec<String>> =
                        HashMap::new();
                    for card_network in card_networks {
                        if let Some(vector_of_connectors) = inner_hashmap.get_mut(&card_network) {
                            let connector = element.connector.clone();
                            vector_of_connectors.push(connector);
                        } else {
                            let connector = element.connector.clone();
                            inner_hashmap.insert(card_network, vec![connector]);
                        }
                    }
                    payment_method_hm.insert(payment_method_type, inner_hashmap);
                }
            } else {
                let mut inner_hashmap: HashMap<api_enums::CardNetwork, Vec<String>> =
                    HashMap::new();
                for card_network in card_networks {
                    if let Some(vector_of_connectors) = inner_hashmap.get_mut(&card_network) {
                        let connector = element.connector.clone();
                        vector_of_connectors.push(connector);
                    } else {
                        let connector = element.connector.clone();
                        inner_hashmap.insert(card_network, vec![connector]);
                    }
                }
                let payment_method_type_hm = HashMap::from([(payment_method_type, inner_hashmap)]);
                card_networks_consolidated_hm.insert(payment_method, payment_method_type_hm);
            }
        }

        if element.payment_method == api_enums::PaymentMethod::BankRedirect {
            let connector = element.connector.clone();
            if let Some(vector_of_connectors) =
                banks_consolidated_hm.get_mut(&element.payment_method_type)
            {
                vector_of_connectors.push(connector);
            } else {
                banks_consolidated_hm.insert(element.payment_method_type, vec![connector]);
            }
        }

        if element.payment_method == api_enums::PaymentMethod::BankDebit {
            let connector = element.connector.clone();
            if let Some(vector_of_connectors) =
                bank_debits_consolidated_hm.get_mut(&element.payment_method_type)
            {
                vector_of_connectors.push(connector);
            } else {
                bank_debits_consolidated_hm.insert(element.payment_method_type, vec![connector]);
            }
        }

        if element.payment_method == api_enums::PaymentMethod::BankTransfer {
            let connector = element.connector.clone();
            if let Some(vector_of_connectors) =
                bank_transfer_consolidated_hm.get_mut(&element.payment_method_type)
            {
                vector_of_connectors.push(connector);
            } else {
                bank_transfer_consolidated_hm.insert(element.payment_method_type, vec![connector]);
            }
        }
    }

    let mut payment_method_responses: Vec<ResponsePaymentMethodsEnabled> = vec![];
    for key in payment_experiences_consolidated_hm.iter() {
        let mut payment_method_types = vec![];
        for payment_method_types_hm in key.1 {
            let mut payment_experience_types = vec![];
            for payment_experience_type in payment_method_types_hm.1 {
                payment_experience_types.push(PaymentExperienceTypes {
                    payment_experience_type: *payment_experience_type.0,
                    eligible_connectors: payment_experience_type.1.clone(),
                })
            }

            payment_method_types.push(ResponsePaymentMethodTypes {
                payment_method_type: *payment_method_types_hm.0,
                payment_experience: Some(payment_experience_types),
                card_networks: None,
                bank_names: None,
                bank_debits: None,
                bank_transfers: None,
                // Required fields for PayLater payment method
                required_fields: required_fields_hm
                    .get(key.0)
                    .and_then(|inner_hm| inner_hm.get(payment_method_types_hm.0))
                    .cloned(),
                surcharge_details: None,
            })
        }

        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: *key.0,
            payment_method_types,
        })
    }

    for key in card_networks_consolidated_hm.iter() {
        let mut payment_method_types = vec![];
        for payment_method_types_hm in key.1 {
            let mut card_network_types = vec![];
            for card_network_type in payment_method_types_hm.1 {
                card_network_types.push(CardNetworkTypes {
                    card_network: card_network_type.0.clone(),
                    eligible_connectors: card_network_type.1.clone(),
                    surcharge_details: None,
                })
            }

            payment_method_types.push(ResponsePaymentMethodTypes {
                payment_method_type: *payment_method_types_hm.0,
                card_networks: Some(card_network_types),
                payment_experience: None,
                bank_names: None,
                bank_debits: None,
                bank_transfers: None,
                // Required fields for Card payment method
                required_fields: required_fields_hm
                    .get(key.0)
                    .and_then(|inner_hm| inner_hm.get(payment_method_types_hm.0))
                    .cloned(),
                surcharge_details: None,
            })
        }

        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: *key.0,
            payment_method_types,
        })
    }

    let mut bank_redirect_payment_method_types = vec![];

    for key in banks_consolidated_hm.iter() {
        let payment_method_type = *key.0;
        let connectors = key.1.clone();
        let bank_names = get_banks(&state, payment_method_type, connectors)?;
        bank_redirect_payment_method_types.push({
            ResponsePaymentMethodTypes {
                payment_method_type,
                bank_names: Some(bank_names),
                payment_experience: None,
                card_networks: None,
                bank_debits: None,
                bank_transfers: None,
                // Required fields for BankRedirect payment method
                required_fields: required_fields_hm
                    .get(&api_enums::PaymentMethod::BankRedirect)
                    .and_then(|inner_hm| inner_hm.get(key.0))
                    .cloned(),
                surcharge_details: None,
            }
        })
    }

    if !bank_redirect_payment_method_types.is_empty() {
        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::BankRedirect,
            payment_method_types: bank_redirect_payment_method_types,
        });
    }

    let mut bank_debit_payment_method_types = vec![];

    for key in bank_debits_consolidated_hm.iter() {
        let payment_method_type = *key.0;
        let connectors = key.1.clone();
        bank_debit_payment_method_types.push({
            ResponsePaymentMethodTypes {
                payment_method_type,
                bank_names: None,
                payment_experience: None,
                card_networks: None,
                bank_debits: Some(api_models::payment_methods::BankDebitTypes {
                    eligible_connectors: connectors.clone(),
                }),
                bank_transfers: None,
                // Required fields for BankDebit payment method
                required_fields: required_fields_hm
                    .get(&api_enums::PaymentMethod::BankDebit)
                    .and_then(|inner_hm| inner_hm.get(key.0))
                    .cloned(),
                surcharge_details: None,
            }
        })
    }

    if !bank_debit_payment_method_types.is_empty() {
        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::BankDebit,
            payment_method_types: bank_debit_payment_method_types,
        });
    }

    let mut bank_transfer_payment_method_types = vec![];

    for key in bank_transfer_consolidated_hm.iter() {
        let payment_method_type = *key.0;
        let connectors = key.1.clone();
        bank_transfer_payment_method_types.push({
            ResponsePaymentMethodTypes {
                payment_method_type,
                bank_names: None,
                payment_experience: None,
                card_networks: None,
                bank_debits: None,
                bank_transfers: Some(api_models::payment_methods::BankTransferTypes {
                    eligible_connectors: connectors,
                }),
                // Required fields for BankTransfer payment method
                required_fields: required_fields_hm
                    .get(&api_enums::PaymentMethod::BankTransfer)
                    .and_then(|inner_hm| inner_hm.get(key.0))
                    .cloned(),
                surcharge_details: None,
            }
        })
    }

    if !bank_transfer_payment_method_types.is_empty() {
        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::BankTransfer,
            payment_method_types: bank_transfer_payment_method_types,
        });
    }

    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodListResponse {
            redirect_url: merchant_account.return_url,
            merchant_name: merchant_account.merchant_name,
            payment_type,
            payment_methods: payment_method_responses,
            mandate_payment: payment_attempt.and_then(|inner| inner.mandate_details).map(
                |d| match d {
                    data_models::mandates::MandateDataType::SingleUse(i) => {
                        api::MandateType::SingleUse(api::MandateAmountData {
                            amount: i.amount,
                            currency: i.currency,
                            start_date: i.start_date,
                            end_date: i.end_date,
                            metadata: i.metadata,
                        })
                    }
                    data_models::mandates::MandateDataType::MultiUse(Some(i)) => {
                        api::MandateType::MultiUse(Some(api::MandateAmountData {
                            amount: i.amount,
                            currency: i.currency,
                            start_date: i.start_date,
                            end_date: i.end_date,
                            metadata: i.metadata,
                        }))
                    }
                    data_models::mandates::MandateDataType::MultiUse(None) => {
                        api::MandateType::MultiUse(None)
                    }
                },
            ),
            show_surcharge_breakup_screen: false,
        },
    ))
}

#[allow(clippy::too_many_arguments)]
pub async fn filter_payment_methods(
    payment_methods: Vec<serde_json::Value>,
    req: &mut api::PaymentMethodListRequest,
    resp: &mut Vec<ResponsePaymentMethodIntermediate>,
    payment_intent: Option<&storage::PaymentIntent>,
    payment_attempt: Option<&storage::PaymentAttempt>,
    address: Option<&domain::Address>,
    connector: String,
    config: &settings::ConnectorFilters,
    supported_payment_methods_for_mandate: &settings::SupportedPaymentMethodsForMandate,
) -> errors::CustomResult<(), errors::ApiErrorResponse> {
    for payment_method in payment_methods.into_iter() {
        let parse_result = serde_json::from_value::<PaymentMethodsEnabled>(payment_method);
        if let Ok(payment_methods_enabled) = parse_result {
            let payment_method = payment_methods_enabled.payment_method;

            let allowed_payment_method_types = payment_intent
                .and_then(|payment_intent| {
                    payment_intent
                        .allowed_payment_method_types
                        .clone()
                        .map(|val| val.parse_value("Vec<PaymentMethodType>"))
                        .transpose()
                        .unwrap_or_else(|error| {
                            logger::error!(%error, "Failed to deserialize PaymentIntent allowed_payment_method_types"); None
                        })
                });

            for payment_method_type_info in payment_methods_enabled
                .payment_method_types
                .unwrap_or_default()
            {
                if filter_recurring_based(&payment_method_type_info, req.recurring_enabled)
                    && filter_installment_based(
                        &payment_method_type_info,
                        req.installment_payment_enabled,
                    )
                    && filter_amount_based(&payment_method_type_info, req.amount)
                {
                    let mut payment_method_object = payment_method_type_info;

                    let filter;
                    (
                        payment_method_object.accepted_countries,
                        req.accepted_countries,
                        filter,
                    ) = filter_pm_country_based(
                        &payment_method_object.accepted_countries,
                        &req.accepted_countries,
                    );
                    let filter2;
                    (
                        payment_method_object.accepted_currencies,
                        req.accepted_currencies,
                        filter2,
                    ) = filter_pm_currencies_based(
                        &payment_method_object.accepted_currencies,
                        &req.accepted_currencies,
                    );

                    let filter4 = filter_pm_card_network_based(
                        payment_method_object.card_networks.as_ref(),
                        req.card_networks.as_ref(),
                        &payment_method_object.payment_method_type,
                    );

                    let filter3 = if let Some(payment_intent) = payment_intent {
                        filter_payment_country_based(&payment_method_object, address).await?
                            && filter_payment_currency_based(payment_intent, &payment_method_object)
                            && filter_payment_amount_based(payment_intent, &payment_method_object)
                            && filter_payment_mandate_based(payment_attempt, &payment_method_object)
                                .await?
                    } else {
                        true
                    };

                    let filter5 = filter_pm_based_on_config(
                        config,
                        &connector,
                        &payment_method_object.payment_method_type,
                        payment_attempt,
                        &mut payment_method_object.card_networks,
                        &address.and_then(|inner| inner.country),
                        payment_attempt.and_then(|value| value.currency),
                    );

                    let filter6 = filter_pm_based_on_allowed_types(
                        allowed_payment_method_types.as_ref(),
                        &payment_method_object.payment_method_type,
                    );

                    let connector_variant = api_enums::Connector::from_str(connector.as_str())
                        .into_report()
                        .change_context(errors::ConnectorError::InvalidConnectorName)
                        .change_context(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "connector",
                        })
                        .attach_printable_lazy(|| {
                            format!("unable to parse connector name {connector:?}")
                        })?;
                    let filter7 = payment_attempt
                        .and_then(|attempt| attempt.mandate_details.as_ref())
                        .map(|_mandate_details| {
                            filter_pm_based_on_supported_payments_for_mandate(
                                supported_payment_methods_for_mandate,
                                &payment_method,
                                &payment_method_object.payment_method_type,
                                connector_variant,
                            )
                        })
                        .unwrap_or(true);

                    let connector = connector.clone();

                    let response_pm_type = ResponsePaymentMethodIntermediate::new(
                        payment_method_object,
                        connector,
                        payment_method,
                    );

                    if filter && filter2 && filter3 && filter4 && filter5 && filter6 && filter7 {
                        resp.push(response_pm_type);
                    }
                }
            }
        }
    }
    Ok(())
}

fn filter_pm_based_on_supported_payments_for_mandate(
    supported_payment_methods_for_mandate: &settings::SupportedPaymentMethodsForMandate,
    payment_method: &api_enums::PaymentMethod,
    payment_method_type: &api_enums::PaymentMethodType,
    connector: api_enums::Connector,
) -> bool {
    supported_payment_methods_for_mandate
        .0
        .get(payment_method)
        .and_then(|payment_method_type_hm| payment_method_type_hm.0.get(payment_method_type))
        .map(|supported_connectors| supported_connectors.connector_list.contains(&connector))
        .unwrap_or(false)
}

fn filter_pm_based_on_config<'a>(
    config: &'a crate::configs::settings::ConnectorFilters,
    connector: &'a str,
    payment_method_type: &'a api_enums::PaymentMethodType,
    payment_attempt: Option<&storage::PaymentAttempt>,
    card_network: &mut Option<Vec<api_enums::CardNetwork>>,
    country: &Option<api_enums::CountryAlpha2>,
    currency: Option<api_enums::Currency>,
) -> bool {
    config
        .0
        .get(connector)
        .or_else(|| config.0.get("default"))
        .and_then(|inner| match payment_method_type {
            api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit => {
                card_network_filter(country, currency, card_network, inner);

                payment_attempt
                    .and_then(|inner| inner.capture_method)
                    .and_then(|capture_method| {
                        (capture_method == storage_enums::CaptureMethod::Manual).then(|| {
                            filter_pm_based_on_capture_method_used(inner, payment_method_type)
                        })
                    })
            }
            payment_method_type => inner
                .0
                .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
                    *payment_method_type,
                ))
                .map(|value| global_country_currency_filter(value, country, currency)),
        })
        .unwrap_or(true)
}

///Filters the payment method list on basis of Capture methods, checks whether the connector issues Manual payments using cards or not if not it won't be visible in payment methods list
fn filter_pm_based_on_capture_method_used(
    payment_method_filters: &settings::PaymentMethodFilters,
    payment_method_type: &api_enums::PaymentMethodType,
) -> bool {
    payment_method_filters
        .0
        .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
            *payment_method_type,
        ))
        .and_then(|v| v.not_available_flows)
        .and_then(|v| v.capture_method)
        .map(|v| !matches!(v, api_enums::CaptureMethod::Manual))
        .unwrap_or(true)
}

fn card_network_filter(
    country: &Option<api_enums::CountryAlpha2>,
    currency: Option<api_enums::Currency>,
    card_network: &mut Option<Vec<api_enums::CardNetwork>>,
    payment_method_filters: &settings::PaymentMethodFilters,
) {
    if let Some(value) = card_network.as_mut() {
        let filtered_card_networks = value
            .iter()
            .filter(|&element| {
                let key = settings::PaymentMethodFilterKey::CardNetwork(element.clone());
                payment_method_filters
                    .0
                    .get(&key)
                    .map(|value| global_country_currency_filter(value, country, currency))
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();
        *value = filtered_card_networks;
    }
}

fn global_country_currency_filter(
    item: &settings::CurrencyCountryFlowFilter,
    country: &Option<api_enums::CountryAlpha2>,
    currency: Option<api_enums::Currency>,
) -> bool {
    let country_condition = item
        .country
        .as_ref()
        .zip(country.as_ref())
        .map(|(lhs, rhs)| lhs.contains(rhs));
    let currency_condition = item
        .currency
        .as_ref()
        .zip(currency)
        .map(|(lhs, rhs)| lhs.contains(&rhs));
    country_condition.unwrap_or(true) && currency_condition.unwrap_or(true)
}

fn filter_pm_card_network_based(
    pm_card_networks: Option<&Vec<api_enums::CardNetwork>>,
    request_card_networks: Option<&Vec<api_enums::CardNetwork>>,
    pm_type: &api_enums::PaymentMethodType,
) -> bool {
    match pm_type {
        api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit => {
            match (pm_card_networks, request_card_networks) {
                (Some(pm_card_networks), Some(request_card_networks)) => request_card_networks
                    .iter()
                    .all(|card_network| pm_card_networks.contains(card_network)),
                (None, Some(_)) => false,
                _ => true,
            }
        }
        _ => true,
    }
}
fn filter_pm_country_based(
    accepted_countries: &Option<admin::AcceptedCountries>,
    req_country_list: &Option<Vec<api_enums::CountryAlpha2>>,
) -> (
    Option<admin::AcceptedCountries>,
    Option<Vec<api_enums::CountryAlpha2>>,
    bool,
) {
    match (accepted_countries, req_country_list) {
        (None, None) => (None, None, true),
        (None, Some(ref r)) => (
            Some(admin::AcceptedCountries::EnableOnly(r.to_vec())),
            Some(r.to_vec()),
            true,
        ),
        (Some(l), None) => (Some(l.to_owned()), None, true),
        (Some(l), Some(ref r)) => {
            let updated = match l {
                admin::AcceptedCountries::EnableOnly(acc) => {
                    filter_accepted_enum_based(&Some(acc.clone()), &Some(r.to_owned()))
                        .map(admin::AcceptedCountries::EnableOnly)
                }

                admin::AcceptedCountries::DisableOnly(den) => {
                    filter_disabled_enum_based(&Some(den.clone()), &Some(r.to_owned()))
                        .map(admin::AcceptedCountries::DisableOnly)
                }

                admin::AcceptedCountries::AllAccepted => {
                    Some(admin::AcceptedCountries::AllAccepted)
                }
            };

            (updated, Some(r.to_vec()), true)
        }
    }
}

fn filter_pm_currencies_based(
    accepted_currency: &Option<admin::AcceptedCurrencies>,
    req_currency_list: &Option<Vec<api_enums::Currency>>,
) -> (
    Option<admin::AcceptedCurrencies>,
    Option<Vec<api_enums::Currency>>,
    bool,
) {
    match (accepted_currency, req_currency_list) {
        (None, None) => (None, None, true),
        (None, Some(ref r)) => (
            Some(admin::AcceptedCurrencies::EnableOnly(r.to_vec())),
            Some(r.to_vec()),
            true,
        ),
        (Some(l), None) => (Some(l.to_owned()), None, true),
        (Some(l), Some(ref r)) => {
            let updated = match l {
                admin::AcceptedCurrencies::EnableOnly(acc) => {
                    filter_accepted_enum_based(&Some(acc.clone()), &Some(r.to_owned()))
                        .map(admin::AcceptedCurrencies::EnableOnly)
                }

                admin::AcceptedCurrencies::DisableOnly(den) => {
                    filter_disabled_enum_based(&Some(den.clone()), &Some(r.to_owned()))
                        .map(admin::AcceptedCurrencies::DisableOnly)
                }

                admin::AcceptedCurrencies::AllAccepted => {
                    Some(admin::AcceptedCurrencies::AllAccepted)
                }
            };

            (updated, Some(r.to_vec()), true)
        }
    }
}

fn filter_accepted_enum_based<T: Eq + std::hash::Hash + Clone>(
    left: &Option<Vec<T>>,
    right: &Option<Vec<T>>,
) -> Option<Vec<T>> {
    match (left, right) {
        (Some(ref l), Some(ref r)) => {
            let a: HashSet<&T> = HashSet::from_iter(l.iter());
            let b: HashSet<&T> = HashSet::from_iter(r.iter());

            let y: Vec<T> = a.intersection(&b).map(|&i| i.to_owned()).collect();
            Some(y)
        }
        (Some(ref l), None) => Some(l.to_vec()),
        (_, _) => None,
    }
}

fn filter_disabled_enum_based<T: Eq + std::hash::Hash + Clone>(
    left: &Option<Vec<T>>,
    right: &Option<Vec<T>>,
) -> Option<Vec<T>> {
    match (left, right) {
        (Some(ref l), Some(ref r)) => {
            let mut enabled = Vec::new();
            for element in r {
                if !l.contains(element) {
                    enabled.push(element.to_owned());
                }
            }
            Some(enabled)
        }
        (None, Some(r)) => Some(r.to_vec()),
        (_, _) => None,
    }
}

fn filter_amount_based(payment_method: &RequestPaymentMethodTypes, amount: Option<i64>) -> bool {
    let min_check = amount
        .and_then(|amt| {
            payment_method
                .minimum_amount
                .map(|min_amt| amt >= min_amt.into())
        })
        .unwrap_or(true);
    let max_check = amount
        .and_then(|amt| {
            payment_method
                .maximum_amount
                .map(|max_amt| amt <= max_amt.into())
        })
        .unwrap_or(true);
    // let min_check = match (amount, payment_method.minimum_amount) {
    //     (Some(amt), Some(min_amt)) => amt >= min_amt,
    //     (_, _) => true,
    // };
    // let max_check = match (amount, payment_method.maximum_amount) {
    //     (Some(amt), Some(max_amt)) => amt <= max_amt,
    //     (_, _) => true,
    // };
    min_check && max_check
}

fn filter_pm_based_on_allowed_types(
    allowed_types: Option<&Vec<api_enums::PaymentMethodType>>,
    payment_method_type: &api_enums::PaymentMethodType,
) -> bool {
    allowed_types.map_or(true, |pm| pm.contains(payment_method_type))
}

fn filter_recurring_based(
    payment_method: &RequestPaymentMethodTypes,
    recurring_enabled: Option<bool>,
) -> bool {
    recurring_enabled.map_or(true, |enabled| payment_method.recurring_enabled == enabled)
}

fn filter_installment_based(
    payment_method: &RequestPaymentMethodTypes,
    installment_payment_enabled: Option<bool>,
) -> bool {
    installment_payment_enabled.map_or(true, |enabled| {
        payment_method.installment_payment_enabled == enabled
    })
}

async fn filter_payment_country_based(
    pm: &RequestPaymentMethodTypes,
    address: Option<&domain::Address>,
) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
    Ok(address.map_or(true, |address| {
        address.country.as_ref().map_or(true, |country| {
            pm.accepted_countries.as_ref().map_or(true, |ac| match ac {
                admin::AcceptedCountries::EnableOnly(acc) => acc.contains(country),
                admin::AcceptedCountries::DisableOnly(den) => !den.contains(country),
                admin::AcceptedCountries::AllAccepted => true,
            })
        })
    }))
}

fn filter_payment_currency_based(
    payment_intent: &storage::PaymentIntent,
    pm: &RequestPaymentMethodTypes,
) -> bool {
    payment_intent.currency.map_or(true, |currency| {
        pm.accepted_currencies.as_ref().map_or(true, |ac| match ac {
            admin::AcceptedCurrencies::EnableOnly(acc) => acc.contains(&currency),
            admin::AcceptedCurrencies::DisableOnly(den) => !den.contains(&currency),
            admin::AcceptedCurrencies::AllAccepted => true,
        })
    })
}

fn filter_payment_amount_based(
    payment_intent: &storage::PaymentIntent,
    pm: &RequestPaymentMethodTypes,
) -> bool {
    let amount = payment_intent.amount;
    pm.maximum_amount.map_or(true, |amt| amount < amt.into())
        && pm.minimum_amount.map_or(true, |amt| amount > amt.into())
}

async fn filter_payment_mandate_based(
    payment_attempt: Option<&storage::PaymentAttempt>,
    pm: &RequestPaymentMethodTypes,
) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
    let recurring_filter = if !pm.recurring_enabled {
        payment_attempt.map_or(true, |pa| pa.mandate_id.is_none())
    } else {
        true
    };
    Ok(recurring_filter)
}

pub async fn do_list_customer_pm_fetch_customer_if_not_passed(
    state: routes::AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: Option<api::PaymentMethodListRequest>,
    customer_id: Option<&str>,
) -> errors::RouterResponse<api::CustomerPaymentMethodsListResponse> {
    let db = state.store.as_ref();
    if let Some(customer_id) = customer_id {
        list_customer_payment_method(&state, merchant_account, key_store, None, customer_id).await
    } else {
        let cloned_secret = req.and_then(|r| r.client_secret.as_ref().cloned());
        let payment_intent = helpers::verify_payment_intent_time_and_client_secret(
            db,
            &merchant_account,
            cloned_secret,
        )
        .await?;
        let customer_id = payment_intent
            .as_ref()
            .and_then(|intent| intent.customer_id.to_owned())
            .ok_or(errors::ApiErrorResponse::CustomerNotFound)?;
        list_customer_payment_method(
            &state,
            merchant_account,
            key_store,
            payment_intent,
            &customer_id,
        )
        .await
    }
}

pub async fn list_customer_payment_method(
    state: &routes::AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    payment_intent: Option<storage::PaymentIntent>,
    customer_id: &str,
) -> errors::RouterResponse<api::CustomerPaymentMethodsListResponse> {
    let db = &*state.store;
    db.find_customer_by_customer_id_merchant_id(
        customer_id,
        &merchant_account.merchant_id,
        &key_store,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let key = key_store.key.get_inner().peek();

    let is_requires_cvv = db
        .find_config_by_key_unwrap_or(
            format!("{}_requires_cvv", merchant_account.merchant_id).as_str(),
            Some("true".to_string()),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch requires_cvv config")?;

    let requires_cvv = is_requires_cvv.config != "false";

    let resp = db
        .find_payment_method_by_customer_id_merchant_id_list(
            customer_id,
            &merchant_account.merchant_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    //let mca = query::find_mca_by_merchant_id(conn, &merchant_account.merchant_id)?;
    let mut customer_pms = Vec::new();
    for pm in resp.into_iter() {
        let parent_payment_method_token = generate_id(consts::ID_LENGTH, "token");
        let hyperswitch_token = generate_id(consts::ID_LENGTH, "token");

        let card = if pm.payment_method == enums::PaymentMethod::Card {
            get_card_details(&pm, key, state, &hyperswitch_token).await?
        } else {
            None
        };

        #[cfg(feature = "payouts")]
        let pmd = if pm.payment_method == enums::PaymentMethod::BankTransfer {
            Some(
                get_lookup_key_for_payout_method(state, &key_store, &hyperswitch_token, &pm)
                    .await?,
            )
        } else {
            None
        };
        //Need validation for enabled payment method ,querying MCA
        let pma = api::CustomerPaymentMethod {
            payment_token: parent_payment_method_token.to_owned(),
            customer_id: pm.customer_id,
            payment_method: pm.payment_method,
            payment_method_type: pm.payment_method_type,
            payment_method_issuer: pm.payment_method_issuer,
            card,
            metadata: pm.metadata,
            payment_method_issuer_code: pm.payment_method_issuer_code,
            recurring_enabled: false,
            installment_payment_enabled: false,
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
            created: Some(pm.created_at),
            #[cfg(feature = "payouts")]
            bank_transfer: pmd,
            #[cfg(not(feature = "payouts"))]
            bank_transfer: None,
            requires_cvv,
        };
        customer_pms.push(pma.to_owned());

        let intent_created = payment_intent.as_ref().map(|intent| intent.created_at);

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;
        ParentPaymentMethodToken::create_key_for_token((
            &parent_payment_method_token,
            pma.payment_method,
        ))
        .insert(intent_created, hyperswitch_token, state)
        .await?;

        if let Some(metadata) = pma.metadata {
            let pm_metadata_vec: payment_methods::PaymentMethodMetadata = metadata
                .parse_value("PaymentMethodMetadata")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to deserialize metadata to PaymentmethodMetadata struct",
                )?;

            for pm_metadata in pm_metadata_vec.payment_method_tokenization {
                let key = format!(
                    "pm_token_{}_{}_{}",
                    parent_payment_method_token, pma.payment_method, pm_metadata.0
                );
                let current_datetime_utc = common_utils::date_time::now();
                let time_elapsed = current_datetime_utc
                    - payment_intent
                        .as_ref()
                        .map(|intent| intent.created_at)
                        .unwrap_or_else(|| current_datetime_utc);
                redis_conn
                    .set_key_with_expiry(
                        &key,
                        pm_metadata.1,
                        consts::TOKEN_TTL - time_elapsed.whole_seconds(),
                    )
                    .await
                    .change_context(errors::StorageError::KVError)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to add data in redis")?;
            }
        }
    }

    let response = api::CustomerPaymentMethodsListResponse {
        customer_payment_methods: customer_pms,
    };

    Ok(services::ApplicationResponse::Json(response))
}

async fn get_card_details(
    pm: &payment_method::PaymentMethod,
    key: &[u8],
    state: &routes::AppState,
    hyperswitch_token: &str,
) -> errors::RouterResult<Option<api::CardDetailFromLocker>> {
    let mut _card_decrypted =
        decrypt::<serde_json::Value, masking::WithType>(pm.payment_method_data.clone(), key)
            .await
            .change_context(errors::StorageError::DecryptionError)
            .attach_printable("unable to decrypt card details")
            .ok()
            .flatten()
            .map(|x| x.into_inner().expose())
            .and_then(|v| serde_json::from_value::<PaymentMethodsData>(v).ok())
            .and_then(|pmd| match pmd {
                PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
                _ => None,
            });

    Ok(Some(
        get_lookup_key_from_locker(state, hyperswitch_token, pm).await?,
    ))
}

pub async fn get_lookup_key_from_locker(
    state: &routes::AppState,
    payment_token: &str,
    pm: &storage::PaymentMethod,
) -> errors::RouterResult<api::CardDetailFromLocker> {
    let card = get_card_from_locker(
        state,
        &pm.customer_id,
        &pm.merchant_id,
        &pm.payment_method_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Error getting card from card vault")?;
    let card_detail = payment_methods::get_card_detail(pm, card)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Get Card Details Failed")?;
    let card = card_detail.clone();
    let resp =
        BasiliskCardSupport::create_payment_method_data_in_locker(state, payment_token, card, pm)
            .await?;
    Ok(resp)
}

#[cfg(feature = "payouts")]
pub async fn get_lookup_key_for_payout_method(
    state: &routes::AppState,
    key_store: &domain::MerchantKeyStore,
    payout_token: &str,
    pm: &storage::PaymentMethod,
) -> errors::RouterResult<api::BankPayout> {
    let payment_method = get_payment_method_from_hs_locker(
        state,
        key_store,
        &pm.customer_id,
        &pm.merchant_id,
        &pm.payment_method_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Error getting payment method from locker")?;
    let pm_parsed: api::PayoutMethodData = payment_method
        .peek()
        .to_string()
        .parse_struct("PayoutMethodData")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    match &pm_parsed {
        api::PayoutMethodData::Bank(bank) => {
            vault::Vault::store_payout_method_data_in_locker(
                state,
                Some(payout_token.to_string()),
                &pm_parsed,
                Some(pm.customer_id.to_owned()),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error storing payout method data in temporary locker")?;
            Ok(bank.to_owned())
        }
        api::PayoutMethodData::Card(_) => Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Expected bank details, found card details instead".to_string(),
        }
        .into()),
    }
}

pub struct BasiliskCardSupport;

#[cfg(not(feature = "basilisk"))]
impl BasiliskCardSupport {
    async fn create_payment_method_data_in_locker(
        state: &routes::AppState,
        payment_token: &str,
        card: api::CardDetailFromLocker,
        pm: &storage::PaymentMethod,
    ) -> errors::RouterResult<api::CardDetailFromLocker> {
        let card_number = card.card_number.clone().get_required_value("card_number")?;
        let card_exp_month = card
            .expiry_month
            .clone()
            .expose_option()
            .get_required_value("expiry_month")?;
        let card_exp_year = card
            .expiry_year
            .clone()
            .expose_option()
            .get_required_value("expiry_year")?;
        let card_holder_name = card
            .card_holder_name
            .clone()
            .expose_option()
            .unwrap_or_default();
        let value1 = payment_methods::mk_card_value1(
            card_number,
            card_exp_year,
            card_exp_month,
            Some(card_holder_name),
            None,
            None,
            None,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value1 for locker")?;
        let value2 = payment_methods::mk_card_value2(
            None,
            None,
            None,
            Some(pm.customer_id.to_string()),
            Some(pm.payment_method_id.to_string()),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value2 for locker")?;

        let value1 = vault::VaultPaymentMethod::Card(value1);
        let value2 = vault::VaultPaymentMethod::Card(value2);

        let value1 = utils::Encode::<vault::VaultPaymentMethod>::encode_to_string_of_json(&value1)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value1 construction failed when saving card to locker")?;

        let value2 = utils::Encode::<vault::VaultPaymentMethod>::encode_to_string_of_json(&value2)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value2 construction failed when saving card to locker")?;

        let db_value = vault::MockTokenizeDBValue { value1, value2 };

        let value_string =
            utils::Encode::<vault::MockTokenizeDBValue>::encode_to_string_of_json(&db_value)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Mock tokenize value construction failed when saving card to locker",
                )?;

        let db = &*state.store;

        let already_present = db.find_config_by_key(payment_token).await;

        if already_present.is_err() {
            let config = storage::ConfigNew {
                key: payment_token.to_string(),
                config: value_string,
            };

            db.insert_config(config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Mock tokenization save to db failed")?;
        } else {
            let config_update = storage::ConfigUpdate::Update {
                config: Some(value_string),
            };

            db.update_config_by_key(payment_token, config_update)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Mock tokenization db update failed")?;
        }

        Ok(card)
    }
}

#[cfg(feature = "basilisk")]
impl BasiliskCardSupport {
    #[instrument(skip_all)]
    async fn create_payment_method_data_in_locker(
        state: &routes::AppState,
        payment_token: &str,
        card: api::CardDetailFromLocker,
        pm: &storage::PaymentMethod,
    ) -> errors::RouterResult<api::CardDetailFromLocker> {
        let card_number = card.card_number.clone().get_required_value("card_number")?;
        let card_exp_month = card
            .expiry_month
            .clone()
            .expose_option()
            .get_required_value("expiry_month")?;
        let card_exp_year = card
            .expiry_year
            .clone()
            .expose_option()
            .get_required_value("expiry_year")?;
        let card_holder_name = card
            .card_holder_name
            .clone()
            .expose_option()
            .unwrap_or_default();
        let value1 = payment_methods::mk_card_value1(
            card_number,
            card_exp_year,
            card_exp_month,
            Some(card_holder_name),
            None,
            None,
            None,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value1 for locker")?;
        let value2 = payment_methods::mk_card_value2(
            None,
            None,
            None,
            Some(pm.customer_id.to_string()),
            Some(pm.payment_method_id.to_string()),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value2 for locker")?;

        let value1 = vault::VaultPaymentMethod::Card(value1);
        let value2 = vault::VaultPaymentMethod::Card(value2);

        let value1 = utils::Encode::<vault::VaultPaymentMethod>::encode_to_string_of_json(&value1)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value1 construction failed when saving card to locker")?;

        let value2 = utils::Encode::<vault::VaultPaymentMethod>::encode_to_string_of_json(&value2)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value2 construction failed when saving card to locker")?;

        let lookup_key =
            vault::create_tokenize(state, value1, Some(value2), payment_token.to_string()).await?;
        vault::add_delete_tokenized_data_task(
            &*state.store,
            &lookup_key,
            enums::PaymentMethod::Card,
        )
        .await?;
        metrics::TOKENIZED_DATA_COUNT.add(&metrics::CONTEXT, 1, &[]);
        Ok(card)
    }
}

#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    state: routes::AppState,
    pm: api::PaymentMethodId,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = state.store.as_ref();
    let pm = db
        .find_payment_method(&pm.payment_method_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    let card = if pm.payment_method == enums::PaymentMethod::Card {
        let card = get_card_from_locker(
            &state,
            &pm.customer_id,
            &pm.merchant_id,
            &pm.payment_method_id,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting card from card vault")?;
        let card_detail = payment_methods::get_card_detail(&pm, card)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while getting card details from locker")?;
        Some(card_detail)
    } else {
        None
    };
    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodResponse {
            merchant_id: pm.merchant_id,
            customer_id: Some(pm.customer_id),
            payment_method_id: pm.payment_method_id,
            payment_method: pm.payment_method,
            payment_method_type: pm.payment_method_type,
            card,
            metadata: pm.metadata,
            created: Some(pm.created_at),
            recurring_enabled: false,           //[#219]
            installment_payment_enabled: false, //[#219]
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), //[#219],
        },
    ))
}

#[instrument(skip_all)]
pub async fn delete_payment_method(
    state: routes::AppState,
    merchant_account: domain::MerchantAccount,
    pm_id: api::PaymentMethodId,
) -> errors::RouterResponse<api::PaymentMethodDeleteResponse> {
    let db = state.store.as_ref();
    let key = db
        .find_payment_method(pm_id.payment_method_id.as_str())
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    if key.payment_method == enums::PaymentMethod::Card {
        let response = delete_card_from_locker(
            &state,
            &key.customer_id,
            &key.merchant_id,
            pm_id.payment_method_id.as_str(),
        )
        .await?;

        if response.status == "SUCCESS" {
            logger::info!("Card From locker deleted Successfully!");
        } else {
            logger::error!("Error: Deleting Card From Locker!\n{:#?}", response);
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }

    db.delete_payment_method_by_merchant_id_payment_method_id(
        &merchant_account.merchant_id,
        pm_id.payment_method_id.as_str(),
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodDeleteResponse {
            payment_method_id: key.payment_method_id,
            deleted: true,
        },
    ))
}

pub async fn create_encrypted_payment_method_data(
    key_store: &domain::MerchantKeyStore,
    pm_data: Option<PaymentMethodsData>,
) -> Option<Encryption> {
    let key = key_store.key.get_inner().peek();

    let pm_data_encrypted: Option<Encryption> = pm_data
        .as_ref()
        .map(utils::Encode::<PaymentMethodsData>::encode_to_value)
        .transpose()
        .change_context(errors::StorageError::SerializationFailed)
        .attach_printable("Unable to convert payment method data to a value")
        .unwrap_or_else(|err| {
            logger::error!(err=?err);
            None
        })
        .map(masking::Secret::<_, masking::WithType>::new)
        .async_lift(|inner| encrypt_optional(inner, key))
        .await
        .change_context(errors::StorageError::EncryptionError)
        .attach_printable("Unable to encrypt payment method data")
        .unwrap_or_else(|err| {
            logger::error!(err=?err);
            None
        })
        .map(|details| details.into());

    pm_data_encrypted
}
