use std::collections::HashSet;

use common_utils::{consts, ext_traits::AsyncExt, generate_id};
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, StorageErrorExt},
        payment_methods::{transformers as payment_methods, vault},
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
    utils::{self, BytesExt, ConnectorResponseExt, OptionExt},
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
    merchant_account: &storage::MerchantAccount,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    req.validate()?;
    let merchant_id = &merchant_account.merchant_id;
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;
    match req.card.clone() {
        Some(card) => add_card(state, req, card, customer_id, merchant_account)
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
                merchant_id,
            )
            .await
            .map_err(|error| {
                error.to_duplicate_response(errors::ApiErrorResponse::DuplicatePaymentMethod)
            })?;
            Ok(api::PaymentMethodResponse {
                merchant_id: merchant_id.to_string(),
                customer_id: Some(customer_id),
                payment_method_id: payment_method_id.to_string(),
                payment_method: req.payment_method,
                payment_method_type: req.payment_method_type,
                payment_method_issuer: req.payment_method_issuer,
                card: None,
                metadata: req.metadata,
                created: Some(common_utils::date_time::now()),
                payment_method_issuer_code: req.payment_method_issuer_code,
                recurring_enabled: false,           //[#219]
                installment_payment_enabled: false, //[#219]
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), //[#219]
            })
        }
    }
    .map(services::ApplicationResponse::Json)
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
    add_payment_method(state, new_pm, &merchant_account).await
}

