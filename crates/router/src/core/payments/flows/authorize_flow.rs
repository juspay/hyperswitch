use std::str::FromStr;

use async_trait::async_trait;
use common_enums as enums;
use common_types::payments as common_payments_types;
#[cfg(feature = "v2")]
use common_utils::types::MinorUnit;
use common_utils::{errors, ext_traits::ValueExt, id_type, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client;
use hyperswitch_connectors::constants as connector_consts;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentConfirmData;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    payments as domain_payments, router_data,
    router_data_v2::{flow_common_types, PaymentFlowData},
    router_flow_types, router_request_types, router_response_types,
};
use hyperswitch_interfaces::{
    api::{self as api_interface, gateway, ConnectorSpecifications},
    consts as interface_consts, errors as interface_errors,
    unified_connector_service::transformers as ucs_transformers,
};
use masking::ExposeInterface;
use unified_connector_service_client::payments as payments_grpc;
use unified_connector_service_masking::ExposeInterface as UcsMaskingExposeInterface;

// use router_env::tracing::Instrument;
use super::{ConstructFlowSpecificData, Feature};
#[cfg(feature = "v2")]
use crate::core::unified_connector_service::{
    get_access_token_from_ucs_response,
    handle_unified_connector_service_response_for_payment_authorize,
    handle_unified_connector_service_response_for_payment_repeat, set_access_token_for_ucs,
    ucs_logging_wrapper,
};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, flows::gateway_context, gateway as payments_gateway,
            helpers, session_token, tokenization, transformers, PaymentData,
        },
        unified_connector_service,
    },
    logger,
    routes::{metrics, SessionState},
    services::{self, api::ConnectorValidation},
    types::{self, api, domain, transformers::ForeignTryFrom},
    utils::OptionExt,
};

#[cfg(feature = "v2")]
#[async_trait]
impl
    ConstructFlowSpecificData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentConfirmData<api::Authorize>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        processor: &domain::Processor,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<domain_payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(transformers::construct_payment_router_data_for_authorize(
            state,
            self.clone(),
            connector_id,
            processor,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
        ))
        .await
    }

    async fn get_merchant_recipient_data<'a>(
        &self,
        state: &SessionState,
        processor: &domain::Processor,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        let is_open_banking = &self
            .payment_attempt
            .get_payment_method()
            .get_required_value("PaymentMethod")?
            .eq(&enums::PaymentMethod::OpenBanking);

        if *is_open_banking {
            payments::get_merchant_bank_data_for_open_banking_connectors(
                merchant_connector_account,
                processor,
                connector,
                state,
            )
            .await
        } else {
            Ok(None)
        }
    }

    fn add_guest_customer(
        &self,
        router_data: &mut types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        guest_customer: &Option<hyperswitch_domain_models::payments::GuestCustomer>,
    ) -> RouterResult<()> {
        if let Some(guest_customer_data) = guest_customer {
            router_data.request.guest_customer = Some(guest_customer_data.clone());
        }
        Ok(())
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl
    ConstructFlowSpecificData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentData<api::Authorize>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        processor: &domain::Processor,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<domain_payments::HeaderPayload>,
        _payment_method: Option<common_enums::PaymentMethod>,
        _payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<
        types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(transformers::construct_payment_router_data::<
            api::Authorize,
            types::PaymentsAuthorizeData,
        >(
            state,
            self.clone(),
            connector_id,
            processor,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
            None,
            None,
        ))
        .await
    }

    async fn get_merchant_recipient_data<'a>(
        &self,
        state: &SessionState,
        processor: &domain::Processor,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        match &self.payment_intent.is_payment_processor_token_flow {
            Some(true) => Ok(None),
            Some(false) | None => {
                let is_open_banking = &self
                    .payment_attempt
                    .get_payment_method()
                    .get_required_value("PaymentMethod")?
                    .eq(&enums::PaymentMethod::OpenBanking);

                Ok(if *is_open_banking {
                    payments::get_merchant_bank_data_for_open_banking_connectors(
                        merchant_connector_account,
                        processor,
                        connector,
                        state,
                    )
                    .await?
                } else {
                    None
                })
            }
        }
    }
}

