use std::{fmt::Debug, marker::PhantomData, str::FromStr};

use api_models::payments::{FrmMessage, RequestSurchargeDetails};
use common_utils::{consts::X_HS_LATENCY, fp_utils};
use diesel_models::ephemeral_key;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

use super::{flows::Feature, PaymentData};
use crate::{
    configs::settings::{ConnectorRequestReferenceIdConfig, Server},
    connector::{Helcim, Nexinets},
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments::{self, helpers},
        utils as core_utils,
    },
    routes::{metrics, AppState},
    services::{self, RedirectForm},
    types::{
        self, api, domain,
        storage::{self, enums},
        transformers::{ForeignFrom, ForeignInto, ForeignTryFrom},
        MultipleCaptureRequestData,
    },
    utils::{OptionExt, ValueExt},
};

#[instrument(skip_all)]
pub async fn construct_payment_router_data<'a, F, T>(
    state: &'a AppState,
    payment_data: PaymentData<F>,
    connector_id: &str,
    merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
) -> RouterResult<types::RouterData<F, T, types::PaymentsResponseData>>
where
    T: TryFrom<PaymentAdditionalData<'a, F>>,
    types::RouterData<F, T, types::PaymentsResponseData>: Feature<F, T>,
    F: Clone,
    error_stack::Report<errors::ApiErrorResponse>:
        From<<T as TryFrom<PaymentAdditionalData<'a, F>>>::Error>,
{
    let (payment_method, router_data);

    fp_utils::when(merchant_connector_account.is_disabled(), || {
        Err(errors::ApiErrorResponse::MerchantConnectorAccountDisabled)
    })?;

    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();

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

    let resource_id = match payment_data
        .payment_attempt
        .connector_transaction_id
        .clone()
    {
        Some(id) => types::ResponseId::ConnectorTransactionId(id),
        None => types::ResponseId::NoResponseId,
    };

    // [#44]: why should response be filled during request
    let response = Ok(types::PaymentsResponseData::TransactionResponse {
        resource_id,
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: None,
    });

    let additional_data = PaymentAdditionalData {
        router_base_url: state.conf.server.base_url.clone(),
        connector_name: connector_id.to_string(),
        payment_data: payment_data.clone(),
        state,
    };

    let customer_id = customer.to_owned().map(|customer| customer.customer_id);

    let supported_connector = &state
        .conf
        .multiple_api_version_supported_connectors
        .supported_connectors;
    let connector_enum = api_models::enums::Connector::from_str(connector_id)
        .into_report()
        .change_context(errors::ConnectorError::InvalidConnectorName)
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector",
        })
        .attach_printable_lazy(|| format!("unable to parse connector name {connector_id:?}"))?;

    let connector_api_version = if supported_connector.contains(&connector_enum) {
        state
            .store
            .find_config_by_key(&format!("connector_api_version_{connector_id}"))
            .await
            .map(|value| value.config)
            .ok()
    } else {
        None
    };

    let apple_pay_flow = payments::decide_apple_pay_flow(
        &payment_data.payment_attempt.payment_method_type,
        &Some(merchant_connector_account.clone()),
    );

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
        response,
        amount_captured: payment_data.payment_intent.amount_captured,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: payment_data.pm_token.map(types::PaymentMethodToken::Token),
        connector_customer: payment_data.connector_customer_id,
        recurring_mandate_payment_data: payment_data.recurring_mandate_payment_data,
        connector_request_reference_id: core_utils::get_connector_request_reference_id(
            &state.conf,
            &merchant_account.merchant_id,
            &payment_data.payment_attempt,
        ),
        preprocessing_id: payment_data.payment_attempt.preprocessing_step_id,
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        payment_method_balance: None,
        connector_api_version,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow,
    };

    Ok(router_data)
}

pub trait ToResponse<Req, D, Op>
where
    Self: Sized,
    Op: Debug,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        req: Option<Req>,
        data: D,
        customer: Option<domain::Customer>,
        auth_flow: services::AuthFlow,
        server: &Server,
        operation: Op,
        connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self>;
}

