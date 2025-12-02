use std::{collections::HashMap, str::FromStr};

use async_trait::async_trait;
use common_enums::{self, enums};
use common_utils::{id_type, types::MinorUnit, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client;
use hyperswitch_domain_models::payments as domain_payments;
use hyperswitch_interfaces::{
    api::gateway,
    unified_connector_service::{
        get_payments_response_from_ucs_webhook_content,
        handle_unified_connector_service_response_for_payment_get,
    },
};
use unified_connector_service_client::payments as payments_grpc;
use unified_connector_service_masking::ExposeInterface;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    connector::utils::RouterData,
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
        unified_connector_service::{
            build_unified_connector_service_auth_metadata, extract_connector_response_from_ucs,
            get_access_token_from_ucs_response, set_access_token_for_ucs, ucs_logging_wrapper,
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
        platform: &domain::Platform,
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
            platform,
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
        platform: &domain::Platform,
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
            platform,
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
        _platform: &domain::Platform,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        Box::pin(access_token::add_access_token(
            state,
            connector,
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
        header_payload: &domain_payments::HeaderPayload,
        lineage_ids: grpc_client::LineageIds,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        platform: &domain::Platform,
        _connector_data: &api::ConnectorData,
        unified_connector_service_execution_mode: enums::ExecutionMode,
        merchant_order_reference_id: Option<String>,
        call_connector_action: common_enums::CallConnectorAction,
        creds_identifier: Option<String>,
    ) -> RouterResult<()> {
        match call_connector_action {
            common_enums::CallConnectorAction::UCSConsumeResponse(transform_data_bytes) => {
                let webhook_content: payments_grpc::WebhookResponseContent =
                    serde_json::from_slice(&transform_data_bytes)
                        .change_context(ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to deserialize UCS webhook transform data")?;

                let payment_get_response =
                    get_payments_response_from_ucs_webhook_content(webhook_content)
                        .change_context(ApiErrorResponse::WebhookProcessingFailure)
                        .attach_printable(
                            "Failed to construct payments response from UCS webhook content",
                        )?;

                let (router_data_response, status_code) =
                    handle_unified_connector_service_response_for_payment_get(
                        payment_get_response.clone(),
                    )
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to deserialize UCS response")?;

                let router_data_response = router_data_response.map(|(response, status)| {
                    self.status = status;
                    response
                });

                let connector_response = extract_connector_response_from_ucs(
                    payment_get_response.connector_response.as_ref(),
                );

                self.response = router_data_response;
                self.amount_captured = payment_get_response.captured_amount;
                self.minor_amount_captured = payment_get_response
                    .minor_captured_amount
                    .map(MinorUnit::new);
                self.raw_connector_response = payment_get_response
                    .raw_connector_response
                    .clone()
                    .map(|raw_connector_response| raw_connector_response.expose().into());
                self.connector_http_status_code = Some(status_code);

                connector_response.map(|customer_response| {
                    self.connector_response = Some(customer_response);
                });
            }
            common_enums::CallConnectorAction::UCSHandleResponse(_)
            | common_enums::CallConnectorAction::Trigger => {
                let connector_name = self.connector.clone();
                let connector_enum =
                    common_enums::connector_enums::Connector::from_str(&connector_name)
                        .change_context(ApiErrorResponse::IncorrectConnectorNameGiven)?;

                let is_ucs_psync_disabled = state
                    .conf
                    .grpc_client
                    .unified_connector_service
                    .as_ref()
                    .is_some_and(|config| {
                        config
                            .ucs_psync_disabled_connectors
                            .contains(&connector_enum)
                    });

                if is_ucs_psync_disabled {
                    logger::info!(
                        "UCS PSync call disabled for connector: {}, skipping UCS call",
                        connector_name
                    );
                    return Ok(());
                }
                let client = state
                    .grpc_client
                    .unified_connector_service_client
                    .clone()
                    .ok_or(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to fetch Unified Connector Service client")?;

                let payment_get_request =
                    payments_grpc::PaymentServiceGetRequest::foreign_try_from((
                        &*self,
                        call_connector_action,
                    ))
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to construct Payment Get Request")?;

                let merchant_connector_id = merchant_connector_account.get_mca_id();

                let connector_auth_metadata = build_unified_connector_service_auth_metadata(
                    merchant_connector_account,
                    platform,
                )
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to construct request metadata")?;
                let merchant_reference_id = header_payload
                    .x_reference_id
                    .clone()
                    .or(merchant_order_reference_id)
                    .map(|id| id_type::PaymentReferenceId::from_str(id.as_str()))
                    .transpose()
                    .inspect_err(
                        |err| logger::warn!(error=?err, "Invalid Merchant ReferenceId found"),
                    )
                    .ok()
                    .flatten()
                    .map(ucs_types::UcsReferenceId::Payment);
                let header_payload = state
                    .get_grpc_headers_ucs(unified_connector_service_execution_mode)
                    .external_vault_proxy_metadata(None)
                    .merchant_reference_id(merchant_reference_id)
                    .lineage_ids(lineage_ids);
                let connector_name = self.connector.clone();
                let (updated_router_data, _) = Box::pin(ucs_logging_wrapper(
                    self.clone(),
                    state,
                    payment_get_request,
                    header_payload,
                    |mut router_data, payment_get_request, grpc_headers| async move {
                        let response = client
                            .payment_get(payment_get_request, connector_auth_metadata, grpc_headers)
                            .await
                            .change_context(ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to get payment")?;

                        let payment_get_response = response.into_inner();

                        let (router_data_response, status_code) =
                            handle_unified_connector_service_response_for_payment_get(
                                payment_get_response.clone(),
                            )
                            .change_context(ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to deserialize UCS response")?;

                        // Extract and store access token if present
                        if let Some(access_token) = get_access_token_from_ucs_response(
                            state,
                            platform,
                            &connector_name,
                            merchant_connector_id.as_ref(),
                            creds_identifier.clone(),
                            payment_get_response.state.as_ref(),
                        )
                        .await
                        {
                            if let Err(error) = set_access_token_for_ucs(
                                state,
                                platform,
                                &connector_name,
                                access_token,
                                merchant_connector_id.as_ref(),
                                creds_identifier,
                            )
                            .await
                            {
                                logger::error!(
                                    ?error,
                                    "Failed to store UCS access token from psync response"
                                );
                            } else {
                                logger::debug!(
                                    "Successfully stored access token from UCS psync response"
                                );
                            }
                        }

                        let router_data_response =
                            router_data_response.map(|(response, status)| {
                                router_data.status = status;
                                response
                            });
                        router_data.response = router_data_response;
                        router_data.amount_captured = payment_get_response.captured_amount;
                        router_data.minor_amount_captured = payment_get_response
                            .minor_captured_amount
                            .map(MinorUnit::new);
                        router_data.raw_connector_response = payment_get_response
                            .raw_connector_response
                            .clone()
                            .map(|raw_connector_response| raw_connector_response.expose().into());
                        router_data.connector_http_status_code = Some(status_code);

                        let connector_response = extract_connector_response_from_ucs(
                            payment_get_response.connector_response.as_ref(),
                        );

                        connector_response.map(|customer_response| {
                            router_data.connector_response = Some(customer_response);
                        });

                        Ok((router_data, (), payment_get_response))
                    },
                ))
                .await?;

                // Copy back the updated data
                *self = updated_router_data;
            }
            common_enums::CallConnectorAction::HandleResponse(_)
            | common_enums::CallConnectorAction::Avoid
            | common_enums::CallConnectorAction::StatusUpdate { .. } => {
                Err(ApiErrorResponse::InternalServerError).attach_printable(
                    "Invalid CallConnectorAction for payment sync via UCS Gateway system",
                )?
            }
        }
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
