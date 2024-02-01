use std::collections::HashMap;

use async_trait::async_trait;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::AppState,
    services::{self, logger},
    types::{self, api, domain},
};

#[async_trait]
impl ConstructFlowSpecificData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for PaymentData<api::PSync>
{
        /// Constructs router data for payments synchronization using the provided parameters and returns a result containing the router data for the specified payment types.
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<
        types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    > {
        Box::pin(transformers::construct_payment_router_data::<
            api::PSync,
            types::PaymentsSyncData,
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
impl Feature<api::PSync, types::PaymentsSyncData>
    for types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
{
        /// Asynchronously decides the flows for handling payment sync requests based on the provided parameters.
    async fn decide_flows<'a>(
        mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        _customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        _merchant_account: &domain::MerchantAccount,
        connector_request: Option<services::Request>,
        _key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::PSync,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let capture_sync_method_result = connector_integration
            .get_multiple_capture_sync_method()
            .to_payment_failed_response();

        match (self.request.sync_type.clone(), capture_sync_method_result) {
            (
                types::SyncRequestType::MultipleCaptureSync(pending_connector_capture_id_list),
                Ok(services::CaptureSyncMethod::Individual),
            ) => {
                let resp = self
                    .execute_connector_processing_step_for_each_capture(
                        state,
                        pending_connector_capture_id_list,
                        call_connector_action,
                        connector_integration,
                    )
                    .await?;
                Ok(resp)
            }
            (types::SyncRequestType::MultipleCaptureSync(_), Err(err)) => Err(err),
            _ => {
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

        /// Asynchronously adds an access token for a given merchant account using the provided connector data and application state.
    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self).await
    }

        /// Asynchronously builds a flow-specific connector request based on the provided state, connector data, and call connector action. 
    /// Returns a tuple containing an optional services::Request and a boolean indicating success.
    async fn build_flow_specific_connector_request(
        &mut self,
        state: &AppState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        let request = match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                //validate_psync_reference_id if call_connector_action is trigger
                if connector
                    .connector
                    .validate_psync_reference_id(self)
                    .is_err()
                {
                    logger::warn!(
                        "validate_psync_reference_id failed, hence skipping call to connector"
                    );
                    return Ok((None, false));
                }
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

impl types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData> {
        /// Asynchronously executes the connector processing step for each capture, based on the given parameters.
    /// If the call connector action is to handle response, it only calls the connector once. Otherwise, it calls the connector for every capture ID in the pending list and updates the capture sync response map accordingly.
    /// 
    /// # Arguments
    /// * `state` - The application state
    /// * `pending_connector_capture_id_list` - The list of pending connector capture IDs
    /// * `call_connector_action` - The action to be performed when calling the connector
    /// * `connector_integration` - The boxed connector integration
    /// 
    /// # Returns
    /// The updated Self with the capture sync response map and response set accordingly.
    async fn execute_connector_processing_step_for_each_capture(
        mut self,
        state: &AppState,
        pending_connector_capture_id_list: Vec<String>,
        call_connector_action: payments::CallConnectorAction,
        connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::PSync,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    ) -> RouterResult<Self> {
        let mut capture_sync_response_map = HashMap::new();
        if let payments::CallConnectorAction::HandleResponse(_) = call_connector_action {
            // webhook consume flow, only call connector once. Since there will only be a single event in every webhook
            let resp = services::execute_connector_processing_step(
                state,
                connector_integration.clone(),
                &self,
                call_connector_action.clone(),
                None,
            )
            .await
            .to_payment_failed_response()?;
            Ok(resp)
        } else {
            // in trigger, call connector for every capture_id
            for connector_capture_id in pending_connector_capture_id_list {
                self.request.connector_transaction_id =
                    types::ResponseId::ConnectorTransactionId(connector_capture_id.clone());
                let resp = services::execute_connector_processing_step(
                    state,
                    connector_integration.clone(),
                    &self,
                    call_connector_action.clone(),
                    None,
                )
                .await
                .to_payment_failed_response()?;
                match resp.response {
                    Err(err) => {
                        capture_sync_response_map.insert(connector_capture_id, types::CaptureSyncResponse::Error {
                            code: err.code,
                            message: err.message,
                            reason: err.reason,
                            status_code: err.status_code,
                            amount: None,
                        });
                    },
                    Ok(types::PaymentsResponseData::MultipleCaptureResponse { capture_sync_response_list })=> {
                        capture_sync_response_map.extend(capture_sync_response_list.into_iter());
                    }
                    _ => Err(ApiErrorResponse::PreconditionFailed { message: "Response type must be PaymentsResponseData::MultipleCaptureResponse for payment sync".into() })?,
                };
            }
            self.response = Ok(types::PaymentsResponseData::MultipleCaptureResponse {
                capture_sync_response_list: capture_sync_response_map,
            });
            Ok(self)
        }
    }
}
