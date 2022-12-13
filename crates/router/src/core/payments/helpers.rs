use std::borrow::Cow;

// TODO : Evaluate all the helper functions ()
use error_stack::{report, IntoReport, ResultExt};
use masking::{ExposeOptionInterface, PeekInterface};
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
    pii::Secret,
    routes::AppState,
    services,
    types::{
        self,
        api::{self, enums as api_enums, CustomerAcceptanceExt, MandateValidationFieldsExt},
        storage::{self, enums as storage_enums, ephemeral_key},
        transformers::ForeignInto,
    },
    utils::{
        self,
        crypto::{self, SignMessage},
        OptionExt, ValueExt,
    },
};

pub async fn get_address_for_payment_request(
    db: &dyn StorageInterface,
    req_address: Option<&api::Address>,
    address_id: Option<&str>,
    merchant_id: &str,
    customer_id: &Option<String>,
) -> CustomResult<Option<storage::Address>, errors::ApiErrorResponse> {
    // TODO: Refactor this function for more readability (TryFrom)
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
                            customer_id: customer_id.to_string(),
                            merchant_id: merchant_id.to_string(),
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
                get_token_for_recurring_mandate(state, request, merchant_id).await?;
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
    merchant_id: &str,
) -> RouterResult<(Option<String>, Option<storage_enums::PaymentMethodType>)> {
    let db = &*state.store;
    let mandate_id = req.mandate_id.clone().get_required_value("mandate_id")?;

    let mandate = db
        .find_mandate_by_merchant_id_mandate_id(merchant_id, mandate_id.as_str())
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::MandateNotFound))?;

    // TODO: Make currency in payments request as Currency enum

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
        req.currency.clone().get_required_value("currency")?,
        mandate.clone(),
    )?;

    let payment_method = db
        .find_payment_method(payment_method_id.as_str())
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
        })?;

    let token = Uuid::new_v4().to_string();

    let _ = cards::get_lookup_key_from_locker(state, &token, &payment_method).await?;

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
    op_amount: Option<api::Amount>,
    op_amount_to_capture: Option<i32>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    match (op_amount, op_amount_to_capture) {
        (None, _) => Ok(()),
        (Some(_amount), None) => Ok(()),
        (Some(amount), Some(amount_to_capture)) => {
            match amount {
                api::Amount::Value(amount_inner) => {
                    // If both amount and amount to capture is present
                    // then amount to be capture should be less than or equal to request amount
                    utils::when(
                        !amount_to_capture.le(&amount_inner),
                        Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                            message: format!(
                            "amount_to_capture is greater than amount capture_amount: {:?} request_amount: {:?}",
                            amount_to_capture, amount
                        )
                        })),
                    )
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
    request_amount: i32,
    request_currency: String,
    mandate: storage::Mandate,
) -> RouterResult<()> {
    match mandate.mandate_type {
        storage_enums::MandateType::SingleUse => utils::when(
            mandate
                .mandate_amount
                .map(|mandate_amount| request_amount > mandate_amount)
                .unwrap_or(true),
            Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                reason: "request amount is greater than mandate amount".to_string()
            })),
        ),
        storage::enums::MandateType::MultiUse => utils::when(
            mandate
                .mandate_amount
                .map(|mandate_amount| {
                    (mandate.amount_captured.unwrap_or(0) + request_amount) > mandate_amount
                })
                .unwrap_or(false),
            Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                reason: "request amount is greater than mandate amount".to_string()
            })),
        ),
    }?;
    utils::when(
        mandate
            .mandate_currency
            .map(|mandate_currency| mandate_currency.to_string() != request_currency)
            .unwrap_or(true),
        Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
            reason: "cross currency mandates not supported".to_string()
        })),
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
    payment_method_type: Option<storage_enums::PaymentMethodType>,
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
                        payment_method: payment_method_type.foreign_into(),
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

