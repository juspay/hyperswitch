use std::{fmt::Debug, marker::PhantomData, str::FromStr};

use api_models::payments::{FrmMessage, GetAddressFromPaymentMethodData, RequestSurchargeDetails};
#[cfg(feature = "payouts")]
use api_models::payouts::PayoutAttemptResponse;
use common_enums::RequestIncrementalAuthorization;
use common_utils::{consts::X_HS_LATENCY, fp_utils};
use diesel_models::ephemeral_key;
use error_stack::{report, ResultExt};
use masking::Maskable;
use router_env::{instrument, tracing};

use super::{flows::Feature, types::AuthenticationData, PaymentData};
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
    customer: &'a Option<domain::Customer>,
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

    let test_mode = merchant_connector_account.is_test_mode_on();

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
        incremental_authorization_allowed: None,
    });

    let additional_data = PaymentAdditionalData {
        router_base_url: state.conf.server.base_url.clone(),
        connector_name: connector_id.to_string(),
        payment_data: payment_data.clone(),
        state,
        customer_data: customer,
    };

    let customer_id = customer.to_owned().map(|customer| customer.customer_id);

    let supported_connector = &state
        .conf
        .multiple_api_version_supported_connectors
        .supported_connectors;
    let connector_enum = api_models::enums::Connector::from_str(connector_id)
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
        Some(merchant_connector_account),
    );

    let payment_method_data_billing = payment_data
        .payment_method_data
        .as_ref()
        .and_then(|payment_method_data| payment_method_data.get_billing_address());

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
        address: payment_data
            .address
            .unify_with_payment_method_data_billing(payment_method_data_billing),
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
        payment_method_status: payment_data.payment_method_info.map(|info| info.status),
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
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        connector_response: None,
    };

    Ok(router_data)
}

pub trait ToResponse<D, Op>
where
    Self: Sized,
    Op: Debug,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
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

impl<F, Op> ToResponse<PaymentData<F>, Op> for api::PaymentsResponse
where
    F: Clone,
    Op: Debug,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
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

impl<F, Op> ToResponse<PaymentData<F>, Op> for api::PaymentsSessionResponse
where
    F: Clone,
    Op: Debug,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
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