#[instrument(skip_all)]
pub async fn add_card(
    state: &routes::AppState,
    req: api::CreatePaymentMethod,
    card: api::CardDetail,
    customer_id: String,
    merchant_account: &storage::MerchantAccount,
) -> errors::CustomResult<api::PaymentMethodResponse, errors::VaultError> {
    let locker = &state.conf.locker;
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let locker_id = merchant_account
        .locker_id
        .to_owned()
        .get_required_value("locker_id")
        .change_context(errors::VaultError::SaveCardFailed)?;

    let request = payment_methods::mk_add_card_request(
        locker,
        &card,
        &customer_id,
        &req,
        &locker_id,
        merchant_id,
    )?;
    let response = if !locker.mock_locker {
        let response = services::call_connector_api(state, request)
            .await
            .change_context(errors::VaultError::SaveCardFailed)?;

        let response: payment_methods::AddCardResponse = match response {
            Ok(card) => card
                .response
                .parse_struct("AddCardResponse")
                .change_context(errors::VaultError::ResponseDeserializationFailed),
            Err(err) => Err(report!(errors::VaultError::UnexpectedResponseError(
                err.response
            ))),
        }?;
        response
    } else {
        let card_id = generate_id(consts::ID_LENGTH, "card");
        mock_add_card(db, &card_id, &card, None, None, Some(&customer_id)).await?
    };

    if let Some(false) = response.duplicate {
        create_payment_method(db, &req, &customer_id, &response.card_id, merchant_id)
            .await
            .change_context(errors::VaultError::PaymentMethodCreationFailed)?;
    } else {
        match db.find_payment_method(&response.card_id).await {
            Ok(_) => (),
            Err(err) => {
                if err.current_context().is_db_not_found() {
                    create_payment_method(db, &req, &customer_id, &response.card_id, merchant_id)
                        .await
                        .change_context(errors::VaultError::PaymentMethodCreationFailed)?;
                } else {
                    Err(errors::VaultError::PaymentMethodCreationFailed)?;
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
    customer_id: Option<&str>,
) -> errors::CustomResult<payment_methods::AddCardResponse, errors::VaultError> {
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
        customer_id: customer_id.map(str::to_string),
    };

    let response = db
        .insert_locker_mock_up(locker_mock_up)
        .await
        .change_context(errors::VaultError::SaveCardFailed)?;
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

#[instrument(skip_all)]
pub async fn get_card_from_legacy_locker<'a>(
    state: &'a routes::AppState,
    locker_id: &'a str,
    card_id: &'a str,
) -> errors::RouterResult<payment_methods::GetCardResponse> {
    let locker = &state.conf.locker;
    let request = payment_methods::mk_get_card_request(locker, locker_id, card_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Making get card request failed")?;
    let get_card_result = if !locker.mock_locker {
        let response = services::call_connector_api(state, request)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while executing call_connector_api for get_card");

        response.get_response_inner("AddCardResponse")?
    } else {
        let (get_card_response, _) = mock_get_card(&*state.store, card_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while fetching card from mock_locker")?;
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

    let card_delete_failure_message = "Failed while deleting card from card_locker";
    let delete_card_resp = if !locker.mock_locker {
        services::call_connector_api(state, request)
            .await
            .get_response_inner("DeleteCardResponse")?
    } else {
        mock_delete_card(&*state.store, card_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(card_delete_failure_message)?
    };

    Ok(delete_card_resp)
}

pub async fn list_payment_methods(
    db: &dyn db::StorageInterface,
    merchant_account: storage::MerchantAccount,
    mut req: api::ListPaymentMethodRequest,
) -> errors::RouterResponse<api::ListPaymentMethodResponse> {
    let payment_intent = helpers::verify_client_secret(
        db,
        merchant_account.storage_scheme,
        req.client_secret.clone(),
        &merchant_account.merchant_id,
    )
    .await?;
    let address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(db, pi.billing_address_id.clone()).await
        })
        .await
        .transpose()?
        .flatten();

    let payment_attempt = payment_intent
        .as_ref()
        .async_map(|pi| async {
            db.find_payment_attempt_by_payment_id_merchant_id(
                &pi.payment_id,
                &pi.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)
        })
        .await
        .transpose()?;

    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_list(&merchant_account.merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let mut response: HashSet<api::ListPaymentMethod> = HashSet::new();
    for mca in all_mcas {
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
            address.as_ref(),
        )
        .await?;
    }

    response
        .is_empty()
        .then(|| Err(report!(errors::ApiErrorResponse::PaymentMethodNotFound)))
        .unwrap_or(Ok(services::ApplicationResponse::Json(
            api::ListPaymentMethodResponse {
                redirect_url: merchant_account.return_url,
                payment_methods: response,
            },
        )))
}

async fn filter_payment_methods(
    payment_methods: Vec<serde_json::Value>,
    req: &mut api::ListPaymentMethodRequest,
    resp: &mut HashSet<api::ListPaymentMethod>,
    payment_intent: Option<&storage::PaymentIntent>,
    payment_attempt: Option<&storage::PaymentAttempt>,
    address: Option<&storage::Address>,
) -> errors::CustomResult<(), errors::ApiErrorResponse> {
    for payment_method in payment_methods.into_iter() {
        if let Ok(payment_method_object) =
            serde_json::from_value::<api::ListPaymentMethod>(payment_method)
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
                let filter3 = if let Some(payment_intent) = payment_intent {
                    filter_payment_country_based(&payment_method_object, address).await?
                        && filter_payment_currency_based(payment_intent, &payment_method_object)
                        && filter_payment_amount_based(payment_intent, &payment_method_object)
                        && filter_payment_mandate_based(payment_attempt, &payment_method_object)
                            .await?
                } else {
                    true
                };

                if filter && filter2 && filter3 {
                    resp.insert(payment_method_object);
                }
            }
        }
    }
    Ok(())
}

fn filter_accepted_enum_based<T: Eq + std::hash::Hash + Clone>(
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

fn filter_amount_based(payment_method: &api::ListPaymentMethod, amount: Option<i64>) -> bool {
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
    payment_method: &api::ListPaymentMethod,
    recurring_enabled: Option<bool>,
) -> bool {
    recurring_enabled.map_or(true, |enabled| payment_method.recurring_enabled == enabled)
}

fn filter_installment_based(
    payment_method: &api::ListPaymentMethod,
    installment_payment_enabled: Option<bool>,
) -> bool {
    installment_payment_enabled.map_or(true, |enabled| {
        payment_method.installment_payment_enabled == enabled
    })
}

async fn filter_payment_country_based(
    pm: &api::ListPaymentMethod,
    address: Option<&storage::Address>,
) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
    Ok(address.map_or(true, |address| {
        address.country.as_ref().map_or(true, |country| {
            pm.accepted_countries
                .clone()
                .map_or(true, |ac| ac.contains(country))
        })
    }))
}

fn filter_payment_currency_based(
    payment_intent: &storage::PaymentIntent,
    pm: &api::ListPaymentMethod,
) -> bool {
    payment_intent.currency.map_or(true, |currency| {
        pm.accepted_currencies
            .clone()
            .map_or(true, |ac| ac.contains(&currency.foreign_into()))
    })
}

fn filter_payment_amount_based(
    payment_intent: &storage::PaymentIntent,
    pm: &api::ListPaymentMethod,
) -> bool {
    let amount = payment_intent.amount;
    pm.maximum_amount.map_or(true, |amt| amount < amt)
        && pm.minimum_amount.map_or(true, |amt| amount > amt)
}

async fn filter_payment_mandate_based(
    payment_attempt: Option<&storage::PaymentAttempt>,
    pm: &api::ListPaymentMethod,
) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
    let recurring_filter = if !pm.recurring_enabled {
        payment_attempt.map_or(true, |pa| pa.mandate_id.is_none())
    } else {
        true
    };
    Ok(recurring_filter)
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

    let mut enabled_methods: HashSet<api::ListPaymentMethod> = HashSet::new();
    for mca in all_mcas {
        let payment_methods = match mca.payment_methods_enabled {
            Some(pm) => pm,
            None => continue,
        };

        for payment_method in payment_methods.into_iter() {
            if let Ok(payment_method_object) =
                serde_json::from_value::<api::ListPaymentMethod>(payment_method)
            {
                enabled_methods.insert(payment_method_object);
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
            let locker_id = merchant_account
                .locker_id
                .to_owned()
                .get_required_value("locker_id")?;
            Some(get_lookup_key_from_locker(state, &payment_token, &pm, &locker_id).await?)
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
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
            created: Some(pm.created_at),
        };
        vec.push(pma);
    }

    let response = api::ListCustomerPaymentMethodsResponse {
        enabled_payment_methods: enabled_methods,
        customer_payment_methods: vec,
    };

    Ok(services::ApplicationResponse::Json(response))
}

pub async fn get_lookup_key_from_locker(
    state: &routes::AppState,
    payment_token: &str,
    pm: &storage::PaymentMethod,
    locker_id: &str,
) -> errors::RouterResult<api::CardDetailFromLocker> {
    let get_card_resp =
        get_card_from_legacy_locker(state, locker_id, pm.payment_method_id.as_str()).await?;
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

        let value1 = vault::VaultPaymentMethod::Card(value1);
        let value2 = vault::VaultPaymentMethod::Card(value2);

        let value1 = utils::Encode::<vault::VaultPaymentMethod>::encode_to_string_of_json(&value1)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value1 construction failed when saving card to locker")?;

        let value2 = utils::Encode::<vault::VaultPaymentMethod>::encode_to_string_of_json(&value2)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value2 construction failed when saving card to locker")?;

        vault::create_tokenize(state, value1, Some(value2), payment_token.to_string()).await?;
        Ok(card)
    }
}

#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    state: &routes::AppState,
    pm: api::PaymentMethodId,
    merchant_account: storage::MerchantAccount,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = &*state.store;
    let pm = db
        .find_payment_method(&pm.payment_method_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;
    let card = if pm.payment_method == enums::PaymentMethodType::Card {
        let locker_id = merchant_account.locker_id.get_required_value("locker_id")?;
        let get_card_resp =
            get_card_from_legacy_locker(state, &locker_id, &pm.payment_method_id).await?;
        let card_detail = payment_methods::get_card_detail(&pm, get_card_resp.card)
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
            payment_method: pm.payment_method.foreign_into(),
            payment_method_type: pm.payment_method_type.map(ForeignInto::foreign_into),
            payment_method_issuer: pm.payment_method_issuer,
            card,
            metadata: pm.metadata,
            created: Some(pm.created_at),
            payment_method_issuer_code: pm
                .payment_method_issuer_code
                .map(ForeignInto::foreign_into),
            recurring_enabled: false,           //[#219]
            installment_payment_enabled: false, //[#219]
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), //[#219],
        },
    ))
}

#[instrument(skip_all)]
pub async fn delete_payment_method(
    state: &routes::AppState,
    merchant_account: storage::MerchantAccount,
    pm: api::PaymentMethodId,
) -> errors::RouterResponse<api::DeletePaymentMethodResponse> {
    let (_, supplementary_data) =
        vault::Vault::get_payment_method_data_from_locker(state, &pm.payment_method_id).await?;
    let payment_method_id = supplementary_data
        .payment_method_id
        .map_or(Err(errors::ApiErrorResponse::PaymentMethodNotFound), Ok)?;
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

    Ok(services::ApplicationResponse::Json(
        api::DeletePaymentMethodResponse {
            payment_method_id: pm.payment_method_id,
            deleted: true,
        },
    ))
}
