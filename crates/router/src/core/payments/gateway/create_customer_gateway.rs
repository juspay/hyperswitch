use std::str::FromStr;

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::RouterData, router_flow_types as domain,
    router_request_types::ConnectorCustomerData,
};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    core::{
        payments::gateway::context::RouterGatewayContext, unified_connector_service,
        unified_connector_service::handle_unified_connector_service_response_for_create_connector_customer,
    },
    routes::SessionState,
    types::{self, transformers::ForeignTryFrom},
};

// =============================================================================
// PaymentGateway Implementation for domain::CreateConnectorCustomer
// =============================================================================

/// Implementation of PaymentGateway for CreateConnectorCustomer flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        Self,
        ConnectorCustomerData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::CreateConnectorCustomer
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, ConnectorCustomerData, types::PaymentsResponseData>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            ConnectorCustomerData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<Self, ConnectorCustomerData, types::PaymentsResponseData>,
        call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<Self, ConnectorCustomerData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        let connector_name = router_data.connector.clone();
        let _connector_enum = common_enums::connector_enums::Connector::from_str(&connector_name)
            .change_context(ConnectorError::InvalidConnectorName)?;
        let merchant_connector_account = context.merchant_connector_account;
        let platform = context.platform;
        let lineage_ids = context.lineage_ids;
        let unified_connector_service_execution_mode = context.execution_mode;

        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        let create_connector_customer_request =
            payments_grpc::PaymentServiceCreateConnectorCustomerRequest::foreign_try_from((
                router_data,
                call_connector_action,
            ))
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct Create Connector Customer Request")?;

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                &platform,
            )
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct request metadata")?;

        let grpc_headers = state
            .get_grpc_headers_ucs(unified_connector_service_execution_mode)
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(None)
            .lineage_ids(lineage_ids);
        let updated_router_data = Box::pin(unified_connector_service::ucs_logging_wrapper_new(
            router_data.clone(),
            state,
            create_connector_customer_request,
            grpc_headers,
            |mut router_data, create_connector_customer_request, grpc_headers| async move {
                let response = Box::pin(client.create_connector_customer(
                    create_connector_customer_request,
                    connector_auth_metadata,
                    grpc_headers,
                ))
                .await
                .attach_printable("Failed to create connector customer")?;

                let create_connector_customer_response = response.into_inner();

                let (connector_customer_result, status_code) =
                    handle_unified_connector_service_response_for_create_connector_customer(
                        create_connector_customer_response.clone(),
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                router_data.response = connector_customer_result;
                router_data.connector_http_status_code = Some(status_code);

                Ok((router_data, create_connector_customer_response))
            },
        ))
        .await
        .change_context(ConnectorError::ResponseHandlingFailed)?;

        Ok(updated_router_data)
    }
}

/// Implementation of FlowGateway for CreateConnectorCustomer
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        ConnectorCustomerData,
        types::PaymentsResponseData,
        RouterGatewayContext,
    > for domain::CreateConnectorCustomer
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, ConnectorCustomerData, types::PaymentsResponseData>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            ConnectorCustomerData,
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
