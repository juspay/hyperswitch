use std::borrow::Cow;

use common_utils::{ext_traits::AsyncExt, fp_utils};
// TODO : Evaluate all the helper functions ()
use error_stack::{report, IntoReport, ResultExt};
use masking::ExposeOptionInterface;
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
        payment_methods::{cards, vault},
    },
    db::StorageInterface,
    routes::AppState,
    scheduler::{metrics, workflows::payment_sync},
    services,
    types::{
        api::{self, enums as api_enums, CustomerAcceptanceExt, MandateValidationFieldsExt},
        storage::{self, enums as storage_enums, ephemeral_key},
        transformers::ForeignInto,
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
    merchant_id: &str,
    customer_id: &Option<String>,
) -> CustomResult<Option<storage::Address>, errors::ApiErrorResponse> {
    Ok(match req_address {
        Some(address) => {
            match address_id {
                Some(id) => Some(
                    db.update_address(id.to_owned(), address.foreign_into())
                        .await
                        .map_err(|err| {
                            err.to_not_found_response(errors::ApiErrorResponse::AddressNotFound)
                        })?,
                ),
                None => {
                    // generate a new address here
                    let customer_id = customer_id
                        .as_deref()
                        .get_required_value("customer_id")
                        .change_context(errors::ApiErrorResponse::CustomerNotFound)?;

                    let address_details = address.address.clone().unwrap_or_default();
                    Some(
                        db.insert_address(storage::AddressNew {
                            phone_number: address.phone.as_ref().and_then(|a| a.number.clone()),
                            country_code: address
                                .phone
                                .as_ref()
                                .and_then(|a| a.country_code.clone()),
                            customer_id: customer_id.to_string(),
                            merchant_id: merchant_id.to_string(),

                            ..address_details.foreign_into()
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
    merchant_account: &storage::MerchantAccount,
) -> RouterResult<(
    Option<String>,
    Option<storage_enums::PaymentMethodType>,
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
                request.payment_method.map(ForeignInto::foreign_into),
                Some(setup_mandate),
            ))
        }
        Some(api::MandateTxnType::RecurringMandateTxn) => {
            let (token_, payment_method_type_) =
                get_token_for_recurring_mandate(state, request, merchant_account).await?;
            Ok((token_, payment_method_type_, None))
        }
        None => Ok((
            request.payment_token.to_owned(),
            request.payment_method.map(ForeignInto::foreign_into),
            request.mandate_data.clone(),
        )),
    }
}

pub async fn get_token_for_recurring_mandate(
    state: &AppState,
    req: &api::PaymentsRequest,
    merchant_account: &storage::MerchantAccount,
) -> RouterResult<(Option<String>, Option<storage_enums::PaymentMethodType>)> {
    let db = &*state.store;
    let mandate_id = req.mandate_id.clone().get_required_value("mandate_id")?;

    let mandate = db
        .find_mandate_by_merchant_id_mandate_id(&merchant_account.merchant_id, mandate_id.as_str())
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::MandateNotFound))?;

    let customer = req.customer_id.clone().get_required_value("customer_id")?;

    let payment_method_id = {
        if mandate.customer_id != customer {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "customer_id must match mandate customer_id".into()
            }))?
        }
        if mandate.mandate_status != storage_enums::MandateStatus::Active {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "mandate is not active".into()
            }))?
        };
        mandate.payment_method_id.clone()
    };
    verify_mandate_details(
        req.amount.get_required_value("amount")?.into(),
        req.currency.get_required_value("currency")?,
        mandate.clone(),
    )?;

    let payment_method = db
        .find_payment_method(payment_method_id.as_str())
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;

    let token = Uuid::new_v4().to_string();
    let locker_id = merchant_account
        .locker_id
        .to_owned()
        .get_required_value("locker_id")?;
    let _ = cards::get_lookup_key_from_locker(state, &token, &payment_method, &locker_id).await?;

    if let Some(payment_method_from_request) = req.payment_method {
        let pm: storage_enums::PaymentMethodType = payment_method_from_request.foreign_into();
        if pm != payment_method.payment_method {
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

    utils::when(merchant_id.ne(request_merchant_id), || {
        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "Invalid `merchant_id`: {request_merchant_id} not found in merchant account"
            )
        }))
    })
}

