use std::str::FromStr;

use async_trait::async_trait;
use common_enums::{self, enums};
use common_types::payments as common_payments_types;
use common_utils::{id_type, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client;
use hyperswitch_domain_models::payments as domain_payments;
use hyperswitch_interfaces::api::ConnectorSpecifications;
use router_env::logger;
use unified_connector_service_client::payments as payments_grpc;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, helpers, tokenization, transformers, PaymentData,
        },
        unified_connector_service::{
            build_unified_connector_service_auth_metadata, get_access_token_from_ucs_response,
            handle_unified_connector_service_response_for_payment_register,
            set_access_token_for_ucs, ucs_logging_wrapper,
        },
    },
    routes::SessionState,
    services,
    types::{
        self, api, domain,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
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
        platform: &domain::Platform,
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
        platform: &domain::Platform,
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
                platform,
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
        _gateway_context: payments::gateway::context::RouterGatewayContext,
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
        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action.clone(),
            connector_request,
            None,
        )
        .await
        .to_setup_mandate_failed_response()?;
        Ok(resp)
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

    async fn create_connector_customer<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<String>> {
        customers::create_connector_customer(
            state,
            connector,
            self,
            types::ConnectorCustomerData::try_from(self.request.to_owned())?,
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

    async fn call_unified_connector_service<'a>(
        &mut self,
        state: &SessionState,
        header_payload: &domain_payments::HeaderPayload,
        lineage_ids: grpc_client::LineageIds,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        platform: &domain::Platform,
        _connector_data: &api::ConnectorData,
        unified_connector_service_execution_mode: enums::ExecutionMode,
        merchant_order_reference_id: Option<String>,
        _call_connector_action: common_enums::CallConnectorAction,
        creds_identifier: Option<String>,
    ) -> RouterResult<()> {
        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        let payment_register_request =
            payments_grpc::PaymentServiceRegisterRequest::foreign_try_from(&*self)
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to construct Payment Setup Mandate Request")?;

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
        let header_payload = state
            .get_grpc_headers_ucs(unified_connector_service_execution_mode)
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(merchant_reference_id)
            .lineage_ids(lineage_ids);
        let connector_name = self.connector.clone();
        let (updated_router_data, _) = Box::pin(ucs_logging_wrapper(
            self.clone(),
            state,
            payment_register_request,
            header_payload,
            |mut router_data, payment_register_request, grpc_headers| async move {
                let response = client
                    .payment_setup_mandate(
                        payment_register_request,
                        connector_auth_metadata,
                        grpc_headers,
                    )
                    .await
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to Setup Mandate payment")?;

                let payment_register_response = response.into_inner();

                let ucs_data = handle_unified_connector_service_response_for_payment_register(
                    payment_register_response.clone(),
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
                    payment_register_response.state.as_ref(),
                )
                .await
                {
                    if let Err(error) = set_access_token_for_ucs(
                        state,
                        platform,
                        &connector_name,
                        access_token,
                        merchant_connector_id.as_ref(),
                        creds_identifier,
                    )
                    .await
                    {
                        logger::error!(
                            ?error,
                            "Failed to store UCS access token from setup mandate response"
                        );
                    } else {
                        logger::debug!(
                            "Successfully stored access token from UCS setup mandate response"
                        );
                    }
                }
                let router_data_response =
                    ucs_data.router_data_response.map(|(response, status)| {
                        router_data.status = status;
                        response
                    });
                router_data.response = router_data_response;
                router_data.connector_http_status_code = Some(ucs_data.status_code);

                // Populate connector_customer_id if present
                ucs_data.connector_customer_id.map(|connector_customer_id| {
                    router_data.connector_customer = Some(connector_customer_id);
                });

                Ok((router_data, (), payment_register_response))
            },
        ))
        .await?;

        // Copy back the updated data
        *self = updated_router_data;
        Ok(())
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
