use std::collections;

use common_utils::{consts, generate_id};
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    configs::settings,
    core::{
        errors::{self, StorageErrorExt},
        payment_methods::transformers as payment_methods,
        payments::helpers,
    },
    db,
    pii::prelude::*,
    routes, services,
    types::{
        api::{self, CreatePaymentMethodExt},
        storage::{self, enums},
        transformers::ForeignInto,
    },
    utils::{self, BytesExt, OptionExt, StringExt, ValueExt},
};

#[instrument(skip_all)]
pub async fn create_payment_method(
    db: &dyn db::StorageInterface,
    req: &api::CreatePaymentMethod,
    customer_id: &str,
    payment_method_id: &str,
    merchant_id: &str,
) -> errors::CustomResult<storage::PaymentMethod, errors::StorageError> {
    let response = db
        .insert_payment_method(storage::PaymentMethodNew {
            customer_id: customer_id.to_string(),
            merchant_id: merchant_id.to_string(),
            payment_method_id: payment_method_id.to_string(),
            payment_method: req.payment_method.foreign_into(),
            payment_method_type: req.payment_method_type.map(ForeignInto::foreign_into),
            payment_method_issuer: req.payment_method_issuer.clone(),
            metadata: req.metadata.clone(),
            ..storage::PaymentMethodNew::default()
        })
        .await?;

    Ok(response)
}

#[instrument(skip_all)]
pub async fn add_payment_method(
    state: &routes::AppState,
    req: api::CreatePaymentMethod,
    merchant_id: String,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    req.validate()?;

    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;
    match req.card.clone() {
        Some(card) => add_card(state, req, card, customer_id, &merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add Card Failed"),
        None => {
            let payment_method_id = generate_id(consts::ID_LENGTH, "pm");
            create_payment_method(
                &*state.store,
                &req,
                &customer_id,
                &payment_method_id,
                &merchant_id,
            )
            .await
            .map_err(|error| {
                error.to_duplicate_response(errors::ApiErrorResponse::DuplicatePaymentMethod)
            })?;
            Ok(api::PaymentMethodResponse {
                merchant_id,
                customer_id: Some(customer_id),
                payment_method_id: payment_method_id.to_string(),
                payment_method: req.payment_method,
                payment_method_type: req.payment_method_type,
                payment_method_issuer: req.payment_method_issuer,
                card: None,
                metadata: req.metadata,
                created: Some(common_utils::date_time::now()),
                payment_method_issuer_code: req.payment_method_issuer_code,
                recurring_enabled: false,           //TODO
                installment_payment_enabled: false, //TODO
                payment_experience: Some(vec![
                    api_models::payment_methods::PaymentExperience::RedirectToUrl,
                ]), //TODO
            })
        }
    }
    .map(services::BachResponse::Json)
}

#[instrument(skip_all)]
pub async fn update_customer_payment_method(
    state: &routes::AppState,
    merchant_account: storage::MerchantAccount,
    req: api::UpdatePaymentMethod,
    payment_method_id: &str,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = &*state.store;
    let pm = db
        .delete_payment_method_by_merchant_id_payment_method_id(
            &merchant_account.merchant_id,
            payment_method_id,
        )
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;
    if pm.payment_method == enums::PaymentMethodType::Card {
        delete_card(state, &pm.merchant_id, &pm.payment_method_id).await?;
    };
    let new_pm = api::CreatePaymentMethod {
        payment_method: pm.payment_method.foreign_into(),
        payment_method_type: pm.payment_method_type.map(|x| x.foreign_into()),
        payment_method_issuer: pm.payment_method_issuer,
        payment_method_issuer_code: pm.payment_method_issuer_code.map(|x| x.foreign_into()),
        card: req.card,
        metadata: req.metadata,
        customer_id: Some(pm.customer_id),
    };
    add_payment_method(state, new_pm, merchant_account.merchant_id).await
}

#[instrument(skip_all)]
pub async fn add_card(
    state: &routes::AppState,
    req: api::CreatePaymentMethod,
    card: api::CardDetail,
    customer_id: String,
    merchant_id: &str,
) -> errors::CustomResult<api::PaymentMethodResponse, errors::CardVaultError> {
    let locker = &state.conf.locker;
    let db = &*state.store;
    let request = payment_methods::mk_add_card_request(locker, &card, &customer_id, &req)?;
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
        let card_id = generate_id(consts::ID_LENGTH, "card");
        mock_add_card(db, &card_id, &card, None, None).await?
    };

    if let Some(false) = response.duplicate {
        create_payment_method(db, &req, &customer_id, &response.card_id, merchant_id)
            .await
            .change_context(errors::CardVaultError::PaymentMethodCreationFailed)?;
    } else {
        match db.find_payment_method(&response.card_id).await {
            Ok(_) => (),
            Err(err) => {
                if err.current_context().is_db_not_found() {
                    create_payment_method(db, &req, &customer_id, &response.card_id, merchant_id)
                        .await
                        .change_context(errors::CardVaultError::PaymentMethodCreationFailed)?;
                } else {
                    Err(errors::CardVaultError::PaymentMethodCreationFailed)?;
                }
            }
        }
    }
    let payment_method_resp =
        payment_methods::mk_add_card_response(card, response, req, merchant_id);
    Ok(payment_method_resp)
}

