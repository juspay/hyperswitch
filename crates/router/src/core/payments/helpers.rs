use std::borrow::Cow;

// TODO : Evaluate all the helper functions ()
use error_stack::{report, IntoReport, ResultExt};
use masking::{PeekInterface, PeekOptionInterface};
use router_env::{instrument, tracing};
use uuid::Uuid;

use super::{
    operations::{BoxedOperation, Operation, PaymentResponse},
    CustomerDetails, PaymentData,
};
use crate::{
    configs::settings::Server,
    consts,
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::cards,
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        api,
        storage::{self, enums, ephemeral_key},
    },
    utils::{
        self,
        crypto::{self, SignMessage},
        OptionExt,
    },
};

pub async fn get_address_for_payment_request(
    db: &dyn StorageInterface,
    req_address: Option<&api::Address>,
    address_id: Option<&str>,
) -> CustomResult<Option<storage::Address>, errors::ApiErrorResponse> {
    // TODO: Refactor this function for more readability (TryFrom)
    Ok(match req_address {
        Some(address) => {
            match address_id {
                Some(id) => Some(
                    db.update_address(id.to_owned(), address.into())
                        .await
                        .map_err(|err| {
                            err.to_not_found_response(errors::ApiErrorResponse::AddressNotFound)
                        })?,
                ),
                None => {
                    // generate a new address here
                    Some(
                        db.insert_address(storage::AddressNew {
                            city: address.address.as_ref().and_then(|a| a.city.clone()),
                            country: address.address.as_ref().and_then(|a| a.country.clone()),
                            line1: address.address.as_ref().and_then(|a| a.line1.clone()),
                            line2: address.address.as_ref().and_then(|a| a.line2.clone()),
                            line3: address.address.as_ref().and_then(|a| a.line3.clone()),
                            state: address.address.as_ref().and_then(|a| a.state.clone()),
                            zip: address.address.as_ref().and_then(|a| a.zip.clone()),
                            first_name: address.address.as_ref().and_then(|a| a.first_name.clone()),
                            last_name: address.address.as_ref().and_then(|a| a.last_name.clone()),
                            phone_number: address.phone.as_ref().and_then(|a| a.number.clone()),
                            country_code: address
                                .phone
                                .as_ref()
                                .and_then(|a| a.country_code.clone()),
                            ..storage::AddressNew::default()
                        })
                        .await
                        .map_err(|_| errors::ApiErrorResponse::InternalServerError)?,
                    )
                }
            }
        }
        None => match address_id {
            Some(id) => Some(db.find_address(id).await).transpose().map_err(|err| {
                err.to_not_found_response(errors::ApiErrorResponse::AddressNotFound)
            })?,
            None => None,
        },
    })
}

pub async fn get_address_by_id(
    db: &dyn StorageInterface,
    address_id: Option<String>,
) -> CustomResult<Option<storage::Address>, errors::ApiErrorResponse> {
    match address_id {
        None => Ok(None),
        Some(address_id) => Ok(db.find_address(&address_id).await.ok()),
    }
}

pub async fn get_token_pm_type_mandate_details(
    state: &AppState,
    request: &api::PaymentsRequest,
    mandate_type: Option<api::MandateTxnType>,
    merchant_id: &str,
) -> RouterResult<(
    Option<String>,
    Option<enums::PaymentMethodType>,
    Option<api::MandateData>,
)> {
    match mandate_type {
        Some(api::MandateTxnType::NewMandateTxn) => {
            let setup_mandate = request
                .mandate_data
                .clone()
                .get_required_value("mandate_data")?;
            Ok((
                request.payment_token.to_owned(),
                request.payment_method,
                Some(setup_mandate),
            ))
        }
        Some(api::MandateTxnType::RecurringMandateTxn) => {
            let (token_, payment_method_type_) =
                get_token_for_recurring_mandate(state, request, merchant_id).await?;
            Ok((token_, payment_method_type_, None))
        }
        None => Ok((
            request.payment_token.to_owned(),
            request.payment_method,
            request.mandate_data.clone(),
        )),
    }
}

