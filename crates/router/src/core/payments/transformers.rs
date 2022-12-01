use std::{fmt::Debug, marker::PhantomData};

use error_stack::ResultExt;
use router_env::{instrument, tracing};

use super::{flows::Feature, PaymentAddress, PaymentData};
use crate::{
    configs::settings::Server,
    core::{
        errors::{self, RouterResponse, RouterResult, StorageErrorExt},
        payments::{self, helpers},
    },
    db::Db,
    routes::AppState,
    services::{self, RedirectForm},
    types::{
        self,
        api::{self, NextAction, PaymentsResponse},
        storage::{self, enums},
    },
    utils::{OptionExt, ValueExt},
};

#[instrument(skip_all)]
pub async fn construct_payment_router_data<'a, F, T>(
    state: &'a AppState,
    payment_data: PaymentData<F>,
    connector_id: &str,
    merchant_account: &storage::MerchantAccount,
) -> RouterResult<(
    PaymentData<F>,
    types::RouterData<F, T, types::PaymentsResponseData>,
)>
where
    T: TryFrom<PaymentData<F>>,
    types::RouterData<F, T, types::PaymentsResponseData>: Feature<F, T>,
    F: Clone,
    error_stack::Report<errors::ApiErrorResponse>:
        std::convert::From<<T as TryFrom<PaymentData<F>>>::Error>,
{
    //TODO: everytime parsing the json may have impact?

    let (merchant_connector_account, payment_method, router_data);
    let db = &state.store as &dyn Db;
    merchant_connector_account = db
        .find_merchant_connector_account_by_merchant_id_connector(
            &merchant_account.merchant_id,
            connector_id,
        )
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound)
        })?;

    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .connector_account_details
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    payment_method = payment_data
        .payment_attempt
        .payment_method
        .or(payment_data.payment_attempt.payment_method)
        .get_required_value("payment_method_type")?;

    let response = payment_data
        .payment_attempt
        .connector_transaction_id
        .as_ref()
        .map(|id| types::PaymentsResponseData {
            connector_transaction_id: id.to_string(),
            //TODO: Add redirection details here
            redirection_data: None,
            redirect: false,
        });

    let orca_return_url = Some(helpers::create_redirect_url(
        &state.conf.server,
        &payment_data.payment_attempt,
    ));

    router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.clone(),
        connector: merchant_connector_account.connector_name,
        payment_id: payment_data.payment_attempt.payment_id.clone(),
        status: payment_data.payment_attempt.status,
        amount: payment_data.amount,
        currency: payment_data.currency,
        payment_method,
        connector_auth_type: auth_type,
        description: payment_data.payment_intent.description.clone(),
        return_url: payment_data.payment_intent.return_url.clone(),
        orca_return_url,
        payment_method_id: payment_data.payment_attempt.payment_method_id.clone(),
        address: payment_data.address.clone(),
        auth_type: payment_data
            .payment_attempt
            .authentication_type
            .unwrap_or_default(),

        request: T::try_from(payment_data.clone())?,

        response: response.map_or_else(|| Err(types::ErrorResponse::default()), Ok),
    };

    Ok((payment_data, router_data))
}