pub(crate) fn client_secret_auth<P>(
    payload: P,
    auth_type: &services::api::MerchantAuthentication,
) -> RouterResult<P>
where
    P: services::Authenticate,
{
    match auth_type {
        services::MerchantAuthentication::PublishableKey => {
            payload
                .get_client_secret()
                .check_value_present("client_secret")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "client_secret".to_owned(),
                })?;
            Ok(payload)
        }
        services::api::MerchantAuthentication::ApiKey => {
            if payload.get_client_secret().is_some() {
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

pub async fn get_connector_default(
    merchant_account: &storage::MerchantAccount,
    state: &AppState,
) -> CustomResult<api::ConnectorCallType, errors::ApiErrorResponse> {
    let connectors = &state.conf.connectors;
    let vec_val: Vec<serde_json::Value> = merchant_account
        .custom_routing_rules
        .clone()
        .parse_value("CustomRoutingRulesVec")
        .change_context(errors::ConnectorError::RoutingRulesParsingError)
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let custom_routing_rules: api::CustomRoutingRules = vec_val[0]
        .clone()
        .parse_value("CustomRoutingRules")
        .change_context(errors::ConnectorError::RoutingRulesParsingError)
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let connector_names = custom_routing_rules
        .connectors_pecking_order
        .unwrap_or_else(|| vec!["stripe".to_string()]);

    //use routing rules if configured by merchant else query MCA as per PM
    let connector_list: types::ConnectorsList = types::ConnectorsList {
        connectors: connector_names,
    };

    let connector_name = connector_list
        .connectors
        .first()
        .get_required_value("connectors")
        .change_context(errors::ConnectorError::FailedToObtainPreferredConnector)
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .as_str();

    let connector_data = api::ConnectorData::get_connector_by_name(connectors, connector_name)?;

    Ok(api::ConnectorCallType::Single(connector_data))
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
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    txn_id: &str,
    _payment_attempt: &storage::PaymentAttempt,
    request: &Option<api::PaymentMethod>,
    token: &Option<String>,
    card_cvc: Option<Secret<String>>,
) -> RouterResult<(
    BoxedOperation<'a, F, R>,
    Option<api::PaymentMethod>,
    Option<String>,
)> {
    let (payment_method, payment_token) = match (request, token) {
        (_, Some(token)) => Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(
            if payment_method_type == Some(storage_enums::PaymentMethodType::Card) {
                // TODO: Handle token expiry
                let pm = Vault::get_payment_method_data_from_locker(state, token).await?;
                let updated_pm = match (pm.clone(), card_cvc) {
                    (Some(api::PaymentMethod::Card(card)), Some(card_cvc)) => {
                        let mut updated_card = card;
                        updated_card.card_cvc = card_cvc;
                        Vault::store_payment_method_data_in_locker(state, txn_id, &updated_card)
                            .await?;
                        Some(api::PaymentMethod::Card(updated_card))
                    }
                    (_, _) => pm,
                };
                (updated_pm, Some(token.to_string()))
            } else {
                // TODO: Implement token flow for other payment methods
                (None, Some(token.to_string()))
            },
        ),
        (pm @ Some(api::PaymentMethod::Card(card)), _) => {
            Vault::store_payment_method_data_in_locker(state, txn_id, card).await?;
            Ok((pm.to_owned(), Some(txn_id.to_string())))
        }
        (pm @ Some(api::PaymentMethod::PayLater(_)), _) => Ok((pm.to_owned(), None)),
        (pm @ Some(api::PaymentMethod::Wallet(_)), _) => Ok((pm.to_owned(), None)),
        _ => Ok((None, None)),
    }?;

    Ok((operation, payment_method, payment_token))
}

pub struct Vault {}

#[cfg(not(feature = "basilisk"))]
impl Vault {
    #[instrument(skip_all)]
    pub async fn get_payment_method_data_from_locker(
        state: &AppState,
        lookup_key: &str,
    ) -> RouterResult<Option<api::PaymentMethod>> {
        let (resp, card_cvc) = cards::mock_get_card(&*state.store, lookup_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let card = resp.card;
        let card_number = card
            .card_number
            .expose_option()
            .get_required_value("card_number")?;
        let card_exp_month = card
            .card_exp_month
            .expose_option()
            .get_required_value("expiry_month")?;
        let card_exp_year = card
            .card_exp_year
            .expose_option()
            .get_required_value("expiry_year")?;
        let card_holder_name = card.name_on_card.expose_option().unwrap_or_default();
        let card = api::PaymentMethod::Card(api::CCard {
            card_number: card_number.into(),
            card_exp_month: card_exp_month.into(),
            card_exp_year: card_exp_year.into(),
            card_holder_name: card_holder_name.into(),
            card_cvc: card_cvc.unwrap_or_default().into(),
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
        cards::mock_add_card(db, txn_id, &card_detail, Some(card.card_cvc.peek().clone()))
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
pub(crate) fn validate_capture_method(
    capture_method: storage_enums::CaptureMethod,
) -> RouterResult<()> {
    utils::when(
        capture_method == storage_enums::CaptureMethod::Automatic,
        Err(report!(errors::ApiErrorResponse::PaymentUnexpectedState {
            field_name: "capture_method".to_string(),
            current_flow: "captured".to_string(),
            current_value: capture_method.to_string(),
            states: "manual_single, manual_multiple, scheduled".to_string()
        })),
    )
}

#[instrument(skip_all)]
pub(crate) fn validate_status(status: storage_enums::IntentStatus) -> RouterResult<()> {
    utils::when(
        status != storage_enums::IntentStatus::RequiresCapture,
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

pub fn can_call_connector(status: storage_enums::IntentStatus) -> bool {
    matches!(
        status,
        storage_enums::IntentStatus::Failed
            | storage_enums::IntentStatus::Processing
            | storage_enums::IntentStatus::Succeeded
            | storage_enums::IntentStatus::RequiresCustomerAction
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
    storage_scheme: storage_enums::MerchantStorageScheme,
) -> CustomResult<Vec<storage::PaymentIntent>, errors::StorageError> {
    let result = db
        .filter_payment_intent_by_constraints(merchant_id, constraints, storage_scheme)
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
            services::Method::Post.to_string()
        } else {
            services::Method::Get.to_string()
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
        (Some(req_cs), Some(pi_cs)) => utils::when(
            req_cs.ne(pi_cs),
            Err(errors::ApiErrorResponse::ClientSecretInvalid),
        ),
        _ => Ok(()),
    }
}

// A function to perform database lookup and then verify the client secret
pub(crate) async fn verify_client_secret(
    db: &dyn StorageInterface,
    storage_scheme: storage_enums::MerchantStorageScheme,
    client_secret: Option<String>,
    merchant_id: &str,
) -> error_stack::Result<(), errors::ApiErrorResponse> {
    match client_secret {
        None => Ok(()),
        Some(cs) => {
            let payment_id = cs.split('_').take(2).collect::<Vec<&str>>().join("_");

            let payment_intent = db
                .find_payment_intent_by_payment_id_merchant_id(
                    &payment_id,
                    merchant_id,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

            authenticate_client_secret(Some(&cs), payment_intent.client_secret.as_ref())
                .map_err(|err| err.into())
        }
    }
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