pub async fn get_token_for_recurring_mandate(
    state: &AppState,
    req: &api::PaymentsRequest,
    merchant_id: &str,
) -> RouterResult<(Option<String>, Option<enums::PaymentMethodType>)> {
    let db = &*state.store;
    let mandate_id = req.mandate_id.clone().get_required_value("mandate_id")?;

    let mandate = db
        .find_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id.as_str())
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::MandateNotFound))?;

    let customer = req.customer_id.clone().get_required_value("customer_id")?;

    let payment_method_id = {
        if mandate.customer_id != customer {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "customer_id must match mandate customer_id".into()
            }))?
        }
        if mandate.mandate_status != enums::MandateStatus::Active {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "mandate is not active".into()
            }))?
        };
        mandate.payment_method_id
    };

    let payment_method = db
        .find_payment_method(payment_method_id.as_str())
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;

    let token = Uuid::new_v4().to_string();

    let _ = cards::get_lookup_key_from_locker(state, &token, &payment_method).await?;

    if let Some(payment_method_from_request) = req.payment_method {
        if payment_method_from_request != payment_method.payment_method {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "payment method in request does not match previously provided payment \
                          method information"
                    .into()
            }))?
        }
    };

    Ok((Some(token), Some(payment_method.payment_method)))
}

#[instrument(skip_all)]
/// Check weather the merchant id in the request
/// and merchant id in the merchant account are same.
pub fn validate_merchant_id(
    merchant_id: &str,
    request_merchant_id: Option<&str>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // Get Merchant Id from the merchant
    // or get from merchant account

    let request_merchant_id = request_merchant_id.unwrap_or(merchant_id);

    utils::when(
        merchant_id.ne(request_merchant_id),
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "Invalid `merchant_id`: {request_merchant_id} not found in merchant account"
            )
        })),
    )
}

#[instrument(skip_all)]
pub fn validate_request_amount_and_amount_to_capture(
    op_amount: Option<i32>,
    op_amount_to_capture: Option<i32>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    // If both amount and amount to capture is present
    // then amount to be capture should be less than or equal to request amount

    let is_capture_amount_valid = op_amount
        .and_then(|amount| {
            op_amount_to_capture.map(|amount_to_capture| amount_to_capture.le(&amount))
        })
        .unwrap_or(true);

    utils::when(
        !is_capture_amount_valid,
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
            "amount_to_capture is greater than amount capture_amount: {:?} request_amount: {:?}",
            op_amount_to_capture, op_amount
        )
        })),
    )
}

pub fn validate_mandate(
    req: impl Into<api::MandateValidationFields>,
) -> RouterResult<Option<api::MandateTxnType>> {
    let req: api::MandateValidationFields = req.into();
    match req.is_mandate() {
        Some(api::MandateTxnType::NewMandateTxn) => {
            validate_new_mandate_request(req)?;
            Ok(Some(api::MandateTxnType::NewMandateTxn))
        }
        Some(api::MandateTxnType::RecurringMandateTxn) => {
            validate_recurring_mandate(req)?;
            Ok(Some(api::MandateTxnType::RecurringMandateTxn))
        }
        None => Ok(None),
    }
}

fn validate_new_mandate_request(req: api::MandateValidationFields) -> RouterResult<()> {
    let confirm = req.confirm.get_required_value("confirm")?;

    if !confirm {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`confirm` must be `true` for mandates".into()
        }))?
    }

    let _ = req.customer_id.as_ref().get_required_value("customer_id")?;

    let mandate_data = req
        .mandate_data
        .clone()
        .get_required_value("mandate_data")?;

    if enums::FutureUsage::OnSession
        == req
            .setup_future_usage
            .get_required_value("setup_future_usage")?
    {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`setup_future_usage` must be `off_session` for mandates".into()
        }))?
    };

    if (mandate_data.customer_acceptance.acceptance_type == api::AcceptanceType::Online)
        && mandate_data.customer_acceptance.online.is_none()
    {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`mandate_data.customer_acceptance.online` is required when \
                      `mandate_data.customer_acceptance.acceptance_type` is `online`"
                .into()
        }))?
    }

    Ok(())
}

pub fn create_startpay_url(
    server: &Server,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
) -> String {
    format!(
        "{}/payments/start/{}/{}/{}",
        server.base_url,
        payment_intent.payment_id,
        payment_intent.merchant_id,
        payment_attempt.txn_id
    )
}

