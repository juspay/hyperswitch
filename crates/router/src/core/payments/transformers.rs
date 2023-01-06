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
    routes::AppState,
    services::{self, RedirectForm},
    types::{
        self, api,
        storage::{self, enums},
        transformers::ForeignInto,
    },
    utils::{OptionExt, ValueExt},
};

#[instrument(skip_all)]
pub async fn construct_payment_router_data<'a, F, T>(
    state: &'a AppState,
    payment_data: PaymentData<F>,
    connector_id: &str,
    merchant_account: &storage::MerchantAccount,
) -> RouterResult<types::RouterData<F, T, types::PaymentsResponseData>>
where
    T: TryFrom<PaymentData<F>>,
    types::RouterData<F, T, types::PaymentsResponseData>: Feature<F, T>,
    F: Clone,
    error_stack::Report<errors::ApiErrorResponse>: From<<T as TryFrom<PaymentData<F>>>::Error>,
{
    let (merchant_connector_account, payment_method, router_data);
    let db = &*state.store;
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

    // [#44]: why should response be filled during request
    let response = payment_data
        .payment_attempt
        .connector_transaction_id
        .as_ref()
        .map(|id| types::PaymentsResponseData::TransactionResponse {
            resource_id: types::ResponseId::ConnectorTransactionId(id.to_string()),
            redirection_data: None,
            redirect: false,
            mandate_reference: None,
        });

    let orca_return_url = Some(helpers::create_redirect_url(
        &state.conf.server,
        &payment_data.payment_attempt,
        &merchant_connector_account.connector_name,
    ));

    router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.clone(),
        connector: merchant_connector_account.connector_name,
        payment_id: payment_data.payment_attempt.payment_id.clone(),
        status: payment_data.payment_attempt.status,
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
        connector_meta_data: merchant_connector_account.metadata,
        request: T::try_from(payment_data.clone())?,
        response: response.map_or_else(|| Err(types::ErrorResponse::default()), Ok),
        amount_captured: payment_data.payment_intent.amount_captured,
    };

    Ok(router_data)
}

pub trait ToResponse<Req, D, Op>
where
    Self: From<Req>,
    Op: Debug,
{
    fn generate_response(
        req: Option<Req>,
        data: D,
        customer: Option<storage::Customer>,
        auth_flow: services::AuthFlow,
        server: &Server,
        operation: Op,
    ) -> RouterResponse<Self>;
}

impl<F, Req, Op> ToResponse<Req, PaymentData<F>, Op> for api::PaymentsResponse
where
    Self: From<Req>,
    F: Clone,
    Op: Debug,
{
    fn generate_response(
        req: Option<Req>,
        payment_data: PaymentData<F>,
        customer: Option<storage::Customer>,
        auth_flow: services::AuthFlow,
        server: &Server,
        operation: Op,
    ) -> RouterResponse<Self> {
        payments_to_payments_response(
            req,
            payment_data.payment_attempt,
            payment_data.payment_intent,
            payment_data.refunds,
            payment_data.payment_method_data,
            customer,
            auth_flow,
            payment_data.address,
            server,
            payment_data.connector_response.authentication_data,
            operation,
        )
    }
}

impl<F, Req, Op> ToResponse<Req, PaymentData<F>, Op> for api::PaymentsSessionResponse
where
    Self: From<Req>,
    F: Clone,
    Op: Debug,
{
    fn generate_response(
        _req: Option<Req>,
        payment_data: PaymentData<F>,
        _customer: Option<storage::Customer>,
        _auth_flow: services::AuthFlow,
        _server: &Server,
        _operation: Op,
    ) -> RouterResponse<Self> {
        Ok(services::BachResponse::Json(Self {
            session_token: payment_data.sessions_token,
            payment_id: payment_data.payment_attempt.payment_id,
            client_secret: payment_data
                .payment_intent
                .client_secret
                .get_required_value("client_secret")?
                .into(),
        }))
    }
}