#[instrument(skip_all)]
pub fn validate_request_amount_and_amount_to_capture(
    op_amount: Option<api::Amount>,
    op_amount_to_capture: Option<i64>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    match (op_amount, op_amount_to_capture) {
        (None, _) => Ok(()),
        (Some(_amount), None) => Ok(()),
        (Some(amount), Some(amount_to_capture)) => {
            match amount {
                api::Amount::Value(amount_inner) => {
                    // If both amount and amount to capture is present
                    // then amount to be capture should be less than or equal to request amount
                    utils::when(!amount_to_capture.le(&amount_inner.get()), || {
                        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                            message: format!(
                            "amount_to_capture is greater than amount capture_amount: {amount_to_capture:?} request_amount: {amount:?}"
                        )
                        }))
                    })
                }
                api::Amount::Zero => {
                    // If the amount is Null but still amount_to_capture is passed this is invalid and
                    Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                        message: "amount_to_capture should not exist for when amount = 0"
                            .to_string()
                    }))
                }
            }
        }
    }
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

    if api_enums::FutureUsage::OnSession
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

pub fn validate_customer_id_mandatory_cases(
    has_shipping: bool,
    has_billing: bool,
    has_setup_future_usage: bool,
    customer_id: &Option<String>,
) -> RouterResult<()> {
    match (
        has_shipping,
        has_billing,
        has_setup_future_usage,
        customer_id,
    ) {
        (true, _, _, None) | (_, true, _, None) | (_, _, true, None) => {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "customer_id is mandatory when shipping or billing \
                address is given or when setup_future_usage is given"
                    .to_string(),
            })
            .into_report()
        }
        _ => Ok(()),
    }
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
        payment_attempt.attempt_id
    )
}

pub fn create_redirect_url(
    server: &Server,
    payment_attempt: &storage::PaymentAttempt,
    connector_name: &String,
) -> String {
    format!(
        "{}/payments/{}/{}/response/{}",
        server.base_url, payment_attempt.payment_id, payment_attempt.merchant_id, connector_name
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

pub fn verify_mandate_details(
    request_amount: i64,
    request_currency: api_enums::Currency,
    mandate: storage::Mandate,
) -> RouterResult<()> {
    match mandate.mandate_type {
        storage_enums::MandateType::SingleUse => utils::when(
            mandate
                .mandate_amount
                .map(|mandate_amount| request_amount > mandate_amount)
                .unwrap_or(true),
            || {
                Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                    reason: "request amount is greater than mandate amount".to_string()
                }))
            },
        ),
        storage::enums::MandateType::MultiUse => utils::when(
            mandate
                .mandate_amount
                .map(|mandate_amount| {
                    (mandate.amount_captured.unwrap_or(0) + request_amount) > mandate_amount
                })
                .unwrap_or(false),
            || {
                Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                    reason: "request amount is greater than mandate amount".to_string()
                }))
            },
        ),
    }?;
    utils::when(
        mandate
            .mandate_currency
            .map(|mandate_currency| mandate_currency != request_currency.foreign_into())
            .unwrap_or(false),
        || {
            Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                reason: "cross currency mandates not supported".to_string()
            }))
        },
    )
}

#[instrument(skip_all)]
pub fn payment_attempt_status_fsm(
    payment_method_data: &Option<api::PaymentMethod>,
    confirm: Option<bool>,
) -> storage_enums::AttemptStatus {
    match payment_method_data {
        Some(_) => match confirm {
            Some(true) => storage_enums::AttemptStatus::Pending,
            _ => storage_enums::AttemptStatus::ConfirmationAwaited,
        },
        None => storage_enums::AttemptStatus::PaymentMethodAwaited,
    }
}

