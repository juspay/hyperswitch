use std::str::FromStr;

use async_trait::async_trait;
use common_enums as enums;
use common_types::payments as common_payments_types;
#[cfg(feature = "v2")]
use common_utils::types::MinorUnit;
use common_utils::{errors, ext_traits::ValueExt, id_type, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentConfirmData;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse, payments as domain_payments,
    router_data_v2::PaymentFlowData, router_response_types,
};
use hyperswitch_interfaces::{
    api::{self as api_interface, gateway, ConnectorSpecifications},
    errors as interface_errors,
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
        unified_connector_service::{
            self, build_unified_connector_service_auth_metadata, ucs_logging_wrapper_granular,
        },
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
        platform: &domain::Platform,
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
            platform,
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
        platform: &domain::Platform,
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
                platform,
                connector,
                state,
            )
            .await
        } else {
            Ok(None)
        }
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
        platform: &domain::Platform,
        customer: &Option<domain::Customer>,
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
            platform,
            customer,
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
        platform: &domain::Platform,
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
                        platform,
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

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        _platform: &domain::Platform,
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
            let authorize_request_data = self.request.clone();
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
            ))
            .await?;
            // Convert back to CompleteAuthorize router data while preserving preprocessing response data
            let pre_authenticate_response = pre_authenticate_router_data.response.clone();
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
                api_models::enums::Connector::Redsys => match &authorize_router_data.response {
                    Ok(types::PaymentsResponseData::TransactionResponse {
                        connector_metadata,
                        ..
                    }) => {
                        let three_ds_invoke_data: Option<
                            api_models::payments::PaymentsConnectorThreeDsInvokeData,
                        > = connector_metadata.clone().and_then(|metadata| {
                            metadata
                                .parse_value("PaymentsConnectorThreeDsInvokeData")
                                .ok()
                        });
                        three_ds_invoke_data.is_none()
                    }
                    _ => false,
                },
                api_models::enums::Connector::Nuvei => true,
                _ => false,
            };
            Ok((authorize_router_data, should_continue_after_preauthenticate))
        } else {
            Ok((self, true))
        }
    }

    async fn preprocessing_steps<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        authorize_preprocessing_steps(state, &self, true, connector).await
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

    async fn create_order_at_connector(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        should_continue_payment: bool,
        gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<Option<types::CreateOrderResult>> {
        if connector
            .connector_name
            .requires_order_creation_before_payment(self.payment_method)
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

            let resp = gateway::execute_payment_gateway(
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

            let create_order_resp = match resp.response {
                Ok(res) => {
                    if let types::PaymentsResponseData::PaymentsCreateOrderResponse { order_id } =
                        res
                    {
                        Ok(order_id)
                    } else {
                        Err(error_stack::report!(ApiErrorResponse::InternalServerError)
                            .attach_printable(format!(
                                "Unexpected response format from connector: {res:?}",
                            )))?
                    }
                }
                Err(error) => Err(error),
            };

            Ok(Some(types::CreateOrderResult {
                create_order_result: create_order_resp,
            }))
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
                self.response =
                    Ok(types::PaymentsResponseData::PaymentsCreateOrderResponse { order_id });
            }
            Err(err) => {
                self.response = Err(err.clone());
            }
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

pub async fn authorize_preprocessing_steps<F: Clone>(
    state: &SessionState,
    router_data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    confirm: bool,
    connector: &api::ConnectorData,
) -> RouterResult<types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>> {
    if confirm {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let preprocessing_request_data =
            types::PaymentsPreProcessingData::try_from(router_data.request.to_owned())?;

        let preprocessing_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
            Err(types::ErrorResponse::default());

        let preprocessing_router_data =
            helpers::router_data_type_conversion::<_, api::PreProcessing, _, _, _, _>(
                router_data.clone(),
                preprocessing_request_data,
                preprocessing_response_data,
            );

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &preprocessing_router_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_payment_failed_response()?;

        metrics::PREPROCESSING_STEPS_COUNT.add(
            1,
            router_env::metric_attributes!(
                ("connector", connector.connector_name.to_string()),
                ("payment_method", router_data.payment_method.to_string()),
                (
                    "payment_method_type",
                    router_data
                        .request
                        .payment_method_type
                        .map(|inner| inner.to_string())
                        .unwrap_or("null".to_string()),
                ),
            ),
        );
        let mut authorize_router_data = helpers::router_data_type_conversion::<_, F, _, _, _, _>(
            resp.clone(),
            router_data.request.to_owned(),
            resp.response.clone(),
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
        } else if connector.connector_name == api_models::enums::Connector::Shift4 {
            if resp.request.enrolled_for_3ds {
                authorize_router_data.response = resp.response;
                authorize_router_data.status = resp.status;
            } else {
                authorize_router_data.request.enrolled_for_3ds = false;
            }
        }
        Ok(authorize_router_data)
    } else {
        Ok(router_data.clone())
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
                    charges,
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
    platform: &domain::Platform,
    connector: enums::connector_enums::Connector,
    unified_connector_service_execution_mode: enums::ExecutionMode,
    merchant_order_reference_id: Option<String>,
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

    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        platform,
        router_data.connector.clone(),
    )
    .change_context(interface_errors::ConnectorError::RequestEncodingFailed)
    .attach_printable("Failed to construct request metadata")?;
    let merchant_reference_id = header_payload
        .x_reference_id
        .clone()
        .or(merchant_order_reference_id)
        .map(|id| id_type::PaymentReferenceId::from_str(id.as_str()))
        .transpose()
        .inspect_err(|err| logger::warn!(error=?err, "Invalid Merchant ReferenceId found"))
        .ok()
        .flatten()
        .map(ucs_types::UcsReferenceId::Payment);
    let headers_builder = state
        .get_grpc_headers_ucs(unified_connector_service_execution_mode)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_reference_id)
        .lineage_ids(lineage_ids);
    Box::pin(ucs_logging_wrapper_granular(
        router_data.clone(),
        state,
        payment_pre_authenticate_request,
        headers_builder,
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
                )
                .attach_printable("Failed to deserialize UCS response")?;

            let router_data_response = router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });
            let router_data_response = match router_data_response {
                Ok(response) => Ok(transform_response_for_pre_authenticate_flow(connector, response)?),
                Err(err) => Err(err)
            };
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
