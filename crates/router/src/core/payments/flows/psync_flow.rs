use std::collections::HashMap;

use async_trait::async_trait;
use error_stack::ResultExt;
use masking::Secret;
use unified_connector_service_client::payments as payments_grpc;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    connector::utils::RouterData,
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
        unified_connector_service::{
            build_unified_connector_service_auth_metadata,
            handle_unified_connector_service_response_for_payment_get,
        },
    },
    routes::SessionState,
    services::{self, api::ConnectorValidation, logger},
    types::{self, api, domain, transformers::ForeignTryFrom},
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
        merchant_context: &domain::MerchantContext,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
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
            merchant_context,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
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
        merchant_context: &domain::MerchantContext,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    > {
        Box::pin(transformers::construct_router_data_for_psync(
            state,
            self.clone(),
            connector_id,
            merchant_context,
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
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        return_raw_connector_response: Option<bool>,
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
                let mut new_router_data = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    &self,
                    call_connector_action,
                    connector_request,
                    return_raw_connector_response,
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
        merchant_context: &domain::MerchantContext,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        Box::pin(access_token::add_access_token(
            state,
            connector,
            merchant_context,
            self,
            creds_identifier,
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

    async fn call_unified_connector_service<'a>(
        &mut self,
        state: &SessionState,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        merchant_context: &domain::MerchantContext,
    ) -> RouterResult<()> {
        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        let payment_get_request = payments_grpc::PaymentServiceGetRequest::foreign_try_from(self)
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct Payment Get Request")?;

        let connector_auth_metadata = build_unified_connector_service_auth_metadata(
            merchant_connector_account,
            merchant_context,
        )
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to construct request metadata")?;

        let response = client
            .payment_get(
                payment_get_request,
                connector_auth_metadata,
                state.get_grpc_headers(),
            )
            .await
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get payment")?;

        let payment_get_response = response.into_inner();

        let (status, router_data_response, status_code) =
            handle_unified_connector_service_response_for_payment_get(payment_get_response.clone())
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to deserialize UCS response")?;

        self.status = status;
        self.response = router_data_response;
        self.raw_connector_response = payment_get_response.raw_connector_response.map(Secret::new);
        self.connector_http_status_code = Some(status_code);

        Ok(())
    }
}

#[async_trait]
pub trait RouterDataPSync
where
    Self: Sized,
{
    async fn execute_connector_processing_step_for_each_capture(
        &self,
        _state: &SessionState,
        _pending_connector_capture_id_list: Vec<String>,
        _call_connector_action: payments::CallConnectorAction,
        _connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::PSync,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
        _return_raw_connector_response: Option<bool>,
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
                let resp = services::execute_connector_processing_step(
                    state,
                    connector_integration.clone_box(),
                    &cloned_router_data,
                    call_connector_action.clone(),
                    None,
                    return_raw_connector_response,
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