impl<F, Op> ToResponse<PaymentData<F>, Op> for api::VerifyResponse
where
    F: Clone,
    Op: Debug,
{
    #[allow(clippy::too_many_arguments)]
    fn generate_response(
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
                mandate_id: data
                    .mandate_id
                    .and_then(|mandate_ids| mandate_ids.mandate_id),
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
pub fn payments_to_payments_response<Op, F: Clone>(
    payment_data: PaymentData<F>,
    captures: Option<Vec<storage::Capture>>,
    customer: Option<domain::Customer>,
    auth_flow: services::AuthFlow,
    server: &Server,
    operation: &Op,
    connector_request_reference_id_config: &ConnectorRequestReferenceIdConfig,
    connector_http_status_code: Option<u16>,
    external_latency: Option<u128>,
    _is_latency_header_enabled: Option<bool>,
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

    let incremental_authorizations_response = if payment_data.authorizations.is_empty() {
        None
    } else {
        Some(
            payment_data
                .authorizations
                .into_iter()
                .map(ForeignInto::foreign_into)
                .collect(),
        )
    };

    let external_authentication_details = payment_data
        .authentication
        .as_ref()
        .map(ForeignInto::foreign_into);

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

    let payment_method_data =
        additional_payment_method_data.map(api::PaymentMethodDataResponse::from);

    let payment_method_data_response = payment_method_data.map(|payment_method_data| {
        api_models::payments::PaymentMethodDataResponseWithBilling {
            payment_method_data,
            billing: payment_data
                .address
                .get_request_payment_method_billing()
                .cloned(),
        }
    });

    let mut headers = connector_http_status_code
        .map(|status_code| {
            vec![(
                "connector_http_status_code".to_string(),
                Maskable::new_normal(status_code.to_string()),
            )]
        })
        .unwrap_or_default();
    if let Some(payment_confirm_source) = payment_intent.payment_confirm_source {
        headers.push((
            "payment_confirm_source".to_string(),
            Maskable::new_normal(payment_confirm_source.to_string()),
        ))
    }

    let customer_details_response = customer.as_ref().map(ForeignInto::foreign_into);

    headers.extend(
        external_latency
            .map(|latency| {
                vec![(
                    X_HS_LATENCY.to_string(),
                    Maskable::new_normal(latency.to_string()),
                )]
            })
            .unwrap_or_default(),
    );

    let output = if payments::is_start_pay(&operation)
        && payment_attempt.authentication_data.is_some()
    {
        let redirection_data = payment_attempt
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

        let bank_transfer_next_steps = bank_transfer_next_steps_check(payment_attempt.clone())?;

        let next_action_voucher = voucher_next_steps_check(payment_attempt.clone())?;

        let next_action_containing_qr_code_url = qr_code_next_steps_check(payment_attempt.clone())?;

        let next_action_containing_wait_screen =
            wait_screen_next_steps_check(payment_attempt.clone())?;

        if payment_intent.status == enums::IntentStatus::RequiresCustomerAction
            || bank_transfer_next_steps.is_some()
            || next_action_voucher.is_some()
            || next_action_containing_qr_code_url.is_some()
            || next_action_containing_wait_screen.is_some()
            || payment_data.authentication.is_some()
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
                            api_models::payments::NextActionData::foreign_from(qr_code_data)
                        }))
                        .or(next_action_containing_wait_screen.map(|wait_screen_data| {
                            api_models::payments::NextActionData::WaitScreenInformation {
                                display_from_timestamp: wait_screen_data.display_from_timestamp,
                                display_to_timestamp: wait_screen_data.display_to_timestamp,
                            }
                        }))
                        .or(payment_attempt.authentication_data.as_ref().map(|_| {
                            api_models::payments::NextActionData::RedirectToUrl {
                                redirect_to_url: helpers::create_startpay_url(
                                    server,
                                    &payment_attempt,
                                    &payment_intent,
                                ),
                            }
                        }))
                        .or(match payment_data.authentication.as_ref(){
                            Some(authentication) => {
                                if payment_intent.status == common_enums::IntentStatus::RequiresCustomerAction && authentication.cavv.is_none() && authentication.is_separate_authn_required(){
                                    // if preAuthn and separate authentication needed.
                                    let poll_config = payment_data.poll_config.unwrap_or_default();
                                    let request_poll_id = core_utils::get_external_authentication_request_poll_id(&payment_intent.payment_id);
                                    let payment_connector_name = payment_attempt.connector
                                        .as_ref()
                                        .get_required_value("connector")?;
                                    Some(api_models::payments::NextActionData::ThreeDsInvoke {
                                        three_ds_data: api_models::payments::ThreeDsData {
                                            three_ds_authentication_url: helpers::create_authentication_url(&server.base_url, &payment_attempt),
                                            three_ds_authorize_url: helpers::create_authorize_url(
                                                &server.base_url,
                                                &payment_attempt,
                                                payment_connector_name,
                                            ),
                                            three_ds_method_details: authentication.three_ds_method_url.as_ref().zip(authentication.three_ds_method_data.as_ref()).map(|(three_ds_method_url,three_ds_method_data )|{
                                                api_models::payments::ThreeDsMethodData::AcsThreeDsMethodData {
                                                    three_ds_method_data_submission: true,
                                                    three_ds_method_data: Some(three_ds_method_data.clone()),
                                                    three_ds_method_url: Some(three_ds_method_url.to_owned()),
                                                }
                                            }).unwrap_or(api_models::payments::ThreeDsMethodData::AcsThreeDsMethodData {
                                                    three_ds_method_data_submission: false,
                                                    three_ds_method_data: None,
                                                    three_ds_method_url: None,
                                            }),
                                            poll_config: api_models::payments::PollConfigResponse {poll_id: request_poll_id, delay_in_secs: poll_config.delay_in_secs, frequency: poll_config.frequency},
                                        },
                                    })
                                }else{
                                    None
                                }
                            },
                            None => None
                        });
        };

        // next action check for third party sdk session (for ex: Apple pay through trustpay has third party sdk session response)
        if third_party_sdk_session_next_action(&payment_attempt, operation) {
            next_action_response = Some(
                api_models::payments::NextActionData::ThirdPartySdkSessionToken {
                    session_token: payment_data.sessions_token.first().cloned(),
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
                .set_net_amount(payment_attempt.net_amount)
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
                                api::MandateType::SingleUse(api::payments::MandateAmountData {
                                    amount: i.amount,
                                    currency: i.currency,
                                    start_date: i.start_date,
                                    end_date: i.end_date,
                                    metadata: i.metadata,
                                })
                            }
                            data_models::mandates::MandateDataType::MultiUse(None) => {
                                api::MandateType::MultiUse(None)
                            }
                        }),
                        update_mandate_id: d.update_mandate_id,
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
                .set_shipping(payment_data.address.get_shipping().cloned())
                .set_billing(payment_data.address.get_payment_billing().cloned())
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
                .set_allowed_payment_method_types(payment_intent.allowed_payment_method_types)
                .set_ephemeral_key(payment_data.ephemeral_key.map(ForeignFrom::foreign_from))
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
                .set_unified_code(payment_attempt.unified_code)
                .set_unified_message(payment_attempt.unified_message)
                .set_incremental_authorization_allowed(
                    payment_intent.incremental_authorization_allowed,
                )
                .set_external_authentication_details(external_authentication_details)
                .set_fingerprint(payment_intent.fingerprint_id)
                .set_authorization_count(payment_intent.authorization_count)
                .set_incremental_authorizations(incremental_authorizations_response)
                .set_expires_on(payment_intent.session_expiry)
                .set_external_3ds_authentication_attempted(
                    payment_attempt.external_three_ds_authentication_attempted,
                )
                .set_payment_method_id(payment_attempt.payment_method_id)
                .set_payment_method_status(payment_data.payment_method_info.map(|info| info.status))
                .set_customer(customer_details_response.clone())
                .set_browser_info(payment_attempt.browser_info)
                .set_updated(Some(payment_intent.modified_at))
                .to_owned(),
            headers,
        ))
    };

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

    Ok(output)
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
) -> RouterResult<Option<api_models::payments::QrCodeInformation>> {
    let qr_code_steps: Option<Result<api_models::payments::QrCodeInformation, _>> = payment_attempt
        .connector_metadata
        .map(|metadata| metadata.parse_value("QrCodeInformation"));

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
            profile_id: pi.profile_id,
            merchant_connector_id: pa.merchant_connector_id,
            ..Default::default()
        }
    }
}

