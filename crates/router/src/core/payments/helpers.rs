use std::borrow::Cow;

// TODO : Evaluate all the helper functions ()
use error_stack::{report, IntoReport, ResultExt};
use masking::{PeekInterface, PeekOptionInterface};
use rand::Rng;
use router_env::{instrument, tracing};

use super::{
    operations::{BoxedOperation, Operation, PaymentResponse},
    CustomerDetails, PaymentData,
};
use crate::{
    configs::settings::{Keys, Server},
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::cards,
    },
    db::{mandate::IMandate, payment_method::IPaymentMethod, temp_card::ITempCard, Db},
    routes::AppState,
    services,
    types::{
        api::{self, PgRedirectResponse},
        storage::{self, enums},
    },
    utils::{
        self,
        crypto::{self, SignMessage},
        OptionExt,
    },
};

pub async fn get_address_for_payment_request(
    db: &dyn Db,
    req_address: Option<&api::Address>,
    address_id: Option<&str>,
) -> CustomResult<Option<storage::Address>, errors::ApiErrorResponse> {
    // TODO: Refactoring this function for more redability (TryFrom)
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
    db: &dyn Db,
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
    Option<i32>,
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
                request.payment_token,
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
            request.payment_token,
            request.payment_method,
            request.mandate_data.clone(),
        )),
    }
}