pub fn create_redirect_url(server: &Server, payment_attempt: &storage::PaymentAttempt) -> String {
    format!(
        "{}/payments/{}/{}/response/{}",
        server.base_url,
        payment_attempt.payment_id,
        payment_attempt.merchant_id,
        payment_attempt.connector
    )
}
fn validate_recurring_mandate(req: api::MandateValidationFields) -> RouterResult<()> {
    req.mandate_id.check_value_present("mandate_id")?;

    req.customer_id.check_value_present("customer_id")?;

    let confirm = req.confirm.get_required_value("confirm")?;
    if !confirm {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`confirm` must be `true` for mandates".into()
        }))?
    }

    let off_session = req.off_session.get_required_value("off_session")?;
    if !off_session {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "`off_session` should be `true` for mandates".into()
        }))?
    }

    Ok(())
}

#[instrument(skip_all)]
pub fn payment_attempt_status_fsm(
    payment_method_data: &Option<api::PaymentMethod>,
    confirm: Option<bool>,
) -> enums::AttemptStatus {
    match payment_method_data {
        Some(_) => match confirm {
            Some(true) => enums::AttemptStatus::Pending,
            _ => enums::AttemptStatus::ConfirmationAwaited,
        },
        None => enums::AttemptStatus::PaymentMethodAwaited,
    }
}

pub fn payment_intent_status_fsm(
    payment_method_data: &Option<api::PaymentMethod>,
    confirm: Option<bool>,
) -> enums::IntentStatus {
    match payment_method_data {
        Some(_) => match confirm {
            Some(true) => enums::IntentStatus::RequiresCustomerAction,
            _ => enums::IntentStatus::RequiresConfirmation,
        },
        None => enums::IntentStatus::RequiresPaymentMethod,
    }
}

pub fn response_operation<'a, F, R>() -> BoxedOperation<'a, F, R>
where
    F: Send + Clone,
    PaymentResponse: Operation<F, R>,
{
    Box::new(PaymentResponse)
}

pub async fn amap<A, B, E, F, Fut>(value: Result<A, E>, func: F) -> Result<B, E>
where
    F: FnOnce(A) -> Fut,
    Fut: futures::Future<Output = Result<B, E>>,
{
    match value {
        Ok(a) => func(a).await,
        Err(err) => Err(err),
    }
}

#[instrument(skip_all)]
pub(crate) async fn call_payment_method(
    state: &AppState,
    merchant_id: &str,
    payment_method: Option<&api::PaymentMethod>,
    payment_method_type: Option<enums::PaymentMethodType>,
    maybe_customer: &Option<storage::Customer>,
) -> RouterResult<api::PaymentMethodResponse> {
    match payment_method {
        Some(pm_data) => match payment_method_type {
            Some(payment_method_type) => match pm_data {
                api::PaymentMethod::Card(card) => {
                    //TODO: get it from temp_card
                    let card_detail = api::CardDetail {
                        card_number: card.card_number.clone(),
                        card_exp_month: card.card_exp_month.clone(),
                        card_exp_year: card.card_exp_year.clone(),
                        card_holder_name: Some(card.card_holder_name.clone()),
                    };
                    match maybe_customer {
                        Some(customer) => {
                            let customer_id = customer.customer_id.clone();
                            let payment_method_request = api::CreatePaymentMethod {
                                merchant_id: Some(merchant_id.to_string()),
                                payment_method: payment_method_type,
                                payment_method_type: None,
                                payment_method_issuer: None,
                                payment_method_issuer_code: None,
                                card: Some(card_detail),
                                metadata: None,
                                customer_id: Some(customer_id),
                            };
                            let resp = cards::add_payment_method(
                                state,
                                payment_method_request,
                                merchant_id.to_string(),
                            )
                            .await
                            .attach_printable("Error on adding payment method")?;
                            match resp {
                                crate::services::BachResponse::Json(payment_method) => {
                                    Ok(payment_method)
                                }
                                _ => Err(report!(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable("Error on adding payment method")),
                            }
                        }
                        None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "customer".to_string()
                        })
                        .attach_printable("Missing Customer Object")),
                    }
                }
                _ => {
                    let payment_method_request = api::CreatePaymentMethod {
                        merchant_id: Some(merchant_id.to_string()),
                        payment_method: payment_method_type,
                        payment_method_type: None,
                        payment_method_issuer: None,
                        payment_method_issuer_code: None,
                        card: None,
                        metadata: None,
                        customer_id: None,
                    };
                    let resp = cards::add_payment_method(
                        state,
                        payment_method_request,
                        merchant_id.to_string(),
                    )
                    .await
                    .attach_printable("Error on adding payment method")?;
                    match resp {
                        crate::services::BachResponse::Json(payment_method) => Ok(payment_method),
                        _ => Err(report!(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Error on adding payment method")),
                    }
                }
            },
            None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_type".to_string()
            })
            .attach_printable("PaymentMethodType Required")),
        },
        None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "payment_method_data".to_string()
        })
        .attach_printable("PaymentMethodData required Or Card is already saved")),
    }
}

