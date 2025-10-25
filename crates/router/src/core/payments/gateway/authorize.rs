//! PaymentGateway implementation for api::Authorize flow
//!
//! This module implements the PaymentGateway trait for the Authorize flow,
//! handling both regular payments (payment_authorize) and mandate payments (payment_repeat).

use async_trait::async_trait;
use common_enums::{CallConnectorAction, ExecutionMode, ExecutionPath};
use common_utils::{errors::CustomResult, request::Request};
use error_stack::ResultExt;
use external_services::grpc_client::{self, LineageIds};
use hyperswitch_domain_models::{
    merchant_context::MerchantContext, payments::HeaderPayload, router_data::RouterData,
    router_flow_types as domain,
};
use hyperswitch_interfaces::{
    api::gateway as payment_gateway,
    connector_integration_interface::{BoxedConnectorIntegrationInterface, RouterDataConversion},
    errors::ConnectorError,
};
use common_utils::errors::ErrorSwitch;
use masking::Secret;
use unified_connector_service_client::payments as payments_grpc;
use crate::core::unified_connector_service::build_unified_connector_service_auth_metadata;
use super::{
    context::RouterGatewayContext,
    helpers::{build_grpc_auth_metadata, build_merchant_reference_id, get_grpc_client},
};
use crate::{
    core::{
        payments::helpers,
        unified_connector_service::{
            handle_unified_connector_service_response_for_payment_authorize,
            handle_unified_connector_service_response_for_payment_repeat, ucs_logging_wrapper,
        },
    },
    routes::SessionState,
    types::{self, transformers::ForeignTryFrom},
};

// /// Gateway struct for api::Authorize flow
// #[derive(Debug, Clone, Copy)]
// pub struct AuthorizeGateway;

/// Implementation of PaymentGateway for domain::Authorize flow
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::Authorize
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,>,
{
    async fn execute(
        self: Box<Self>,
        state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::Authorize,
            RCD,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        router_data: &RouterData<
            domain::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        context: RouterGatewayContext<'static>,
    ) -> CustomResult<
        RouterData<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        ConnectorError,
    > {
        // Determine which GRPC endpoint to call based on mandate_id
        let updated_router_data = if router_data.request.mandate_id.is_some() {
            // Call payment_repeat for mandate payments
            execute_payment_repeat(
                state,
                router_data,
                context.merchant_context,
                context.header_payload,
                context.lineage_ids,
                context.merchant_connector_account,
                context.execution_mode,
                context.execution_path,
            )
            .await?
        } else {
            // Call payment_authorize for regular payments
            execute_payment_authorize(
                state,
                router_data,
                context.merchant_context,
                context.header_payload,
                context.lineage_ids,
                context.merchant_connector_account,
                context.execution_mode,
                context.execution_path,
            )
            .await?
        };

        Ok(updated_router_data)
    }
}

/// Implementation of FlowGateway for domain::Authorize
///
/// This allows the flow to provide its specific gateway based on execution path
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::Authorize
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
            RouterGatewayContext<'static>,
        >,
    > {
        match execution_path {
            ExecutionPath::Direct => {
                Box::new(payment_gateway::DirectGateway)
            }
            ExecutionPath::UnifiedConnectorService
            | ExecutionPath::ShadowUnifiedConnectorService => {
                Box::new(domain::Authorize)
            }
        }
    }
}

/// Execute payment_authorize GRPC call
async fn execute_payment_authorize(
    state: &SessionState,
    router_data: &RouterData<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    >,
    merchant_context: &MerchantContext,
    header_payload: &HeaderPayload,
    lineage_ids: LineageIds,
    #[cfg(feature = "v1")]
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")]
    merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
    execution_mode: ExecutionMode,
    execution_path: ExecutionPath,
) -> CustomResult<
    RouterData<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ConnectorError,
> {
    // Get GRPC client
    let client = get_grpc_client(state)?;

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

    // Build GRPC request
    let payment_authorize_request =
        payments_grpc::PaymentServiceAuthorizeRequest::foreign_try_from(router_data)
            .change_context(ConnectorError::RequestEncodingFailed)?;

    // Execute GRPC call with logging wrapper
    let updated_router_data = Box::pin(ucs_logging_wrapper(
        router_data.clone(),
        state,
        payment_authorize_request,
        headers_builder,
        |mut router_data, payment_authorize_request, grpc_headers| async move {
            let response = client
                .payment_authorize(
                    payment_authorize_request,
                    connector_auth_metadata,
                    grpc_headers,
                )
                .await
                .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)?;

            let payment_authorize_response = response.into_inner();

            let (router_data_response, status_code) =
                handle_unified_connector_service_response_for_payment_authorize(
                    payment_authorize_response.clone(),
                )
                .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)?;

            let router_data_response = router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });

            router_data.response = router_data_response;
            router_data.raw_connector_response = payment_authorize_response
                .raw_connector_response
                .clone()
                .map(Secret::new);
            router_data.connector_http_status_code = Some(status_code);

            Ok((router_data, payment_authorize_response))
        },
    ))
    .await
    .map_err(|err| err.change_context(ConnectorError::ProcessingStepFailed(None)))?;

    Ok(updated_router_data)
}

