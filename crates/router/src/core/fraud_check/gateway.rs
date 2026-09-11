//! Gateway implementations for FRM flows.
//!
//! Mirrors `core::payments::gateway`: each flow that can execute on the Unified
//! Connector Service implements [`PaymentGateway`] for the UCS path and
//! [`FlowGateway`] to select between that and [`DirectGateway`].
//!
//! Only `Checkout` — the pre-authorization risk evaluation — has a UCS
//! equivalent today. The notify-style flows (`Transaction`, `Sale`,
//! `Fulfillment`, `RecordReturn`) stay on the direct path.

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::RouterData, router_flow_types::fraud_check as frm_api,
    router_request_types::fraud_check::FraudCheckCheckoutData,
    router_response_types::fraud_check::FraudCheckResponseData,
};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    core::{
        payments::gateway::context::RouterGatewayContext,
        unified_connector_service::{self, frm as ucs_frm},
    },
    routes::SessionState,
    services::logger,
    types::transformers::ForeignTryFrom,
};

/// UCS gateway for the FRM `Checkout` flow.
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        Self,
        FraudCheckCheckoutData,
        FraudCheckResponseData,
        RouterGatewayContext,
    > for frm_api::Checkout
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, FraudCheckCheckoutData, FraudCheckResponseData>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            Self,
            RCD,
            FraudCheckCheckoutData,
            FraudCheckResponseData,
        >,
        router_data: &RouterData<Self, FraudCheckCheckoutData, FraudCheckResponseData>,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext,
    ) -> CustomResult<
        RouterData<Self, FraudCheckCheckoutData, FraudCheckResponseData>,
        ConnectorError,
    > {
        let merchant_connector_account = context.merchant_connector_account;
        let processor = &context.processor;
        let lineage_ids = context.lineage_ids;
        let execution_mode = context.execution_mode;

        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        // Bearer-authenticated providers (Kount) need an OAuth token in
        // `state.access_token`; static-key providers (nSure) get `None` and
        // ignore it. Failure here is non-fatal — the connector surfaces a
        // clear auth error rather than us guessing.
        let mut router_data = router_data.clone();
        if router_data.access_token.is_none() {
            router_data.access_token = ucs_frm::get_frm_access_token(
                state,
                processor,
                &router_data,
                &merchant_connector_account,
                lineage_ids.clone(),
                execution_mode,
            )
            .await
            .unwrap_or_else(|err| {
                logger::warn!(
                    error = ?err,
                    "Failed to obtain an FRM access token; continuing without one"
                );
                None
            });
        }

        let pre_risk_check_request =
            payments_grpc::FrmServicePreRiskCheckRequest::foreign_try_from(&router_data)
                .change_context(ConnectorError::RequestEncodingFailed)
                .attach_printable("Failed to construct FRM pre risk check request")?;

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                processor.get_account().get_id(),
                router_data.connector.clone(),
            )
            .change_context(ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to construct request metadata")?;

        let header_payload = state
            .get_grpc_headers_ucs(execution_mode)
            .external_vault_proxy_metadata(None)
            .merchant_reference_id(None)
            .resource_id(None)
            .lineage_ids(lineage_ids);

        Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
            router_data,
            state,
            pre_risk_check_request,
            header_payload,
            execution_mode,
            |mut router_data, pre_risk_check_request, grpc_headers| async move {
                let response = match client
                    .frm_pre_risk_check(
                        pre_risk_check_request,
                        connector_auth_metadata,
                        grpc_headers,
                    )
                    .await
                {
                    Ok(resp) => resp,
                    // UCS connector errors are handled by the wrapper — see `ucs_logging_wrapper_granular`.
                    Err(report) => {
                        return Err(report.attach_printable("Failed to run FRM pre risk check"));
                    }
                };

                let pre_risk_check_response = response.into_inner();

                let frm_response =
                    unified_connector_service::handle_unified_connector_service_response_for_frm_pre_risk_check(
                        pre_risk_check_response.clone(),
                    )
                    .attach_printable("Failed to deserialize UCS response")?;

                let status_code =
                    hyperswitch_interfaces::unified_connector_service::transformers::convert_connector_service_status_code(
                        pre_risk_check_response.status_code,
                    )?;

                router_data.response = Ok(frm_response);
                router_data.connector_http_status_code = Some(status_code);

                Ok((router_data, (), pre_risk_check_response))
            },
        ))
        .await
        .map(|(router_data, _)| router_data)
        .map_err(payment_gateway::convert_ucs_error_to_connector_error)
    }
}

impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        FraudCheckCheckoutData,
        FraudCheckResponseData,
        RouterGatewayContext,
    > for frm_api::Checkout
where
    RCD: Clone
        + Send
        + Sync
        + 'static
        + RouterDataConversion<Self, FraudCheckCheckoutData, FraudCheckResponseData>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            FraudCheckCheckoutData,
            FraudCheckResponseData,
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
