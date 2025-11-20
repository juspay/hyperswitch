use std::str::FromStr;

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, id_type, request::Request, ucs_types};
use error_stack::ResultExt;
use hyperswitch_domain_models::{router_data::RouterData, router_flow_types as domain};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
    unified_connector_service::handle_unified_connector_service_response_for_payment_get,
};
use unified_connector_service_client::payments as payments_grpc;
use unified_connector_service_masking::ExposeInterface as UcsMaskingExposeInterface;

use crate::{
    core::{payments::gateway::context::RouterGatewayContext, unified_connector_service},
    routes::SessionState,
    services::logger,
    types::{self, transformers::ForeignTryFrom, MinorUnit},
};

// =============================================================================
// PaymentGateway Implementation for domain::PSync
// =============================================================================

/// Implementation of PaymentGateway for api::PSync flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        Self,
        types::PaymentsSyncData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::PSync
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PaymentsSyncData, types::PaymentsResponseData>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<Self, types::PaymentsSyncData, types::PaymentsResponseData>,
        call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<Self, types::PaymentsSyncData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        let connector_name = router_data.connector.clone();
        let connector_enum = common_enums::connector_enums::Connector::from_str(&connector_name)
            .change_context(ConnectorError::InvalidConnectorName)?;
        let merchant_connector_account = context.merchant_connector_account;
        let creds_identifier = context.creds_identifier;
        let platform = context.platform;
        let lineage_ids = context.lineage_ids;
        let header_payload = context.header_payload;
        let unified_connector_service_execution_mode = context.execution_mode;
        let merchant_order_reference_id = header_payload.x_reference_id.clone();
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
            return Ok(router_data.clone());
        }
        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        let payment_get_request = payments_grpc::PaymentServiceGetRequest::foreign_try_from((
            router_data,
            call_connector_action,
        ))
        .change_context(ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to construct Payment Get Request")?;

        let merchant_connector_id = merchant_connector_account.get_mca_id();

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                &platform,
            )
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct request metadata")?;
        let merchant_reference_id = header_payload
            .x_reference_id
            .clone()
            .or(merchant_order_reference_id)
            .map(|id| id_type::PaymentReferenceId::from_str(id.as_str()))
            .transpose()
            .inspect_err(|err| logger::warn!(error=?err, "Invalid Merchant ReferenceId found"))
            .ok()
            .flatten()
            .map(ucs_types::UcsReferenceId::Payment);
        let header_payload = state
            .get_grpc_headers_ucs(unified_connector_service_execution_mode)
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(merchant_reference_id)
            .lineage_ids(lineage_ids);
        let connector_name = router_data.connector.clone();
        let updated_router_data = Box::pin(unified_connector_service::ucs_logging_wrapper_new(
            router_data.clone(),
            state,
            payment_get_request,
            header_payload,
            |mut router_data, payment_get_request, grpc_headers| async move {
                let response = client
                    .payment_get(payment_get_request, connector_auth_metadata, grpc_headers)
                    .await
                    .attach_printable("Failed to get payment")?;

                let payment_get_response = response.into_inner();

                let (router_data_response, status_code) =
                    handle_unified_connector_service_response_for_payment_get(
                        payment_get_response.clone(),
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                // Extract and store access token if present
                if let Some(access_token) =
                    unified_connector_service::get_access_token_from_ucs_response(
                        state,
                        &platform,
                        &connector_name,
                        merchant_connector_id.as_ref(),
                        creds_identifier.clone(),
                        payment_get_response.state.as_ref(),
                    )
                    .await
                {
                    if let Err(error) = unified_connector_service::set_access_token_for_ucs(
                        state,
                        &platform,
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
                        logger::debug!("Successfully stored access token from UCS psync response");
                    }
                }

                let router_data_response = router_data_response.map(|(response, status)| {
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

                Ok((router_data, payment_get_response))
            },
        ))
        .await
        .change_context(ConnectorError::ResponseHandlingFailed)?;

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
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, types::PaymentsSyncData, types::PaymentsResponseData>,
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
            ExecutionPath::Direct => Box::new(payment_gateway::DirectGateway),
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => Box::new(Self),
        }
    }
}