impl<F, Req, Op> ToResponse<Req, PaymentData<F>, Op> for api::VerifyResponse
where
    Self: From<Req>,
    F: Clone,
    Op: Debug,
{
    fn generate_response(
        _req: Option<Req>,
        data: PaymentData<F>,
        customer: Option<storage::Customer>,
        _auth_flow: services::AuthFlow,
        _server: &Server,
        _operation: Op,
    ) -> RouterResponse<Self> {
        Ok(services::BachResponse::Json(Self {
            verify_id: Some(data.payment_intent.payment_id),
            merchant_id: Some(data.payment_intent.merchant_id),
            client_secret: data.payment_intent.client_secret.map(masking::Secret::new),
            customer_id: customer.as_ref().map(|x| x.customer_id.clone()),
            email: customer
                .as_ref()
                .and_then(|cus| cus.email.as_ref().map(|s| s.to_owned())),
            name: customer
                .as_ref()
                .and_then(|cus| cus.name.as_ref().map(|s| s.to_owned().into())),
            phone: customer
                .as_ref()
                .and_then(|cus| cus.phone.as_ref().map(|s| s.to_owned())),
            mandate_id: data.mandate_id.map(|mandate_ids| mandate_ids.mandate_id),
            payment_method: data
                .payment_attempt
                .payment_method
                .map(ForeignInto::foreign_into),
            payment_method_data: data
                .payment_method_data
                .map(api::PaymentMethodDataResponse::from),
            payment_token: data.token,
            error_code: None,
            error_message: data.payment_attempt.error_message,
        }))
    }
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
    payment_method_data: Option<api::PaymentMethod>,
    customer: Option<storage::Customer>,
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
    let mandate_id = payment_attempt.mandate_id.clone();
    let refunds_response = if refunds.is_empty() {
        None
    } else {
        Some(refunds.into_iter().map(ForeignInto::foreign_into).collect())
    };

    Ok(match payment_request {
        Some(request) => {
            if payments::is_start_pay(&operation) && redirection_data.is_some() {
                let redirection_data = redirection_data.get_required_value("redirection_data")?;
                let form: RedirectForm = serde_json::from_value(redirection_data)
                    .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;
                services::BachResponse::Form(form)
            } else {
                let mut response: api::PaymentsResponse = request.into();
                let mut next_action_response = None;
                if payment_intent.status == enums::IntentStatus::RequiresCustomerAction {
                    next_action_response = Some(api::NextAction {
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
                        .set_status(payment_intent.status.foreign_into())
                        .set_amount(payment_attempt.amount)
                        .set_amount_capturable(None)
                        .set_amount_received(payment_intent.amount_captured)
                        .set_connector(payment_attempt.connector)
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
                            payment_attempt
                                .payment_method
                                .map(ForeignInto::foreign_into),
                            auth_flow == services::AuthFlow::Merchant,
                        )
                        .set_payment_method_data(
                            payment_method_data.map(api::PaymentMethodDataResponse::from),
                            auth_flow == services::AuthFlow::Merchant,
                        )
                        .set_payment_token(payment_attempt.payment_token)
                        .set_error_message(payment_attempt.error_message)
                        .set_shipping(address.shipping)
                        .set_billing(address.billing)
                        .to_owned()
                        .set_next_action(next_action_response)
                        .set_return_url(payment_intent.return_url)
                        .set_authentication_type(
                            payment_attempt
                                .authentication_type
                                .map(ForeignInto::foreign_into),
                        )
                        .set_statement_descriptor_name(payment_intent.statement_descriptor_name)
                        .set_statement_descriptor_suffix(payment_intent.statement_descriptor_suffix)
                        .set_setup_future_usage(
                            payment_intent
                                .setup_future_usage
                                .map(ForeignInto::foreign_into),
                        )
                        .set_capture_method(
                            payment_attempt
                                .capture_method
                                .map(ForeignInto::foreign_into),
                        )
                        .to_owned(),
                )
            }
        }
        None => services::BachResponse::Json(api::PaymentsResponse {
            payment_id: Some(payment_attempt.payment_id),
            merchant_id: Some(payment_attempt.merchant_id),
            status: payment_intent.status.foreign_into(),
            amount: payment_attempt.amount,
            amount_capturable: None,
            amount_received: payment_intent.amount_captured,
            client_secret: payment_intent.client_secret.map(masking::Secret::new),
            created: Some(payment_intent.created_at),
            currency,
            customer_id: payment_intent.customer_id,
            description: payment_intent.description,
            refunds: refunds_response,
            payment_method: payment_attempt
                .payment_method
                .map(ForeignInto::foreign_into),
            capture_method: payment_attempt
                .capture_method
                .map(ForeignInto::foreign_into),
            error_message: payment_attempt.error_message,
            error_code: payment_attempt.error_code,
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
            cancellation_reason: payment_attempt.cancellation_reason,
            payment_token: payment_attempt.payment_token,
            ..Default::default()
        }),
    })
}

impl<F: Clone> TryFrom<PaymentData<F>> for types::PaymentsAuthorizeData {
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

        let parsed_metadata: Option<api_models::payments::Metadata> = payment_data
            .payment_intent
            .metadata
            .map(|metadata_value| {
                metadata_value
                    .parse_value("metadata")
                    .change_context(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "metadata",
                    })
                    .attach_printable("unable to parse metadata")
            })
            .transpose()
            .unwrap_or_default();

        let order_details = parsed_metadata.map(|data| data.order_details);

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
            amount: payment_data.amount.into(),
            currency: payment_data.currency,
            browser_info,
            email: payment_data.email,
            order_details,
        })
    }
}

