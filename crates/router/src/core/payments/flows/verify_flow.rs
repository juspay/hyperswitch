use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        mandate,
        payments::{self, access_token, customers, tokenization, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{self, api, domain},
};

#[async_trait]
impl ConstructFlowSpecificData<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>
    for PaymentData<api::Verify>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<types::VerifyRouterData> {
        transformers::construct_payment_router_data::<api::Verify, types::VerifyRequestData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
        )
        .await
    }
}

#[async_trait]
impl Feature<api::Verify, types::VerifyRequestData> for types::VerifyRouterData {
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &domain::MerchantAccount,
        connector_request: Option<services::Request>,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::Verify,
            types::VerifyRequestData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action,
            connector_request,
        )
        .await
        .to_verify_failed_response()?;

        let pm_id = tokenization::save_payment_method(
            state,
            connector,
            resp.to_owned(),
            maybe_customer,
            merchant_account,
            self.request.payment_method_type.clone(),
        )
        .await?;

        mandate::mandate_procedure(state, resp, maybe_customer, pm_id).await
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

    async fn create_connector_customer<'a>(
        &self,
        state: &AppState,
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
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::Verify,
                    types::VerifyRequestData,
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
}

impl TryFrom<types::VerifyRequestData> for types::ConnectorCustomerData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(data: types::VerifyRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.email,
            description: None,
            phone: None,
            name: None,
            preprocessing_id: None,
        })
    }
}

impl types::VerifyRouterData {
    pub async fn decide_flow<'a, 'b>(
        &'b self,
        state: &'a AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<domain::Customer>,
        confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<Self> {
        match confirm {
            Some(true) => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::Verify,
                    types::VerifyRequestData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();
                let resp = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    self,
                    call_connector_action,
                    None,
                )
                .await
                .to_verify_failed_response()?;

                let payment_method_type = self.request.payment_method_type.clone();
                let pm_id = tokenization::save_payment_method(
                    state,
                    connector,
                    resp.to_owned(),
                    maybe_customer,
                    merchant_account,
                    payment_method_type,
                )
                .await?;

                Ok(mandate::mandate_procedure(state, resp, maybe_customer, pm_id).await?)
            }
            _ => Ok(self.clone()),
        }
    }
}

impl mandate::MandateBehaviour for types::VerifyRequestData {
    fn get_amount(&self) -> i64 {
        0
    }

    fn get_setup_future_usage(&self) -> Option<storage_models::enums::FutureUsage> {
        self.setup_future_usage
    }

    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }

    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>) {
        self.mandate_id = new_mandate_id;
    }

    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }

    fn get_setup_mandate_details(&self) -> Option<&api_models::payments::MandateData> {
        self.setup_mandate_details.as_ref()
    }
}

impl TryFrom<types::VerifyRequestData> for types::PaymentMethodTokenizationData {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(data: types::VerifyRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
        })
    }
}