#[instrument(skip_all)]
// try to use router data here so that already validated things , we don't want to repeat the validations.
// Add internal value not found and external value not found so that we can give 500 / Internal server error for internal value not found
#[allow(clippy::too_many_arguments)]
pub fn payments_to_payments_response<R, Op>(
    payment_request: Option<R>,
    payment_attempt: storage::PaymentAttempt,
    payment_intent: storage::PaymentIntent,
    refunds: Vec<storage::Refund>,
    mandate_id: Option<String>,
    payment_method_data: Option<api::PaymentMethod>,
    customer: Option<api::CustomerResponse>,
    auth_flow: services::AuthFlow,
    address: PaymentAddress,
    server: &Server,
    redirection_data: Option<serde_json::Value>,
    operation: Op,
) -> RouterResponse<api::PaymentsResponse>
where
    api::PaymentsResponse: From<R>,
    Op: Debug,
{
    let currency = payment_attempt
        .currency
        .as_ref()
        .get_required_value("currency")?
        .to_string();

    let refunds_response = if refunds.is_empty() {
        None
    } else {
        Some(refunds.into_iter().map(From::from).collect())
    };

    Ok(match payment_request {
        Some(request) => {
            if payments::is_start_pay(&operation) && redirection_data.is_some() {
                let redirection_data = redirection_data.get_required_value("redirection_data")?;
                let form: RedirectForm = serde_json::from_value(redirection_data)
                    .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;
                services::BachResponse::Form(form)
            } else {
                let mut response: PaymentsResponse = request.into();
                let mut next_action_response = None;
                if payment_intent.status == enums::IntentStatus::RequiresCustomerAction {
                    next_action_response = Some(NextAction {
                        next_action_type: api::NextActionType::RedirectToUrl,
                        redirect_to_url: Some(helpers::create_startpay_url(
                            server,
                            &payment_attempt,
                            &payment_intent,
                        )),
                    })
                }

                services::BachResponse::Json(
                    response
                        .set_payment_id(Some(payment_attempt.payment_id))
                        .set_merchant_id(Some(payment_attempt.merchant_id))
                        .set_status(payment_intent.status)
                        .set_amount(payment_attempt.amount)
                        .set_amount_capturable(None)
                        .set_amount_received(payment_intent.amount_captured)
                        .set_client_secret(payment_intent.client_secret.map(masking::Secret::new))
                        .set_created(Some(payment_intent.created_at))
                        .set_currency(currency)
                        .set_customer_id(customer.as_ref().map(|cus| cus.clone().customer_id))
                        .set_email(
                            customer
                                .as_ref()
                                .and_then(|cus| cus.email.as_ref().map(|s| s.to_owned())),
                        )
                        .set_name(
                            customer
                                .as_ref()
                                .and_then(|cus| cus.name.as_ref().map(|s| s.to_owned().into())),
                        )
                        .set_phone(
                            customer
                                .as_ref()
                                .and_then(|cus| cus.phone.as_ref().map(|s| s.to_owned())),
                        )
                        .set_mandate_id(mandate_id)
                        .set_description(payment_intent.description)
                        .set_refunds(refunds_response) // refunds.iter().map(refund_to_refund_response),
                        .set_payment_method(
                            payment_attempt.payment_method,
                            auth_flow == services::AuthFlow::Merchant,
                        )
                        .set_payment_method_data(
                            payment_method_data.map(api::PaymentMethodDataResponse::from),
                            auth_flow == services::AuthFlow::Merchant,
                        )
                        .set_error_message(payment_attempt.error_message)
                        .set_shipping(address.shipping)
                        .set_billing(address.billing)
                        .to_owned()
                        .set_next_action(next_action_response)
                        .set_return_url(payment_intent.return_url)
                        .set_authentication_type(payment_attempt.authentication_type)
                        .set_statement_descriptor_name(payment_intent.statement_descriptor_name)
                        .set_statement_descriptor_suffix(payment_intent.statement_descriptor_suffix)
                        .set_setup_future_usage(payment_intent.setup_future_usage)
                        .set_capture_method(payment_attempt.capture_method)
                        .to_owned(),
                )
            }
        }
        None => services::BachResponse::Json(PaymentsResponse {
            payment_id: Some(payment_attempt.payment_id),
            merchant_id: Some(payment_attempt.merchant_id),
            status: payment_intent.status,
            amount: payment_attempt.amount,
            amount_capturable: None,
            amount_received: payment_intent.amount_captured,
            client_secret: payment_intent.client_secret.map(masking::Secret::new),
            created: Some(payment_intent.created_at),
            currency,
            customer_id: payment_intent.customer_id,
            description: payment_intent.description,
            refunds: refunds_response,
            payment_method: payment_attempt.payment_method,
            capture_method: payment_attempt.capture_method,
            error_message: payment_attempt.error_message,
            payment_method_data: payment_method_data.map(api::PaymentMethodDataResponse::from),
            email: customer
                .as_ref()
                .and_then(|cus| cus.email.as_ref().map(|s| s.to_owned())),
            name: customer
                .as_ref()
                .and_then(|cus| cus.name.as_ref().map(|s| s.to_owned().into())),
            phone: customer
                .as_ref()
                .and_then(|cus| cus.phone.as_ref().map(|s| s.to_owned())),
            mandate_id,
            shipping: address.shipping,
            billing: address.billing,
            ..Default::default()
        }),
    })
}

impl<F: Clone> TryFrom<PaymentData<F>> for types::PaymentsRequestData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        let browser_info: Option<types::BrowserInformation> = payment_data
            .payment_attempt
            .browser_info
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;
        Ok(Self {
            payment_method_data: {
                let payment_method_type = payment_data
                    .payment_attempt
                    .payment_method
                    .get_required_value("payment_method_type")?;

                match payment_method_type {
                    enums::PaymentMethodType::Paypal => api::PaymentMethod::Paypal,
                    _ => payment_data
                        .payment_method_data
                        .get_required_value("payment_method_data")?,
                }
            },
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            mandate_id: payment_data.mandate_id.clone(),
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            setup_mandate_details: payment_data.setup_mandate.clone(),
            confirm: payment_data.payment_attempt.confirm,
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            capture_method: payment_data.payment_attempt.capture_method,
            browser_info,
        })
    }
}

impl<F: Clone> TryFrom<PaymentData<F>> for types::PaymentsRequestSyncData {
    type Error = errors::ApiErrorResponse;

    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            connector_transaction_id: payment_data
                .payment_attempt
                .connector_transaction_id
                .ok_or(errors::ApiErrorResponse::SuccessfulPaymentNotFound)?,
            encoded_data: payment_data.connector_response.encoded_data,
        })
    }
}

impl<F: Clone> TryFrom<PaymentData<F>> for types::PaymentsRequestCaptureData {
    type Error = errors::ApiErrorResponse;

    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_to_capture: payment_data.payment_attempt.amount_to_capture,
            connector_transaction_id: payment_data
                .payment_attempt
                .connector_transaction_id
                .ok_or(errors::ApiErrorResponse::MerchantConnectorAccountNotFound)?,
        })
    }
}

impl<F: Clone> TryFrom<PaymentData<F>> for types::PaymentRequestCancelData {
    type Error = errors::ApiErrorResponse;

    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            connector_transaction_id: payment_data
                .payment_attempt
                .connector_transaction_id
                .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "connector_transaction_id".to_string(),
                })?,
            cancellation_reason: payment_data.payment_attempt.cancellation_reason,
        })
    }
}
