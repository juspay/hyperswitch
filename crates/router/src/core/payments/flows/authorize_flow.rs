use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{self, transformers},
    },
    routes::AppState,
    scheduler::metrics,
    services,
    types::{
        self, api,
        storage::{self, enums as storage_enums},
    },
};

#[async_trait]
impl<'st>
    ConstructFlowSpecificData<
        'st,
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for payments::PaymentData<api::Authorize>
{
    async fn construct_router_data(
        &self,
        state: &'st AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::PaymentsAuthorizeRouterData<'st>> {
        transformers::construct_payment_router_data::<api::Authorize, types::PaymentsAuthorizeData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
        )
        .await
    }
}

#[async_trait]
impl<'st> Feature<'st, api::Authorize, types::PaymentsAuthorizeData>
    for types::PaymentsAuthorizeRouterData<'st>
{
    type Output<'a> = types::PaymentsAuthorizeRouterData<'a>;
    async fn decide_flows(
        self,
        state: &'st AppState,
        connector: &api::ConnectorData,
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<Self::Output<'st>> {
        let resp = self
            .decide_flow(
                state,
                connector,
                customer,
                call_connector_action,
                storage_scheme,
            )
            .await?;

        metrics::PAYMENT_COUNT.add(&metrics::CONTEXT, 1, &[]); // Metrics

        Ok(resp)
    }
}

impl<'st> types::PaymentsAuthorizeRouterData<'st> {
    pub async fn decide_flow<'a>(
        self,
        state: &'st AppState,
        connector: &api::ConnectorData,
        maybe_customer: &'a Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _storage_scheme: storage_enums::MerchantStorageScheme,
    ) -> RouterResult<types::PaymentsAuthorizeRouterData<'st>> {
        let connector_integration: services::BoxedConnectorIntegration<
            'static,
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            self,
            call_connector_action,
        )
        .await
        .map_err(|error| error.to_payment_failed_response())?;

        mandate::mandate_procedure(state, resp, maybe_customer).await
    }
}

impl mandate::MandateBehaviour for types::PaymentsAuthorizeData {
    fn get_amount(&self) -> i64 {
        self.amount
    }
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }
    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethod {
        self.payment_method_data.clone()
    }
    fn get_setup_future_usage(&self) -> Option<storage_models::enums::FutureUsage> {
        self.setup_future_usage
    }
    fn get_setup_mandate_details(&self) -> Option<&api_models::payments::MandateData> {
        self.setup_mandate_details.as_ref()
    }

    fn set_mandate_id(&mut self, new_mandate_id: api_models::payments::MandateIds) {
        self.mandate_id = Some(new_mandate_id);
    }
}
