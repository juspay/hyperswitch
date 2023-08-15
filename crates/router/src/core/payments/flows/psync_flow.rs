use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, access_token, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{self, api, domain, storage::enums, Capturable},
    utils::OptionExt,
};

#[async_trait]
impl ConstructFlowSpecificData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for PaymentData<api::PSync>
{
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
    ) -> RouterResult<
        types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    > {
        transformers::construct_payment_router_data::<api::PSync, types::PaymentsSyncData>(
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
impl Feature<api::PSync, types::PaymentsSyncData>
    for types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
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
            api::PSync,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        match self.request.get_multiple_capture_data() {
            Some(multiple_capture_data) => {
                match connector_integration
                    .get_capture_sync_method()
                    .to_payment_failed_response()?
                {
                    services::CaptureSyncMethod::Individual => {
                        let pending_captures = multiple_capture_data.get_pending_captures();
                        let mut capture_status_update_list = Vec::new();
                        for capture in pending_captures.into_iter() {
                            self.request.connector_transaction_id =
                                types::ResponseId::ConnectorTransactionId(
                                    capture
                                        .connector_transaction_id
                                        .clone()
                                        .get_required_value("connector_transaction_id")?,
                                );
                            let resp = services::execute_connector_processing_step(
                                state,
                                connector_integration.clone(),
                                &self,
                                call_connector_action.clone(),
                                None,
                            )
                            .await
                            .to_payment_failed_response()?;
                            let attempt_status = match resp.response {
                                Ok(_) => resp.status,
                                Err(_) => enums::AttemptStatus::Pending,
                            };
                            capture_status_update_list.push((capture.to_owned(), attempt_status));
                        }
                        self.multiple_capture_sync_response = Some(capture_status_update_list);
                        Ok(self)
                    }
                    services::CaptureSyncMethod::Bulk => {
                        // RouterData::multiple_capture_sync_response needs to be populated at connector side
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
                }
            }
            None => {
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

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        let request = match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                let connector_integration: services::BoxedConnectorIntegration<
                    '_,
                    api::PSync,
                    types::PaymentsSyncData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                connector_integration
                    .build_request(self, &state.conf.connectors)
                    .to_payment_failed_response()?
            }
            _ => None,
        };

        Ok((request, true))
    }
}
