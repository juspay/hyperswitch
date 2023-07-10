use std::{fmt::Debug, marker::PhantomData};

use api_models::payments::OrderDetailsWithAmount;
use common_utils::fp_utils;
use error_stack::ResultExt;
use router_env::{instrument, tracing};
use storage_models::{ephemeral_key, payment_attempt::PaymentListFilters};

use super::{flows::Feature, PaymentAddress, PaymentData};
use crate::{
    configs::settings::Server,
    connector::{Nexinets, Paypal},
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments::{self, helpers},
    },
    routes::{metrics, AppState},
    services::{self, RedirectForm},
    types::{
        self, api, domain,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignInto},
    },
    utils::{self, OptionExt, ValueExt},
};

#[instrument(skip_all)]
pub async fn construct_payment_router_data<'a, F, T>(
    state: &'a AppState,
    payment_data: PaymentData<F>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
) -> RouterResult<types::RouterData<F, T, types::PaymentsResponseData>>
where
    T: TryFrom<PaymentAdditionalData<'a, F>>,
    types::RouterData<F, T, types::PaymentsResponseData>: Feature<F, T>,
    F: Clone,
    error_stack::Report<errors::ApiErrorResponse>:
        From<<T as TryFrom<PaymentAdditionalData<'a, F>>>::Error>,
{
    let (merchant_connector_account, payment_method, router_data);
    let connector_label = helpers::get_connector_label(
        payment_data.payment_intent.business_country,
        &payment_data.payment_intent.business_label,
        payment_data.payment_attempt.business_sub_label.as_ref(),
        connector_id,
    );

    merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
        &connector_label,
        payment_data.creds_identifier.to_owned(),
        key_store,
    )
    .await?;

    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

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
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
        });

    let additional_data = PaymentAdditionalData {
        router_base_url: state.conf.server.base_url.clone(),
        connector_name: connector_id.to_string(),
        payment_data: payment_data.clone(),
        state,
    };

    let customer_id = customer.to_owned().map(|customer| customer.customer_id);

    router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: merchant_account.merchant_id.clone(),
        customer_id,
        connector: connector_id.to_owned(),
        payment_id: payment_data.payment_attempt.payment_id.clone(),
        attempt_id: payment_data.payment_attempt.attempt_id.clone(),
        status: payment_data.payment_attempt.status,
        payment_method,
        connector_auth_type: auth_type,
        description: payment_data.payment_intent.description.clone(),
        return_url: payment_data.payment_intent.return_url.clone(),
        payment_method_id: payment_data.payment_attempt.payment_method_id.clone(),
        address: payment_data.address.clone(),
        auth_type: payment_data
            .payment_attempt
            .authentication_type
            .unwrap_or_default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        request: T::try_from(additional_data)?,
        response: response.map_or_else(|| Err(types::ErrorResponse::default()), Ok),
        amount_captured: payment_data.payment_intent.amount_captured,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: payment_data.pm_token,
        connector_customer: payment_data.connector_customer_id,
        preprocessing_id: payment_data.payment_attempt.preprocessing_step_id,
    };

    Ok(router_data)
}

pub trait ToResponse<Req, D, Op>
where
    Self: Sized,
    Op: Debug,
{
    fn generate_response(
        req: Option<Req>,
        data: D,
        customer: Option<domain::Customer>,
        auth_flow: services::AuthFlow,
        server: &Server,
        operation: Op,
    ) -> RouterResponse<Self>;
}