#[cfg(feature = "payouts")]
impl ForeignFrom<(storage::Payouts, storage::PayoutAttempt, domain::Customer)>
    for api::PayoutCreateResponse
{
    fn foreign_from(item: (storage::Payouts, storage::PayoutAttempt, domain::Customer)) -> Self {
        let (payout, payout_attempt, customer) = item;
        let attempt = PayoutAttemptResponse {
            attempt_id: payout_attempt.payout_attempt_id,
            status: payout_attempt.status,
            amount: payout.amount,
            currency: Some(payout.destination_currency),
            connector: payout_attempt.connector.clone(),
            error_code: payout_attempt.error_code.clone(),
            error_message: payout_attempt.error_message.clone(),
            payment_method: Some(payout.payout_type),
            payout_method_type: None,
            connector_transaction_id: Some(payout_attempt.connector_payout_id),
            cancellation_reason: None,
            unified_code: None,
            unified_message: None,
        };
        let attempts = vec![attempt];
        Self {
            payout_id: payout.payout_id,
            merchant_id: payout.merchant_id,
            amount: payout.amount,
            currency: payout.destination_currency,
            connector: payout_attempt.connector,
            payout_type: payout.payout_type,
            customer_id: customer.customer_id,
            auto_fulfill: payout.auto_fulfill,
            email: customer.email,
            name: customer.name,
            phone: customer.phone,
            phone_country_code: customer.phone_country_code,
            return_url: payout.return_url,
            business_country: payout_attempt.business_country,
            business_label: payout_attempt.business_label,
            description: payout.description,
            entity_type: payout.entity_type,
            recurring: payout.recurring,
            metadata: payout.metadata,
            status: payout_attempt.status,
            error_message: payout_attempt.error_message,
            error_code: payout_attempt.error_code,
            profile_id: payout.profile_id,
            created: Some(payout.created_at),
            attempts: Some(attempts),
            billing: None,
            client_secret: None,
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
        if payment_attempt.payment_method_type != Some(diesel_models::enums::PaymentMethodType::Pix)
        {
            let bank_transfer_next_steps: Option<api_models::payments::BankTransferNextStepsData> =
                payment_attempt
                    .connector_metadata
                    .map(|metadata| {
                        metadata
                            .parse_value("NextStepsRequirements")
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable(
                                "Failed to parse the Value to NextRequirements struct",
                            )
                    })
                    .transpose()?;
            bank_transfer_next_steps
        } else {
            None
        }
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
        requires_shipping: order_details.requires_shipping,
        product_id: order_details.product_id,
        category: order_details.category,
        sub_category: order_details.sub_category,
        brand: order_details.brand,
        product_type: order_details.product_type,
    }])
}