pub async fn get_token_for_recurring_mandate(
    state: &AppState,
    req: &api::PaymentsRequest,
    merchant_id: &str,
) -> RouterResult<(Option<i32>, Option<enums::PaymentMethodType>)> {
    let db = &state.store;
    let mandate_id = req.mandate_id.clone().get_required_value("mandate_id")?;

    let mandate = db
        .find_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id.as_str())
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::MandateNotFound))?;

    let customer = req.customer_id.clone().get_required_value("customer_id")?;

    let payment_method_id = {
        if mandate.customer_id != customer {
            Err(report!(errors::ValidateError)
                .attach_printable("Invalid Mandate ID")
                .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "customer_id".to_string(),
                    expected_format: "customer_id must match mandate customer_id".to_string(),
                }))?
        }
        if mandate.mandate_status != enums::MandateStatus::Active {
            Err(report!(errors::ValidateError)
                .attach_printable("Mandate is not active")
                .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "mandate_id".to_string(),
                    expected_format: "mandate_id of an active mandate".to_string(),
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

    let token = rand::thread_rng().gen::<i32>();

    let _ = crate::core::payment_methods::cards::get_tempcard_from_payment_method(
        state,
        token,
        &payment_method,
    )
    .await?;

    if let Some(payment_method_from_request) = req.payment_method {
        if payment_method_from_request != payment_method.payment_method {
            Err(report!(errors::ValidateError)
                .attach_printable("Invalid Mandate ID")
                .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "payment_method".to_string(),
                    expected_format: "valid payment method information".to_string(),
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
) -> CustomResult<(), errors::ValidateError> {
    // Get Merchant Id from the merchant
    // or get from merchant account

    let request_merchant_id = request_merchant_id.unwrap_or(merchant_id);

    utils::when(
        merchant_id.ne(request_merchant_id),
        Err(report!(errors::ValidateError).attach_printable(format!(
            "Invalid merchant_id: {request_merchant_id} not found in merchant account"
        ))),
    )
}

#[instrument(skip_all)]
pub fn validate_request_amount_and_amount_to_capture(
    op_amount: Option<i32>,
    op_amount_to_capture: Option<i32>,
) -> CustomResult<(), errors::ValidateError> {
    // If both amount and amount to capture is present
    // then amount to be capture should be less than or equal to request amount

    let is_capture_amount_valid = op_amount
        .and_then(|amount| {
            op_amount_to_capture.map(|amount_to_capture| amount_to_capture.le(&amount))
        })
        .unwrap_or(true);

    utils::when(
        !is_capture_amount_valid,
        Err(report!(errors::ValidateError).attach_printable(format!(
            "amount_to_capture is greater than amount capture_amount: {:?} request_amount: {:?}",
            op_amount_to_capture, op_amount
        ))),
    )
}

pub fn validate_mandate(req: &api::PaymentsRequest) -> RouterResult<Option<api::MandateTxnType>> {
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

fn validate_new_mandate_request(req: &api::PaymentsRequest) -> RouterResult<()> {
    let confirm = req.confirm.get_required_value("confirm")?;

    if !confirm {
        Err(report!(errors::ValidateError)
            .attach_printable("Confirm should be true for mandates")
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "confirm".to_string(),
                expected_format: "confirm must be true for mandates".to_string(),
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
        Err(report!(errors::ValidateError)
            .attach_printable("Key 'setup_future_usage' should be 'off_session' for mandates")
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "setup_future_usage".to_string(),
                expected_format: "setup_future_usage must be off_session for mandates".to_string(),
            }))?
    };

    if (mandate_data.customer_acceptance.acceptance_type == api::AcceptanceType::Online)
        && mandate_data.customer_acceptance.online.is_none()
    {
        Err(report!(errors::ValidateError)
        .attach_printable("Key 'mandate_data.customer_acceptance.online' is required when 'mandate_data.customer_acceptance.acceptance_type' is 'online'")
        .change_context(errors::ApiErrorResponse::MissingRequiredField { field_name: "mandate_data.customer_acceptance.online".to_string() }))?
    }

    Ok(())
}

pub fn create_server_url(server: &Server) -> String {
    if server.host.eq("127.0.0.1") || server.host.eq("localhost") {
        format!("http://{}:{}", server.host, server.port)
    } else {
        format!("https://{}", server.host)
    }
}
pub fn create_startpay_url(
    server: &Server,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
) -> String {
    let server_url = create_server_url(server);

    format!(
        "{}/payments/start/{}/{}/{}",
        server_url, payment_intent.payment_id, payment_intent.merchant_id, payment_attempt.txn_id
    )
}

pub fn create_redirect_url(server: &Server, payment_attempt: &storage::PaymentAttempt) -> String {
    let server_url = create_server_url(server);

    format!(
        "{}/payments/{}/{}/response/{}",
        server_url,
        payment_attempt.payment_id,
        payment_attempt.merchant_id,
        payment_attempt.connector
    )
}
fn validate_recurring_mandate(req: &api::PaymentsRequest) -> RouterResult<()> {
    req.mandate_id.check_value_present("mandate_id")?;

    req.customer_id.check_value_present("customer_id")?;

    let confirm = req.confirm.get_required_value("confirm")?;
    if !confirm {
        Err(report!(errors::ValidateError)
            .attach_printable("Confirm should be true for mandates")
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "confirm".to_string(),
                expected_format: "confirm must be true for mandates".to_string(),
            }))?
    }

    let off_session = req.off_session.get_required_value("off_session")?;
    if !off_session {
        Err(report!(errors::ValidateError)
            .attach_printable("off_session should be true for mandates")
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "off_session".to_string(),
                expected_format: "off_session must be true for mandates".to_string(),
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
    maybe_customer: &Option<api::CustomerResponse>,
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
    db: &dyn Db,
    customer_id: Option<String>,
    merchant_id: &str,
) -> CustomResult<Option<api::CustomerResponse>, errors::StorageError> {
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
    db: &dyn Db,
    payment_data: &mut PaymentData<F>,
    req: Option<CustomerDetails>,
    merchant_id: &str,
) -> CustomResult<(BoxedOperation<'a, F, R>, Option<api::CustomerResponse>), errors::StorageError> {
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
                    db.insert_customer(api::CreateCustomerRequest {
                        customer_id: customer_id.to_string(),
                        merchant_id: merchant_id.to_string(),
                        name: req.name.peek_cloning(),
                        email: req.email.clone(),
                        phone: req.phone.clone(),
                        phone_country_code: req.phone_country_code.clone(),
                        ..api::CreateCustomerRequest::default()
                    })
                    .await
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
    token: Option<i32>,
) -> RouterResult<(BoxedOperation<'a, F, R>, Option<api::PaymentMethod>)> {
    let payment_method = match (request, token) {
        (_, Some(token)) => Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(
            if payment_method == Some(enums::PaymentMethodType::Card) {
                // TODO: Handle token expiry
                payment_method_data_from_temp_card(
                    &state.conf.keys,
                    state
                        .store
                        .find_tempcard_by_token(&token)
                        .await
                        .map_err(|error| {
                            error.to_not_found_response(
                                errors::ApiErrorResponse::PaymentMethodNotFound,
                            )
                        })?,
                )
                .await?
            } else {
                // TODO: Implement token flow for other payment methods
                None
            },
        ),
        (pm @ Some(api::PaymentMethod::Card(card)), _) => {
            create_temp_card(state, txn_id, card)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)?;
            Ok(pm.to_owned())
        }
        (pm @ Some(api::PaymentMethod::PayLater(_)), _) => Ok(pm.to_owned()),
        _ => Ok(None),
    }?;

    let payment_method = match payment_method {
        Some(pm) => Some(pm),
        None => {
            let temp_card = state.store.find_tempcard_by_transaction_id(txn_id).await;
            if let Ok(Some(temp_card)) = temp_card {
                payment_method_data_from_temp_card(&state.conf.keys, temp_card).await?
            } else {
                None
            }
        }
    };

    Ok((operation, payment_method))
}

#[instrument(skip_all)]
pub async fn payment_method_data_from_temp_card(
    keys: &Keys,
    temp_card: storage::TempCard,
) -> RouterResult<Option<api::PaymentMethod>> {
    match temp_card.card_info {
        Some(card_info_val) => {
            let card_info = cards::get_card_info_from_value(keys, card_info_val).await?;
            let card_info_split: Vec<&str> = card_info.split(":::").collect();

            let card = api::PaymentMethod::Card(api::CCard {
                card_number: card_info_split[0].to_string().into(),
                card_exp_month: card_info_split[1].to_string().into(),
                card_exp_year: card_info_split[2].to_string().into(),
                card_holder_name: card_info_split[3].to_string().into(),
                card_cvc: if card_info_split.len() > 4 {
                    card_info_split[4].to_string().into()
                } else {
                    "".to_string().into()
                },
            });
            Ok(Some(card))
        }
        None => Ok(None),
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
        date_created: crate::utils::date_time::now(),
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
    db: &dyn Db,
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
    let redirection_response = make_pg_redirect_response(payment_id, response, connector);

    let return_url = make_merchant_url_with_response(merchant_account, redirection_response)
        .attach_printable("Failed to make merchant url with response")?;

    make_url_with_signature(&return_url, merchant_account)
}

pub fn make_merchant_url_with_response(
    merchant_account: &storage::MerchantAccount,
    redirection_response: PgRedirectResponse,
) -> RouterResult<String> {
    let url = merchant_account
        .return_url
        .as_ref()
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

pub fn make_pg_redirect_response(
    payment_id: String,
    response: api::PaymentsResponse,
    connector: String,
) -> PgRedirectResponse {
    PgRedirectResponse {
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