#[async_trait]
impl Feature<api::Authorize, types::PaymentsAuthorizeData> for types::PaymentsAuthorizeRouterData {
    async fn decide_flows<'a>(
        mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        business_profile: &domain::Profile,
        header_payload: domain_payments::HeaderPayload,
        return_raw_connector_response: Option<bool>,
        gateway_context: gateway_context::RouterGatewayContext,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        if self.should_proceed_with_authorize() {
            self.decide_authentication_type();
            logger::debug!(auth_type=?self.auth_type);
            let mut auth_router_data = gateway::execute_payment_gateway(
                state,
                connector_integration,
                &self,
                call_connector_action.clone(),
                connector_request,
                return_raw_connector_response,
                gateway_context.clone(),
            )
            .await
            .to_payment_failed_response()?;

            // Initiating Integrity check
            let integrity_result = helpers::check_integrity_based_on_flow(
                &auth_router_data.request,
                &auth_router_data.response,
            );
            auth_router_data.integrity_check = integrity_result;
            metrics::PAYMENT_COUNT.add(1, &[]); // Move outside of the if block
            match auth_router_data.response.clone() {
                Err(_) => Ok(auth_router_data),
                Ok(authorize_response) => {
                    // Check if the Capture API should be called based on the connector and other parameters
                    if super::should_initiate_capture_flow(
                        &connector.connector_name,
                        self.request.customer_acceptance,
                        self.request.capture_method,
                        self.request.setup_future_usage,
                        auth_router_data.status,
                    ) {
                        auth_router_data = Box::pin(process_capture_flow(
                            auth_router_data,
                            authorize_response,
                            state,
                            connector,
                            call_connector_action.clone(),
                            business_profile,
                            header_payload,
                            gateway_context,
                        ))
                        .await?;
                    }
                    Ok(auth_router_data)
                }
            }
        } else {
            Ok(self.clone())
        }
    }

    async fn balance_check_flow<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<types::BalanceCheckResult> {
        if connector.connector.is_balance_check_flow_required(
            api_interface::CurrentFlowInfo::Authorize {
                auth_type: &self.auth_type,
                request_data: &self.request,
            },
        ) {
            logger::info!(
                "Balance check flow is required for connector: {}",
                connector.connector_name
            );
            let balance_check_request_data =
                router_request_types::GiftCardBalanceCheckRequestData::try_from(
                    self.request.to_owned(),
                )?;
            let balance_check_response_data: Result<
                router_response_types::GiftCardBalanceCheckResponseData,
                types::ErrorResponse,
            > = Err(types::ErrorResponse::default());
            let balance_check_router_data = helpers::router_data_type_conversion::<
                _,
                router_flow_types::GiftCardBalanceCheck,
                _,
                _,
                _,
                _,
            >(
                self.clone(),
                balance_check_request_data,
                balance_check_response_data,
            );

            let connector_integration: services::connector_integration_interface::BoxedConnectorIntegrationInterface<
                router_flow_types::GiftCardBalanceCheck,
                flow_common_types::GiftCardBalanceCheckFlowData,
                router_request_types::GiftCardBalanceCheckRequestData,
                router_response_types::GiftCardBalanceCheckResponseData,
            > = connector.connector.get_connector_integration();

            let response_router_data = services::execute_connector_processing_step(
                state,
                connector_integration,
                &balance_check_router_data,
                payments::CallConnectorAction::Trigger,
                None,
                None,
            )
            .await
            .to_payment_failed_response()?;

            let balance_check_result = match &response_router_data.response {
                Ok(router_response_types::GiftCardBalanceCheckResponseData {
                    balance,
                    currency,
                }) => {
                    logger::info!(
                        "Requested amount and currency: {}, {}",
                        self.request.minor_amount,
                        self.request.currency
                    );
                    logger::info!(
                        "Balance amount and currency recieved from connector : {}, {}",
                        balance,
                        currency
                    );
                    if *balance >= self.request.minor_amount {
                        Ok(Some(router_data::PaymentMethodBalance {
                            amount: *balance,
                            currency: *currency,
                        }))
                    } else {
                        // If balance is insufficient, return a connector error response
                        // At this point, connector would have returned a success response with balance details
                        Err(router_data::ErrorResponse {
                            code: interface_consts::NO_ERROR_CODE.to_string(),
                            message: interface_consts::NO_ERROR_MESSAGE.to_string(),
                            reason: Some(connector_consts::LOW_BALANCE_ERROR_MESSAGE.to_string()),
                            status_code: response_router_data
                                .connector_http_status_code
                                .unwrap_or(200),
                            attempt_status: Some(enums::AttemptStatus::Failure),
                            connector_transaction_id: None,
                            connector_response_reference_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        })
                    }
                }
                Err(err) => Err(err.clone()),
            };
            Ok(types::BalanceCheckResult {
                // Continue with the payment only if ok response is recieved from balance check
                should_continue_payment: balance_check_result.is_ok(),
                balance_check_result,
            })
        } else {
            Ok(types::BalanceCheckResult {
                balance_check_result: Ok(None),
                should_continue_payment: true,
            })
        }
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        _processor: &domain::Processor,
        creds_identifier: Option<&str>,
        gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<types::AddAccessTokenResult> {
        Box::pin(access_token::add_access_token(
            state,
            connector,
            self,
            creds_identifier,
            gateway_context,
        ))
        .await
    }

    async fn add_session_token<'a>(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<()>
    where
        Self: Sized,
    {
        self.session_token =
            session_token::add_session_token_if_needed(self, state, connector, gateway_context)
                .await?;
        Ok(())
    }

    async fn add_payment_method_token<'a>(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        tokenization_action: &payments::TokenizationAction,
        should_continue_payment: bool,
        gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<types::PaymentMethodTokenResult> {
        let request = self.request.clone();
        tokenization::add_payment_method_token(
            state,
            connector,
            tokenization_action,
            self,
            types::PaymentMethodTokenizationData::try_from(request)?,
            should_continue_payment,
            gateway_context,
        )
        .await
    }

    async fn pre_authentication_step<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
        gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<(Self, bool)>
    where
        Self: Sized,
    {
        if connector.connector.is_pre_authentication_flow_required(
            api_interface::CurrentFlowInfo::Authorize {
                auth_type: &self.auth_type,
                request_data: &self.request,
            },
        ) {
            logger::info!(
                "Pre-authentication flow is required for connector: {}",
                connector.connector_name
            );
            let mut authorize_request_data = self.request.clone();
            let pre_authenticate_request_data =
                types::PaymentsPreAuthenticateData::try_from(self.request.to_owned())?;

            let pre_authenticate_response_data: Result<
                types::PaymentsResponseData,
                types::ErrorResponse,
            > = Err(types::ErrorResponse::default());
            let pre_authenticate_router_data =
                helpers::router_data_type_conversion::<_, api::PreAuthenticate, _, _, _, _>(
                    self.clone(),
                    pre_authenticate_request_data,
                    pre_authenticate_response_data,
                );
            let pre_authenticate_router_data = Box::pin(payments_gateway::handle_gateway_call::<
                _,
                _,
                _,
                PaymentFlowData,
                _,
            >(
                state,
                pre_authenticate_router_data,
                connector,
                gateway_context,
                payments::CallConnectorAction::Trigger,
                None,
                None,
            ))
            .await?;

            // Convert back to CompleteAuthorize router data while preserving pre authentication response data
            let pre_authenticate_response = pre_authenticate_router_data.response.clone();

            authorize_request_data.ucs_authentication_data =
                if let Ok(types::PaymentsResponseData::TransactionResponse {
                    ref authentication_data,
                    ..
                }) = pre_authenticate_response
                {
                    authentication_data.clone().map(|boxed| *boxed)
                } else {
                    None
                };

            let mut authorize_router_data =
                helpers::router_data_type_conversion::<_, api::Authorize, _, _, _, _>(
                    pre_authenticate_router_data,
                    authorize_request_data,
                    pre_authenticate_response,
                );

            if let Ok(types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                enrolled_v2,
                related_transaction_id,
            }) = &authorize_router_data.response
            {
                let (enrolled_for_3ds, related_transaction_id) =
                    (*enrolled_v2, related_transaction_id.clone());
                authorize_router_data.request.enrolled_for_3ds = enrolled_for_3ds;
                authorize_router_data.request.related_transaction_id = related_transaction_id;
            }

            let should_continue_after_preauthenticate = match connector.connector_name {
                // connector specific handling to decide whether to continue with authorize or not should not be done here
                // this is just a temporary fix for Redsys and Shift4 connectors
                api_models::enums::Connector::Redsys => match &authorize_router_data.response {
                    Ok(types::PaymentsResponseData::TransactionResponse {
                        connector_metadata,
                        redirection_data,
                        ..
                    }) => {
                        let has_ucs_redirection = redirection_data.is_some();

                        let has_hyperswitch_three_ds_invoke_data: bool =
                            connector_metadata.clone().and_then(|metadata| {
                                metadata
                                    .parse_value::<api_models::payments::PaymentsConnectorThreeDsInvokeData>("PaymentsConnectorThreeDsInvokeData")
                                    .ok()
                            }).is_some();

                        // Continue only if neither UCS nor hyperswitch indicates a redirect is needed
                        !has_ucs_redirection && !has_hyperswitch_three_ds_invoke_data
                    }
                    _ => false,
                },
                api_models::enums::Connector::Shift4 => true,
                api_models::enums::Connector::Nuvei => true,
                _ => false,
            };
            Ok((authorize_router_data, should_continue_after_preauthenticate))
        } else {
            Ok((self, true))
        }
    }

    async fn authentication_step<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
        gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<(Self, bool)>
    where
        Self: Sized,
    {
        if connector.connector.is_authentication_flow_required(
            api_interface::CurrentFlowInfo::Authorize {
                auth_type: &self.auth_type,
                request_data: &self.request,
            },
        ) {
            logger::info!(
                "Authentication flow is required for connector: {}",
                connector.connector_name
            );
            let mut authorize_request_data = self.request.clone();

            let mut authenticate_request_data =
                types::PaymentsAuthenticateData::try_from(self.request.to_owned())?;

            authenticate_request_data.authentication_data =
                authorize_request_data.ucs_authentication_data.clone();

            let authenticate_response_data: Result<
                types::PaymentsResponseData,
                types::ErrorResponse,
            > = Err(types::ErrorResponse::default());

            let authenticate_router_data =
                helpers::router_data_type_conversion::<_, api::Authenticate, _, _, _, _>(
                    self.clone(),
                    authenticate_request_data,
                    authenticate_response_data,
                );

            // Call UCS for Authenticate flow and store authentication result for next step
            let authenticate_router_data = Box::pin(payments_gateway::handle_gateway_call::<
                _,
                _,
                _,
                PaymentFlowData,
                _,
            >(
                state,
                authenticate_router_data,
                connector,
                gateway_context,
                payments::CallConnectorAction::Trigger,
                None,
                None,
            ))
            .await?;

            let authenticate_response = authenticate_router_data.response.clone();

            // Extract authentication_data from authenticate response
            let authentication_data_clone =
                if let Ok(types::PaymentsResponseData::TransactionResponse {
                    connector_metadata,
                    authentication_data,
                    ..
                }) = &authenticate_router_data.response
                {
                    connector_metadata.clone_into(&mut authorize_request_data.metadata);
                    authorize_request_data.ucs_authentication_data =
                        authentication_data.clone().map(|data| *data);

                    authentication_data.clone()
                } else {
                    None
                };

            let mut authorize_router_data =
                helpers::router_data_type_conversion::<_, api::Authorize, _, _, _, _>(
                    authenticate_router_data.clone(),
                    authorize_request_data.clone(),
                    authenticate_response.clone(),
                );

            // Merge authentication_data into connector_metadata for persistence
            // This ensures authentication_data is available in CompleteAuthorize flow
            if let Some(auth_data) = authentication_data_clone {
                if let Ok(types::PaymentsResponseData::TransactionResponse {
                    connector_metadata,
                    ..
                }) = &mut authorize_router_data.response
                {
                    *connector_metadata = Some(serde_json::json!({
                        "authentication_data": auth_data
                    }));
                }
            }

            let should_continue_after_authenticate = match &authorize_router_data.response {
                Ok(types::PaymentsResponseData::TransactionResponse {
                    connector_metadata,
                    redirection_data,
                    ..
                }) => match connector.connector_name {
                    api_models::enums::Connector::Redsys => {
                        // For UCS Redsys: if redirection_data is present (3DS challenge), don't continue
                        // For hyperswitch native: check connector_metadata for PaymentsConnectorThreeDsInvokeData
                        let has_ucs_redirection = redirection_data.is_some();

                        let has_hyperswitch_three_ds_invoke_data: bool =
                            connector_metadata.clone().and_then(|metadata| {
                                metadata
                                    .parse_value::<api_models::payments::PaymentsConnectorThreeDsInvokeData>("PaymentsConnectorThreeDsInvokeData")
                                    .ok()
                            }).is_some();

                        let payment_status = !matches!(
                            authorize_router_data.status,
                            common_enums::AttemptStatus::AuthenticationFailed
                                | common_enums::AttemptStatus::Failure
                                | common_enums::AttemptStatus::Charged
                                | common_enums::AttemptStatus::PartialCharged
                                | common_enums::AttemptStatus::Authorized
                        );

                        // Continue only if neither UCS nor hyperswitch indicates a redirect is needed
                        !has_ucs_redirection
                            && !has_hyperswitch_three_ds_invoke_data
                            && payment_status
                    }
                    _ => false,
                },
                _ => false,
            };
            Ok((authorize_router_data, should_continue_after_authenticate))
        } else {
            Ok((self, true))
        }
    }

    async fn postprocessing_steps<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        authorize_postprocessing_steps(state, &self, true, connector).await
    }

    async fn create_connector_customer<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<Option<String>> {
        customers::create_connector_customer(
            state,
            connector,
            self,
            types::ConnectorCustomerData::try_from(self)?,
            gateway_context,
        )
        .await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                connector
                    .connector
                    .validate_connector_against_payment_request(
                        self.request.capture_method,
                        self.payment_method,
                        self.request.payment_method_type,
                    )
                    .to_payment_failed_response()?;

                // Check if the connector supports mandate payment
                // if the payment_method_type does not support mandate for the given connector, downgrade the setup future usage to on session
                if self.request.setup_future_usage
                    == Some(diesel_models::enums::FutureUsage::OffSession)
                    && !self
                        .request
                        .payment_method_type
                        .and_then(|payment_method_type| {
                            state
                                .conf
                                .mandates
                                .supported_payment_methods
                                .0
                                .get(&enums::PaymentMethod::from(payment_method_type))
                                .and_then(|supported_pm_for_mandates| {
                                    supported_pm_for_mandates.0.get(&payment_method_type).map(
                                        |supported_connector_for_mandates| {
                                            supported_connector_for_mandates
                                                .connector_list
                                                .contains(&connector.connector_name)
                                        },
                                    )
                                })
                        })
                        .unwrap_or(false)
                {
                    // downgrade the setup future usage to on session
                    self.request.setup_future_usage =
                        Some(diesel_models::enums::FutureUsage::OnSession);
                };

                if crate::connector::utils::PaymentsAuthorizeRequestData::is_customer_initiated_mandate_payment(
                    &self.request,
                ) {
                    connector
                        .connector
                        .validate_mandate_payment(
                            self.request.payment_method_type,
                            self.request.payment_method_data.clone(),
                        )
                        .to_payment_failed_response()?;
                };

                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                    api::Authorize,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                metrics::EXECUTE_PRETASK_COUNT.add(
                    1,
                    router_env::metric_attributes!(
                        ("connector", connector.connector_name.to_string()),
                        ("flow", format!("{:?}", api::Authorize)),
                    ),
                );

                logger::debug!(completed_pre_tasks=?true);

                if self.should_proceed_with_authorize() {
                    self.decide_authentication_type();
                    logger::debug!(auth_type=?self.auth_type);

                    Ok((
                        connector_integration
                            .build_request(self, &state.conf.connectors)
                            .to_payment_failed_response()?,
                        true,
                    ))
                } else {
                    Ok((None, false))
                }
            }
            _ => Ok((None, true)),
        }
    }

    async fn settlement_split_call<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<(Self, bool)> {
        if connector.connector.is_settlement_split_call_required(
            api_interface::CurrentFlowInfo::Authorize {
                auth_type: &self.auth_type,
                request_data: &self.request,
            },
        ) {
            logger::info!(
                "Settlement Split call is required for connector: {}",
                connector.connector_name
            );
            let authorize_request_data = self.request.clone();
            let settlement_split_request_data =
                router_request_types::SettlementSplitRequestData::try_from(
                    self.request.to_owned(),
                )?;
            let settlement_split_response_data: Result<
                types::PaymentsResponseData,
                types::ErrorResponse,
            > = Err(types::ErrorResponse::default());
            let settlement_split_router_data = helpers::router_data_type_conversion::<
                _,
                router_flow_types::SettlementSplitCreate,
                _,
                _,
                _,
                _,
            >(
                self.clone(),
                settlement_split_request_data,
                settlement_split_response_data,
            );
            let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                router_flow_types::SettlementSplitCreate,
                router_request_types::SettlementSplitRequestData,
                types::PaymentsResponseData,
            > = connector.connector.get_connector_integration();
            let settlement_split_router_data = services::execute_connector_processing_step(
                state,
                connector_integration,
                &settlement_split_router_data,
                payments::CallConnectorAction::Trigger,
                None,
                None,
            )
            .await
            .to_payment_failed_response()?;
            // Convert back to Authorize router data while preserving preprocessing response data
            let settlement_split_response = settlement_split_router_data.response.clone();
            let authorize_router_data =
                helpers::router_data_type_conversion::<_, api::Authorize, _, _, _, _>(
                    settlement_split_router_data,
                    authorize_request_data,
                    settlement_split_response,
                );
            // Continue the payment only if settlement split call was successful
            let should_continue_payment = authorize_router_data.response.is_ok();
            Ok((authorize_router_data, should_continue_payment))
        } else {
            // If the connector does not require settlement split call, return the original router data
            // with should_continue_payment as true
            Ok((self, true))
        }
    }

    async fn create_order_at_connector(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        should_continue_payment: bool,
        gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<Option<types::CreateOrderResult>> {
        let is_order_create_bloated_connector = connector.connector.is_order_create_flow_required(
            api_interface::CurrentFlowInfo::Authorize {
                auth_type: &self.auth_type,
                request_data: &self.request,
            },
        );
        if (connector
            .connector_name
            .requires_order_creation_before_payment(self.payment_method)
            || is_order_create_bloated_connector)
            && should_continue_payment
        {
            let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                api::CreateOrder,
                types::CreateOrderRequestData,
                types::PaymentsResponseData,
            > = connector.connector.get_connector_integration();

            let request_data = types::CreateOrderRequestData::try_from(self.request.clone())?;

            let response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
                Err(types::ErrorResponse::default());

            let createorder_router_data =
                helpers::router_data_type_conversion::<_, api::CreateOrder, _, _, _, _>(
                    self.clone(),
                    request_data,
                    response_data,
                );

            let order_create_response_router_data = gateway::execute_payment_gateway(
                state,
                connector_integration,
                &createorder_router_data,
                payments::CallConnectorAction::Trigger,
                None,
                None,
                gateway_context.clone(),
            )
            .await
            .to_payment_failed_response()?;

            let order_create_response = order_create_response_router_data.response.clone();

            let create_order_resp = match &order_create_response {
                Ok(types::PaymentsResponseData::PaymentsCreateOrderResponse {
                    order_id,
                    session_token,
                }) => {
                    let should_continue_further = if session_token.is_some() {
                        // if SDK session token is returned in order create response, do not continue and return control to SDK
                        false
                    } else {
                        should_continue_payment
                    };
                    types::CreateOrderResult {
                        create_order_result: Ok(order_id.clone()),
                        should_continue_further,
                    }
                }
                // Some connector return PreProcessingResponse and TransactionResponse response type
                // Rest of the match statements are temporary fixes for satisfying current connector side response handling
                // Create Order response must always be PaymentsResponseData::PaymentsCreateOrderResponse only
                Ok(types::PaymentsResponseData::PreProcessingResponse {
                    pre_processing_id,
                    session_token,
                    ..
                }) => {
                    let should_continue_further = if session_token.is_some() {
                        // if SDK session token is returned in order create response, do not continue and return control to SDK
                        false
                    } else {
                        should_continue_payment
                    };
                    types::CreateOrderResult {
                        create_order_result: Ok(pre_processing_id.get_string_repr().clone()),
                        should_continue_further,
                    }
                }
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
                    ..
                }) => {
                    let order_id = resource_id
                        .get_connector_transaction_id()
                        .change_context(ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "unable to get connector_transaction_id during order create",
                        )?;
                    let should_continue_further = if redirection_data.is_some() {
                        // if redirection_data is returned in order create response, do not continue and return control to SDK
                        false
                    } else {
                        should_continue_payment
                    };
                    types::CreateOrderResult {
                        create_order_result: Ok(order_id),
                        should_continue_further,
                    }
                }
                Ok(res) => Err(error_stack::report!(ApiErrorResponse::InternalServerError)
                    .attach_printable(format!(
                        "Unexpected response format from connector: {res:?}",
                    )))?,
                Err(error) => types::CreateOrderResult {
                    create_order_result: Err(error.clone()),
                    should_continue_further: false,
                },
            };
            // persist order create response
            *self = helpers::router_data_type_conversion::<_, api::Authorize, _, _, _, _>(
                order_create_response_router_data,
                self.request.clone(),
                order_create_response,
            );
            Ok(Some(create_order_resp))
        } else {
            // If the connector does not require order creation, return None
            Ok(None)
        }
    }

    fn update_router_data_with_create_order_response(
        &mut self,
        create_order_result: types::CreateOrderResult,
    ) {
        match create_order_result.create_order_result {
            Ok(order_id) => {
                self.request.order_id = Some(order_id.clone()); // ? why this is assigned here and ucs also wants this to populate data
            }
            Err(_err) => (),
        }
    }
}

