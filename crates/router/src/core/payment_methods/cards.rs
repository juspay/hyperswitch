use std::{collections::HashSet, hash::Hash};

use error_stack::{report, ResultExt};
use rand::Rng;
use router_env::{tracing, tracing::instrument};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    configs::settings::Keys,
    core::{
        errors::{self, CustomResult, RouterResponse, RouterResult, StorageErrorExt},
        payment_methods::transformers as payment_methods,
    },
    db::{
        merchant_connector_account::IMerchantConnectorAccount, payment_method::IPaymentMethod,
        temp_card::ITempCard, Db,
    },
    pii::prelude::*,
    routes::AppState,
    services,
    types::{
        api,
        storage::{self, enums},
    },
    utils::{self, BytesExt, OptionExt, ValueExt},
};

#[instrument(skip_all)]
pub async fn create_payment_method(
    db: &dyn Db,
    req: &api::CreatePaymentMethod,
    customer_id: String,
    payment_method_id: String,
    merchant_id: &str,
) -> CustomResult<storage::PaymentMethod, errors::StorageError> {
    let response = db
        .insert_payment_method(storage::PaymentMethodNew {
            customer_id,
            merchant_id: merchant_id.to_string(),
            payment_method_id,
            payment_method: req.payment_method,
            payment_method_type: req.payment_method_type,
            payment_method_issuer: req.payment_method_issuer.clone(),
            ..storage::PaymentMethodNew::default()
        })
        .await?;

    Ok(response)
}

#[instrument(skip_all)]
pub async fn add_payment_method(
    state: &AppState,
    req: api::CreatePaymentMethod,
    merchant_id: String,
) -> RouterResponse<api::PaymentMethodResponse> {
    req.validate()?;

    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;
    match req.card.clone() {
        Some(card) => add_card(state, req, card, customer_id, &merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add Card Failed"),
        None => {
            create_payment_method(
                &state.store,
                &req,
                customer_id,
                "payment_method_id".to_owned(),
                &merchant_id,
            ) //TODO where will we get this for other payment_method
            .await
            .map_err(|error| {
                error.to_duplicate_response(errors::ApiErrorResponse::DuplicatePaymentMethod)
            })?;
            Ok(api::PaymentMethodResponse {
                payment_method_id: String::from("payment_method_id"),
                payment_method: req.payment_method,
                payment_method_type: req.payment_method_type,
                payment_method_issuer: req.payment_method_issuer,
                card: None,
                metadata: req.metadata,
                created: Some(crate::utils::date_time::now()),
                payment_method_issuer_code: req.payment_method_issuer_code,
                recurring_enabled: false,           //TODO
                installment_payment_enabled: false, //TODO
                payment_experience: Some(vec!["redirect_to_url".to_string()]), //TODO
            })
        }
    }
    .map(services::BachResponse::Json)
}

#[instrument(skip_all)]
pub async fn add_card(
    state: &AppState,
    req: api::CreatePaymentMethod,
    card: api::CardDetail,
    customer_id: String,
    merchant_id: &str,
) -> CustomResult<api::PaymentMethodResponse, errors::CardVaultError> {
    let locker = &state.conf.locker;
    let db = &state.store;
    let request = payment_methods::mk_add_card_request(locker, &card, &customer_id, &req)?;
    // FIXME use call_api 2. Serde's handle should be inside the generic function
    let response = if !locker.mock_locker {
        let response = services::call_connector_api(state, request)
            .await
            .change_context(errors::CardVaultError::SaveCardFailed)?;

        let response: payment_methods::AddCardResponse = match response {
            Ok(card) => card
                .response
                .parse_struct("AddCardResponse")
                .change_context(errors::CardVaultError::ResponseDeserializationFailed),
            Err(err) => Err(report!(errors::CardVaultError::UnexpectedResponseError(
                err.response
            ))),
        }?;
        response
    } else {
        mock_add_card(db, &card).await?
    };

    create_payment_method(
        db,
        &req,
        customer_id.to_string(),
        response.card_id.to_owned(),
        merchant_id,
    )
    .await
    .change_context(errors::CardVaultError::PaymentMethodCreationFailed)?;
    let payment_method_resp = payment_methods::mk_add_card_response(card, response, req);
    Ok(payment_method_resp)
}

#[instrument(skip_all)]
pub async fn mock_add_card(
    db: &dyn Db,
    card: &api::CardDetail,
) -> CustomResult<payment_methods::AddCardResponse, errors::CardVaultError> {
    let locker_mock_up = storage::LockerMockUpNew {
        card_id: Uuid::new_v4().to_string(),
        external_id: Uuid::new_v4().to_string(),
        card_fingerprint: Uuid::new_v4().to_string(),
        card_global_fingerprint: Uuid::new_v4().to_string(),
        merchant_id: "mm01".to_string(),
        card_number: card.card_number.peek().to_string(),
        card_exp_year: card.card_exp_year.peek().to_string(),
        card_exp_month: card.card_exp_month.peek().to_string(),
    };

    let response = db
        .insert_locker_mock_up(locker_mock_up)
        .await
        .change_context(errors::CardVaultError::SaveCardFailed)?;
    Ok(payment_methods::AddCardResponse {
        card_id: response.card_id,
        external_id: response.external_id,
        card_fingerprint: response.card_fingerprint.into(),
        card_global_fingerprint: response.card_global_fingerprint.into(),
        merchant_id: Some(response.merchant_id),
        card_number: Some(response.card_number.into()),
        card_exp_year: Some(response.card_exp_year.into()),
        card_exp_month: Some(response.card_exp_month.into()),
        name_on_card: None,
        nickname: response.nickname,
        customer_id: response.customer_id,
        duplicate: response.duplicate,
    })
}

#[instrument(skip_all)]
pub async fn mock_get_card<'a>(
    db: &dyn Db,
    card_id: &'a str,
) -> CustomResult<payment_methods::GetCardResponse, errors::CardVaultError> {
    let locker_mock_up = db
        .find_locker_by_card_id(card_id)
        .await
        .change_context(errors::CardVaultError::FetchCardFailed)?;
    let add_card_response = payment_methods::AddCardResponse {
        card_id: locker_mock_up.card_id,
        external_id: locker_mock_up.external_id,
        card_fingerprint: locker_mock_up.card_fingerprint.into(),
        card_global_fingerprint: locker_mock_up.card_global_fingerprint.into(),
        merchant_id: Some(locker_mock_up.merchant_id),
        card_number: Some(locker_mock_up.card_number.into()),
        card_exp_year: Some(locker_mock_up.card_exp_year.into()),
        card_exp_month: Some(locker_mock_up.card_exp_month.into()),
        name_on_card: None,
        nickname: locker_mock_up.nickname,
        customer_id: locker_mock_up.customer_id,
        duplicate: locker_mock_up.duplicate,
    };
    Ok(payment_methods::GetCardResponse {
        card: add_card_response,
    })
}