impl<F, Req, Op> ToResponse<Req, PaymentData<F>, Op> for api::PaymentsResponse
where
    F: Clone,
    Op: Debug,
{
    fn generate_response(
        req: Option<Req>,
        payment_data: PaymentData<F>,
        customer: Option<domain::Customer>,
        auth_flow: services::AuthFlow,
        server: &Server,
        operation: Op,
    ) -> RouterResponse<Self> {
        payments_to_payments_response(
            req,
            payment_data.payment_attempt,
            payment_data.payment_intent,
            payment_data.refunds,
            payment_data.disputes,
            payment_data.payment_method_data,
            customer,
            auth_flow,
            payment_data.address,
            server,
            payment_data.connector_response.authentication_data,
            &operation,
            payment_data.ephemeral_key,
            payment_data.sessions_token,
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
        _customer: Option<domain::Customer>,
        _auth_flow: services::AuthFlow,
        _server: &Server,
        _operation: Op,
    ) -> RouterResponse<Self> {
        Ok(services::ApplicationResponse::Json(Self {
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
        customer: Option<domain::Customer>,
        _auth_flow: services::AuthFlow,
        _server: &Server,
        _operation: Op,
    ) -> RouterResponse<Self> {
        Ok(services::ApplicationResponse::Json(Self {
            verify_id: Some(data.payment_intent.payment_id),
            merchant_id: Some(data.payment_intent.merchant_id),
            client_secret: data.payment_intent.client_secret.map(masking::Secret::new),
            customer_id: customer.as_ref().map(|x| x.customer_id.clone()),
            email: customer
                .as_ref()
                .and_then(|cus| cus.email.as_ref().map(|s| s.to_owned())),
            name: customer
                .as_ref()
                .and_then(|cus| cus.name.as_ref().map(|s| s.to_owned())),
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
            error_code: data.payment_attempt.error_code,
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
    disputes: Vec<storage::Dispute>,
    payment_method_data: Option<api::PaymentMethodData>,
    customer: Option<domain::Customer>,
    auth_flow: services::AuthFlow,
    address: PaymentAddress,
    server: &Server,
    redirection_data: Option<serde_json::Value>,
    operation: &Op,
    ephemeral_key_option: Option<ephemeral_key::EphemeralKey>,
    session_tokens: Vec<api::SessionToken>,
) -> RouterResponse<api::PaymentsResponse>
where
    Op: Debug,
{
    let currency = payment_attempt
        .currency
        .as_ref()
        .get_required_value("currency")?;
    let amount = utils::to_currency_base_unit(payment_attempt.amount, *currency).change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "amount",
        },
    )?;
    let mandate_id = payment_attempt.mandate_id.clone();
    let refunds_response = if refunds.is_empty() {
        None
    } else {
        Some(refunds.into_iter().map(ForeignInto::foreign_into).collect())
    };
    let disputes_response = if disputes.is_empty() {
        None
    } else {
        Some(
            disputes
                .into_iter()
                .map(ForeignInto::foreign_into)
                .collect(),
        )
    };
    let merchant_id = payment_attempt.merchant_id.to_owned();
    let payment_method_type = payment_attempt
        .payment_method_type
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or("".to_owned());
    let payment_method = payment_attempt
        .payment_method
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or("".to_owned());

    let output = Ok(match payment_request {
        Some(_request) => {
            if payments::is_start_pay(&operation) && redirection_data.is_some() {
                let redirection_data = redirection_data.get_required_value("redirection_data")?;
                let form: RedirectForm = serde_json::from_value(redirection_data)
                    .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;
                services::ApplicationResponse::Form(Box::new(services::RedirectionFormData {
                    redirect_form: form,
                    payment_method_data,
                    amount,
                    currency: currency.to_string(),
                }))
            } else {
                let mut next_action_response = None;

                let bank_transfer_next_steps =
                    bank_transfer_next_steps_check(payment_attempt.clone())?;

                let next_action_containing_qr_code =
                    qr_code_next_steps_check(payment_attempt.clone())?;

                if payment_intent.status == enums::IntentStatus::RequiresCustomerAction
                    || bank_transfer_next_steps.is_some()
                {
                    next_action_response = bank_transfer_next_steps
                        .map(|bank_transfer| {
                            api_models::payments::NextActionData::DisplayBankTransferInformation {
                                bank_transfer_steps_and_charges_details: bank_transfer,
                            }
                        })
                        .or(next_action_containing_qr_code.map(|qr_code_data| {
                            api_models::payments::NextActionData::QrCodeInformation {
                                image_data_url: qr_code_data.image_data_url,
                            }
                        }))
                        .or(Some(api_models::payments::NextActionData::RedirectToUrl {
                            redirect_to_url: helpers::create_startpay_url(
                                server,
                                &payment_attempt,
                                &payment_intent,
                            ),
                        }));
                };

                // next action check for third party sdk session (for ex: Apple pay through trustpay has third party sdk session response)
                if third_party_sdk_session_next_action(&payment_attempt, operation) {
                    next_action_response = Some(
                        api_models::payments::NextActionData::ThirdPartySdkSessionToken {
                            session_token: session_tokens.get(0).cloned(),
                        },
                    )
                }

                let mut response: api::PaymentsResponse = Default::default();
                let routed_through = payment_attempt.connector.clone();

                let connector_label = routed_through.as_ref().map(|connector_name| {
                    helpers::get_connector_label(
                        payment_intent.business_country,
                        &payment_intent.business_label,
                        payment_attempt.business_sub_label.as_ref(),
                        connector_name,
                    )
                });

                let amount_captured = payment_intent.amount_captured.unwrap_or_default();
                let amount_capturable = Some(payment_attempt.amount - amount_captured);
                services::ApplicationResponse::Json(
                    response
                        .set_payment_id(Some(payment_attempt.payment_id))
                        .set_merchant_id(Some(payment_attempt.merchant_id))
                        .set_status(payment_intent.status.foreign_into())
                        .set_amount(payment_attempt.amount)
                        .set_amount_capturable(amount_capturable)
                        .set_amount_received(payment_intent.amount_captured)
                        .set_connector(routed_through)
                        .set_client_secret(payment_intent.client_secret.map(masking::Secret::new))
                        .set_created(Some(payment_intent.created_at))
                        .set_currency(currency.to_string())
                        .set_customer_id(customer.as_ref().map(|cus| cus.clone().customer_id))
                        .set_email(
                            customer
                                .as_ref()
                                .and_then(|cus| cus.email.as_ref().map(|s| s.to_owned())),
                        )
                        .set_name(
                            customer
                                .as_ref()
                                .and_then(|cus| cus.name.as_ref().map(|s| s.to_owned())),
                        )
                        .set_phone(
                            customer
                                .as_ref()
                                .and_then(|cus| cus.phone.as_ref().map(|s| s.to_owned())),
                        )
                        .set_mandate_id(mandate_id)
                        .set_description(payment_intent.description)
                        .set_refunds(refunds_response) // refunds.iter().map(refund_to_refund_response),
                        .set_disputes(disputes_response)
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
                        .set_error_message(payment_attempt.error_reason)
                        .set_error_code(payment_attempt.error_code)
                        .set_shipping(address.shipping)
                        .set_billing(address.billing)
                        .set_next_action(next_action_response)
                        .set_return_url(payment_intent.return_url)
                        .set_cancellation_reason(payment_attempt.cancellation_reason)
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
                        .set_payment_experience(
                            payment_attempt
                                .payment_experience
                                .map(ForeignInto::foreign_into),
                        )
                        .set_payment_method_type(
                            payment_attempt
                                .payment_method_type
                                .map(ForeignInto::foreign_into),
                        )
                        .set_metadata(payment_intent.metadata)
                        .set_order_details(payment_intent.order_details)
                        .set_connector_label(connector_label)
                        .set_business_country(payment_intent.business_country)
                        .set_business_label(payment_intent.business_label)
                        .set_business_sub_label(payment_attempt.business_sub_label)
                        .set_allowed_payment_method_types(
                            payment_intent.allowed_payment_method_types,
                        )
                        .set_ephemeral_key(ephemeral_key_option.map(ForeignFrom::foreign_from))
                        .set_manual_retry_allowed(helpers::is_manual_retry_allowed(
                            &payment_intent.status,
                            &payment_attempt.status,
                        ))
                        .set_connector_transaction_id(payment_attempt.connector_transaction_id)
                        .set_feature_metadata(payment_intent.feature_metadata)
                        .set_connector_metadata(payment_intent.connector_metadata)
                        .to_owned(),
                )
            }
        }
        None => services::ApplicationResponse::Json(api::PaymentsResponse {
            payment_id: Some(payment_attempt.payment_id),
            merchant_id: Some(payment_attempt.merchant_id),
            status: payment_intent.status.foreign_into(),
            amount: payment_attempt.amount,
            amount_capturable: None,
            amount_received: payment_intent.amount_captured,
            client_secret: payment_intent.client_secret.map(masking::Secret::new),
            created: Some(payment_intent.created_at),
            currency: currency.to_string(),
            customer_id: payment_intent.customer_id,
            description: payment_intent.description,
            refunds: refunds_response,
            disputes: disputes_response,
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
                .and_then(|cus| cus.name.as_ref().map(|s| s.to_owned())),
            phone: customer
                .as_ref()
                .and_then(|cus| cus.phone.as_ref().map(|s| s.to_owned())),
            mandate_id,
            shipping: address.shipping,
            billing: address.billing,
            cancellation_reason: payment_attempt.cancellation_reason,
            payment_token: payment_attempt.payment_token,
            metadata: payment_intent.metadata,
            manual_retry_allowed: helpers::is_manual_retry_allowed(
                &payment_intent.status,
                &payment_attempt.status,
            ),
            order_details: payment_intent.order_details,
            connector_transaction_id: payment_attempt.connector_transaction_id,
            feature_metadata: payment_intent.feature_metadata,
            connector_metadata: payment_intent.connector_metadata,
            allowed_payment_method_types: payment_intent.allowed_payment_method_types,
            ..Default::default()
        }),
    });

    metrics::PAYMENT_OPS_COUNT.add(
        &metrics::CONTEXT,
        1,
        &[
            metrics::request::add_attributes("operation", format!("{:?}", operation)),
            metrics::request::add_attributes("merchant", merchant_id),
            metrics::request::add_attributes("payment_method_type", payment_method_type),
            metrics::request::add_attributes("payment_method", payment_method),
        ],
    );

    output
}

pub fn third_party_sdk_session_next_action<Op>(
    payment_attempt: &storage::PaymentAttempt,
    operation: &Op,
) -> bool
where
    Op: Debug,
{
    // If the operation is confirm, we will send session token response in next action
    if format!("{operation:?}").eq("PaymentConfirm") {
        payment_attempt
            .connector
            .as_ref()
            .map(|connector| matches!(connector.as_str(), "trustpay"))
            .and_then(|is_connector_supports_third_party_sdk| {
                if is_connector_supports_third_party_sdk {
                    payment_attempt
                        .payment_method
                        .map(|pm| matches!(pm, storage_models::enums::PaymentMethod::Wallet))
                } else {
                    Some(false)
                }
            })
            .unwrap_or(false)
    } else {
        false
    }
}

pub fn qr_code_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::QrCodeNextStepsInstruction>> {
    let qr_code_steps: Option<Result<api_models::payments::QrCodeNextStepsInstruction, _>> =
        payment_attempt
            .connector_metadata
            .map(|metadata| metadata.parse_value("QrCodeNextStepsInstruction"));

    let qr_code_instructions = qr_code_steps.transpose().ok().flatten();
    Ok(qr_code_instructions)
}

impl ForeignFrom<(storage::PaymentIntent, storage::PaymentAttempt)> for api::PaymentsResponse {
    fn foreign_from(item: (storage::PaymentIntent, storage::PaymentAttempt)) -> Self {
        let pi = item.0;
        let pa = item.1;
        Self {
            payment_id: Some(pi.payment_id),
            merchant_id: Some(pi.merchant_id),
            status: pi.status.foreign_into(),
            amount: pi.amount,
            amount_capturable: pi.amount_captured,
            client_secret: pi.client_secret.map(|s| s.into()),
            created: Some(pi.created_at),
            currency: pi.currency.map(|c| c.to_string()).unwrap_or_default(),
            description: pi.description,
            metadata: pi.metadata,
            order_details: pi.order_details,
            customer_id: pi.customer_id,
            connector: pa.connector,
            payment_method: pa.payment_method.map(ForeignInto::foreign_into),
            payment_method_type: pa.payment_method_type.map(ForeignInto::foreign_into),
            ..Default::default()
        }
    }
}

impl ForeignFrom<PaymentListFilters> for api_models::payments::PaymentListFilters {
    fn foreign_from(item: PaymentListFilters) -> Self {
        Self {
            connector: item.connector,
            currency: item
                .currency
                .into_iter()
                .map(ForeignInto::foreign_into)
                .collect(),
            status: item
                .status
                .into_iter()
                .map(ForeignInto::foreign_into)
                .collect(),
            payment_method: item
                .payment_method
                .into_iter()
                .map(ForeignInto::foreign_into)
                .collect(),
        }
    }
}

impl ForeignFrom<ephemeral_key::EphemeralKey> for api::ephemeral_key::EphemeralKeyCreateResponse {
    fn foreign_from(from: ephemeral_key::EphemeralKey) -> Self {
        Self {
            customer_id: from.customer_id,
            created_at: from.created_at,
            expires: from.expires,
            secret: from.secret,
        }
    }
}

pub fn bank_transfer_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::BankTransferNextStepsData>> {
    let bank_transfer_next_step = if let Some(storage_models::enums::PaymentMethod::BankTransfer) =
        payment_attempt.payment_method
    {
        let bank_transfer_next_steps: Option<api_models::payments::BankTransferNextStepsData> =
            payment_attempt
                .connector_metadata
                .map(|metadata| {
                    metadata
                        .parse_value("NextStepsRequirements")
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to parse the Value to NextRequirements struct")
                })
                .transpose()?;
        bank_transfer_next_steps
    } else {
        None
    };
    Ok(bank_transfer_next_step)
}

pub fn change_order_details_to_new_type(
    order_amount: i64,
    order_details: api_models::payments::OrderDetails,
) -> Option<Vec<OrderDetailsWithAmount>> {
    Some(vec![OrderDetailsWithAmount {
        product_name: order_details.product_name,
        quantity: order_details.quantity,
        amount: order_amount,
    }])
}

#[derive(Clone)]
pub struct PaymentAdditionalData<'a, F>
where
    F: Clone,
{
    router_base_url: String,
    connector_name: String,
    payment_data: PaymentData<F>,
    state: &'a AppState,
}
impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsAuthorizeData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data.clone();
        let router_base_url = &additional_data.router_base_url;
        let connector_name = &additional_data.connector_name;
        let attempt = &payment_data.payment_attempt;
        let browser_info: Option<types::BrowserInformation> = attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        let order_category = additional_data
            .payment_data
            .payment_intent
            .connector_metadata
            .map(|cm| {
                cm.parse_value::<api_models::payments::ConnectorMetadata>("ConnectorMetadata")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed parsing ConnectorMetadata")
            })
            .transpose()?
            .and_then(|cm| cm.noon.and_then(|noon| noon.order_category));

        let order_details = additional_data
            .payment_data
            .payment_intent
            .order_details
            .map(|order_details| {
                order_details
                    .iter()
                    .map(|data| {
                        data.to_owned()
                            .parse_value("OrderDetailsWithAmount")
                            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                                field_name: "OrderDetailsWithAmount",
                            })
                            .attach_printable("Unable to parse OrderDetailsWithAmount")
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;

        let complete_authorize_url = Some(helpers::create_complete_authorize_url(
            router_base_url,
            attempt,
            connector_name,
        ));

        let webhook_url = Some(helpers::create_webhook_url(
            router_base_url,
            &attempt.merchant_id,
            connector_name,
        ));
        let router_return_url = Some(helpers::create_redirect_url(
            router_base_url,
            attempt,
            connector_name,
            payment_data.creds_identifier.as_deref(),
        ));

        // payment_method_data is not required during recurring mandate payment, in such case keep default PaymentMethodData as MandatePayment
        let payment_method_data = payment_data.payment_method_data.or_else(|| {
            if payment_data.mandate_id.is_some() {
                Some(api_models::payments::PaymentMethodData::MandatePayment)
            } else {
                None
            }
        });
        Ok(Self {
            payment_method_data: payment_method_data.get_required_value("payment_method_data")?,
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            mandate_id: payment_data.mandate_id.clone(),
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            setup_mandate_details: payment_data.setup_mandate.clone(),
            confirm: payment_data.payment_attempt.confirm,
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            statement_descriptor: payment_data.payment_intent.statement_descriptor_name,
            capture_method: payment_data.payment_attempt.capture_method,
            amount: payment_data.amount.into(),
            currency: payment_data.currency,
            browser_info,
            email: payment_data.email,
            payment_experience: payment_data.payment_attempt.payment_experience,
            order_details,
            order_category,
            session_token: None,
            enrolled_for_3ds: true,
            related_transaction_id: None,
            payment_method_type: payment_data.payment_attempt.payment_method_type,
            router_return_url,
            webhook_url,
            complete_authorize_url,
            customer_id: None,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsSyncData {
    type Error = errors::ApiErrorResponse;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        Ok(Self {
            mandate_id: payment_data.mandate_id.clone(),
            connector_transaction_id: match payment_data.payment_attempt.connector_transaction_id {
                Some(connector_txn_id) => {
                    types::ResponseId::ConnectorTransactionId(connector_txn_id)
                }
                None => types::ResponseId::NoResponseId,
            },
            encoded_data: payment_data.connector_response.encoded_data,
            capture_method: payment_data.payment_attempt.capture_method,
            connector_meta: payment_data.payment_attempt.connector_metadata,
        })
    }
}

impl api::ConnectorTransactionId for Paypal {
    fn connector_transaction_id(
        &self,
        payment_attempt: storage::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        let payment_method = payment_attempt.payment_method;
        let metadata = Self::connector_transaction_id(
            self,
            payment_method,
            &payment_attempt.connector_metadata,
        );
        match metadata {
            Ok(data) => Ok(data),
            _ => Err(errors::ApiErrorResponse::ResourceIdNotFound),
        }
    }
}

impl api::ConnectorTransactionId for Nexinets {
    fn connector_transaction_id(
        &self,
        payment_attempt: storage::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        let metadata = Self::connector_transaction_id(self, &payment_attempt.connector_metadata);
        metadata.map_err(|_| errors::ApiErrorResponse::ResourceIdNotFound)
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsCaptureData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
        )?;
        let amount_to_capture: i64 = payment_data
            .payment_attempt
            .amount_to_capture
            .map_or(payment_data.amount.into(), |capture_amount| capture_amount);
        Ok(Self {
            amount_to_capture,
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_data.payment_attempt.clone())?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            payment_amount: payment_data.amount.into(),
            connector_meta: payment_data.payment_attempt.connector_metadata,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsCancelData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
        )?;
        Ok(Self {
            amount: Some(payment_data.amount.into()),
            currency: Some(payment_data.currency),
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_data.payment_attempt.clone())?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            cancellation_reason: payment_data.payment_attempt.cancellation_reason,
            connector_meta: payment_data.payment_attempt.connector_metadata,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsSessionData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data.clone();

        let order_details = additional_data
            .payment_data
            .payment_intent
            .order_details
            .map(|order_details| {
                order_details
                    .iter()
                    .map(|data| {
                        data.to_owned()
                            .parse_value("OrderDetailsWithAmount")
                            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                                field_name: "OrderDetailsWithAmount",
                            })
                            .attach_printable("Unable to parse OrderDetailsWithAmount")
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;

        Ok(Self {
            amount: payment_data.amount.into(),
            currency: payment_data.currency,
            country: payment_data.address.billing.and_then(|billing_address| {
                billing_address.address.and_then(|address| address.country)
            }),
            order_details,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::VerifyRequestData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let router_base_url = &additional_data.router_base_url;
        let connector_name = &additional_data.connector_name;
        let attempt = &payment_data.payment_attempt;
        let router_return_url = Some(helpers::create_redirect_url(
            router_base_url,
            attempt,
            connector_name,
            payment_data.creds_identifier.as_deref(),
        ));
        Ok(Self {
            currency: payment_data.currency,
            confirm: true,
            payment_method_data: payment_data
                .payment_method_data
                .get_required_value("payment_method_data")?,
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            mandate_id: payment_data.mandate_id.clone(),
            setup_mandate_details: payment_data.setup_mandate,
            router_return_url,
            email: payment_data.email,
            return_url: payment_data.payment_intent.return_url,
            payment_method_type: attempt.payment_method_type.clone(),
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::CompleteAuthorizeData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let browser_info: Option<types::BrowserInformation> = payment_data
            .payment_attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        let redirect_response = payment_data.redirect_response.map(|redirect| {
            types::CompleteAuthorizeRedirectResponse {
                params: redirect.param,
                payload: redirect.json_payload,
            }
        });

        Ok(Self {
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
            payment_method_data: payment_data.payment_method_data,
            connector_transaction_id: payment_data.connector_response.connector_transaction_id,
            redirect_response,
            connector_meta: payment_data.payment_attempt.connector_metadata,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsPreProcessingData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let payment_method_data = payment_data.payment_method_data;

        Ok(Self {
            payment_method_data,
            email: payment_data.email,
            currency: Some(payment_data.currency),
            amount: Some(payment_data.amount.into()),
            payment_method_type: payment_data.payment_attempt.payment_method_type,
        })
    }
}