pub trait RouterDataAuthorize {
    fn decide_authentication_type(&mut self);

    /// to decide if we need to proceed with authorize or not, Eg: If any of the pretask returns `redirection_response` then we should not proceed with authorize call
    fn should_proceed_with_authorize(&self) -> bool;
}

impl RouterDataAuthorize for types::PaymentsAuthorizeRouterData {
    fn decide_authentication_type(&mut self) {
        if let hyperswitch_domain_models::payment_method_data::PaymentMethodData::Wallet(
            hyperswitch_domain_models::payment_method_data::WalletData::GooglePay(google_pay_data),
        ) = &self.request.payment_method_data
        {
            if let Some(assurance_details) = google_pay_data.info.assurance_details.as_ref() {
                // Step up the transaction to 3DS when either assurance_details.card_holder_authenticated or assurance_details.account_verified is false
                if !assurance_details.card_holder_authenticated
                    || !assurance_details.account_verified
                {
                    logger::info!("Googlepay transaction stepped up to 3DS");
                    self.auth_type = diesel_models::enums::AuthenticationType::ThreeDs;
                }
            }
        }
        if self.auth_type == diesel_models::enums::AuthenticationType::ThreeDs
            && !self.request.enrolled_for_3ds
        {
            self.auth_type = diesel_models::enums::AuthenticationType::NoThreeDs
        }
    }