#[instrument(skip_all)]
pub async fn mock_add_card(
    db: &dyn db::StorageInterface,
    card_id: &str,
    card: &api::CardDetail,
    card_cvc: Option<String>,
    payment_method_id: Option<String>,
) -> errors::CustomResult<payment_methods::AddCardResponse, errors::CardVaultError> {
    let locker_mock_up = storage::LockerMockUpNew {
        card_id: card_id.to_string(),
        external_id: uuid::Uuid::new_v4().to_string(),
        card_fingerprint: uuid::Uuid::new_v4().to_string(),
        card_global_fingerprint: uuid::Uuid::new_v4().to_string(),
        merchant_id: "mm01".to_string(),
        card_number: card.card_number.peek().to_string(),
        card_exp_year: card.card_exp_year.peek().to_string(),
        card_exp_month: card.card_exp_month.peek().to_string(),
        card_cvc,
        payment_method_id,
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
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<(payment_methods::GetCardResponse, Option<String>), errors::CardVaultError>
{
    let locker_mock_up = db
        .find_locker_by_card_id(card_id)
        .await
        .change_context(errors::CardVaultError::FetchCardFailed)?;
    let add_card_response = payment_methods::AddCardResponse {
        card_id: locker_mock_up
            .payment_method_id
            .unwrap_or(locker_mock_up.card_id),
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
    Ok((
        payment_methods::GetCardResponse {
            card: add_card_response,
        },
        locker_mock_up.card_cvc,
    ))
}

#[instrument(skip_all)]
pub async fn mock_delete_card<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::DeleteCardResponse, errors::CardVaultError> {
    let locker_mock_up = db
        .delete_locker_mock_up(card_id)
        .await
        .change_context(errors::CardVaultError::FetchCardFailed)?;
    Ok(payment_methods::DeleteCardResponse {
        card_id: locker_mock_up.card_id,
        external_id: locker_mock_up.external_id,
        card_isin: None,
        status: "SUCCESS".to_string(),
    })
}

#[instrument(skip_all)]
pub async fn get_card_from_legacy_locker<'a>(
    state: &'a routes::AppState,
    merchant_id: &'a str,
    card_id: &'a str,
) -> errors::RouterResult<payment_methods::GetCardResponse> {
    let locker = &state.conf.locker;
    let request = payment_methods::mk_get_card_request(locker, merchant_id, card_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Making get card request failed")?;
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
        let (get_card_response, _) = mock_get_card(&*state.store, card_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        get_card_response
    };

    Ok(get_card_result)
}

#[instrument(skip_all)]
pub async fn delete_card<'a>(
    state: &'a routes::AppState,
    merchant_id: &'a str,
    card_id: &'a str,
) -> errors::RouterResult<payment_methods::DeleteCardResponse> {
    let locker = &state.conf.locker;
    let request = payment_methods::mk_delete_card_request(&state.conf.locker, merchant_id, card_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Making Delete card request Failed")?;
    let delete_card_resp = if !locker.mock_locker {
        services::call_connector_api(state, request)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?
            .map_err(|_x| errors::ApiErrorResponse::InternalServerError)?
            .response
            .parse_struct("DeleteCardResponse")
            .change_context(errors::ApiErrorResponse::InternalServerError)?
    } else {
        mock_delete_card(&*state.store, card_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?
    };

    Ok(delete_card_resp)
}

pub async fn list_payment_methods(
    db: &dyn db::StorageInterface,
    merchant_account: storage::MerchantAccount,
    mut req: api::ListPaymentMethodRequest,
) -> errors::RouterResponse<Vec<api::ListPaymentMethodResponse>> {
    helpers::verify_client_secret(
        db,
        merchant_account.storage_scheme,
        req.client_secret.clone(),
        &merchant_account.merchant_id,
    )
    .await?;

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
    payment_methods: Vec<serde_json::Value>,
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

fn filter_accepted_enum_based<T: Eq + std::hash::Hash + Clone>(
    left: &Option<Vec<T>>,
    right: &Option<Vec<T>>,
) -> (Option<Vec<T>>, Option<Vec<T>>, bool) {
    match (left, right) {
        (Some(ref l), Some(ref r)) => {
            let a: collections::HashSet<&T> = collections::HashSet::from_iter(l.iter());
            let b: collections::HashSet<&T> = collections::HashSet::from_iter(r.iter());

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
    state: &routes::AppState,
    merchant_account: storage::MerchantAccount,
    customer_id: &str,
) -> errors::RouterResponse<api::ListCustomerPaymentMethodsResponse> {
    let db = &*state.store;
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
        let payment_token = generate_id(consts::ID_LENGTH, "token");
        let card = if pm.payment_method == enums::PaymentMethodType::Card {
            Some(get_lookup_key_from_locker(state, &payment_token, &pm).await?)
        } else {
            None
        };
        //Need validation for enabled payment method ,querying MCA
        let pma = api::CustomerPaymentMethod {
            payment_token: payment_token.to_string(),
            customer_id: pm.customer_id,
            payment_method: pm.payment_method.foreign_into(),
            payment_method_type: pm.payment_method_type.map(ForeignInto::foreign_into),
            payment_method_issuer: pm.payment_method_issuer,
            card,
            metadata: pm.metadata,
            payment_method_issuer_code: pm
                .payment_method_issuer_code
                .map(ForeignInto::foreign_into),
            recurring_enabled: false,
            installment_payment_enabled: false,
            payment_experience: Some(vec![
                api_models::payment_methods::PaymentExperience::RedirectToUrl,
            ]),
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

pub async fn get_lookup_key_from_locker(
    state: &routes::AppState,
    payment_token: &str,
    pm: &storage::PaymentMethod,
) -> errors::RouterResult<api::CardDetailFromLocker> {
    let get_card_resp = get_card_from_legacy_locker(
        state,
        pm.merchant_id.as_str(),
        pm.payment_method_id.as_str(),
    )
    .await?;
    let card_detail = payment_methods::get_card_detail(pm, get_card_resp.card)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Get Card Details Failed")?;
    let card = card_detail.clone();
    let resp =
        BasiliskCardSupport::create_payment_method_data_in_locker(state, payment_token, card, pm)
            .await?;
    Ok(resp)
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
        let card_number = card
            .card_number
            .clone()
            .expose_option()
            .get_required_value("card_number")?;
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
        let card_detail = api::CardDetail {
            card_number: card_number.into(),
            card_exp_month: card_exp_month.into(),
            card_exp_year: card_exp_year.into(),
            card_holder_name: Some(card_holder_name.into()),
        };
        let db = &*state.store;
        mock_add_card(
            db,
            payment_token,
            &card_detail,
            None,
            Some(pm.payment_method_id.to_string()),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Add Card Failed")?;
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
        let card_number = card
            .card_number
            .clone()
            .expose_option()
            .get_required_value("card_number")?;
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
        let card_fingerprint = card
            .card_fingerprint
            .clone()
            .expose_option()
            .get_required_value("card_fingerprint")?;
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
            Some(card_fingerprint),
            None,
            Some(pm.customer_id.to_string()),
            Some(pm.payment_method_id.to_string()),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value2 for locker")?;
        create_tokenize(state, value1, Some(value2), payment_token.to_string()).await?;
        Ok(card)
    }
}

pub async fn get_card_info_value(
    keys: &settings::Keys,
    card_info: String,
) -> errors::RouterResult<serde_json::Value> {
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
    keys: &settings::Keys,
    card_info: serde_json::Value,
) -> errors::RouterResult<String> {
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
    state: &routes::AppState,
    pm: api::PaymentMethodId,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = &*state.store;
    let pm = db
        .find_payment_method(&pm.payment_method_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;
    let card = if pm.payment_method == enums::PaymentMethodType::Card {
        let get_card_resp =
            get_card_from_legacy_locker(state, &pm.merchant_id, &pm.payment_method_id).await?;
        let card_detail = payment_methods::get_card_detail(&pm, get_card_resp.card)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Some(card_detail)
    } else {
        None
    };
    Ok(services::BachResponse::Json(api::PaymentMethodResponse {
        merchant_id: pm.merchant_id,
        customer_id: Some(pm.customer_id),
        payment_method_id: pm.payment_method_id,
        payment_method: pm.payment_method.foreign_into(),
        payment_method_type: pm.payment_method_type.map(ForeignInto::foreign_into),
        payment_method_issuer: pm.payment_method_issuer,
        card,
        metadata: pm.metadata,
        created: Some(pm.created_at),
        payment_method_issuer_code: pm.payment_method_issuer_code.map(ForeignInto::foreign_into),
        recurring_enabled: false,           //TODO
        installment_payment_enabled: false, //TODO
        payment_experience: Some(vec![
            api_models::payment_methods::PaymentExperience::RedirectToUrl,
        ]), //TODO,
    }))
}

#[instrument(skip_all)]
pub async fn delete_payment_method(
    state: &routes::AppState,
    merchant_account: storage::MerchantAccount,
    pm: api::PaymentMethodId,
) -> errors::RouterResponse<api::DeletePaymentMethodResponse> {
    let (_, value2) =
        helpers::Vault::get_payment_method_data_from_locker(state, &pm.payment_method_id).await?;
    let payment_method_id = value2.map_or(
        Err(errors::ApiErrorResponse::PaymentMethodNotFound),
        |pm_value2| {
            pm_value2
                .payment_method_id
                .map_or(Err(errors::ApiErrorResponse::PaymentMethodNotFound), |x| {
                    Ok(x)
                })
        },
    )?;
    let pm = state
        .store
        .delete_payment_method_by_merchant_id_payment_method_id(
            &merchant_account.merchant_id,
            &payment_method_id,
        )
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;

    if pm.payment_method == enums::PaymentMethodType::Card {
        let response = delete_card(state, &pm.merchant_id, &payment_method_id).await?;
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

//------------------------------------------------TokenizeService------------------------------------------------
pub async fn create_tokenize(
    state: &routes::AppState,
    value1: String,
    value2: Option<String>,
    lookup_key: String,
) -> errors::RouterResult<String> {
    let payload_to_be_encrypted = api::TokenizePayloadRequest {
        value1,
        value2: value2.unwrap_or_default(),
        lookup_key,
        service_name: "CARD".to_string(),
    };
    let payload = serde_json::to_string(&payload_to_be_encrypted)
        .map_err(|_x| errors::ApiErrorResponse::InternalServerError)?;
    let encrypted_payload = services::encrypt_jwe(&state.conf.jwekey, &payload)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encrypt JWE response")?;

    let create_tokenize_request = api::TokenizePayloadEncrypted {
        payload: encrypted_payload,
        key_id: services::get_key_id(&state.conf.jwekey).to_string(),
        version: Some("0".to_string()),
    };
    let request = payment_methods::mk_crud_locker_request(
        &state.conf.locker,
        "/tokenize",
        create_tokenize_request,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Making tokenize request failed")?;
    let response = services::call_connector_api(state, request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    match response {
        Ok(r) => {
            let resp: api::TokenizePayloadEncrypted = r
                .response
                .parse_struct("TokenizePayloadEncrypted")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Decoding Failed for TokenizePayloadEncrypted")?;
            let decrypted_payload =
                services::decrypt_jwe(&state.conf.jwekey, &resp.payload, &resp.key_id)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Decrypt Jwe failed for TokenizePayloadEncrypted")?;
            let get_response: api::GetTokenizePayloadResponse = decrypted_payload
                .parse_struct("GetTokenizePayloadResponse")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Error getting GetTokenizePayloadResponse from tokenize response",
                )?;
            Ok(get_response.lookup_key)
        }
        Err(err) => Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!("Got 4xx from the basilisk locker: {err:?}"))),
    }
}

pub async fn get_tokenized_data(
    state: &routes::AppState,
    lookup_key: &str,
    should_get_value2: bool,
) -> errors::RouterResult<api::TokenizePayloadRequest> {
    let payload_to_be_encrypted = api::GetTokenizePayloadRequest {
        lookup_key: lookup_key.to_string(),
        get_value2: should_get_value2,
    };
    let payload = serde_json::to_string(&payload_to_be_encrypted)
        .map_err(|_x| errors::ApiErrorResponse::InternalServerError)?;
    let encrypted_payload = services::encrypt_jwe(&state.conf.jwekey, &payload)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encrypt JWE response")?;
    let create_tokenize_request = api::TokenizePayloadEncrypted {
        payload: encrypted_payload,
        key_id: services::get_key_id(&state.conf.jwekey).to_string(),
        version: Some("0".to_string()),
    };
    let request = payment_methods::mk_crud_locker_request(
        &state.conf.locker,
        "/tokenize/get",
        create_tokenize_request,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Making Get Tokenized request failed")?;
    let response = services::call_connector_api(state, request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    match response {
        Ok(r) => {
            let resp: api::TokenizePayloadEncrypted = r
                .response
                .parse_struct("TokenizePayloadEncrypted")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Decoding Failed for TokenizePayloadEncrypted")?;
            let decrypted_payload =
                services::decrypt_jwe(&state.conf.jwekey, &resp.payload, &resp.key_id)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "GetTokenizedApi: Decrypt Jwe failed for TokenizePayloadEncrypted",
                    )?;
            let get_response: api::TokenizePayloadRequest = decrypted_payload
                .parse_struct("TokenizePayloadRequest")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error getting TokenizePayloadRequest from tokenize response")?;
            Ok(get_response)
        }
        Err(err) => Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!("Got 4xx from the basilisk locker: {err:?}"))),
    }
}

pub async fn delete_tokenized_data(
    state: &routes::AppState,
    lookup_key: &str,
) -> errors::RouterResult<String> {
    let payload_to_be_encrypted = api::DeleteTokenizeByTokenRequest {
        lookup_key: lookup_key.to_string(),
    };
    let payload = serde_json::to_string(&payload_to_be_encrypted)
        .map_err(|_x| errors::ApiErrorResponse::InternalServerError)?;
    let encrypted_payload = services::encrypt_jwe(&state.conf.jwekey, &payload)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encrypt JWE response")?;
    let create_tokenize_request = api::TokenizePayloadEncrypted {
        payload: encrypted_payload,
        key_id: services::get_key_id(&state.conf.jwekey).to_string(),
        version: Some("0".to_string()),
    };
    let request = payment_methods::mk_crud_locker_request(
        &state.conf.locker,
        "/tokenize/delete/token",
        create_tokenize_request,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Making Delete Tokenized request failed")?;
    let response = services::call_connector_api(state, request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    match response {
        Ok(r) => {
            let resp: api::TokenizePayloadEncrypted = r
                .response
                .parse_struct("TokenizePayloadEncrypted")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Decoding Failed for TokenizePayloadEncrypted")?;
            let decrypted_payload =
                services::decrypt_jwe(&state.conf.jwekey, &resp.payload, &resp.key_id)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "DeleteTokenizedApi: Decrypt Jwe failed for TokenizePayloadEncrypted",
                    )?;
            let delete_response = decrypted_payload
                .parse_struct("Delete")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Error getting TokenizePayloadEncrypted from tokenize response",
                )?;
            Ok(delete_response)
        }
        Err(err) => Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!("Got 4xx from the basilisk locker: {err:?}"))),
    }
}