impl<F: Clone> TryFrom<PaymentData<F>> for types::PaymentsSyncData {
    type Error = errors::ApiErrorResponse;

    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            connector_transaction_id: match payment_data.payment_attempt.connector_transaction_id {
                Some(connector_txn_id) => {
                    types::ResponseId::ConnectorTransactionId(connector_txn_id)
                }
                None => types::ResponseId::NoResponseId,
            },
            encoded_data: payment_data.connector_response.encoded_data,
        })
    }
}

impl<F: Clone> TryFrom<PaymentData<F>> for types::PaymentsCaptureData {
    type Error = errors::ApiErrorResponse;

    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_to_capture: payment_data.payment_attempt.amount_to_capture,
            connector_transaction_id: payment_data
                .payment_attempt
                .connector_transaction_id
                .ok_or(errors::ApiErrorResponse::MerchantConnectorAccountNotFound)?,
            currency: payment_data.currency,
            amount: payment_data.amount.into(),
        })
    }
}

impl<F: Clone> TryFrom<PaymentData<F>> for types::PaymentsCancelData {
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

impl<F: Clone> TryFrom<PaymentData<F>> for types::PaymentsSessionData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        let parsed_metadata: Option<api_models::payments::Metadata> = payment_data
            .payment_intent
            .metadata
            .map(|metadata_value| {
                metadata_value
                    .parse_value("metadata")
                    .change_context(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "metadata",
                    })
                    .attach_printable("unable to parse metadata")
            })
            .transpose()
            .unwrap_or_default();

        let order_details = parsed_metadata.map(|data| data.order_details);

        Ok(Self {
            amount: payment_data.amount.into(),
            currency: payment_data.currency,
            country: payment_data
                .address
                .billing
                .and_then(|billing_address| billing_address.address.map(|address| address.country))
                .flatten(),
            order_details,
        })
    }
}

impl<F: Clone> TryFrom<PaymentData<F>> for types::VerifyRequestData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            confirm: true,
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
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            mandate_id: payment_data.mandate_id.clone(),
            setup_mandate_details: payment_data.setup_mandate,
        })
    }
}