    /// to decide if we need to proceed with authorize or not, Eg: If any of the pretask returns `redirection_response` then we should not proceed with authorize call
    fn should_proceed_with_authorize(&self) -> bool {
        match &self.response {
            Ok(types::PaymentsResponseData::TransactionResponse {
                redirection_data, ..
            }) => !redirection_data.is_some(),
            _ => true,
        }
    }
}

impl mandate::MandateBehaviour for types::PaymentsAuthorizeData {
    fn get_amount(&self) -> i64 {
        self.amount
    }
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }
    fn get_payment_method_data(&self) -> domain::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }
    fn get_setup_future_usage(&self) -> Option<diesel_models::enums::FutureUsage> {
        self.setup_future_usage
    }
    fn get_setup_mandate_details(
        &self,
    ) -> Option<&hyperswitch_domain_models::mandates::MandateData> {
        self.setup_mandate_details.as_ref()
    }

    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>) {
        self.mandate_id = new_mandate_id;
    }
    fn get_customer_acceptance(&self) -> Option<common_payments_types::CustomerAcceptance> {
        self.customer_acceptance.clone()
    }
}

pub async fn authorize_postprocessing_steps<F: Clone>(
    state: &SessionState,
    router_data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    confirm: bool,
    connector: &api::ConnectorData,
) -> RouterResult<types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>> {
    if confirm {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::PostProcessing,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let postprocessing_request_data =
            types::PaymentsPostProcessingData::try_from(router_data.to_owned())?;

        let postprocessing_response_data: Result<
            types::PaymentsResponseData,
            types::ErrorResponse,
        > = Err(types::ErrorResponse::default());

        let postprocessing_router_data =
            helpers::router_data_type_conversion::<_, api::PostProcessing, _, _, _, _>(
                router_data.clone(),
                postprocessing_request_data,
                postprocessing_response_data,
            );

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &postprocessing_router_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_payment_failed_response()?;

        let authorize_router_data = helpers::router_data_type_conversion::<_, F, _, _, _, _>(
            resp.clone(),
            router_data.request.to_owned(),
            resp.response,
        );

        Ok(authorize_router_data)
    } else {
        Ok(router_data.clone())
    }
}