impl ForeignFrom<api_models::payments::QrCodeInformation> for api_models::payments::NextActionData {
    fn foreign_from(qr_info: api_models::payments::QrCodeInformation) -> Self {
        match qr_info {
            api_models::payments::QrCodeInformation::QrCodeUrl {
                image_data_url,
                qr_code_url,
                display_to_timestamp,
            } => Self::QrCodeInformation {
                image_data_url: Some(image_data_url),
                qr_code_url: Some(qr_code_url),
                display_to_timestamp,
            },
            api_models::payments::QrCodeInformation::QrDataUrl {
                image_data_url,
                display_to_timestamp,
            } => Self::QrCodeInformation {
                image_data_url: Some(image_data_url),
                display_to_timestamp,
                qr_code_url: None,
            },
            api_models::payments::QrCodeInformation::QrCodeImageUrl {
                qr_code_url,
                display_to_timestamp,
            } => Self::QrCodeInformation {
                qr_code_url: Some(qr_code_url),
                display_to_timestamp,
                image_data_url: None,
            },
        }
    }
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
    customer_data: &'a Option<domain::Customer>,
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
        let amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.final_amount)
            .unwrap_or(payment_data.amount.into());

        let customer_name = additional_data
            .customer_data
            .as_ref()
            .and_then(|customer_data| {
                customer_data
                    .name
                    .as_ref()
                    .map(|customer| customer.clone().into_inner())
            });

        Ok(Self {
            payment_method_data: From::from(
                payment_method_data.get_required_value("payment_method_data")?,
            ),
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            mandate_id: payment_data.mandate_id.clone(),
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            setup_mandate_details: payment_data.setup_mandate.clone(),
            confirm: payment_data.payment_attempt.confirm,
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            statement_descriptor: payment_data.payment_intent.statement_descriptor_name,
            capture_method: payment_data.payment_attempt.capture_method,
            amount,
            currency: payment_data.currency,
            browser_info,
            email: payment_data.email,
            customer_name,
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
            request_incremental_authorization: matches!(
                payment_data
                    .payment_intent
                    .request_incremental_authorization,
                Some(RequestIncrementalAuthorization::True)
                    | Some(RequestIncrementalAuthorization::Default)
            ),
            metadata: additional_data.payment_data.payment_intent.metadata,
            authentication_data: payment_data
                .authentication
                .as_ref()
                .map(AuthenticationData::foreign_try_from)
                .transpose()?,
            customer_acceptance: payment_data.customer_acceptance,
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
            encoded_data: payment_data.payment_attempt.encoded_data,
            capture_method: payment_data.payment_attempt.capture_method,
            connector_meta: payment_data.payment_attempt.connector_metadata,
            sync_type: match payment_data.multiple_capture_data {
                Some(multiple_capture_data) => types::SyncRequestType::MultipleCaptureSync(
                    multiple_capture_data.get_pending_connector_capture_ids(),
                ),
                None => types::SyncRequestType::SinglePaymentSync,
            },
            payment_method_type: payment_data.payment_attempt.payment_method_type,
            currency: payment_data.currency,
        })
    }
}