pub(crate) fn client_secret_auth(
    payload: api::PaymentsRequest,
    auth_type: &services::api::MerchantAuthentication,
) -> RouterResult<api::PaymentsRequest> {
    match auth_type {
        services::MerchantAuthentication::PublishableKey => {
            payload
                .client_secret
                .check_value_present("client_secret")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "client_secret".to_owned(),
                })?;
            Ok(payload)
        }
        services::api::MerchantAuthentication::ApiKey => {
            if payload.client_secret.is_some() {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "client_secret is not a valid parameter".to_owned(),
                }))
            } else {
                Ok(payload)
            }
        }
        _ => Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unexpected Auth type")),
    }
}

pub async fn get_customer_from_details(
    db: &dyn StorageInterface,
    customer_id: Option<String>,
    merchant_id: &str,
) -> CustomResult<Option<storage::Customer>, errors::StorageError> {
    match customer_id {
        None => Ok(None),
        Some(c_id) => {
            db.find_customer_optional_by_customer_id_merchant_id(&c_id, merchant_id)
                .await
        }
    }
}

#[instrument(skip_all)]
pub async fn create_customer_if_not_exist<'a, F: Clone, R>(
    operation: BoxedOperation<'a, F, R>,
    db: &dyn StorageInterface,
    payment_data: &mut PaymentData<F>,
    req: Option<CustomerDetails>,
    merchant_id: &str,
) -> CustomResult<(BoxedOperation<'a, F, R>, Option<storage::Customer>), errors::StorageError> {
    let req = req
        .get_required_value("customer")
        .change_context(errors::StorageError::ValueNotFound("customer".to_owned()))?;
    let optional_customer = match req.customer_id.as_ref() {
        Some(customer_id) => {
            let customer_data = db
                .find_customer_optional_by_customer_id_merchant_id(customer_id, merchant_id)
                .await?;
            Some(match customer_data {
                Some(c) => Ok(c),
                None => {
                    let new_customer = storage::CustomerNew {
                        customer_id: customer_id.to_string(),
                        merchant_id: merchant_id.to_string(),
                        name: req.name.peek_cloning(),
                        email: req.email.clone(),
                        phone: req.phone.clone(),
                        phone_country_code: req.phone_country_code.clone(),
                        ..storage::CustomerNew::default()
                    };

                    db.insert_customer(new_customer).await
                }
            })
        }
        None => match &payment_data.payment_intent.customer_id {
            None => None,
            Some(customer_id) => db
                .find_customer_optional_by_customer_id_merchant_id(customer_id, merchant_id)
                .await?
                .map(Ok),
        },
    };
    Ok((
        operation,
        match optional_customer {
            Some(customer) => {
                let customer = customer?;

                payment_data.payment_intent.customer_id = Some(customer.customer_id.clone());

                Some(customer)
            }
            None => None,
        },
    ))
}

