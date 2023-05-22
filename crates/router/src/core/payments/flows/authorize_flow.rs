use async_trait::async_trait;
use error_stack;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        mandate,
        payments::{self, access_token, customers, tokenization, transformers, PaymentData},
    },
    logger,
    routes::{metrics, AppState},
    services,
    types::{self, api, storage},
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
        merchant_account: &storage::MerchantAccount,
        customer: &Option<storage::Customer>,
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
            customer,
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
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<Self> {
        let resp = self
            .decide_flow(
                state,
                connector,
                customer,
                Some(true),
                call_connector_action,
                merchant_account,
            )
            .await;

        metrics::PAYMENT_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics

        resp
    }

    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

    async fn add_payment_method_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        tokenization_action: &payments::TokenizationAction,
    ) -> RouterResult<Option<String>> {
        tokenization::add_payment_method_token(
            state,
            connector,
            tokenization_action,
            self,
            types::PaymentMethodTokenizationData::try_from(self.request.to_owned())?,
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
        connector_customer_map: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> RouterResult<(Option<String>, Option<storage::CustomerUpdate>)> {
        customers::create_connector_customer(
            state,
            connector,
            self,
            types::ConnectorCustomerData::try_from(self)?,
            connector_customer_map,
        )
        .await
    }
}

impl types::PaymentsAuthorizeRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b mut self,
        state: &'a AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<storage::Customer>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<Self> {
        match confirm {
            Some(true) => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::Authorize,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();
                connector_integration
                    .execute_pretasks(self, state)
                    .await
                    .map_err(|error| error.to_payment_failed_response())?;
                logger::debug!(completed_pre_tasks=?true);
                if self.should_proceed_with_authorize() {
                    self.decide_authentication_type();
                    logger::debug!(auth_type=?self.auth_type);
                    let resp = services::execute_connector_processing_step(
                        state,
                        connector_integration,
                        self,
                        call_connector_action,
                    )
                    .await
                    .map_err(|error| error.to_payment_failed_response())?;

                    let pm_id = tokenization::save_payment_method(
                        state,
                        connector,
                        resp.to_owned(),
                        maybe_customer,
                        merchant_account,
                    )
                    .await?;

                    Ok(mandate::mandate_procedure(state, resp, maybe_customer, pm_id).await?)
                } else {
                    Ok(self.clone())
                }
            }
            _ => Ok(self.clone()),
        }
    }

    fn decide_authentication_type(&mut self) {
        if self.auth_type == storage_models::enums::AuthenticationType::ThreeDs
            && !self.request.enrolled_for_3ds
        {
            self.auth_type = storage_models::enums::AuthenticationType::NoThreeDs
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

pub enum Action {
    Update,
    Insert,
    Skip,
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
    fn get_setup_future_usage(&self) -> Option<storage_models::enums::FutureUsage> {
        self.setup_future_usage
    }
    fn get_setup_mandate_details(&self) -> Option<&api_models::payments::MandateData> {
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
        )
        .await
        .map_err(|error| error.to_payment_failed_response())?;

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
        })
    }
}

impl TryFrom<types::PaymentsAuthorizeData> for types::PaymentsPreProcessingData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(data: types::PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.email,
            currency: Some(data.currency),
        })
    }
}