impl<F>
    ForeignTryFrom<types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>>
    for types::PaymentsCaptureData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn foreign_try_from(
        item: types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: item.connector.clone(),
                status_code: err.status_code,
                reason: err.reason,
            })?;

        Ok(Self {
            amount_to_capture: item.request.amount,
            currency: item.request.currency,
            connector_transaction_id: types::PaymentsResponseData::get_connector_transaction_id(
                &response,
            )?,
            payment_amount: item.request.amount,
            multiple_capture_data: None,
            connector_meta: types::PaymentsResponseData::get_connector_metadata(&response)
                .map(|secret| secret.expose()),
            browser_info: None,
            metadata: None,
            capture_method: item.request.capture_method,
            minor_payment_amount: item.request.minor_amount,
            minor_amount_to_capture: item.request.minor_amount,
            integrity_object: None,
            split_payments: item.request.split_payments,
            webhook_url: item.request.webhook_url,
            merchant_order_reference_id: item.request.merchant_order_reference_id,
        })
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_capture_flow(
    mut router_data: types::RouterData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    >,
    authorize_response: types::PaymentsResponseData,
    state: &SessionState,
    connector: &api::ConnectorData,
    call_connector_action: payments::CallConnectorAction,
    business_profile: &domain::Profile,
    header_payload: domain_payments::HeaderPayload,
    context: gateway_context::RouterGatewayContext,
) -> RouterResult<
    types::RouterData<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
