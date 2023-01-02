use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
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
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for PaymentData<api::Void>
{
    async fn construct_router_data(
        &self,
        state: &'st AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::PaymentsCancelRouterData<'st>> {
        transformers::construct_payment_router_data::<api::Void, types::PaymentsCancelData>(
            state,
            self.clone(),
            connector_id,
            merchant_account,
        )
        .await
    }
}

#[async_trait]
impl<'st> Feature<'st, api::Void, types::PaymentsCancelData>
    for types::PaymentsCancelRouterData<'st>
{
    type Output<'rd> = types::PaymentsCancelRouterData<'st>;
    async fn decide_flows(
        self,
        state: &'st AppState,
        connector: &api::ConnectorData,
        customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> RouterResult<Self> {
        self.decide_flow(
            state,
            connector,
            customer,
            Some(true),
            call_connector_action,
        )
        .await
    }
}

impl<'st> types::PaymentsCancelRouterData<'st> {
    pub async fn decide_flow(
        self,
        state: &'st AppState,
        connector: &api::ConnectorData,
        _maybe_customer: &Option<storage::Customer>,
        _confirm: Option<bool>,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<types::PaymentsCancelRouterData<'st>> {
        let connector_integration: services::BoxedConnectorIntegration<
            'static,
            api::Void,
            types::PaymentsCancelData,
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

        Ok(resp)
    }
}
