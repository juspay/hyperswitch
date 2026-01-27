use std::collections::HashMap;

use async_trait::async_trait;
use common_enums;
use hyperswitch_domain_models::payments as domain_payments;
use hyperswitch_interfaces::api::gateway;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    connector::utils::RouterData,
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::SessionState,
    services::{self, api::ConnectorValidation, logger},
    types::{self, api, domain},
};

#[cfg(feature = "v1")]
#[async_trait]
impl ConstructFlowSpecificData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for PaymentData<api::PSync>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        processor: &domain::Processor,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<domain_payments::HeaderPayload>,
        _payment_method: Option<common_enums::PaymentMethod>,
        _payment_method_type: Option<common_enums::PaymentMethodType>,
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
            processor,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
            None,
            None,
        ))
        .await
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl ConstructFlowSpecificData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for hyperswitch_domain_models::payments::PaymentStatusData<api::PSync>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        processor: &domain::Processor,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<domain_payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    > {
        Box::pin(transformers::construct_router_data_for_psync(
            state,
            self.clone(),
            connector_id,
            processor,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
        ))
        .await
    }
}

#[async_trait]
impl Feature<api::PSync, types::PaymentsSyncData>
    for types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        _business_profile: &domain::Profile,
        _header_payload: domain_payments::HeaderPayload,
        return_raw_connector_response: Option<bool>,
        gateway_context: payments::flows::gateway_context::RouterGatewayContext,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
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
                let mut new_router_data = self
                    .execute_connector_processing_step_for_each_capture(
                        state,
                        pending_connector_capture_id_list,
                        call_connector_action,
                        connector_integration,
                        return_raw_connector_response,
                        gateway_context,
                    )
                    .await?;
                // Initiating Integrity checks
                let integrity_result = helpers::check_integrity_based_on_flow(
                    &new_router_data.request,
                    &new_router_data.response,
                );

                new_router_data.integrity_check = integrity_result;

                Ok(new_router_data)
            }
            (types::SyncRequestType::MultipleCaptureSync(_), Err(err)) => Err(err),
            _ => {
                // for bulk sync of captures, above logic needs to be handled at connector end
                let mut new_router_data = gateway::execute_payment_gateway(
                    state,
                    connector_integration,
                    &self,
                    call_connector_action,
                    connector_request,
                    return_raw_connector_response,
                    gateway_context,
                )
                .await
                .to_payment_failed_response()?;

                // Initiating Integrity checks
                let integrity_result = helpers::check_integrity_based_on_flow(
                    &new_router_data.request,
                    &new_router_data.response,
                );

                new_router_data.integrity_check = integrity_result;

                Ok(new_router_data)
            }
        }
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        _processor: &domain::Processor,
        creds_identifier: Option<&str>,
        gateway_context: &payments::gateway::context::RouterGatewayContext,
    ) -> RouterResult<types::AddAccessTokenResult> {
        Box::pin(access_token::add_access_token(
            state,
            connector,
            self,
            creds_identifier,
            gateway_context,
        ))
        .await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        let request = match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                //validate_psync_reference_id if call_connector_action is trigger
                if connector
                    .connector
                    .validate_psync_reference_id(
                        &self.request,
                        self.is_three_ds(),
                        self.status,
                        self.connector_meta_data.clone(),
                    )
                    .is_err()
                {
                    logger::warn!(
                        "validate_psync_reference_id failed, hence skipping call to connector"
                    );
                    return Ok((None, false));
                }
                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
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

#[async_trait]
pub trait RouterDataPSync
where
    Self: Sized,
{
    async fn execute_connector_processing_step_for_each_capture(
        &self,
        state: &SessionState,
        pending_connector_capture_id_list: Vec<String>,
        call_connector_action: payments::CallConnectorAction,
        connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::PSync,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
        return_raw_connector_response: Option<bool>,
        gateway_context: payments::flows::gateway_context::RouterGatewayContext,
    ) -> RouterResult<Self>;
}

#[async_trait]
impl RouterDataPSync
    for types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
{
    async fn execute_connector_processing_step_for_each_capture(
        &self,
        state: &SessionState,
        pending_connector_capture_id_list: Vec<String>,
        call_connector_action: payments::CallConnectorAction,
        connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::PSync,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
        return_raw_connector_response: Option<bool>,
        gateway_context: payments::flows::gateway_context::RouterGatewayContext,
    ) -> RouterResult<Self> {
        let mut capture_sync_response_map = HashMap::new();
        if let payments::CallConnectorAction::HandleResponse(_) = call_connector_action {
            // webhook consume flow, only call connector once. Since there will only be a single event in every webhook
            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                self,
                call_connector_action.clone(),
                None,
                return_raw_connector_response,
            )
            .await
            .to_payment_failed_response()?;
            Ok(resp)
        } else {
            // in trigger, call connector for every capture_id
            for connector_capture_id in pending_connector_capture_id_list {
                // TEMPORARY FIX: remove the clone on router data after removing this function as an impl on trait RouterDataPSync
                // TRACKING ISSUE: https://github.com/juspay/hyperswitch/issues/4644
                let mut cloned_router_data = self.clone();
                cloned_router_data.request.connector_transaction_id =
                    types::ResponseId::ConnectorTransactionId(connector_capture_id.clone());
                let resp = gateway::execute_payment_gateway(
                    state,
                    connector_integration.clone_box(),
                    &cloned_router_data,
                    call_connector_action.clone(),
                    None,
                    return_raw_connector_response,
                    gateway_context.clone(),
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
            let mut cloned_router_data = self.clone();
            cloned_router_data.response =
                Ok(types::PaymentsResponseData::MultipleCaptureResponse {
                    capture_sync_response_list: capture_sync_response_map,
                });
            Ok(cloned_router_data)
        }
    }
}
