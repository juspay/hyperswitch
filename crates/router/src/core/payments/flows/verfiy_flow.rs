use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{self, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{
        self, api,
        storage::{self, enums},
    },
};

#[async_trait]
impl<'st>
    ConstructFlowSpecificData<
        'st,
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for PaymentData<api::Verify>
{
    async fn construct_router_data(
        &self,
        state: &'st AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::VerifyRouterData<'st>> {
        transformers::construct_payment_router_data::<api::Verify, types::VerifyRequestData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
        )
        .await
    }
}

#[async_trait]
impl<'st> Feature<'st, api::Verify, types::VerifyRequestData> for types::VerifyRouterData<'st> {
    type Output<'rd> = types::VerifyRouterData<'rd>;
    async fn decide_flows(
        self,
        state: &'st AppState,
        connector: &api::ConnectorData,
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<Self> {
        self.decide_flow(
            state,
            connector,
            customer,
            call_connector_action,
            storage_scheme,
        )
        .await
    }
}

impl<'st> types::VerifyRouterData<'st> {
    pub async fn decide_flow(
        self,
        state: &'st AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<types::VerifyRouterData<'st>> {
        let connector_integration: services::BoxedConnectorIntegration<
            'static,
            api::Verify,
            types::VerifyRequestData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            self,
            call_connector_action,
        )
        .await
        .map_err(|err| err.to_verify_failed_response())?;
        mandate::mandate_procedure(state, resp, maybe_customer).await
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

    fn set_mandate_id(&mut self, new_mandate_id: api_models::payments::MandateIds) {
        self.mandate_id = Some(new_mandate_id);
    }

    fn get_payment_method_data(&self) -> api_models::payments::PaymentMethod {
        self.payment_method_data.clone()
    }

    fn get_setup_mandate_details(&self) -> Option<&api_models::payments::MandateData> {
        self.setup_mandate_details.as_ref()
    }
}