> {
    // Convert RouterData into Capture RouterData
    let capture_router_data = helpers::router_data_type_conversion(
        router_data.clone(),
        types::PaymentsCaptureData::foreign_try_from(router_data.clone())?,
        Err(types::ErrorResponse::default()),
    );

    // Call capture request
    let post_capture_router_data = super::call_capture_request(
        capture_router_data,
        state,
        connector,
        call_connector_action,
        business_profile,
        header_payload,
        context,
    )
    .await;

    // Process capture response
    let (updated_status, updated_response) =
        super::handle_post_capture_response(authorize_response, post_capture_router_data)?;
    router_data.status = updated_status;
    router_data.response = Ok(updated_response);
    Ok(router_data)
}

fn transform_redirection_response_for_pre_authenticate_flow(
    connector: enums::connector_enums::Connector,
    response_data: router_response_types::RedirectForm,
) -> errors::CustomResult<
    router_response_types::RedirectForm,
    ucs_transformers::UnifiedConnectorServiceError,
> {
    match (connector, &response_data) {
        (
            enums::connector_enums::Connector::Cybersource,
            router_response_types::RedirectForm::Form {
                endpoint,
                method: _,
                ref form_fields,
            },
        ) => {
            let access_token = form_fields.get("access_token").cloned().ok_or(
                ucs_transformers::UnifiedConnectorServiceError::MissingRequiredField {
                    field_name: "access_token",
                },
            )?;
            let ddc_url = form_fields.get("ddc_url").unwrap_or(endpoint).clone();
            let reference_id = form_fields.get("reference_id").cloned().ok_or(
                ucs_transformers::UnifiedConnectorServiceError::MissingRequiredField {
                    field_name: "reference_id",
                },
            )?;
            Ok(router_response_types::RedirectForm::CybersourceAuthSetup {
                access_token,
                ddc_url,
                reference_id,
            })
        }
        _ => Ok(response_data),
    }
}
fn transform_response_for_pre_authenticate_flow(
    connector: enums::connector_enums::Connector,
    response_data: router_response_types::PaymentsResponseData,
) -> errors::CustomResult<
    router_response_types::PaymentsResponseData,
    ucs_transformers::UnifiedConnectorServiceError,
