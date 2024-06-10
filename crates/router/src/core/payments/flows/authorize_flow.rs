use async_trait::async_trait;

// use router_env::tracing::Instrument;
use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, helpers, tokenization, transformers, PaymentData,
        },
    },
    logger,
    routes::{metrics, SessionState},
    services,
    types::{self, api, domain, storage, transformers::ForeignFrom},
};

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
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
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
            merchant_account,
            key_store,
            customer,
            merchant_connector_account,
        ))
        .await
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
        _business_profile: &storage::business_profile::BusinessProfile,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        if self.should_proceed_with_authorize() {
            self.decide_authentication_type();
            logger::debug!(auth_type=?self.auth_type);
            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                &self,
                call_connector_action,
                connector_request,
            )
            .await
            .to_payment_failed_response()?;

            metrics::PAYMENT_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics
            Ok(resp)
        } else {
            Ok(self.clone())
        }
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

    async fn add_session_token<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self>
    where
        Self: Sized,
    {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
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
    ) -> RouterResult<Option<String>> {
        let request = self.request.clone();
        tokenization::add_payment_method_token(
            state,
            connector,
            tokenization_action,
            self,
            types::PaymentMethodTokenizationData::try_from(request)?,
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
                    .validate_capture_method(
                        self.request.capture_method,
                        self.request.payment_method_type,
                    )
                    .to_payment_failed_response()?;

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
                }

                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::Authorize,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                metrics::EXECUTE_PRETASK_COUNT.add(
                    &metrics::CONTEXT,
                    1,
                    &[
                        metrics::request::add_attributes(
                            "connector",
                            connector.connector_name.to_string(),
                        ),
                        metrics::request::add_attributes("flow", format!("{:?}", api::Authorize)),
                    ],
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
}

pub trait RouterDataAuthorize {
    fn decide_authentication_type(&mut self);

    /// to decide if we need to proceed with authorize or not, Eg: If any of the pretask returns `redirection_response` then we should not proceed with authorize call
    fn should_proceed_with_authorize(&self) -> bool;
}

impl RouterDataAuthorize for types::PaymentsAuthorizeRouterData {
    fn decide_authentication_type(&mut self) {
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
    fn get_customer_acceptance(&self) -> Option<api_models::payments::CustomerAcceptance> {
        self.customer_acceptance.clone().map(From::from)
    }
}

pub async fn authorize_preprocessing_steps<F: Clone>(
    state: &SessionState,
    router_data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    confirm: bool,
    connector: &api::ConnectorData,
) -> RouterResult<types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>> {
    if confirm {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
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
        )
        .await
        .to_payment_failed_response()?;

        metrics::PREPROCESSING_STEPS_COUNT.add(
            &metrics::CONTEXT,
            1,
            &[
                metrics::request::add_attributes("connector", connector.connector_name.to_string()),
                metrics::request::add_attributes(
                    "payment_method",
                    router_data.payment_method.to_string(),
                ),
                metrics::request::add_attributes(
                    "payment_method_type",
                    router_data
                        .request
                        .payment_method_type
                        .as_ref()
                        .map(|inner| inner.to_string())
                        .unwrap_or("null".to_string()),
                ),
            ],
        );
        let mut authorize_router_data = helpers::router_data_type_conversion::<_, F, _, _, _, _>(
            resp.clone(),
            router_data.request.to_owned(),
            resp.response.clone(),
        );
        if connector.connector_name == api_models::enums::Connector::Airwallex {
            authorize_router_data.reference_id = resp.reference_id;
        } else if connector.connector_name == api_models::enums::Connector::Nuvei {
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
