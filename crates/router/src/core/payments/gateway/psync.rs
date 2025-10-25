//! PaymentGateway implementation for api::PSync flow
//!
//! This module implements the PaymentGateway trait for the PSync (Payment Sync) flow,
//! handling payment status synchronization via the payment_get GRPC endpoint.

use async_trait::async_trait;
use std::str::FromStr;
use common_enums::{connector_enums::Connector, CallConnectorAction, ExecutionMode, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use external_services::grpc_client::LineageIds;
use hyperswitch_domain_models::{
    router_flow_types as domain,
    merchant_context::MerchantContext, payments::HeaderPayload, router_data::RouterData,
};
use hyperswitch_interfaces::{
    api::{self, gateway as payment_gateway},
    api_client::ApiClientWrapper,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
    unified_connector_service::handle_unified_connector_service_response_for_payment_get,
};
use masking::Secret;
use unified_connector_service_client::payments as payments_grpc;
use crate::core::payments::gateway::RouterGatewayContext;
use crate::core::unified_connector_service::build_unified_connector_service_auth_metadata;

use super::helpers::{build_merchant_reference_id, get_grpc_client};
use crate::{
    core::{payments::helpers, unified_connector_service::ucs_logging_wrapper},
    routes::SessionState,
    types::{self, transformers::ForeignTryFrom},
};

/// Implementation of PaymentGateway for api::PSync flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::PSync,
        types::PaymentsSyncData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PSync
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PSync,
        types::PaymentsSyncData,
        types::PaymentsResponseData,>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::PSync,
            RCD,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        // Check if UCS PSync is disabled for this connector
        let connector_enum = Connector::from_str(&router_data.connector)
            .change_context(ConnectorError::InvalidConnectorName)?;

        if is_psync_disabled(state, &connector_enum) {
            return Err(ConnectorError::NotImplemented(format!(
                "UCS PSync disabled for connector: {}",
                router_data.connector
            ))
            .into());
        }

        // Extract required context - all fields are directly available in RouterGatewayContext
        let merchant_context = context.merchant_context;
        let header_payload = context.header_payload;
        let lineage_ids = context.lineage_ids;
        let merchant_connector_account = context.merchant_connector_account;

        // Execute payment_get GRPC call
        let updated_router_data = execute_payment_get(
            state,
            router_data,
            &merchant_connector_account,
            &merchant_context,
            &header_payload,
            lineage_ids,
            context.execution_mode,
            context.execution_path,
        )
        .await?;

        Ok(updated_router_data)
    }
}

/// Implementation of FlowGateway for api::PSync
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsSyncData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PSync
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PSync,
        types::PaymentsSyncData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
            RouterGatewayContext,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => {
                Box::new(payment_gateway::DirectGateway)
            }
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => {
                Box::new(domain::PSync)
            }
        }
    }
}

/// Execute payment_get GRPC call
#[cfg(feature = "v1")]
async fn execute_payment_get(
    state: &SessionState,
    router_data: &RouterData<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    merchant_context: &MerchantContext,
    header_payload: &HeaderPayload,
    lineage_ids: LineageIds,
    execution_mode: ExecutionMode,
    _execution_path: ExecutionPath,
) -> CustomResult<
    RouterData<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    ConnectorError,
> {
    // Get GRPC client
    let client = get_grpc_client(state)?;

    // Build GRPC request
    let payment_get_request = payments_grpc::PaymentServiceGetRequest::foreign_try_from(router_data)
        .change_context(ConnectorError::RequestEncodingFailed)?;

    // Build auth metadata
    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        merchant_context,
    )
    .change_context(ConnectorError::FailedToObtainAuthType)?;

    // Build GRPC headers
    let merchant_order_reference_id = build_merchant_reference_id(header_payload);

    let headers_builder = state
        .get_grpc_headers_ucs(execution_mode)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_order_reference_id)
        .lineage_ids(lineage_ids);

    // Execute GRPC call with logging wrapper
    let updated_router_data = Box::pin(ucs_logging_wrapper(
        router_data.clone(),
        state,
        payment_get_request,
        headers_builder,
        |mut router_data, payment_get_request, grpc_headers| async move {
            let response = client
                .payment_get(payment_get_request, connector_auth_metadata, grpc_headers)
                .await
                .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)?;

            let payment_get_response = response.into_inner();

            let (router_data_response, status_code) =
                handle_unified_connector_service_response_for_payment_get(
                    payment_get_response.clone(),
                )
                .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)?;

            let router_data_response = router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });

            router_data.response = router_data_response;
            router_data.raw_connector_response = payment_get_response
                .raw_connector_response
                .clone()
                .map(Secret::new);
            router_data.connector_http_status_code = Some(status_code);

            Ok((router_data, payment_get_response))
        },
    ))
    .await
    .map_err(|err| err.change_context(ConnectorError::ProcessingStepFailed(None)))?;

    Ok(updated_router_data)
}

/// Execute payment_get GRPC call (v2)
#[cfg(feature = "v2")]
async fn execute_payment_get(
    state: &SessionState,
    router_data: &RouterData<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
    merchant_context: &MerchantContext,
    header_payload: &HeaderPayload,
    lineage_ids: LineageIds,
    execution_mode: ExecutionMode,
    _execution_path: ExecutionPath,
) -> CustomResult<
    RouterData<domain::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    ConnectorError,
> {
    // Get GRPC client
    let client = get_grpc_client(state)?;

    // Build GRPC request
    let payment_get_request = payments_grpc::PaymentServiceGetRequest::foreign_try_from(router_data)
        .change_context(ConnectorError::RequestEncodingFailed)?;

    // Build auth metadata
    let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        merchant_context,
    )
    .change_context(ConnectorError::FailedToObtainAuthType)?;

    // Build GRPC headers
    let merchant_order_reference_id = build_merchant_reference_id(header_payload);

    let headers_builder = state
        .get_grpc_headers_ucs(execution_mode)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_order_reference_id)
        .lineage_ids(lineage_ids);

    // Execute GRPC call with logging wrapper
    let updated_router_data = Box::pin(ucs_logging_wrapper(
        router_data.clone(),
        state,
        payment_get_request,
        headers_builder,
        |mut router_data, payment_get_request, grpc_headers| async move {
            let response = client
                .payment_get(payment_get_request, connector_auth_metadata, grpc_headers)
                .await
                .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)?;

            let payment_get_response = response.into_inner();

            let (router_data_response, status_code) =
                handle_unified_connector_service_response_for_payment_get(
                    payment_get_response.clone(),
                )
                .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)?;

            let router_data_response = router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });

            router_data.response = router_data_response;
            router_data.raw_connector_response = payment_get_response
                .raw_connector_response
                .clone()
                .map(Secret::new);
            router_data.connector_http_status_code = Some(status_code);

            Ok((router_data, payment_get_response))
        },
    ))
    .await
    .map_err(|err| err.change_context(ConnectorError::ProcessingStepFailed(None)))?;

    Ok(updated_router_data)
}

/// Check if UCS PSync is disabled for a connector
fn is_psync_disabled(state: &SessionState, connector: &Connector) -> bool {
    state
        .conf
        .grpc_client
        .unified_connector_service
        .as_ref()
        .is_some_and(|config| config.ucs_psync_disabled_connectors.contains(connector))
}