impl<F: Clone> TryFrom<PaymentAdditionalData<'_, F>>
    for types::PaymentsIncrementalAuthorizationData
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(additional_data: PaymentAdditionalData<'_, F>) -> Result<Self, Self::Error> {
        let payment_data = additional_data.payment_data;
        let connector = api::ConnectorData::get_connector_by_name(
            &additional_data.state.conf.connectors,
            &additional_data.connector_name,
            api::GetToken::Connector,
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )?;
        Ok(Self {
            total_amount: payment_data
                .incremental_authorization_details
                .clone()
                .map(|details| details.total_amount)
                .ok_or(
                    report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                        "missing incremental_authorization_details in payment_data",
                    ),
                )?,
            additional_amount: payment_data
                .incremental_authorization_details
                .clone()
                .map(|details| details.additional_amount)
                .ok_or(
                    report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                        "missing incremental_authorization_details in payment_data",
                    ),
                )?,
            reason: payment_data
                .incremental_authorization_details
                .and_then(|details| details.reason),
            currency: payment_data.currency,
            connector_transaction_id: connector
                .connector
                .connector_transaction_id(payment_data.payment_attempt.clone())?
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?,
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
            metadata: payment_data.payment_intent.metadata,
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
            metadata: payment_data.payment_intent.metadata,
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
        let amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.final_amount)
            .unwrap_or(payment_data.amount.into());

        Ok(Self {
            amount,
            currency: payment_data.currency,
            country: payment_data.address.get_payment_method_billing().and_then(
                |billing_address| {
                    billing_address
                        .address
                        .as_ref()
                        .and_then(|address| address.country)
                },
            ),
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

        let customer_name = additional_data
            .customer_data
            .as_ref()
            .and_then(|customer_data| {
                customer_data
                    .name
                    .as_ref()
                    .map(|customer| customer.clone().into_inner())
            });

        Ok(Self {
            currency: payment_data.currency,
            confirm: true,
            amount: Some(payment_data.amount.into()),
            payment_method_data: From::from(
                payment_data
                    .payment_method_data
                    .get_required_value("payment_method_data")?,
            ),
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            mandate_id: payment_data.mandate_id.clone(),
            setup_mandate_details: payment_data.setup_mandate,
            customer_acceptance: payment_data.customer_acceptance,
            router_return_url,
            email: payment_data.email,
            customer_name,
            return_url: payment_data.payment_intent.return_url,
            browser_info,
            payment_method_type: attempt.payment_method_type,
            request_incremental_authorization: matches!(
                payment_data
                    .payment_intent
                    .request_incremental_authorization,
                Some(RequestIncrementalAuthorization::True)
                    | Some(RequestIncrementalAuthorization::Default)
            ),
            metadata: payment_data.payment_intent.metadata.clone(),
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
        let router_base_url = &additional_data.router_base_url;
        let connector_name = &additional_data.connector_name;
        let attempt = &payment_data.payment_attempt;
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
        let amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.final_amount)
            .unwrap_or(payment_data.amount.into());
        let complete_authorize_url = Some(helpers::create_complete_authorize_url(
            router_base_url,
            attempt,
            connector_name,
        ));
        Ok(Self {
            setup_future_usage: payment_data.payment_intent.setup_future_usage,
            mandate_id: payment_data.mandate_id.clone(),
            off_session: payment_data.mandate_id.as_ref().map(|_| true),
            setup_mandate_details: payment_data.setup_mandate.clone(),
            confirm: payment_data.payment_attempt.confirm,
            statement_descriptor_suffix: payment_data.payment_intent.statement_descriptor_suffix,
            capture_method: payment_data.payment_attempt.capture_method,
            amount,
            currency: payment_data.currency,
            browser_info,
            email: payment_data.email,
            payment_method_data: payment_data.payment_method_data.map(From::from),
            connector_transaction_id: payment_data.payment_attempt.connector_transaction_id,
            redirect_response,
            connector_meta: payment_data.payment_attempt.connector_metadata,
            complete_authorize_url,
            metadata: payment_data.payment_intent.metadata,
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
        let amount = payment_data
            .surcharge_details
            .as_ref()
            .map(|surcharge_details| surcharge_details.final_amount)
            .unwrap_or(payment_data.amount.into());

        Ok(Self {
            payment_method_data: payment_method_data.map(From::from),
            email: payment_data.email,
            currency: Some(payment_data.currency),
            amount: Some(amount),
            payment_method_type: payment_data.payment_attempt.payment_method_type,
            setup_mandate_details: payment_data.setup_mandate,
            capture_method: payment_data.payment_attempt.capture_method,
            order_details,
            router_return_url,
            webhook_url,
            complete_authorize_url,
            browser_info,
            surcharge_details: payment_data.surcharge_details,
            connector_transaction_id: payment_data.payment_attempt.connector_transaction_id,
            redirect_response: None,
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