pub fn payment_intent_status_fsm(
    payment_method_data: &Option<api::PaymentMethod>,
    confirm: Option<bool>,
) -> storage_enums::IntentStatus {
    match payment_method_data {
        Some(_) => match confirm {
            Some(true) => storage_enums::IntentStatus::RequiresCustomerAction,
            _ => storage_enums::IntentStatus::RequiresConfirmation,
        },
        None => storage_enums::IntentStatus::RequiresPaymentMethod,
    }
}

pub async fn add_domain_task_to_pt<Op>(
    operation: &Op,
    state: &AppState,
    payment_attempt: &storage::PaymentAttempt,
) -> CustomResult<(), errors::ApiErrorResponse>
where
    Op: std::fmt::Debug,
{
    if check_if_operation_confirm(operation) {
        let connector_name = payment_attempt
            .connector
            .clone()
            .ok_or(errors::ApiErrorResponse::InternalServerError)?;

        let schedule_time = payment_sync::get_sync_process_schedule_time(
            &*state.store,
            &connector_name,
            &payment_attempt.merchant_id,
            0,
        )
        .await
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while getting process schedule time")?;

        match schedule_time {
            Some(stime) => {
                metrics::TASKS_ADDED_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics
                super::add_process_sync_task(&*state.store, payment_attempt, stime)
                    .await
                    .into_report()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while adding task to process tracker")
            }
            None => Ok(()),
        }
    } else {
        Ok(())
    }
}

pub fn response_operation<'a, F, R>() -> BoxedOperation<'a, F, R>
where
    F: Send + Clone,
    PaymentResponse: Operation<F, R>,
{
    Box::new(PaymentResponse)
}

#[instrument(skip_all)]
pub(crate) async fn call_payment_method(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    payment_method: Option<&api::PaymentMethod>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    maybe_customer: &Option<storage::Customer>,
) -> RouterResult<api::PaymentMethodResponse> {
    match payment_method {
        Some(pm_data) => match payment_method_type {
            Some(payment_method_type) => match pm_data {
                api::PaymentMethod::Card(card) => {
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
                                payment_method: payment_method_type.foreign_into(),
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
                                merchant_account,
                            )
                            .await
                            .attach_printable("Error on adding payment method")?;
                            match resp {
                                crate::services::ApplicationResponse::Json(payment_method) => {
                                    Ok(payment_method)
                                }
                                _ => Err(report!(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable("Error on adding payment method")),
                            }
                        }
                        None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "customer"
                        })
                        .attach_printable("Missing Customer Object")),
                    }
                }
                _ => {
                    let payment_method_request = api::CreatePaymentMethod {
                        payment_method: payment_method_type.foreign_into(),
                        payment_method_type: None,
                        payment_method_issuer: None,
                        payment_method_issuer_code: None,
                        card: None,
                        metadata: None,
                        customer_id: None,
                    };
                    let resp =
                        cards::add_payment_method(state, payment_method_request, merchant_account)
                            .await
                            .attach_printable("Error on adding payment method")?;
                    match resp {
                        crate::services::ApplicationResponse::Json(payment_method) => {
                            Ok(payment_method)
                        }
                        _ => Err(report!(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Error on adding payment method")),
                    }
                }
            },
            None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_type"
            })
            .attach_printable("PaymentMethodType Required")),
        },
        None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "payment_method_data"
        })
        .attach_printable("PaymentMethodData required Or Card is already saved")),
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