> {
    match (connector, response_data.clone()) {
        (
            enums::connector_enums::Connector::Cybersource,
            router_response_types::PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data,
                mandate_reference,
                connector_metadata,
                network_txn_id,
                connector_response_reference_id,
                incremental_authorization_allowed,
                authentication_data,
                charges,
            },
        ) => {
            let redirection_data = Box::new(
                (*redirection_data)
                    .clone()
                    .map(|redirection_data| {
                        transform_redirection_response_for_pre_authenticate_flow(
                            connector,
                            redirection_data,
                        )
                    })
                    .transpose()?,
            );
            Ok(
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
                    mandate_reference,
                    connector_metadata,
                    network_txn_id,
                    connector_response_reference_id,
                    incremental_authorization_allowed,
                    authentication_data,
                    charges,
                },
            )
        }
        // TODO: Temporary solution for Redsys 3DS invoke flow via UCS
        //
        // Currently, UCS returns 3DS invoke data in `redirection_data.form_fields` instead of
        // `connector_metadata`. This workaround extracts the invoke data from form_fields and
        // constructs `PaymentsConnectorThreeDsInvokeData` to populate `connector_metadata`,
        // enabling the `invoke_hidden_iframe` next_action to be correctly generated.
        //
        // For 3DS invoke: form_fields contains threeDsMethodData, threeDSServerTransID, etc.
        //   -> Extract and set as connector_metadata, clear redirection_data
        // For 3DS exempt/challenge: No invoke data in form_fields
        //   -> Keep redirection_data as-is, connector_metadata remains None
        //
        // A permanent solution requires redesigning how UCS passes 3DS invoke data back to
        // Hyperswitch, potentially through dedicated fields in the gRPC response or a unified
        // authentication data structure that covers all 3DS scenarios.
        (
            enums::connector_enums::Connector::Redsys,
            router_response_types::PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data,
                mandate_reference,
                connector_metadata: _,
                network_txn_id,
                connector_response_reference_id,
                incremental_authorization_allowed,
                charges,
                authentication_data,
            },
        ) => {
            // Check if this is a 3DS invoke response by looking at form_fields
            let (connector_metadata, redirection_data) =
                if let Some(ref redirect_form) = *redirection_data {
                    if let router_response_types::RedirectForm::Form {
                        endpoint,
                        form_fields,
                        ..
                    } = redirect_form
                    {
                        // Check for 3DS invoke - has threeDsMethodData and threeDSServerTransID
                        if form_fields.contains_key("threeDsMethodData")
                            && form_fields.contains_key("threeDSServerTransID")
                        {
                            // This is 3DS invoke - construct PaymentsConnectorThreeDsInvokeData
                            let invoke_data =
                                api_models::payments::PaymentsConnectorThreeDsInvokeData {
                                    directory_server_id: form_fields
                                        .get("threeDSServerTransID")
                                        .cloned()
                                        .unwrap_or_default(),
                                    three_ds_method_url: form_fields
                                        .get("threeDsMethodUrl")
                                        .cloned()
                                        .unwrap_or_else(|| endpoint.clone()),
                                    three_ds_method_data: form_fields
                                        .get("threeDsMethodData")
                                        .cloned()
                                        .unwrap_or_default(),
                                    message_version: form_fields.get("messageVersion").cloned(),
                                    three_ds_method_data_submission: form_fields
                                        .get("threeDsMethodDataSubmission")
                                        .map(|v| v == "true")
                                        .unwrap_or(true),
                                };
                            // Set connector_metadata with invoke data, clear redirection_data
                            (serde_json::to_value(&invoke_data).ok(), Box::new(None))
                        } else {
                            // 3DS exempt or challenge - keep redirection_data, no special connector_metadata
                            (None, redirection_data)
                        }
                    } else {
                        // Not a Form type redirect
                        (None, redirection_data)
                    }
                } else {
                    // No redirection data
                    (None, redirection_data)
                };

            Ok(
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
                    mandate_reference,
                    connector_metadata,
                    network_txn_id,
                    connector_response_reference_id,
                    incremental_authorization_allowed,
                    charges,
                    authentication_data,
                },
            )
        }
        _ => Ok(response_data),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn call_unified_connector_service_pre_authenticate(
    router_data: &types::RouterData<
        api::PreAuthenticate,
        types::PaymentsPreAuthenticateData,
        types::PaymentsResponseData,
    >,
    state: &SessionState,
    header_payload: &domain_payments::HeaderPayload,
    lineage_ids: grpc_client::LineageIds,
    #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    processor: &domain::Processor,
    connector: enums::connector_enums::Connector,
    unified_connector_service_execution_mode: enums::ExecutionMode,
) -> errors::CustomResult<
    (
        types::RouterData<
            api::PreAuthenticate,
            types::PaymentsPreAuthenticateData,
            types::PaymentsResponseData,
        >,
        (),
    ),
    interface_errors::ConnectorError,
> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(interface_errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to fetch Unified Connector Service client")?;

    let payment_pre_authenticate_request =
        payments_grpc::PaymentServicePreAuthenticateRequest::foreign_try_from(router_data)
            .change_context(interface_errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct Payment Authorize Request")?;

    let connector_auth_metadata =
        unified_connector_service::build_unified_connector_service_auth_metadata(
            merchant_connector_account,
            processor,
            router_data.connector.clone(),
        )
        .change_context(interface_errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to construct request metadata")?;
    let merchant_reference_id = unified_connector_service::parse_merchant_reference_id(
        header_payload
            .x_reference_id
            .as_deref()
            .unwrap_or(router_data.payment_id.as_str()),
    )
    .map(ucs_types::UcsReferenceId::Payment);
    let resource_id = id_type::PaymentResourceId::from_str(router_data.attempt_id.as_str())
        .inspect_err(
            |err| logger::warn!(error=?err, "Invalid Payment AttemptId for UCS resource id"),
        )
        .ok()
        .map(ucs_types::UcsResourceId::PaymentAttempt);
    let headers_builder = state
        .get_grpc_headers_ucs(unified_connector_service_execution_mode)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_reference_id)
        .resource_id(resource_id)
        .lineage_ids(lineage_ids);
    Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
        router_data.clone(),
        state,
        payment_pre_authenticate_request,
        headers_builder,
        unified_connector_service_execution_mode,
        |mut router_data, payment_pre_authenticate_request, grpc_headers| async move {
            let response = client
                .payment_pre_authenticate(
                    payment_pre_authenticate_request,
                    connector_auth_metadata,
                    grpc_headers,
                )
                .await
                .attach_printable("Failed to authorize payment")?;

            let payment_pre_authenticate_response = response.into_inner();

            let (router_data_response, status_code) =
                unified_connector_service::handle_unified_connector_service_response_for_payment_pre_authenticate(
                    payment_pre_authenticate_response.clone(),
                    router_data.status,
                )
                .attach_printable("Failed to deserialize UCS response")?;

            let router_data_response = router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });
            let router_data_response = match router_data_response {
                Ok(response) => Ok(transform_response_for_pre_authenticate_flow(
                    connector, response,
                )?),
                Err(err) => Err(err),
            };
            // Extract authentication_data from the response to store in connector_metadata
            router_data.response = router_data_response;
            router_data.raw_connector_response = payment_pre_authenticate_response
                .raw_connector_response
                .clone()
                .map(|raw_connector_response| raw_connector_response.expose().into());
            router_data.connector_http_status_code = Some(status_code);

            Ok((router_data, (), payment_pre_authenticate_response))
        },
    ))
    .await
    .change_context(interface_errors::ConnectorError::ResponseHandlingFailed)
}
