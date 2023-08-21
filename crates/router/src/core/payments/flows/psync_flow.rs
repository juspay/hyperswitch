use std::collections::HashMap;

use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, transformers, PaymentData},
    },
    routes::AppState,
    services,
    types::{self, api, domain, Capturable},
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
                        let mut capture_sync_response_list = HashMap::new();
                        let pending_connector_capture_id_list = pending_captures
                            .into_iter()
                            .filter_map(|capture| capture.connector_capture_id.clone());
                        for connector_capture_id in pending_connector_capture_id_list {
                            self.request.connector_transaction_id =
                                types::ResponseId::ConnectorTransactionId(
                                    connector_capture_id.clone(),
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
                            let capture_sync_response = match resp.response {
                                Err(err) => types::CaptureSyncResponse::Error {
                                    code: err.code,
                                    message: err.message,
                                    reason: err.reason,
                                    status_code: err.status_code,
                                },
                                Ok(types::PaymentsResponseData::TransactionResponse {
                                    resource_id,
                                    connector_response_reference_id,
                                    ..
                                }) => types::CaptureSyncResponse::Success {
                                    resource_id,
                                    status: resp.status,
                                    connector_response_reference_id,
                                },
                                // this error is meant for developers
                                _ => Err(ApiErrorResponse::PreconditionFailed { message: "Response type must be PaymentsResponseData::TransactionResponse for payment sync".into() })?,
                            };
                            capture_sync_response_list
                                .insert(connector_capture_id, capture_sync_response);
                        }
                        self.response = Ok(types::PaymentsResponseData::MultipleCaptureResponse {
                            capture_sync_response_list,
                        });
                        Ok(self)
                    }
                    services::CaptureSyncMethod::Bulk => {
                        // for bulk sync of captures, above logic needs to be handled at connector end
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