impl<F, Req, Op> ToResponse<Req, PaymentData<F>, Op> for api::PaymentsResponse
where
    F: Clone,
    Op: Debug,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        req: Option<Req>,
        payment_data: PaymentData<F>,
        customer: Option<domain::Customer>,
        auth_flow: services::AuthFlow,
        server: &Server,
        operation: Op,
        connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        connector_http_status_code: Option<u16>,
        external_latency: Option<u128>,
        is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self> {
        let captures =
            payment_data
                .multiple_capture_data
                .clone()
                .and_then(|multiple_capture_data| {
                    multiple_capture_data
                        .expand_captures
                        .and_then(|should_expand| {
                            should_expand.then_some(
                                multiple_capture_data
                                    .get_all_captures()
                                    .into_iter()
                                    .cloned()
                                    .collect(),
                            )
                        })
                });

        payments_to_payments_response(
            req,
            payment_data,
            captures,
            customer,
            auth_flow,
            server,
            &operation,
            connector_request_reference_id_config,
            connector_http_status_code,
            external_latency,
            is_latency_header_enabled,
        )
    }
}

impl<F, Req, Op> ToResponse<Req, PaymentData<F>, Op> for api::PaymentsSessionResponse
where
    Self: From<Req>,
    F: Clone,
    Op: Debug,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        _req: Option<Req>,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _auth_flow: services::AuthFlow,
        _server: &Server,
        _operation: Op,
        _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self> {
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
                session_token: payment_data.sessions_token,
                payment_id: payment_data.payment_attempt.payment_id,
                client_secret: payment_data
                    .payment_intent
                    .client_secret
                    .get_required_value("client_secret")?
                    .into(),
            },
            vec![],
        )))
    }
}

impl<F, Req, Op> ToResponse<Req, PaymentData<F>, Op> for api::VerifyResponse
where
    Self: From<Req>,
    F: Clone,
    Op: Debug,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
        _req: Option<Req>,
        data: PaymentData<F>,
        customer: Option<domain::Customer>,
        _auth_flow: services::AuthFlow,
        _server: &Server,
        _operation: Op,
        _connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
        _connector_http_status_code: Option<u16>,
        _external_latency: Option<u128>,
        _is_latency_header_enabled: Option<bool>,
    ) -> RouterResponse<Self> {
        let additional_payment_method_data: Option<api_models::payments::AdditionalPaymentData> =
            data.payment_attempt
                .payment_method_data
                .clone()
                .map(|data| data.parse_value("payment_method_data"))
                .transpose()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "payment_method_data",
                })?;
        let payment_method_data_response =
            additional_payment_method_data.map(api::PaymentMethodDataResponse::from);
        Ok(services::ApplicationResponse::JsonWithHeaders((
            Self {
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
                payment_method: data.payment_attempt.payment_method,
                payment_method_data: payment_method_data_response,
                payment_token: data.token,
                error_code: data.payment_attempt.error_code,
                error_message: data.payment_attempt.error_message,
            },
            vec![],
        )))
    }
}