/// Execute payment_repeat GRPC call
async fn execute_payment_repeat(
    state: &SessionState,
    router_data: &RouterData<
        domain::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    >,
    merchant_context: &MerchantContext,
    header_payload: &HeaderPayload,
    lineage_ids: LineageIds,
    #[cfg(feature = "v1")]
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")]
    merchant_connector_account: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccountTypeDetails,
    execution_mode: ExecutionMode,
    execution_path: ExecutionPath,
) -> CustomResult<
    RouterData<domain::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ConnectorError,
> {
    // Get GRPC client
    let client = get_grpc_client(state)?;

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

    // Build GRPC request
    let payment_repeat_request =
        payments_grpc::PaymentServiceRepeatEverythingRequest::foreign_try_from(router_data)
            .change_context(ConnectorError::RequestEncodingFailed)?;

    // Execute GRPC call with logging wrapper
    let updated_router_data = Box::pin(ucs_logging_wrapper(
        router_data.clone(),
        state,
        payment_repeat_request,
        headers_builder,
        |mut router_data, payment_repeat_request, grpc_headers| async move {
            let response = client
                .payment_repeat(payment_repeat_request, connector_auth_metadata, grpc_headers)
                .await
                .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)?;

            let payment_repeat_response = response.into_inner();

            let (router_data_response, status_code) =
                handle_unified_connector_service_response_for_payment_repeat(
                    payment_repeat_response.clone(),
                )
                .change_context(hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse::InternalServerError)?;

            let router_data_response = router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });

            router_data.response = router_data_response;
            router_data.raw_connector_response = payment_repeat_response
                .raw_connector_response
                .clone()
                .map(Secret::new);
            router_data.connector_http_status_code = Some(status_code);

            Ok((router_data, payment_repeat_response))
        },
    ))
    .await
    .map_err(|err| err.change_context(ConnectorError::ProcessingStepFailed(None)))?;

    Ok(updated_router_data)
}



/// Implementation of PaymentGateway for domain::AuthorizeSessionToken flow with todo!()
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::AuthorizeSessionToken
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,>,
{
    async fn execute(
        self: Box<Self>,
        _state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::AuthorizeSessionToken,
            RCD,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        _router_data: &RouterData<
            domain::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        _context: RouterGatewayContext<'static>,
    ) -> CustomResult<
        RouterData<
            domain::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!();
    }
}

/// Implementation of PaymentGateway for domain::PreProcessing flow with todo!()
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::PreProcessing
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,>,
{
    async fn execute(
        self: Box<Self>,
        _state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::PreProcessing,
            RCD,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
        _router_data: &RouterData<
            domain::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        _context: RouterGatewayContext<'static>,
    ) -> CustomResult<
        RouterData<
            domain::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!();
    }
}

/// Implementation of PaymentGateway for domain::PostProcessing flow with todo!()
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::PostProcessing,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::PostProcessing
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PostProcessing,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,>,
{
    async fn execute(
        self: Box<Self>,
        _state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::PostProcessing,
            RCD,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        >,
        _router_data: &RouterData<
            domain::PostProcessing,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        _context: RouterGatewayContext<'static>,
    ) -> CustomResult<
        RouterData<
            domain::PostProcessing,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!();
    }
}

/// Implementation of FlowGateway for domain::AuthorizeSessionToken with todo!()
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::AuthorizeSessionToken
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        _execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
            RouterGatewayContext<'static>,
        >,
    > {
        todo!();
    }
}

/// Implementation of FlowGateway for domain::PreProcessing with todo!()
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::PreProcessing
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        _execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
            RouterGatewayContext<'static>,
        >,
    > {
        todo!();
    }
}

/// Implementation of FlowGateway for domain::PostProcessing with todo!()
impl<RCD>
    payment_gateway::FlowGateway<   
        SessionState,
        RCD,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::PostProcessing
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<  
        domain::PostProcessing,
        types::PaymentsPostProcessingData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        _execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
            RouterGatewayContext<'static>,
        >,
    > {
        todo!();
    }
}

/// Implementation of PaymentGateway for domain::CreateOrder flow with todo!()
#[async_trait]
impl<RCD>
    payment_gateway::PaymentGateway<
        SessionState,
        RCD,
        domain::CreateOrder,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::CreateOrder
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::CreateOrder,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,>,
{
    async fn execute(
        self: Box<Self>,
        _state: &SessionState,
        _connector_integration: BoxedConnectorIntegrationInterface<
            domain::CreateOrder,
            RCD,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
        _router_data: &RouterData<
            domain::CreateOrder,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
        _call_connector_action: CallConnectorAction,
        _connector_request: Option<Request>,
        _return_raw_connector_response: Option<bool>,
        _context: RouterGatewayContext<'static>,
    ) -> CustomResult<
        RouterData<
            domain::CreateOrder,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
        >,
        ConnectorError,
    > {
        todo!();
    }
}

/// Implementation of FlowGateway for domain::CreateOrder with todo!()
impl<RCD>
    payment_gateway::FlowGateway<
        SessionState,
        RCD,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,
        RouterGatewayContext<'static>,
    > for domain::CreateOrder
where
    RCD: Clone + Send + Sync + 'static + RouterDataConversion<
        domain::CreateOrder,
        types::CreateOrderRequestData,
        types::PaymentsResponseData,>,
{
    fn get_gateway(
        _execution_path: ExecutionPath,
    ) -> Box<
        dyn payment_gateway::PaymentGateway<
            SessionState,
            RCD,
            Self,
            types::CreateOrderRequestData,
            types::PaymentsResponseData,
            RouterGatewayContext<'static>,
        >,
    > {
        todo!();
    }
}

