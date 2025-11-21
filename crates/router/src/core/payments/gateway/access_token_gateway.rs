use std::str::FromStr;

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, id_type, request::Request, ucs_types};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::RouterData, router_flow_types as domain,
    router_request_types::AccessTokenRequestData,
};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    core::{payments::gateway::context::RouterGatewayContext, unified_connector_service},
    routes::SessionState,
    services::logger,
    types::transformers::ForeignTryFrom,
};

// =============================================================================
// PaymentGateway Implementation for domain::AccessTokenAuth
// =============================================================================

/// Implementation of PaymentGateway for AccessTokenAuth flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        Self,
        AccessTokenRequestData,
        hyperswitch_domain_models::router_data::AccessToken,
        RouterGatewayContext,
    > for domain::access_token_auth::AccessTokenAuth
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            Self,
            AccessTokenRequestData,
            hyperswitch_domain_models::router_data::AccessToken,
        >,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            AccessTokenRequestData,
            hyperswitch_domain_models::router_data::AccessToken,
        >,
        router_data: &RouterData<
            Self,
            AccessTokenRequestData,
            hyperswitch_domain_models::router_data::AccessToken,
        >,
        call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<
            Self,
            AccessTokenRequestData,
            hyperswitch_domain_models::router_data::AccessToken,
        >,
        ConnectorError,
    > {
        let merchant_connector_account = context.merchant_connector_account;
        let platform = context.platform;
        let lineage_ids = context.lineage_ids;
        let header_payload = context.header_payload;
        let unified_connector_service_execution_mode = context.execution_mode;

        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        let create_access_token_request =
            payments_grpc::PaymentServiceCreateAccessTokenRequest::foreign_try_from((
                router_data,
                call_connector_action,
            ))
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct Create Access Token Request")?;

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account.clone(),
                &platform,
            )
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct request metadata")?;

        let merchant_reference_id = header_payload
            .x_reference_id
            .clone()
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

        let updated_router_data = Box::pin(unified_connector_service::ucs_logging_wrapper_new(
            router_data.clone(),
            state,
            create_access_token_request,
            header_payload,
            |mut router_data, create_access_token_request, grpc_headers| async move {
                let response = client
                    .create_access_token(
                        create_access_token_request,
                        connector_auth_metadata,
                        grpc_headers,
                    )
                    .await
                    .attach_printable("Failed to create access token")?;

                let create_access_token_response = response.into_inner();

                let (access_token_result, status_code) =
                    unified_connector_service::handle_unified_connector_service_response_for_create_access_token(
                        create_access_token_response.clone(),
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                // Update router_data with access token
                match &access_token_result {
                    Ok(access_token) => {
                        router_data.access_token = Some(access_token.clone());
                    }
                    Err(_) => {
                        // Error case - access_token remains None
                    }
                }

                router_data.response = access_token_result;
                router_data.connector_http_status_code = Some(status_code);

                Ok((router_data, create_access_token_response))
            },
        ))
        .await
        .change_context(ConnectorError::ResponseHandlingFailed)?;

        Ok(updated_router_data)
    }
}

/// Implementation of FlowGateway for AccessTokenAuth
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        AccessTokenRequestData,
        hyperswitch_domain_models::router_data::AccessToken,
        RouterGatewayContext,
    > for domain::access_token_auth::AccessTokenAuth
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<
            Self,
            AccessTokenRequestData,
            hyperswitch_domain_models::router_data::AccessToken,
        >,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            AccessTokenRequestData,
            hyperswitch_domain_models::router_data::AccessToken,
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