#[instrument(skip_all)]
// try to use router data here so that already validated things , we don't want to repeat the validations.
// Add internal value not found and external value not found so that we can give 500 / Internal server error for internal value not found
#[allow(clippy::too_many_arguments)]
pub fn payments_to_payments_response<R, Op, F: Clone>(
    payment_request: Option<R>,
    payment_data: PaymentData<F>,
    captures: Option<Vec<storage::Capture>>,
    customer: Option<domain::Customer>,
    auth_flow: services::AuthFlow,
    server: &Server,
    operation: &Op,
    connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
    connector_http_status_code: Option<u16>,
    external_latency: Option<u128>,
    is_latency_header_enabled: Option<bool>,
) -> RouterResponse<api::PaymentsResponse>
where
    Op: Debug,
{
    let payment_attempt = payment_data.payment_attempt;
    let payment_intent = payment_data.payment_intent;
    let payment_link_data = payment_data.payment_link_data;

    let currency = payment_attempt
        .currency
        .as_ref()
        .get_required_value("currency")?;
    let amount = currency
        .to_currency_base_unit(payment_attempt.amount)
        .into_report()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "amount",
        })?;
    let mandate_id = payment_attempt.mandate_id.clone();
    let refunds_response = if payment_data.refunds.is_empty() {
        None
    } else {
        Some(
            payment_data
                .refunds
                .into_iter()
                .map(ForeignInto::foreign_into)
                .collect(),
        )
    };

    let disputes_response = if payment_data.disputes.is_empty() {
        None
    } else {
        Some(
            payment_data
                .disputes
                .into_iter()
                .map(ForeignInto::foreign_into)
                .collect(),
        )
    };

    let attempts_response = payment_data.attempts.map(|attempts| {
        attempts
            .into_iter()
            .map(ForeignInto::foreign_into)
            .collect()
    });

    let captures_response = captures.map(|captures| {
        captures
            .into_iter()
            .map(ForeignInto::foreign_into)
            .collect()
    });

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
    let additional_payment_method_data: Option<api_models::payments::AdditionalPaymentData> =
        payment_attempt
            .payment_method_data
            .clone()
            .map(|data| data.parse_value("payment_method_data"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "payment_method_data",
            })?;
    let surcharge_details =
        payment_attempt
            .surcharge_amount
            .map(|surcharge_amount| RequestSurchargeDetails {
                surcharge_amount,
                tax_amount: payment_attempt.tax_amount,
            });
    let merchant_decision = payment_intent.merchant_decision.to_owned();
    let frm_message = payment_data.frm_message.map(FrmMessage::foreign_from);

    let payment_method_data_response =
        additional_payment_method_data.map(api::PaymentMethodDataResponse::from);

    let mut headers = connector_http_status_code
        .map(|status_code| {
            vec![(
                "connector_http_status_code".to_string(),
                status_code.to_string(),
            )]
        })
        .unwrap_or_default();
    if let Some(payment_confirm_source) = payment_intent.payment_confirm_source {
        headers.push((
            "payment_confirm_source".to_string(),
            payment_confirm_source.to_string(),
        ))
    }
    if Some(true) == is_latency_header_enabled {
        headers.extend(
            external_latency
                .map(|latency| vec![(X_HS_LATENCY.to_string(), latency.to_string())])
                .unwrap_or_default(),
        );
    }
    let output = Ok(match payment_request {
        Some(_request) => {
            if payments::is_start_pay(&operation)
                && payment_data
                    .connector_response
                    .authentication_data
                    .is_some()
            {
                let redirection_data = payment_data
                    .connector_response
                    .authentication_data
                    .get_required_value("redirection_data")?;

                let form: RedirectForm = serde_json::from_value(redirection_data)
                    .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;

                services::ApplicationResponse::Form(Box::new(services::RedirectionFormData {
                    redirect_form: form,
                    payment_method_data: payment_data.payment_method_data,
                    amount,
                    currency: currency.to_string(),
                }))
            } else {
                let mut next_action_response = None;

                let bank_transfer_next_steps =
                    bank_transfer_next_steps_check(payment_attempt.clone())?;

                let next_action_voucher = voucher_next_steps_check(payment_attempt.clone())?;

                let next_action_containing_qr_code_url =
                    qr_code_next_steps_check(payment_attempt.clone())?;

                let next_action_containing_wait_screen =
                    wait_screen_next_steps_check(payment_attempt.clone())?;

                if payment_intent.status == enums::IntentStatus::RequiresCustomerAction
                    || bank_transfer_next_steps.is_some()
                    || next_action_voucher.is_some()
                    || next_action_containing_qr_code_url.is_some()
                    || next_action_containing_wait_screen.is_some()
                {
                    next_action_response = bank_transfer_next_steps
                        .map(|bank_transfer| {
                            api_models::payments::NextActionData::DisplayBankTransferInformation {
                                bank_transfer_steps_and_charges_details: bank_transfer,
                            }
                        })
                        .or(next_action_voucher.map(|voucher_data| {
                            api_models::payments::NextActionData::DisplayVoucherInformation {
                                voucher_details: voucher_data,
                            }
                        }))
                        .or(next_action_containing_qr_code_url.map(|qr_code_data| {
                            api_models::payments::NextActionData::QrCodeInformation {
                                image_data_url: qr_code_data.image_data_url,
                                display_to_timestamp: qr_code_data.display_to_timestamp,
                            }
                        }))
                        .or(next_action_containing_wait_screen.map(|wait_screen_data| {
                            api_models::payments::NextActionData::WaitScreenInformation {
                                display_from_timestamp: wait_screen_data.display_from_timestamp,
                                display_to_timestamp: wait_screen_data.display_to_timestamp,
                            }
                        }))
                        .or(payment_data
                            .connector_response
                            .authentication_data
                            .map(|_| api_models::payments::NextActionData::RedirectToUrl {
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
                            session_token: payment_data.sessions_token.get(0).cloned(),
                        },
                    )
                }

                let mut response: api::PaymentsResponse = Default::default();
                let routed_through = payment_attempt.connector.clone();

                let connector_label = routed_through.as_ref().and_then(|connector_name| {
                    core_utils::get_connector_label(
                        payment_intent.business_country,
                        payment_intent.business_label.as_ref(),
                        payment_attempt.business_sub_label.as_ref(),
                        connector_name,
                    )
                });
                services::ApplicationResponse::JsonWithHeaders((
                    response
                        .set_payment_id(Some(payment_attempt.payment_id))
                        .set_merchant_id(Some(payment_attempt.merchant_id))
                        .set_status(payment_intent.status)
                        .set_amount(payment_attempt.amount)
                        .set_amount_capturable(Some(payment_attempt.amount_capturable))
                        .set_amount_received(payment_intent.amount_captured)
                        .set_surcharge_details(surcharge_details)
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
                        .set_mandate_data(
                            payment_data.setup_mandate.map(|d| api::MandateData {
                                customer_acceptance: d.customer_acceptance.map(|d| {
                                    api::CustomerAcceptance {
                                        acceptance_type: match d.acceptance_type {
                                            data_models::mandates::AcceptanceType::Online => {
                                                api::AcceptanceType::Online
                                            }
                                            data_models::mandates::AcceptanceType::Offline => {
                                                api::AcceptanceType::Offline
                                            }
                                        },
                                        accepted_at: d.accepted_at,
                                        online: d.online.map(|d| api::OnlineMandate {
                                            ip_address: d.ip_address,
                                            user_agent: d.user_agent,
                                        }),
                                    }
                                }),
                                mandate_type: d.mandate_type.map(|d| match d {
                                    data_models::mandates::MandateDataType::MultiUse(Some(i)) => {
                                        api::MandateType::MultiUse(Some(api::MandateAmountData {
                                            amount: i.amount,
                                            currency: i.currency,
                                            start_date: i.start_date,
                                            end_date: i.end_date,
                                            metadata: i.metadata,
                                        }))
                                    }
                                    data_models::mandates::MandateDataType::SingleUse(i) => {
                                        api::MandateType::SingleUse(
                                            api::payments::MandateAmountData {
                                                amount: i.amount,
                                                currency: i.currency,
                                                start_date: i.start_date,
                                                end_date: i.end_date,
                                                metadata: i.metadata,
                                            },
                                        )
                                    }
                                    data_models::mandates::MandateDataType::MultiUse(None) => {
                                        api::MandateType::MultiUse(None)
                                    }
                                }),
                            }),
                            auth_flow == services::AuthFlow::Merchant,
                        )
                        .set_description(payment_intent.description)
                        .set_refunds(refunds_response) // refunds.iter().map(refund_to_refund_response),
                        .set_disputes(disputes_response)
                        .set_attempts(attempts_response)
                        .set_captures(captures_response)
                        .set_payment_method(
                            payment_attempt.payment_method,
                            auth_flow == services::AuthFlow::Merchant,
                        )
                        .set_payment_method_data(
                            payment_method_data_response,
                            auth_flow == services::AuthFlow::Merchant,
                        )
                        .set_payment_token(payment_attempt.payment_token)
                        .set_error_message(
                            payment_attempt
                                .error_reason
                                .or(payment_attempt.error_message),
                        )
                        .set_error_code(payment_attempt.error_code)
                        .set_shipping(payment_data.address.shipping)
                        .set_billing(payment_data.address.billing)
                        .set_next_action(next_action_response)
                        .set_return_url(payment_intent.return_url)
                        .set_cancellation_reason(payment_attempt.cancellation_reason)
                        .set_authentication_type(payment_attempt.authentication_type)
                        .set_statement_descriptor_name(payment_intent.statement_descriptor_name)
                        .set_statement_descriptor_suffix(payment_intent.statement_descriptor_suffix)
                        .set_setup_future_usage(payment_intent.setup_future_usage)
                        .set_capture_method(payment_attempt.capture_method)
                        .set_payment_experience(payment_attempt.payment_experience)
                        .set_payment_method_type(payment_attempt.payment_method_type)
                        .set_metadata(payment_intent.metadata)
                        .set_order_details(payment_intent.order_details)
                        .set_connector_label(connector_label)
                        .set_business_country(payment_intent.business_country)
                        .set_business_label(payment_intent.business_label)
                        .set_business_sub_label(payment_attempt.business_sub_label)
                        .set_allowed_payment_method_types(
                            payment_intent.allowed_payment_method_types,
                        )
                        .set_ephemeral_key(
                            payment_data.ephemeral_key.map(ForeignFrom::foreign_from),
                        )
                        .set_frm_message(frm_message)
                        .set_merchant_decision(merchant_decision)
                        .set_manual_retry_allowed(helpers::is_manual_retry_allowed(
                            &payment_intent.status,
                            &payment_attempt.status,
                            connector_request_reference_id_config,
                            &merchant_id,
                        ))
                        .set_connector_transaction_id(payment_attempt.connector_transaction_id)
                        .set_feature_metadata(payment_intent.feature_metadata)
                        .set_connector_metadata(payment_intent.connector_metadata)
                        .set_reference_id(payment_attempt.connector_response_reference_id)
                        .set_payment_link(payment_link_data)
                        .set_profile_id(payment_intent.profile_id)
                        .set_attempt_count(payment_intent.attempt_count)
                        .set_merchant_connector_id(payment_attempt.merchant_connector_id)
                        .to_owned(),
                    headers,
                ))
            }
        }
        None => services::ApplicationResponse::JsonWithHeaders((
            api::PaymentsResponse {
                payment_id: Some(payment_attempt.payment_id),
                merchant_id: Some(payment_attempt.merchant_id),
                status: payment_intent.status,
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
                attempts: attempts_response,
                captures: captures_response,
                payment_method: payment_attempt.payment_method,
                capture_method: payment_attempt.capture_method,
                error_message: payment_attempt
                    .error_reason
                    .or(payment_attempt.error_message),
                error_code: payment_attempt.error_code,
                payment_method_data: payment_method_data_response,
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
                shipping: payment_data.address.shipping,
                billing: payment_data.address.billing,
                cancellation_reason: payment_attempt.cancellation_reason,
                payment_token: payment_attempt.payment_token,
                metadata: payment_intent.metadata,
                manual_retry_allowed: helpers::is_manual_retry_allowed(
                    &payment_intent.status,
                    &payment_attempt.status,
                    connector_request_reference_id_config,
                    &merchant_id,
                ),
                order_details: payment_intent.order_details,
                frm_message,
                connector_transaction_id: payment_attempt.connector_transaction_id,
                feature_metadata: payment_intent.feature_metadata,
                connector_metadata: payment_intent.connector_metadata,
                allowed_payment_method_types: payment_intent.allowed_payment_method_types,
                reference_id: payment_attempt.connector_response_reference_id,
                attempt_count: payment_intent.attempt_count,
                payment_link: payment_link_data,
                surcharge_details,
                ..Default::default()
            },
            headers,
        )),
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
            .map(|connector| {
                matches!(connector.as_str(), "trustpay") || matches!(connector.as_str(), "payme")
            })
            .and_then(|is_connector_supports_third_party_sdk| {
                if is_connector_supports_third_party_sdk {
                    payment_attempt
                        .payment_method
                        .map(|pm| matches!(pm, diesel_models::enums::PaymentMethod::Wallet))
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

pub fn wait_screen_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::WaitScreenInstructions>> {
    let display_info_with_timer_steps: Option<
        Result<api_models::payments::WaitScreenInstructions, _>,
    > = payment_attempt
        .connector_metadata
        .map(|metadata| metadata.parse_value("WaitScreenInstructions"));

    let display_info_with_timer_instructions =
        display_info_with_timer_steps.transpose().ok().flatten();
    Ok(display_info_with_timer_instructions)
}

impl ForeignFrom<(storage::PaymentIntent, storage::PaymentAttempt)> for api::PaymentsResponse {
    fn foreign_from(item: (storage::PaymentIntent, storage::PaymentAttempt)) -> Self {
        let pi = item.0;
        let pa = item.1;
        Self {
            payment_id: Some(pi.payment_id),
            merchant_id: Some(pi.merchant_id),
            status: pi.status,
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
            payment_method: pa.payment_method,
            payment_method_type: pa.payment_method_type,
            business_label: pi.business_label,
            business_country: pi.business_country,
            business_sub_label: pa.business_sub_label,
            setup_future_usage: pi.setup_future_usage,
            capture_method: pa.capture_method,
            authentication_type: pa.authentication_type,
            connector_transaction_id: pa.connector_transaction_id,
            attempt_count: pi.attempt_count,
            ..Default::default()
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
    let bank_transfer_next_step = if let Some(diesel_models::enums::PaymentMethod::BankTransfer) =
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

pub fn voucher_next_steps_check(
    payment_attempt: storage::PaymentAttempt,
) -> RouterResult<Option<api_models::payments::VoucherNextStepData>> {
    let voucher_next_step = if let Some(diesel_models::enums::PaymentMethod::Voucher) =
        payment_attempt.payment_method
    {
        let voucher_next_steps: Option<api_models::payments::VoucherNextStepData> = payment_attempt
            .connector_metadata
            .map(|metadata| {
                metadata
                    .parse_value("NextStepsRequirements")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse the Value to NextRequirements struct")
            })
            .transpose()?;
        voucher_next_steps
    } else {
        None
    };
    Ok(voucher_next_step)
}

pub fn change_order_details_to_new_type(
    order_amount: i64,
    order_details: api_models::payments::OrderDetails,
) -> Option<Vec<api_models::payments::OrderDetailsWithAmount>> {
    Some(vec![api_models::payments::OrderDetailsWithAmount {
        product_name: order_details.product_name,
        quantity: order_details.quantity,
        amount: order_amount,
        product_img_link: order_details.product_img_link,
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
            surcharge_details: payment_data.surcharge_details,
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
            sync_type: match payment_data.multiple_capture_data {
                Some(multiple_capture_data) => types::SyncRequestType::MultipleCaptureSync(
                    multiple_capture_data.get_pending_connector_capture_ids(),
                ),
                None => types::SyncRequestType::SinglePaymentSync,
            },
        })
    }
}

impl api::ConnectorTransactionId for Helcim {
    fn connector_transaction_id(
        &self,
        payment_attempt: storage::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        if payment_attempt.connector_transaction_id.is_none() {
            let metadata =
                Self::connector_transaction_id(self, &payment_attempt.connector_metadata);
            metadata.map_err(|_| errors::ApiErrorResponse::ResourceIdNotFound)
        } else {
            Ok(payment_attempt.connector_transaction_id)
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
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        let amount_to_capture: i64 = payment_data
            .payment_attempt
            .amount_to_capture
            .map_or(payment_data.amount.into(), |capture_amount| capture_amount);
        let browser_info: Option<types::BrowserInformation> = payment_data
            .payment_attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        Ok(Self {
            amount_to_capture,
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_data.payment_attempt.clone())?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            payment_amount: payment_data.amount.into(),
            connector_meta: payment_data.payment_attempt.connector_metadata,
            multiple_capture_data: match payment_data.multiple_capture_data {
                Some(multiple_capture_data) => Some(MultipleCaptureRequestData {
                    capture_sequence: multiple_capture_data.get_captures_count()?,
                    capture_reference: multiple_capture_data
                        .get_latest_capture()
                        .capture_id
                        .clone(),
                }),
                None => None,
            },
            browser_info,
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
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        let browser_info: Option<types::BrowserInformation> = payment_data
            .payment_attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;
        Ok(Self {
            amount: Some(payment_data.amount.into()),
            currency: Some(payment_data.currency),
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_data.payment_attempt.clone())?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
            cancellation_reason: payment_data.payment_attempt.cancellation_reason,
            connector_meta: payment_data.payment_attempt.connector_metadata,
            browser_info,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsApproveData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        Ok(Self {
            amount: Some(payment_data.amount.into()),
            currency: Some(payment_data.currency),
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::PaymentsRejectData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        Ok(Self {
            amount: Some(payment_data.amount.into()),
            currency: Some(payment_data.currency),
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
            surcharge_details: payment_data.surcharge_details,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>> for types::SetupMandateRequestData {
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
        let browser_info: Option<types::BrowserInformation> = attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;
        Ok(Self {
            currency: payment_data.currency,
            confirm: true,
            amount: Some(payment_data.amount.into()),
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
            browser_info,
            payment_method_type: attempt.payment_method_type,
        })
    }
}

impl TryFrom<types::CaptureSyncResponse> for storage::CaptureUpdate {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(capture_sync_response: types::CaptureSyncResponse) -> Result<Self, Self::Error> {
        match capture_sync_response {
            types::CaptureSyncResponse::Success {
                resource_id,
                status,
                connector_response_reference_id,
                ..
            } => {
                let connector_capture_id = match resource_id {
                    types::ResponseId::ConnectorTransactionId(id) => Some(id),
                    types::ResponseId::EncodedData(_) | types::ResponseId::NoResponseId => None,
                };
                Ok(Self::ResponseUpdate {
                    status: enums::CaptureStatus::foreign_try_from(status)?,
                    connector_capture_id,
                    connector_response_reference_id,
                })
            }
            types::CaptureSyncResponse::Error {
                code,
                message,
                reason,
                status_code,
                ..
            } => Ok(Self::ErrorUpdate {
                status: match status_code {
                    500..=511 => storage::enums::CaptureStatus::Pending,
                    _ => storage::enums::CaptureStatus::Failed,
                },
                error_code: Some(code),
                error_message: Some(message),
                error_reason: reason,
            }),
        }
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
        let router_base_url = &additional_data.router_base_url;
        let attempt = &payment_data.payment_attempt;
        let connector_name = &additional_data.connector_name;

        let order_details = payment_data
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
        let complete_authorize_url = Some(helpers::create_complete_authorize_url(
            router_base_url,
            attempt,
            connector_name,
        ));
        let browser_info: Option<types::BrowserInformation> = payment_data
            .payment_attempt
            .browser_info
            .clone()
            .map(|b| b.parse_value("BrowserInformation"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        Ok(Self {
            payment_method_data,
            email: payment_data.email,
            currency: Some(payment_data.currency),
            amount: Some(payment_data.amount.into()),
            payment_method_type: payment_data.payment_attempt.payment_method_type,
            setup_mandate_details: payment_data.setup_mandate,
            capture_method: payment_data.payment_attempt.capture_method,
            order_details,
            router_return_url,
            webhook_url,
            complete_authorize_url,
            browser_info,
            surcharge_details: payment_data.surcharge_details,
        })
    }
}

impl ForeignFrom<payments::FraudCheck> for FrmMessage {
    fn foreign_from(fraud_check: payments::FraudCheck) -> Self {
        Self {
            frm_name: fraud_check.frm_name,
            frm_transaction_id: fraud_check.frm_transaction_id,
            frm_transaction_type: Some(fraud_check.frm_transaction_type.to_string()),
            frm_status: Some(fraud_check.frm_status.to_string()),
            frm_score: fraud_check.frm_score,
            frm_reason: fraud_check.frm_reason,
            frm_error: fraud_check.frm_error,
        }
    }
}