#[allow(clippy::too_many_arguments)]
pub async fn make_pm_data<'a, F: Clone, R>(
    operation: BoxedOperation<'a, F, R>,
    state: &'a AppState,
    payment_method: Option<enums::PaymentMethodType>,
    txn_id: &str,
    _payment_attempt: &storage::PaymentAttempt,
    request: &Option<api::PaymentMethod>,
    token: &Option<String>,
) -> RouterResult<(BoxedOperation<'a, F, R>, Option<api::PaymentMethod>)> {
    let payment_method = match (request, token) {
        (_, Some(token)) => Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(
            if payment_method == Some(enums::PaymentMethodType::Card) {
                // TODO: Handle token expiry
                Vault::get_payment_method_data_from_locker(state, token).await?
            } else {
                // TODO: Implement token flow for other payment methods
                None
            },
        ),
        (pm @ Some(api::PaymentMethod::Card(card)), _) => {
            Vault::store_payment_method_data_in_locker(state, txn_id, card).await?;
            Ok(pm.to_owned())
        }
        (pm @ Some(api::PaymentMethod::PayLater(_)), _) => Ok(pm.to_owned()),
        (pm @ Some(api::PaymentMethod::Wallet(_)), _) => Ok(pm.to_owned()),
        _ => Ok(None),
    }?;

    let payment_method = match payment_method {
        Some(pm) => Some(pm),
        None => Vault::get_payment_method_data_from_locker(state, txn_id).await?,
    };

    Ok((operation, payment_method))
}

pub struct Vault {}

#[cfg(not(feature = "basilisk"))]
impl Vault {
    #[instrument(skip_all)]
    pub async fn get_payment_method_data_from_locker(
        state: &AppState,
        lookup_key: &str,
    ) -> RouterResult<Option<api::PaymentMethod>> {
        let resp = cards::mock_get_card(&*state.store, lookup_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let card = resp.card;
        let card_number = card
            .card_number
            .peek_cloning()
            .get_required_value("card_number")?;
        let card_exp_month = card
            .card_exp_month
            .peek_cloning()
            .get_required_value("expiry_month")?;
        let card_exp_year = card
            .card_exp_year
            .peek_cloning()
            .get_required_value("expiry_year")?;
        let card_holder_name = card.name_on_card.peek_cloning().unwrap_or_default();
        let card = api::PaymentMethod::Card(api::CCard {
            card_number: card_number.into(),
            card_exp_month: card_exp_month.into(),
            card_exp_year: card_exp_year.into(),
            card_holder_name: card_holder_name.into(),
            card_cvc: "card_cvc".to_string().into(),
        });
        Ok(Some(card))
    }

    #[instrument(skip_all)]
    async fn store_payment_method_data_in_locker(
        state: &AppState,
        txn_id: &str,
        card: &api::CCard,
    ) -> RouterResult<String> {
        let card_detail = api::CardDetail {
            card_number: card.card_number.clone(),
            card_exp_month: card.card_exp_month.clone(),
            card_exp_year: card.card_exp_year.clone(),
            card_holder_name: Some(card.card_holder_name.clone()),
        };
        let db = &*state.store;
        cards::mock_add_card(db, txn_id, &card_detail)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add Card Failed")?;
        Ok(txn_id.to_string())
    }
}

#[cfg(feature = "basilisk")]
use crate::{core::payment_methods::transformers, utils::StringExt};

#[cfg(feature = "basilisk")]
impl Vault {
    #[instrument(skip_all)]
    pub async fn get_payment_method_data_from_locker(
        state: &AppState,
        lookup_key: &str,
    ) -> RouterResult<Option<api::PaymentMethod>> {
        let de_tokenize = cards::get_tokenized_data(state, lookup_key, true).await?;
        let value1: api::TokenizedCardValue1 = de_tokenize
            .value1
            .parse_struct("TokenizedCardValue1")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error parsing TokenizedCardValue1")?;
        let value2 = de_tokenize.value2;
        let card_cvc = if value2.is_empty() {
            //mandatory field in api contract (when querying from legacy locker we don't get cvv), cvv handling needs to done
            "".to_string()
        } else {
            let tk_value2: api::TokenizedCardValue2 = value2
                .parse_struct("TokenizedCardValue2")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error parsing TokenizedCardValue2")?;
            tk_value2.card_security_code.unwrap_or_default()
        };
        let card = api::PaymentMethod::Card(api::CCard {
            card_number: value1.card_number.into(),
            card_exp_month: value1.exp_month.into(),
            card_exp_year: value1.exp_year.into(),
            card_holder_name: value1.name_on_card.unwrap_or_default().into(),
            card_cvc: card_cvc.into(),
        });
        Ok(Some(card))
    }

