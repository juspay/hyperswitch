use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, access_token, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{self, api, domain},
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentData<api::CompleteAuthorize>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<
        types::RouterData<
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        transformers::construct_payment_router_data::<
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
        >(
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
impl Feature<api::CompleteAuthorize, types::CompleteAuthorizeData>
    for types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    >
{
    async fn decide_flows<'a>(
        mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        _customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _merchant_account: &domain::MerchantAccount,
        connector_request: Option<services::Request>,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
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
        .to_payment_failed_response()?;

        Ok(resp)
    }

    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<Option<services::Request>> {
        match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::CompleteAuthorize,
                    types::CompleteAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                connector_integration
                    .build_request(self, &state.conf.connectors)
                    .to_payment_failed_response()
            }
            _ => Ok(None),
        }
    }
}