pub async fn get_connector_default(
    state: &AppState,
    request_connector: Option<&String>,
) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
    let connectors = &state.conf.connectors;
    if let Some(connector_name) = request_connector {
        let connector_data = api::ConnectorData::get_connector_by_name(
            connectors,
            connector_name,
            api::GetToken::Connector,
        )?;
        Ok(api::ConnectorCallType::Single(connector_data))
    } else {
        Ok(api::ConnectorCallType::Routing)
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
                        name: req.name.expose_option(),
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
    payment_data: &mut PaymentData<F>,
) -> RouterResult<(BoxedOperation<'a, F, R>, Option<api::PaymentMethod>)> {
    let request = &payment_data.payment_method_data;
    let token = payment_data.token.clone();
    let card_cvc = payment_data.card_cvc.clone();

    // TODO: Handle case where payment method and token both are present in request properly.
    let payment_method = match (request, token) {
        (_, Some(token)) => {
            let (pm, supplementary_data) = vault::Vault::get_payment_method_data_from_locker(
                state, &token,
            )
            .await
            .attach_printable(
                "Payment method for given token not found or there was a problem fetching it",
            )?;

            utils::when(
                supplementary_data
                    .customer_id
                    .ne(&payment_data.payment_intent.customer_id),
                || {
                    Err(errors::ApiErrorResponse::PreconditionFailed { message: "customer associated with payment method and customer passed in payment are not same".into() })
                },
            )?;

            Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(match pm.clone() {
                Some(api::PaymentMethod::Card(card)) => {
                    payment_data.payment_attempt.payment_method =
                        Some(storage_enums::PaymentMethodType::Card);
                    if let Some(cvc) = card_cvc {
                        let mut updated_card = card;
                        updated_card.card_cvc = cvc;
                        let updated_pm = api::PaymentMethod::Card(updated_card);
                        vault::Vault::store_payment_method_data_in_locker(
                            state,
                            Some(token),
                            &updated_pm,
                            payment_data.payment_intent.customer_id.to_owned(),
                        )
                        .await?;
                        Some(updated_pm)
                    } else {
                        pm
                    }
                }

                Some(api::PaymentMethod::Wallet(wallet_data)) => {
                    payment_data.payment_attempt.payment_method =
                        Some(storage_enums::PaymentMethodType::Wallet);
                    // TODO: Remove redundant update from wallets.
                    if wallet_data.token.is_some() {
                        let updated_pm = api::PaymentMethod::Wallet(wallet_data);
                        vault::Vault::store_payment_method_data_in_locker(
                            state,
                            Some(token),
                            &updated_pm,
                            payment_data.payment_intent.customer_id.to_owned(),
                        )
                        .await?;
                        Some(updated_pm)
                    } else {
                        pm
                    }
                }

                Some(_) => Err(errors::ApiErrorResponse::InternalServerError)
                    .into_report()
                    .attach_printable(
                        "Payment method received from locker is unsupported by locker",
                    )?,

                None => None,
            })
        }
        (pm_opt @ Some(pm @ api::PaymentMethod::Card(_)), _) => {
            let token = vault::Vault::store_payment_method_data_in_locker(
                state,
                None,
                pm,
                payment_data.payment_intent.customer_id.to_owned(),
            )
            .await?;
            payment_data.token = Some(token);
            Ok(pm_opt.to_owned())
        }
        (pm @ Some(api::PaymentMethod::PayLater(_)), _) => Ok(pm.to_owned()),
        (pm_opt @ Some(pm @ api::PaymentMethod::Wallet(_)), _) => {
            let token = vault::Vault::store_payment_method_data_in_locker(
                state,
                None,
                pm,
                payment_data.payment_intent.customer_id.to_owned(),
            )
            .await?;
            payment_data.token = Some(token);
            Ok(pm_opt.to_owned())
        }
        _ => Ok(None),
    }?;

    Ok((operation, payment_method))
}

