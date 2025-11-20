use std::str::FromStr;

use async_trait::async_trait;
use common_enums as enums;
use common_types::payments as common_payments_types;
use common_utils::{id_type, types::MinorUnit, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentConfirmData;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse, payments as domain_payments,
    router_response_types,
};
use hyperswitch_interfaces::{api as api_interface, api::ConnectorSpecifications};
use masking::ExposeInterface;
use unified_connector_service_client::payments as payments_grpc;
use unified_connector_service_masking::ExposeInterface as UcsMaskingExposeInterface;

// use router_env::tracing::Instrument;
use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, flows::gateway_context, helpers, tokenization,
            transformers, PaymentData,
        },
        unified_connector_service::{
            self, build_unified_connector_service_auth_metadata,
            get_access_token_from_ucs_response,
            handle_unified_connector_service_response_for_payment_authorize,
            handle_unified_connector_service_response_for_payment_repeat, set_access_token_for_ucs,
            ucs_logging_wrapper,
        },
    },
    logger,
    routes::{metrics, SessionState},
    services::{self, api::ConnectorValidation},
    types::{
        self, api, domain,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
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
    fn get_current_flow_info(&self) -> Option<api_interface::CurrentFlowInfo<'_>> {
        Some(api_interface::CurrentFlowInfo::Authorize {
            auth_type: &self.auth_type,
            request_data: &self.request,
        })
    }
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
            let mut auth_router_data = services::execute_connector_processing_step(
                state,
                connector_integration,
                &self,
                call_connector_action.clone(),
                connector_request,
                return_raw_connector_response,
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
    ) -> RouterResult<types::AddAccessTokenResult> {
        Box::pin(access_token::add_access_token(
            state,
            connector,
            self,
            creds_identifier,
        ))
        .await
    }

    async fn add_session_token<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self>
    where
        Self: Sized,
    {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let authorize_data = &types::PaymentsAuthorizeSessionTokenRouterData::foreign_from((
            &self,
            types::AuthorizeSessionTokenData::foreign_from(&self),
        ));
        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            authorize_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_payment_failed_response()?;
        let mut router_data = self;
        router_data.session_token = resp.session_token;
        Ok(router_data)
    }

    async fn add_payment_method_token<'a>(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        tokenization_action: &payments::TokenizationAction,
        should_continue_payment: bool,
    ) -> RouterResult<types::PaymentMethodTokenResult> {
        let request = self.request.clone();
        tokenization::add_payment_method_token(
            state,
            connector,
            tokenization_action,
            self,
            types::PaymentMethodTokenizationData::try_from(request)?,
            should_continue_payment,
        )
        .await
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
    ) -> RouterResult<Option<String>> {
        customers::create_connector_customer(
            state,
            connector,
            self,
            types::ConnectorCustomerData::try_from(self)?,
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

            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                &createorder_router_data,
                payments::CallConnectorAction::Trigger,
                None,
                None,
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

    async fn call_unified_connector_service<'a>(
        &mut self,
        state: &SessionState,
        header_payload: &domain_payments::HeaderPayload,
        lineage_ids: grpc_client::LineageIds,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        platform: &domain::Platform,
        connector_data: &api::ConnectorData,
        unified_connector_service_execution_mode: enums::ExecutionMode,
        merchant_order_reference_id: Option<String>,
        _call_connector_action: common_enums::CallConnectorAction,
        creds_identifier: Option<String>,
    ) -> RouterResult<()> {
        if self.request.mandate_id.is_some() {
            Box::pin(call_unified_connector_service_repeat_payment(
                self,
                state,
                header_payload,
                lineage_ids,
                merchant_connector_account,
                platform,
                unified_connector_service_execution_mode,
                merchant_order_reference_id,
                creds_identifier,
            ))
            .await
        } else {
            let alternate_flow = connector_data.connector.get_alternate_flow_if_needed(
                api_interface::CurrentFlowInfo::Authorize {
                    auth_type: &self.auth_type,
                    request_data: &self.request,
                },
            );
            match alternate_flow {
                Some(api_interface::AlternateFlow::PreAuthenticate) => {
                    let authorize_request_data = self.request.clone();
                    let pre_authneticate_request_data =
                        types::PaymentsPreAuthenticateData::try_from(self.request.to_owned())?;
                    let pre_authneticate_response_data: Result<
                        types::PaymentsResponseData,
                        types::ErrorResponse,
                    > = Err(types::ErrorResponse::default());
                    let mut pre_authenticate_router_data = helpers::router_data_type_conversion::<
                        _,
                        api::PreAuthenticate,
                        _,
                        _,
                        _,
                        _,
                    >(
                        self.clone(),
                        pre_authneticate_request_data,
                        pre_authneticate_response_data,
                    );
                    let _ = call_unified_connector_service_pre_authenticate(
                        &mut pre_authenticate_router_data,
                        state,
                        header_payload,
                        lineage_ids,
                        merchant_connector_account,
                        platform,
                        connector_data.connector_name,
                        unified_connector_service_execution_mode,
                        merchant_order_reference_id,
                    )
                    .await;
                    // Convert back to authorize router data while preserving preprocessing response data.
                    let pre_authenticate_response = pre_authenticate_router_data.response.clone();
                    let authorize_router_data =
                        helpers::router_data_type_conversion::<_, api::Authorize, _, _, _, _>(
                            pre_authenticate_router_data,
                            authorize_request_data,
                            pre_authenticate_response,
                        );
                    *self = authorize_router_data;

                    Ok(())
                }
                None => {
                    Box::pin(call_unified_connector_service_authorize(
                        self,
                        state,
                        header_payload,
                        lineage_ids,
                        merchant_connector_account,
                        platform,
                        unified_connector_service_execution_mode,
                        merchant_order_reference_id,
                        creds_identifier,
                    ))
                    .await
                }
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
        if connector.connector_name == api_models::enums::Connector::Nuvei {
            let (enrolled_for_3ds, related_transaction_id) = match &authorize_router_data.response {
                Ok(types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    enrolled_v2,
                    related_transaction_id,
                }) => (*enrolled_v2, related_transaction_id.clone()),
                _ => (false, None),
            };
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

#[allow(clippy::too_many_arguments)]
async fn call_unified_connector_service_authorize(
    router_data: &mut types::RouterData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    >,
    state: &SessionState,
    header_payload: &domain_payments::HeaderPayload,
    lineage_ids: grpc_client::LineageIds,
    #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    platform: &domain::Platform,
    unified_connector_service_execution_mode: enums::ExecutionMode,
    merchant_order_reference_id: Option<String>,
    creds_identifier: Option<String>,
) -> RouterResult<()> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch Unified Connector Service client")?;

    let payment_authorize_request =
        payments_grpc::PaymentServiceAuthorizeRequest::foreign_try_from(&*router_data)
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct Payment Authorize Request")?;

    let merchant_connector_id = merchant_connector_account.get_mca_id();

    let connector_auth_metadata =
        build_unified_connector_service_auth_metadata(merchant_connector_account, platform)
            .change_context(ApiErrorResponse::InternalServerError)
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
    let (updated_router_data, _) = Box::pin(ucs_logging_wrapper(
        router_data.clone(),
        state,
        payment_authorize_request,
        headers_builder,
        |mut router_data, payment_authorize_request, grpc_headers| async move {
            let response = Box::pin(client.payment_authorize(
                payment_authorize_request,
                connector_auth_metadata,
                grpc_headers,
            ))
            .await
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to authorize payment")?;

            let payment_authorize_response = response.into_inner();

            let ucs_data = handle_unified_connector_service_response_for_payment_authorize(
                payment_authorize_response.clone(),
            )
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to deserialize UCS response")?;

            // Extract and store access token if present
            if let Some(access_token) = get_access_token_from_ucs_response(
                state,
                platform,
                &router_data.connector,
                merchant_connector_id.as_ref(),
                creds_identifier.clone(),
                payment_authorize_response.state.as_ref(),
            )
            .await
            {
                if let Err(error) = set_access_token_for_ucs(
                    state,
                    platform,
                    &router_data.connector,
                    access_token,
                    merchant_connector_id.as_ref(),
                    creds_identifier,
                )
                .await
                {
                    logger::error!(
                        ?error,
                        "Failed to store UCS access token from authorize response"
                    );
                } else {
                    logger::debug!("Successfully stored access token from UCS authorize response");
                }
            }

            let router_data_response = ucs_data.router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });
            router_data.response = router_data_response;
            router_data.amount_captured = payment_authorize_response.captured_amount;
            router_data.minor_amount_captured = payment_authorize_response
                .minor_captured_amount
                .map(MinorUnit::new);
            router_data.raw_connector_response = payment_authorize_response
                .raw_connector_response
                .clone()
                .map(|raw_connector_response| raw_connector_response.expose().into());
            router_data.connector_http_status_code = Some(ucs_data.status_code);

            // Populate connector_customer_id if present
            ucs_data.connector_customer_id.map(|connector_customer_id| {
                router_data.connector_customer = Some(connector_customer_id);
            });

            ucs_data.connector_response.map(|customer_response| {
                router_data.connector_response = Some(customer_response);
            });

            Ok((router_data, (), payment_authorize_response))
        },
    ))
    .await?;

    // Copy back the updated data
    *router_data = updated_router_data;
    Ok(())
}
fn transform_redirection_response_for_pre_authenticate_flow(
    connector: enums::connector_enums::Connector,
    response_data: router_response_types::RedirectForm,
) -> RouterResult<router_response_types::RedirectForm> {
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
                ApiErrorResponse::MissingRequiredField {
                    field_name: "access_token",
                },
            )?;
            let ddc_url = form_fields.get("ddc_url").unwrap_or(endpoint).clone();
            let reference_id = form_fields.get("reference_id").cloned().ok_or(
                ApiErrorResponse::MissingRequiredField {
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
) -> RouterResult<router_response_types::PaymentsResponseData> {
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
async fn call_unified_connector_service_pre_authenticate(
    router_data: &mut types::RouterData<
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
) -> RouterResult<()> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch Unified Connector Service client")?;

    let payment_pre_authenticate_request =
        payments_grpc::PaymentServicePreAuthenticateRequest::foreign_try_from(&*router_data)
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct Payment Authorize Request")?;

    let connector_auth_metadata =
        build_unified_connector_service_auth_metadata(merchant_connector_account, platform)
            .change_context(ApiErrorResponse::InternalServerError)
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
    let (updated_router_data, _) = Box::pin(ucs_logging_wrapper(
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
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to authorize payment")?;

            let payment_pre_authenticate_response = response.into_inner();

            let (router_data_response, status_code) =
                unified_connector_service::handle_unified_connector_service_response_for_payment_pre_authenticate(
                    payment_pre_authenticate_response.clone(),
                )
                .change_context(ApiErrorResponse::InternalServerError)
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

            Ok((router_data,(), payment_pre_authenticate_response))
        },
    ))
    .await?;

    // Copy back the updated data
    *router_data = updated_router_data;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn call_unified_connector_service_repeat_payment(
    router_data: &mut types::RouterData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    >,
    state: &SessionState,
    header_payload: &domain_payments::HeaderPayload,
    lineage_ids: grpc_client::LineageIds,
    #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    platform: &domain::Platform,
    unified_connector_service_execution_mode: enums::ExecutionMode,
    merchant_order_reference_id: Option<String>,
    creds_identifier: Option<String>,
) -> RouterResult<()> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch Unified Connector Service client")?;

    let payment_repeat_request =
        payments_grpc::PaymentServiceRepeatEverythingRequest::foreign_try_from(&*router_data)
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct Payment Repeat Request")?;

    let merchant_connector_id = merchant_connector_account.get_mca_id();

    let connector_auth_metadata =
        build_unified_connector_service_auth_metadata(merchant_connector_account, platform)
            .change_context(ApiErrorResponse::InternalServerError)
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
    let (updated_router_data, _) = Box::pin(ucs_logging_wrapper(
        router_data.clone(),
        state,
        payment_repeat_request,
        headers_builder,
        |mut router_data, payment_repeat_request, grpc_headers| async move {
            let response = client
                .payment_repeat(
                    payment_repeat_request,
                    connector_auth_metadata.clone(),
                    grpc_headers,
                )
                .await
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to repeat payment")?;

            let payment_repeat_response = response.into_inner();

            let ucs_data = handle_unified_connector_service_response_for_payment_repeat(
                payment_repeat_response.clone(),
            )
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to deserialize UCS response")?;

            // Extract and store access token if present
            if let Some(access_token) = get_access_token_from_ucs_response(
                state,
                platform,
                &router_data.connector,
                merchant_connector_id.as_ref(),
                creds_identifier.clone(),
                payment_repeat_response.state.as_ref(),
            )
            .await
            {
                if let Err(error) = set_access_token_for_ucs(
                    state,
                    platform,
                    &router_data.connector,
                    access_token,
                    merchant_connector_id.as_ref(),
                    creds_identifier,
                )
                .await
                {
                    logger::error!(
                        ?error,
                        "Failed to store UCS access token from repeat response"
                    );
                } else {
                    logger::debug!("Successfully stored access token from UCS repeat response");
                }
            }

            let router_data_response = ucs_data.router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });
            router_data.response = router_data_response;
            router_data.amount_captured = payment_repeat_response.captured_amount;
            router_data.minor_amount_captured = payment_repeat_response
                .minor_captured_amount
                .map(MinorUnit::new);
            router_data.raw_connector_response = payment_repeat_response
                .raw_connector_response
                .clone()
                .map(|raw_connector_response| raw_connector_response.expose().into());
            router_data.connector_http_status_code = Some(ucs_data.status_code);

            // Populate connector_customer_id if present
            ucs_data.connector_customer_id.map(|connector_customer_id| {
                router_data.connector_customer = Some(connector_customer_id);
            });

            // Populate connector_response if present
            ucs_data.connector_response.map(|connector_response| {
                router_data.connector_response = Some(connector_response);
            });

            Ok((router_data, (), payment_repeat_response))
        },
    ))
    .await?;

    // Copy back the updated data
    *router_data = updated_router_data;
    Ok(())
}
