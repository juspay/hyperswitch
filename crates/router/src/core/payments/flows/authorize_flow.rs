use async_trait::async_trait;
use error_stack;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, helpers, tokenization, transformers, PaymentData,
        },
    },
    logger,
    routes::{metrics, AppState},
    services,
    types::{self, api, domain},
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
        state: &AppState,
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
        transformers::construct_payment_router_data::<api::Authorize, types::PaymentsAuthorizeData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
            merchant_connector_account,
        )
        .await
    }
}
#[async_trait]
impl Feature<api::Authorize, types::PaymentsAuthorizeData> for types::PaymentsAuthorizeRouterData {
    async fn decide_flows<'a>(
        mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &domain::MerchantAccount,
        connector_request: Option<services::Request>,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        connector
            .connector
            .validate_capture_method(self.request.capture_method)
            .to_payment_failed_response()?;
        if self.request.surcharge_details.is_some() {
            connector
                .connector
                .validate_if_surcharge_implemented()
                .to_payment_failed_response()?;
        }

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

            let save_payment_result = tokenization::save_payment_method(
                state,
                connector,
                resp.to_owned(),
                maybe_customer,
                merchant_account,
                self.request.payment_method_type,
                key_store,
            )
            .await;

            let pm_id = match save_payment_result {
                Ok(payment_method_id) => Ok(payment_method_id),
                Err(error) => {
                    if resp.request.setup_mandate_details.clone().is_some() {
                        Err(error)
                    } else {
                        logger::error!(save_payment_method_error=?error);
                        Ok(None)
                    }
                }
            }?;

            Ok(mandate::mandate_procedure(
                state,
                resp,
                maybe_customer,
                pm_id,
                connector.merchant_connector_id.clone(),
            )
            .await?)
        } else {
            Ok(self.clone())
        }
    }

    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

    async fn add_payment_method_token<'a>(
        &mut self,
        state: &AppState,
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
        state: &AppState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        authorize_preprocessing_steps(state, &self, true, connector).await
    }

    async fn create_connector_customer<'a>(
        &self,
        state: &AppState,
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
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::Authorize,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                connector_integration
                    .execute_pretasks(self, state)
                    .await
                    .to_payment_failed_response()?;

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

impl types::PaymentsAuthorizeRouterData {
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
    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }
    fn get_setup_future_usage(&self) -> Option<diesel_models::enums::FutureUsage> {
        self.setup_future_usage
    }
    fn get_setup_mandate_details(&self) -> Option<&data_models::mandates::MandateData> {
        self.setup_mandate_details.as_ref()
    }

    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>) {
        self.mandate_id = new_mandate_id;
    }
}

pub async fn authorize_preprocessing_steps<F: Clone>(
    state: &AppState,
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
            payments::helpers::router_data_type_conversion::<_, api::PreProcessing, _, _, _, _>(
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

        let authorize_router_data =
            payments::helpers::router_data_type_conversion::<_, F, _, _, _, _>(
                resp.clone(),
                router_data.request.to_owned(),
                resp.response,
            );

        Ok(authorize_router_data)
    } else {
        Ok(router_data.clone())
    }
}

impl<F> TryFrom<&types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>>
    for types::ConnectorCustomerData
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(
        data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.request.email.clone(),
            payment_method_data: data.request.payment_method_data.clone(),
            description: None,
            phone: None,
            name: None,
            preprocessing_id: data.preprocessing_id.clone(),
        })
    }
}

impl TryFrom<types::PaymentsAuthorizeData> for types::PaymentMethodTokenizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(data: types::PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            browser_info: data.browser_info,
            currency: data.currency,
            amount: Some(data.amount),
        })
    }
}

impl TryFrom<types::PaymentsAuthorizeData> for types::PaymentsPreProcessingData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(data: types::PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: Some(data.payment_method_data),
            amount: Some(data.amount),
            email: data.email,
            currency: Some(data.currency),
            payment_method_type: data.payment_method_type,
            setup_mandate_details: data.setup_mandate_details,
            capture_method: data.capture_method,
            order_details: data.order_details,
            router_return_url: data.router_return_url,
            webhook_url: data.webhook_url,
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            surcharge_details: data.surcharge_details,
        })
    }
}