#[instrument(skip_all)]
pub(crate) fn validate_capture_method(
    capture_method: storage_enums::CaptureMethod,
) -> RouterResult<()> {
    utils::when(
        capture_method == storage_enums::CaptureMethod::Automatic,
        || {
            Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
                field_name: "capture_method".to_string(),
                current_flow: "captured".to_string(),
                current_value: capture_method.to_string(),
                states: "manual_single, manual_multiple, scheduled".to_string()
            }))
        },
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_status(status: storage_enums::IntentStatus) -> RouterResult<()> {
    utils::when(
        status != storage_enums::IntentStatus::RequiresCapture,
        || {
            Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
                field_name: "payment.status".to_string(),
                current_flow: "captured".to_string(),
                current_value: status.to_string(),
                states: "requires_capture".to_string()
            }))
        },
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_amount_to_capture(
    amount: i64,
    amount_to_capture: Option<i64>,
) -> RouterResult<()> {
    utils::when(
        amount_to_capture.is_some() && (Some(amount) < amount_to_capture),
        || {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "amount_to_capture is greater than amount".to_string()
            }))
        },
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_payment_method_fields_present(
    req: &api::PaymentsRequest,
) -> RouterResult<()> {
    utils::when(
        req.payment_method.is_none() && req.payment_method_data.is_some(),
        || {
            Err(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payent_method",
            })
        },
    )?;

    utils::when(
        req.payment_method.is_some()
            && req.payment_method_data.is_none()
            && req.payment_token.is_none(),
        || {
            Err(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_data",
            })
        },
    )?;

    Ok(())
}

pub fn check_force_psync_precondition(
    status: &storage_enums::AttemptStatus,
    connector_transaction_id: &Option<String>,
) -> bool {
    !matches!(
        status,
        storage_enums::AttemptStatus::Charged
            | storage_enums::AttemptStatus::AutoRefunded
            | storage_enums::AttemptStatus::Voided
            | storage_enums::AttemptStatus::CodInitiated
            | storage_enums::AttemptStatus::Authorized
            | storage_enums::AttemptStatus::Started
            | storage_enums::AttemptStatus::Failure
    ) && connector_transaction_id.is_some()
}

pub fn append_option<T, U, F, V>(func: F, option1: Option<T>, option2: Option<U>) -> Option<V>
where
    F: FnOnce(T, U) -> V,
{
    Some(func(option1?, option2?))
}

#[cfg(feature = "olap")]
pub(super) async fn filter_by_constraints(
    db: &dyn StorageInterface,
    constraints: &api::PaymentListConstraints,
    merchant_id: &str,
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Vec<storage::PaymentIntent>, errors::StorageError> {
    let result = db
        .filter_payment_intent_by_constraints(merchant_id, constraints, storage_scheme)
        .await?;
    Ok(result)
}

#[cfg(feature = "olap")]
pub(super) fn validate_payment_list_request(
    req: &api::PaymentListConstraints,
) -> CustomResult<(), errors::ApiErrorResponse> {
    utils::when(req.limit > 100 || req.limit < 1, || {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "limit should be in between 1 and 100".to_string(),
        })
    })?;
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
                ("payment_intent_client_secret", payment_intent_id),
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
                ("payment_intent_client_secret", payment_intent_id),
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
    Ok(services::ApplicationResponse::Json(ek))
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
    Ok(services::ApplicationResponse::Json(ek))
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
            services::Method::Post.to_string()
        } else {
            services::Method::Get.to_string()
        },
        headers: Vec::new(),
    })
}

pub fn hmac_sha256_sorted_query_params(
    params: &mut [(Cow<'_, str>, Cow<'_, str>)],
    key: &str,
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
    format!("{operations:?}") == "PaymentConfirm"
}

