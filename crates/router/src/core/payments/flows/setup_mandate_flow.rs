use async_trait::async_trait;
use common_enums;
use common_types::payments as common_payments_types;
use hyperswitch_domain_models::{payments as domain_payments, router_data_v2::PaymentFlowData};
use hyperswitch_interfaces::api::{self as api_interface, gateway, ConnectorSpecifications};
use router_env::logger;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, gateway as payments_gateway,
            gateway::context as gateway_context, helpers, session_token, tokenization,
            transformers, PaymentData,
        },
    },
    routes::SessionState,
    services,
    types::{self, api, domain},
};

#[cfg(feature = "v1")]
#[async_trait]
impl
    ConstructFlowSpecificData<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for PaymentData<api::SetupMandate>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        processor: &domain::Processor,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<domain_payments::HeaderPayload>,
        _payment_method: Option<common_enums::PaymentMethod>,
        _payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<types::SetupMandateRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::SetupMandate,
            types::SetupMandateRequestData,
        >(
            state,
            self.clone(),
            connector_id,
            processor,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
            None,
            None,
        ))
        .await
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl
    ConstructFlowSpecificData<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for hyperswitch_domain_models::payments::PaymentConfirmData<api::SetupMandate>
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
    ) -> RouterResult<types::SetupMandateRouterData> {
        Box::pin(
            transformers::construct_payment_router_data_for_setup_mandate(
                state,
                self.clone(),
                connector_id,
                processor,
                customer,
                merchant_connector_account,
                merchant_recipient_data,
                header_payload,
            ),
        )
        .await
    }
}

#[async_trait]
impl Feature<api::SetupMandate, types::SetupMandateRequestData> for types::SetupMandateRouterData {
    async fn decide_flows<'a>(
        mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        _business_profile: &domain::Profile,
        _header_payload: domain_payments::HeaderPayload,
        _return_raw_connector_response: Option<bool>,
        gateway_context: gateway_context::RouterGatewayContext,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        // Change the authentication_type to ThreeDs, for google_pay wallet if card_holder_authenticated or account_verified in assurance_details is false
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
        let resp = gateway::execute_payment_gateway(
            state,
            connector_integration,
            &self,
            call_connector_action.clone(),
            connector_request,
            None,
            gateway_context,
        )
        .await
        .to_setup_mandate_failed_response()?;
        Ok(resp)
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
        if connector
            .connector
            .should_call_tokenization_before_setup_mandate()
        {
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
        } else {
            Ok(types::PaymentMethodTokenResult {
                payment_method_token_result: Ok(None),
                is_payment_method_tokenization_performed: false,
                connector_response: None,
            })
        }
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
            api_interface::CurrentFlowInfo::SetupMandate {
                auth_type: &self.auth_type,
            },
        ) {
            logger::info!(
                "Pre-authentication flow is required for connector: {}",
                connector.connector_name
            );
            let setup_mandate_request_data = self.request.clone();
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
            let pre_authenticate_response = pre_authenticate_router_data.response.clone();
            let mut setup_mandate_router_data =
                helpers::router_data_type_conversion::<_, api::SetupMandate, _, _, _, _>(
                    pre_authenticate_router_data,
                    setup_mandate_request_data,
                    pre_authenticate_response,
                );

            if let Ok(types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                enrolled_v2,
                related_transaction_id,
            }) = &setup_mandate_router_data.response
            {
                let (enrolled_for_3ds, related_transaction_id) =
                    (*enrolled_v2, related_transaction_id.clone());
                setup_mandate_router_data.request.enrolled_for_3ds = enrolled_for_3ds;
                setup_mandate_router_data.request.related_transaction_id = related_transaction_id;
            }

            let should_continue = setup_mandate_router_data.response.is_ok();

            Ok((setup_mandate_router_data, should_continue))
        } else {
            logger::info!(
                "Pre-authentication flow is not required for connector: {} for Setup Mandate flow",
                connector.connector_name
            );
            Ok((self, true))
        }
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
            types::ConnectorCustomerData::try_from(self.request.to_owned())?,
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
                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                    api::SetupMandate,
                    types::SetupMandateRequestData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                Ok((
                    connector_integration
                        .build_request(self, &state.conf.connectors)
                        .to_payment_failed_response()?,
                    true,
                ))
            }
            _ => Ok((None, true)),
        }
    }

    async fn preprocessing_steps<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        setup_mandate_preprocessing_steps(state, &self, true, connector).await
    }
}

impl mandate::MandateBehaviour for types::SetupMandateRequestData {
    fn get_amount(&self) -> i64 {
        0
    }

    fn get_setup_future_usage(&self) -> Option<diesel_models::enums::FutureUsage> {
        self.setup_future_usage
    }

    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }

    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>) {
        self.mandate_id = new_mandate_id;
    }

    fn get_payment_method_data(&self) -> domain::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }

    fn get_setup_mandate_details(
        &self,
    ) -> Option<&hyperswitch_domain_models::mandates::MandateData> {
        self.setup_mandate_details.as_ref()
    }
    fn get_customer_acceptance(&self) -> Option<common_payments_types::CustomerAcceptance> {
        self.customer_acceptance.clone()
    }
}

pub async fn setup_mandate_preprocessing_steps<F: Clone>(
    state: &SessionState,
    router_data: &types::RouterData<F, types::SetupMandateRequestData, types::PaymentsResponseData>,
    confirm: bool,
    connector: &api::ConnectorData,
) -> RouterResult<types::RouterData<F, types::SetupMandateRequestData, types::PaymentsResponseData>>
{
    if confirm {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let preprocessing_request_data =
            types::PaymentsPreProcessingData::try_from(router_data.request.clone())?;

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

        let mut setup_mandate_router_data = helpers::router_data_type_conversion::<_, F, _, _, _, _>(
            resp.clone(),
            router_data.request.to_owned(),
            resp.response.clone(),
        );

        if connector.connector_name == api_models::enums::Connector::Nuvei {
            let (enrolled_for_3ds, related_transaction_id) =
                match &setup_mandate_router_data.response {
                    Ok(types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                        enrolled_v2,
                        related_transaction_id,
                    }) => (*enrolled_v2, related_transaction_id.clone()),
                    _ => (false, None),
                };
            setup_mandate_router_data.request.enrolled_for_3ds = enrolled_for_3ds;
            setup_mandate_router_data.request.related_transaction_id = related_transaction_id;
        }

        Ok(setup_mandate_router_data)
    } else {
        Ok(router_data.clone())
    }
}