    #[instrument(skip_all)]
    async fn store_payment_method_data_in_locker(
        state: &AppState,
        txn_id: &str,
        card: &api::CCard,
    ) -> RouterResult<String> {
        let value1 = transformers::mk_card_value1(
            card.card_number.peek().clone(),
            card.card_exp_year.peek().clone(),
            card.card_exp_month.peek().clone(),
            Some(card.card_holder_name.peek().clone()),
            None,
            None,
            None,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value1 for locker")?;
        let value2 =
            transformers::mk_card_value2(Some(card.card_cvc.peek().clone()), None, None, None)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error getting Value12 for locker")?;
        cards::create_tokenize(state, value1, Some(value2), txn_id.to_string()).await
    }
}

#[instrument(skip_all)]
pub async fn create_temp_card(
    state: &AppState,
    txn_id: &str,
    card: &api::CCard,
) -> RouterResult<storage::TempCard> {
    let (card_info, temp_card);
    card_info = format!(
        "{}:::{}:::{}:::{}:::{}",
        card.card_number.peek(),
        card.card_exp_month.peek(),
        card.card_exp_year.peek(),
        card.card_holder_name.peek(),
        card.card_cvc.peek()
    );

    let card_info_val = cards::get_card_info_value(&state.conf.keys, card_info).await?;
    temp_card = storage::TempCardNew {
        card_info: Some(card_info_val),
        date_created: common_utils::date_time::now(),
        txn_id: Some(txn_id.to_string()),
        id: None,
    };
    state
        .store
        .insert_temp_card(temp_card)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
}

#[instrument(skip_all)]
pub(crate) fn validate_capture_method(capture_method: enums::CaptureMethod) -> RouterResult<()> {
    utils::when(
        capture_method == enums::CaptureMethod::Automatic,
        Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
            field_name: "capture_method".to_string(),
            current_flow: "captured".to_string(),
            current_value: capture_method.to_string(),
            states: "manual_single, manual_multiple, scheduled".to_string()
        })),
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_status(status: enums::IntentStatus) -> RouterResult<()> {
    utils::when(
        status != enums::IntentStatus::RequiresCapture,
        Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
            field_name: "payment.status".to_string(),
            current_flow: "captured".to_string(),
            current_value: status.to_string(),
            states: "requires_capture".to_string()
        })),
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_amount_to_capture(
    amount: i32,
    amount_to_capture: Option<i32>,
) -> RouterResult<()> {
    utils::when(
        amount_to_capture.is_some() && (Some(amount) < amount_to_capture),
        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "amount_to_capture is greater than amount".to_string()
        })),
    )
}

pub fn can_call_connector(status: enums::IntentStatus) -> bool {
    matches!(
        status,
        enums::IntentStatus::Failed
            | enums::IntentStatus::Processing
            | enums::IntentStatus::Succeeded
            | enums::IntentStatus::RequiresCustomerAction
    )
}

pub fn append_option<T, U, F, V>(func: F, option1: Option<T>, option2: Option<U>) -> Option<V>
where
    F: FnOnce(T, U) -> V,
{
    Some(func(option1?, option2?))
}

pub(super) async fn filter_by_constraints(
    db: &dyn StorageInterface,
    constraints: &api::PaymentListConstraints,
    merchant_id: &str,
) -> CustomResult<Vec<storage::PaymentIntent>, errors::StorageError> {
    let result = db
        .filter_payment_intent_by_constraints(merchant_id, constraints)
        .await?;
    Ok(result)
}

pub(super) fn validate_payment_list_request(
    req: &api::PaymentListConstraints,
) -> CustomResult<(), errors::ApiErrorResponse> {
    utils::when(
        req.limit > 100 || req.limit < 1,
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "limit should be in between 1 and 100".to_string(),
        }),
    )?;
    Ok(())
}

pub fn get_handle_response_url(
    payment_id: String,
    merchant_account: &storage::MerchantAccount,
    response: api::PaymentsResponse,
    connector: String,
) -> RouterResult<api::RedirectionResponse> {
    let payments_return_url = response.return_url.as_ref();
    let redirection_response = make_pg_redirect_response(payment_id, &response, connector);

    let return_url = make_merchant_url_with_response(
        merchant_account,
        redirection_response,
        payments_return_url,
    )
    .attach_printable("Failed to make merchant url with response")?;

    make_url_with_signature(&return_url, merchant_account)
}