pub fn generate_mandate(
    merchant_id: String,
    connector: String,
    setup_mandate_details: Option<api::MandateData>,
    customer: &Option<storage::Customer>,
    payment_method_id: String,
    connector_mandate_id: Option<String>,
) -> Option<storage::MandateNew> {
    match (setup_mandate_details, customer) {
        (Some(data), Some(cus)) => {
            let mandate_id = utils::generate_id(consts::ID_LENGTH, "man");

            // The construction of the mandate new must be visible
            let mut new_mandate = storage::MandateNew::default();

            new_mandate
                .set_mandate_id(mandate_id)
                .set_customer_id(cus.customer_id.clone())
                .set_merchant_id(merchant_id)
                .set_payment_method_id(payment_method_id)
                .set_connector(connector)
                .set_mandate_status(storage_enums::MandateStatus::Active)
                .set_connector_mandate_id(connector_mandate_id)
                .set_customer_ip_address(
                    data.customer_acceptance
                        .get_ip_address()
                        .map(masking::Secret::new),
                )
                .set_customer_user_agent(data.customer_acceptance.get_user_agent())
                .set_customer_accepted_at(Some(data.customer_acceptance.get_accepted_at()));

            Some(match data.mandate_type {
                api::MandateType::SingleUse(data) => new_mandate
                    .set_mandate_amount(Some(data.amount))
                    .set_mandate_currency(Some(data.currency.foreign_into()))
                    .set_mandate_type(storage_enums::MandateType::SingleUse)
                    .to_owned(),

                api::MandateType::MultiUse(op_data) => match op_data {
                    Some(data) => new_mandate
                        .set_mandate_amount(Some(data.amount))
                        .set_mandate_currency(Some(data.currency.foreign_into())),
                    None => &mut new_mandate,
                }
                .set_mandate_type(storage_enums::MandateType::MultiUse)
                .to_owned(),
            })
        }
        (_, _) => None,
    }
}

// A function to manually authenticate the client secret
pub(crate) fn authenticate_client_secret(
    request_client_secret: Option<&String>,
    payment_intent_client_secret: Option<&String>,
) -> Result<(), errors::ApiErrorResponse> {
    match (request_client_secret, payment_intent_client_secret) {
        (Some(req_cs), Some(pi_cs)) => utils::when(req_cs.ne(pi_cs), || {
            Err(errors::ApiErrorResponse::ClientSecretInvalid)
        }),
        _ => Ok(()),
    }
}

pub(crate) fn validate_payment_status_against_not_allowed_statuses(
    intent_status: &storage_enums::IntentStatus,
    not_allowed_statuses: &[storage_enums::IntentStatus],
    action: &'static str,
) -> Result<(), errors::ApiErrorResponse> {
    fp_utils::when(not_allowed_statuses.contains(intent_status), || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "You cannot {action} this payment because it has status {intent_status}",
            ),
        })
    })
}

pub(crate) fn validate_pm_or_token_given(
    payment_method: &Option<api_enums::PaymentMethodType>,
    payment_method_data: &Option<api::PaymentMethod>,
    mandate_type: &Option<api::MandateTxnType>,
    token: &Option<String>,
) -> Result<(), errors::ApiErrorResponse> {
    utils::when(
        !matches!(payment_method, Some(api_enums::PaymentMethodType::Paypal))
            && !matches!(mandate_type, Some(api::MandateTxnType::RecurringMandateTxn))
            && token.is_none()
            && (payment_method_data.is_none() || payment_method.is_none()),
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "A payment token or payment method data is required".to_string(),
            })
        },
    )
}

// A function to perform database lookup and then verify the client secret
pub(crate) async fn verify_client_secret(
    db: &dyn StorageInterface,
    storage_scheme: storage_enums::MerchantStorageScheme,
    client_secret: Option<String>,
    merchant_id: &str,
) -> error_stack::Result<Option<storage::PaymentIntent>, errors::ApiErrorResponse> {
    client_secret
        .async_map(|cs| async move {
            let payment_id = get_payment_id_from_client_secret(&cs);

            let payment_intent = db
                .find_payment_intent_by_payment_id_merchant_id(
                    &payment_id,
                    merchant_id,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

            authenticate_client_secret(Some(&cs), payment_intent.client_secret.as_ref())
                .map_err(errors::ApiErrorResponse::from)?;
            Ok(payment_intent)
        })
        .await
        .transpose()
}

#[inline]
pub(crate) fn get_payment_id_from_client_secret(cs: &str) -> String {
    cs.split('_').take(2).collect::<Vec<&str>>().join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticate_client_secret() {
        let req_cs = Some("1".to_string());
        let pi_cs = Some("2".to_string());
        assert!(authenticate_client_secret(req_cs.as_ref(), pi_cs.as_ref()).is_err())
    }
}