#[instrument(skip_all)]
pub async fn get_card<'a>(
    state: &'a AppState,
    merchant_id: &'a str,
    card_id: &'a str,
) -> RouterResult<payment_methods::GetCardResponse> {
    let locker = &state.conf.locker;
    let request = payment_methods::mk_get_card_request(locker, merchant_id, card_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Making get card request failed")?;
    // FIXME use call_api 2. Serde's handle should be inside the generic function
    let get_card_result = if !locker.mock_locker {
        let response = services::call_connector_api(state, request)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        match response {
            Ok(card) => card
                .response
                .parse_struct("AddCardResponse")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Decoding failed for AddCardResponse"),
            Err(err) => Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(format!("Got 4xx from the locker: {err:?}"))),
        }?
    } else {
        mock_get_card(&state.store, card_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?
    };

    Ok(get_card_result)
}

#[instrument(skip_all)]
pub async fn delete_card<'a>(
    state: &'a AppState,
    merchant_id: &'a str,
    card_id: &'a str,
) -> RouterResult<payment_methods::DeleteCardResponse> {
    let request = payment_methods::mk_delete_card_request(&state.conf.locker, merchant_id, card_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Making Delete card request Failed")?;
    // FIXME use call_api 2. Serde's handle should be inside the generic function
    let delete_card_resp = services::call_connector_api(state, request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .map_err(|_x| errors::ApiErrorResponse::InternalServerError)?
        .response
        .parse_struct("DeleteCardResponse")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(delete_card_resp)
}

pub async fn list_payment_methods(
    db: &dyn Db,
    merchant_account: storage::MerchantAccount,
    mut req: api::ListPaymentMethodRequest,
) -> RouterResponse<Vec<api::ListPaymentMethodResponse>> {
    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_list(&merchant_account.merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    // TODO: Deduplicate payment methods
    let mut response: Vec<api::ListPaymentMethodResponse> = Vec::new();
    for mca in all_mcas {
        let payment_methods = match mca.payment_methods_enabled {
            Some(pm) => pm,
            None => continue,
        };

        filter_payment_methods(payment_methods, &mut req, &mut response);
    }
    response
        .is_empty()
        .then(|| Err(report!(errors::ApiErrorResponse::PaymentMethodNotFound)))
        .unwrap_or(Ok(services::BachResponse::Json(response)))
}

fn filter_payment_methods(
    payment_methods: Vec<Value>,
    req: &mut api::ListPaymentMethodRequest,
    resp: &mut Vec<api::ListPaymentMethodResponse>,
) {
    for payment_method in payment_methods.into_iter() {
        if let Ok(payment_method_object) =
            serde_json::from_value::<api::ListPaymentMethodResponse>(payment_method)
        {
            if filter_recurring_based(&payment_method_object, req.recurring_enabled)
                && filter_installment_based(&payment_method_object, req.installment_payment_enabled)
                && filter_amount_based(&payment_method_object, req.amount)
            {
                let mut payment_method_object = payment_method_object;

                let filter;
                (
                    payment_method_object.accepted_countries,
                    req.accepted_countries,
                    filter,
                ) = filter_accepted_enum_based(
                    &payment_method_object.accepted_countries,
                    &req.accepted_countries,
                );

                let filter2;
                (
                    payment_method_object.accepted_currencies,
                    req.accepted_currencies,
                    filter2,
                ) = filter_accepted_enum_based(
                    &payment_method_object.accepted_currencies,
                    &req.accepted_currencies,
                );

                if filter && filter2 {
                    resp.push(payment_method_object);
                }
            }
        }
    }
}

fn filter_accepted_enum_based<T: Eq + Hash + Clone>(
    left: &Option<Vec<T>>,
    right: &Option<Vec<T>>,
) -> (Option<Vec<T>>, Option<Vec<T>>, bool) {
    match (left, right) {
        (Some(ref l), Some(ref r)) => {
            let a: HashSet<&T> = HashSet::from_iter(l.iter());
            let b: HashSet<&T> = HashSet::from_iter(r.iter());

            let y: Vec<T> = a.intersection(&b).map(|&i| i.to_owned()).collect();
            (Some(y), Some(r.to_vec()), true)
        }
        (Some(ref l), None) => (Some(l.to_vec()), None, true),
        (None, Some(ref r)) => (None, Some(r.to_vec()), false),
        (None, None) => (None, None, true),
    }
}

fn filter_amount_based(
    payment_method: &api::ListPaymentMethodResponse,
    amount: Option<i32>,
) -> bool {
    let min_check = amount
        .and_then(|amt| payment_method.minimum_amount.map(|min_amt| amt >= min_amt))
        .unwrap_or(true);
    let max_check = amount
        .and_then(|amt| payment_method.maximum_amount.map(|max_amt| amt <= max_amt))
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

fn filter_recurring_based(
    payment_method: &api::ListPaymentMethodResponse,
    recurring_enabled: Option<bool>,
) -> bool {
    recurring_enabled.map_or(true, |enabled| payment_method.recurring_enabled == enabled)
}

fn filter_installment_based(
    payment_method: &api::ListPaymentMethodResponse,
    installment_payment_enabled: Option<bool>,
) -> bool {
    installment_payment_enabled.map_or(true, |enabled| {
        payment_method.installment_payment_enabled == enabled
    })
}

pub async fn list_customer_payment_method(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    customer_id: &str,
) -> RouterResponse<api::ListCustomerPaymentMethodsResponse> {
    let db = &state.store;
    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_list(&merchant_account.merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    // TODO: Deduplicate payment methods
    let mut enabled_methods: Vec<api::ListPaymentMethodResponse> = Vec::new();
    for mca in all_mcas {
        let payment_methods = match mca.payment_methods_enabled {
            Some(pm) => pm,
            None => continue,
        };

        for payment_method in payment_methods.into_iter() {
            if let Ok(payment_method_object) =
                serde_json::from_value::<api::ListPaymentMethodResponse>(payment_method)
            {
                enabled_methods.push(payment_method_object);
            }
        }
    }

    let resp = db
        .find_payment_method_by_customer_id_merchant_id_list(
            customer_id,
            &merchant_account.merchant_id,
        )
        .await
        .map_err(|err| {
            err.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;
    //let mca = query::find_mca_by_merchant_id(conn, &merchant_account.merchant_id)?;
    if resp.is_empty() {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::PaymentMethodNotFound
        ));
    }
    let mut vec = Vec::new();
    for pm in resp.into_iter() {
        let mut rng = rand::thread_rng();
        let payment_token = rng.gen::<i32>();
        let card = if pm.payment_method == enums::PaymentMethodType::Card {
            Some(get_tempcard_from_payment_method(state, payment_token, &pm).await?)
        } else {
            None
        };
        //Need validation for enabled payment method ,querying MCA
        let pma = api::CustomerPaymentMethod {
            payment_token: payment_token.to_string(),
            customer_id: pm.customer_id,
            payment_method: pm.payment_method,
            payment_method_type: pm.payment_method_type,
            payment_method_issuer: pm.payment_method_issuer,
            card,
            metadata: None,
            payment_method_issuer_code: pm.payment_method_issuer_code,
            recurring_enabled: false,
            installment_payment_enabled: false,
            payment_experience: Some(vec!["redirect_to_url".to_string()]), //TODO chnage to enum
            created: Some(pm.created_at),
        };
        vec.push(pma);
    }

    let response = api::ListCustomerPaymentMethodsResponse {
        enabled_payment_methods: enabled_methods,
        customer_payment_methods: vec,
    };

    Ok(services::BachResponse::Json(response))
}

pub async fn get_tempcard_from_payment_method(
    state: &AppState,
    payment_token: i32,
    pm: &storage::PaymentMethod,
) -> RouterResult<api::CardDetailFromLocker> {
    let get_card_resp = get_card(
        state,
        pm.merchant_id.as_str(),
        pm.payment_method_id.as_str(),
    )
    .await?;
    let card_detail = payment_methods::get_card_detail(pm, get_card_resp.card)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Get Card Details Failed")?;
    let card = card_detail.clone();
    let mut card_info = card_detail
        .card_number
        .peek_cloning()
        .get_required_value("card_number")?;
    card_info.push_str(":::");
    card_info.push_str(
        &card_detail
            .expiry_month
            .peek_cloning()
            .get_required_value("expiry_month")?,
    );
    card_info.push_str(":::");
    card_info.push_str(
        &card_detail
            .expiry_year
            .peek_cloning()
            .get_required_value("expiry_year")?,
    );
    card_info.push_str(":::");
    card_info.push_str(
        &card_detail
            .card_holder_name
            .peek_cloning()
            .unwrap_or_default(),
    );
    let card_info_val = get_card_info_value(&state.conf.keys, card_info).await?;
    let temp_card = storage::TempCard {
        card_info: Some(card_info_val),
        date_created: crate::utils::date_time::now(),
        txn_id: None,
        id: payment_token,
    };

    state
        .store
        .insert_tempcard_with_token(temp_card)
        .await
        .map_err(|error| {
            error.to_duplicate_response(errors::ApiErrorResponse::DuplicatePaymentMethod)
        })?;
    Ok(card)
}

pub async fn get_card_info_value(
    keys: &Keys,
    card_info: String,
) -> RouterResult<serde_json::Value> {
    let key = services::KeyHandler::get_encryption_key(keys)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let enc_card_info = services::encrypt(&card_info, key.as_bytes())
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    utils::Encode::<Vec<u8>>::encode_to_value(&enc_card_info)
        .change_context(errors::CardVaultError::RequestEncodingFailed)
        .change_context(errors::ApiErrorResponse::InternalServerError)
}

pub async fn get_card_info_from_value(
    keys: &Keys,
    card_info: serde_json::Value,
) -> RouterResult<String> {
    let key = services::KeyHandler::get_encryption_key(keys)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let card_info_val: Vec<u8> = card_info
        .parse_value("CardInfo")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    services::decrypt(card_info_val, key.as_bytes())
        .change_context(errors::ApiErrorResponse::InternalServerError)
}

#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    state: &AppState,
    pm: api::PaymentMethodId,
) -> RouterResponse<api::PaymentMethodResponse> {
    let db = &state.store;
    let pm = db
        .find_payment_method(&pm.payment_method_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;
    let card = if pm.payment_method == enums::PaymentMethodType::Card {
        let get_card_resp = get_card(state, &pm.merchant_id, &pm.payment_method_id).await?;
        let card_detail = payment_methods::get_card_detail(&pm, get_card_resp.card)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Some(card_detail)
    } else {
        None
    };
    Ok(services::BachResponse::Json(api::PaymentMethodResponse {
        payment_method_id: pm.payment_method_id,
        payment_method: pm.payment_method,
        payment_method_type: pm.payment_method_type,
        payment_method_issuer: pm.payment_method_issuer,
        card,
        metadata: None, // TODO add in addCard api
        created: Some(pm.created_at),
        payment_method_issuer_code: pm.payment_method_issuer_code,
        recurring_enabled: false,                                      //TODO
        installment_payment_enabled: false,                            //TODO
        payment_experience: Some(vec!["redirect_to_url".to_string()]), //TODO,
    }))
}

#[instrument(skip_all)]
pub async fn delete_payment_method(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    pm: api::PaymentMethodId,
) -> RouterResponse<api::DeletePaymentMethodResponse> {
    let pm = state
        .store
        .delete_payment_method_by_merchant_id_payment_method_id(
            &merchant_account.merchant_id,
            &pm.payment_method_id,
        )
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;

    if pm.payment_method == enums::PaymentMethodType::Card {
        let response = delete_card(state, &pm.merchant_id, &pm.payment_method_id).await?;
        if response.status == "success" {
            print!("Card From locker deleted Successfully")
        } else {
            print!("Error: Deleting Card From Locker")
        }
    };

    Ok(services::BachResponse::Json(
        api::DeletePaymentMethodResponse {
            payment_method_id: pm.payment_method_id,
            deleted: true,
        },
    ))
}