pub fn make_merchant_url_with_response(
    merchant_account: &storage::MerchantAccount,
    redirection_response: api::PgRedirectResponse,
    request_return_url: Option<&String>,
) -> RouterResult<String> {
    // take return url if provided in the request else use merchant return url
    let url = request_return_url
        .or(merchant_account.return_url.as_ref())
        .get_required_value("return_url")?;

    let status_check = redirection_response.status;

    let payment_intent_id = redirection_response.payment_id;

    let merchant_url_with_response = if merchant_account.redirect_to_merchant_with_http_post {
        url::Url::parse_with_params(
            url,
            &[
                ("status", status_check.to_string()),
                ("order_id", payment_intent_id),
            ],
        )
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url with param")?
    } else {
        let amount = redirection_response.amount.get_required_value("amount")?;
        url::Url::parse_with_params(
            url,
            &[
                ("status", status_check.to_string()),
                ("order_id", payment_intent_id),
                ("amount", amount.to_string()),
            ],
        )
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url with param")?
    };

    Ok(merchant_url_with_response.to_string())
}

pub async fn make_ephemeral_key(
    state: &AppState,
    customer_id: String,
    merchant_id: String,
) -> errors::RouterResponse<ephemeral_key::EphemeralKey> {
    let store = &state.store;
    let id = utils::generate_id(consts::ID_LENGTH, "eki");
    let secret = format!("epk_{}", &Uuid::new_v4().simple().to_string());
    let ek = ephemeral_key::EphemeralKeyNew {
        id,
        customer_id,
        merchant_id,
        secret,
    };
    let ek = store
        .create_ephemeral_key(ek, state.conf.eph_key.validity)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to create ephemeral key")?;
    Ok(services::BachResponse::Json(ek))
}

pub async fn delete_ephemeral_key(
    store: &dyn StorageInterface,
    ek_id: String,
) -> errors::RouterResponse<ephemeral_key::EphemeralKey> {
    let ek = store
        .delete_ephemeral_key(&ek_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to delete ephemeral key")?;
    Ok(services::BachResponse::Json(ek))
}

pub fn make_pg_redirect_response(
    payment_id: String,
    response: &api::PaymentsResponse,
    connector: String,
) -> api::PgRedirectResponse {
    api::PgRedirectResponse {
        payment_id,
        status: response.status,
        gateway_id: connector,
        customer_id: response.customer_id.to_owned(),
        amount: Some(response.amount),
    }
}

pub fn make_url_with_signature(
    redirect_url: &str,
    merchant_account: &storage::MerchantAccount,
) -> RouterResult<api::RedirectionResponse> {
    let mut url = url::Url::parse(redirect_url)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse the url")?;

    let mut base_url = url.clone();
    base_url.query_pairs_mut().clear();

    let url = if merchant_account.enable_payment_response_hash {
        let key = merchant_account
            .payment_response_hash_key
            .as_ref()
            .get_required_value("payment_response_hash_key")?;
        let signature = hmac_sha256_sorted_query_params(
            &mut url.query_pairs().collect::<Vec<_>>(),
            key.as_str(),
        )?;

        url.query_pairs_mut()
            .append_pair("signature", &signature)
            .append_pair("signature_algorithm", "HMAC-SHA256");
        url.to_owned()
    } else {
        url.to_owned()
    };

    let parameters = url
        .query_pairs()
        .collect::<Vec<_>>()
        .iter()
        .map(|(key, value)| (key.clone().into_owned(), value.clone().into_owned()))
        .collect::<Vec<_>>();

    Ok(api::RedirectionResponse {
        return_url: base_url.to_string(),
        params: parameters,
        return_url_with_query_params: url.to_string(),
        http_method: if merchant_account.redirect_to_merchant_with_http_post {
            services::Method::Post
        } else {
            services::Method::Get
        },
        headers: Vec::new(),
    })
}

pub fn hmac_sha256_sorted_query_params<'a>(
    params: &mut [(Cow<str>, Cow<str>)],
    key: &'a str,
) -> RouterResult<String> {
    params.sort();
    let final_string = params
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");

    let signature = crypto::HmacSha256::sign_message(
        &crypto::HmacSha256,
        key.as_bytes(),
        final_string.as_bytes(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to sign the message")?;

    Ok(hex::encode(signature))
}

pub fn check_if_operation_confirm<Op: std::fmt::Debug>(operations: Op) -> bool {
    format!("{:?}", operations) == "PaymentConfirm"
}
