//! Macros for PaymentGateway implementations
//!
//! This module provides macros to reduce boilerplate when implementing
//! PaymentGateway and FlowGateway traits for different payment flows.

/// Implement PaymentGateway and FlowGateway for flows with TODO placeholders
///
/// This macro generates both PaymentGateway and FlowGateway implementations
/// for flows that are pending UCS GRPC endpoint availability.
///
/// # Arguments
/// - `flow`: The flow type (e.g., `domain::AuthorizeSessionToken`)
/// - `request_data`: The request data type (e.g., `types::AuthorizeSessionTokenData`)
/// - `response_data`: The response data type (e.g., `types::PaymentsResponseData`)
/// - `reason`: A string literal explaining why this is TODO
///
/// # Example
/// ```rust,ignore
/// impl_payment_gateway_todo! {
///     flow: domain::AuthorizeSessionToken,
///     request_data: types::AuthorizeSessionTokenData,
///     response_data: types::PaymentsResponseData,
///     reason: "UCS GRPC endpoint for session tokens not available - decision pending"
/// }
/// ```
#[macro_export]
macro_rules! impl_payment_gateway_todo {
    (
        flow: $flow:ty,
        request_data: $request:ty,
        response_data: $response:ty,
        reason: $reason:expr
    ) => {
        #[async_trait::async_trait]
        impl<RCD>
            hyperswitch_interfaces::api::gateway::PaymentGateway<
                $crate::routes::SessionState,
                RCD,
                $flow,
                $request,
                $response,
                $crate::core::payments::gateway::context::RouterGatewayContext,
            > for $flow
        where
            RCD: Clone
                + Send
                + Sync
                + 'static
                + hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<
                    $flow,
                    $request,
                    $response,
                >,
        {
            async fn execute(
                self: Box<Self>,
                _state: &$crate::routes::SessionState,
                _connector_integration: hyperswitch_interfaces::connector_integration_interface::BoxedConnectorIntegrationInterface<
                    $flow,
                    RCD,
                    $request,
                    $response,
                >,
                _router_data: &hyperswitch_domain_models::router_data::RouterData<
                    $flow,
                    $request,
                    $response,
                >,
                _call_connector_action: common_enums::CallConnectorAction,
                _connector_request: Option<common_utils::request::Request>,
                _return_raw_connector_response: Option<bool>,
                _context: $crate::core::payments::gateway::context::RouterGatewayContext,
            ) -> common_utils::errors::CustomResult<
                hyperswitch_domain_models::router_data::RouterData<
                    $flow,
                    $request,
                    $response,
                >,
                hyperswitch_interfaces::errors::ConnectorError,
            > {
                todo!($reason)
            }
        }

        impl<RCD>
            hyperswitch_interfaces::api::gateway::FlowGateway<
                $crate::routes::SessionState,
                RCD,
                $request,
                $response,
                $crate::core::payments::gateway::context::RouterGatewayContext,
            > for $flow
        where
            RCD: Clone
                + Send
                + Sync
                + 'static
                + hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<
                    $flow,
                    $request,
                    $response,
                >,
        {
            fn get_gateway(
                _execution_path: common_enums::ExecutionPath,
            ) -> Box<
                dyn hyperswitch_interfaces::api::gateway::PaymentGateway<
                    $crate::routes::SessionState,
                    RCD,
                    Self,
                    $request,
                    $response,
                    $crate::core::payments::gateway::context::RouterGatewayContext,
                >,
            > {
                todo!($reason)
            }
        }
    };
}

/// Implement PaymentGateway and FlowGateway for flows with custom UCS executors
///
/// This macro generates both PaymentGateway and FlowGateway implementations
/// for flows that have fully implemented UCS executors.
///
/// # Arguments
/// - `flow`: The flow type (e.g., `domain::Authorize`)
/// - `request_data`: The request data type (e.g., `types::PaymentsAuthorizeData`)
/// - `response_data`: The response data type (e.g., `types::PaymentsResponseData`)
/// - `execute_body`: The custom execution logic as a token tree
///
/// # Example
/// ```rust,ignore
/// impl_payment_gateway_with_executor! {
///     flow: domain::Authorize,
///     request_data: types::PaymentsAuthorizeData,
///     response_data: types::PaymentsResponseData,
///     execute_body: {
///         let execution_context = RouterUcsExecutionContext::new(
///             &context.merchant_context,
///             &context.header_payload,
///             context.lineage_ids,
///             &context.merchant_connector_account,
///             context.execution_mode,
///         );
///         AuthorizeUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_payment_gateway_with_executor {
    (
        flow: $flow:ty,
        request_data: $request:ty,
        response_data: $response:ty,
        execute_body: { $($body:tt)* }
    ) => {
        #[async_trait::async_trait]
        impl<RCD>
            hyperswitch_interfaces::api::gateway::PaymentGateway<
                $crate::routes::SessionState,
                RCD,
                $flow,
                $request,
                $response,
                $crate::core::payments::gateway::context::RouterGatewayContext,
            > for $flow
        where
            RCD: Clone
                + Send
                + Sync
                + 'static
                + hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<
                    $flow,
                    $request,
                    $response,
                >,
        {
            async fn execute(
                self: Box<Self>,
                state: &$crate::routes::SessionState,
                _connector_integration: hyperswitch_interfaces::connector_integration_interface::BoxedConnectorIntegrationInterface<
                    $flow,
                    RCD,
                    $request,
                    $response,
                >,
                router_data: &hyperswitch_domain_models::router_data::RouterData<
                    $flow,
                    $request,
                    $response,
                >,
                _call_connector_action: common_enums::CallConnectorAction,
                _connector_request: Option<common_utils::request::Request>,
                _return_raw_connector_response: Option<bool>,
                context: $crate::core::payments::gateway::context::RouterGatewayContext,
            ) -> common_utils::errors::CustomResult<
                hyperswitch_domain_models::router_data::RouterData<
                    $flow,
                    $request,
                    $response,
                >,
                hyperswitch_interfaces::errors::ConnectorError,
            > {
                $($body)*
            }
        }

        impl<RCD>
            hyperswitch_interfaces::api::gateway::FlowGateway<
                $crate::routes::SessionState,
                RCD,
                $request,
                $response,
                $crate::core::payments::gateway::context::RouterGatewayContext,
            > for $flow
        where
            RCD: Clone
                + Send
                + Sync
                + 'static
                + hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<
                    $flow,
                    $request,
                    $response,
                >,
        {
            fn get_gateway(
                execution_path: common_enums::ExecutionPath,
            ) -> Box<
                dyn hyperswitch_interfaces::api::gateway::PaymentGateway<
                    $crate::routes::SessionState,
                    RCD,
                    Self,
                    $request,
                    $response,
                    $crate::core::payments::gateway::context::RouterGatewayContext,
                >,
            > {
                match execution_path {
                    common_enums::ExecutionPath::Direct => {
                        Box::new(hyperswitch_interfaces::api::gateway::DirectGateway)
                    }
                    common_enums::ExecutionPath::UnifiedConnectorService
                    | common_enums::ExecutionPath::ShadowUnifiedConnectorService => {
                        // Return a boxed instance of the flow type
                        // Flow types are simple unit-like structs that can be constructed
                        Box::new($flow)
                    }
                }
            }
        }
    };
}

/// Implement PaymentGateway and FlowGateway with conditional executor routing
///
/// This macro generates both PaymentGateway and FlowGateway implementations
/// for flows that need to route between multiple UCS executors based on runtime conditions.
///
/// # Arguments
/// - `flow`: The flow type (e.g., `domain::Authorize`)
/// - `request_data`: The request data type (e.g., `types::PaymentsAuthorizeData`)
/// - `response_data`: The response data type (e.g., `types::PaymentsResponseData`)
/// - `routing_logic`: The conditional logic to determine which executor to use
///
/// # Example
/// ```rust,ignore
/// impl_payment_gateway_with_routing! {
///     flow: domain::Authorize,
///     request_data: types::PaymentsAuthorizeData,
///     response_data: types::PaymentsResponseData,
///     routing_logic: {
///         if router_data.request.mandate_id.is_some() {
///             RepeatUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
///         } else {
///             AuthorizeUcsExecutor::execute_ucs_flow(state, router_data, execution_context).await
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_payment_gateway_with_routing {
    (
        flow: $flow:ty,
        flow_expr: $flow_expr:expr,
        request_data: $request:ty,
        response_data: $response:ty,
        routing_logic: { $($routing:tt)* }
    ) => {
        #[async_trait::async_trait]
        impl<RCD>
            hyperswitch_interfaces::api::gateway::PaymentGateway<
                $crate::routes::SessionState,
                RCD,
                $flow,
                $request,
                $response,
                $crate::core::payments::gateway::context::RouterGatewayContext,
            > for $flow
        where
            RCD: Clone
                + Send
                + Sync
                + 'static
                + hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<
                    $flow,
                    $request,
                    $response,
                >,
        {
            async fn execute(
                self: Box<Self>,
                state: &$crate::routes::SessionState,
                _connector_integration: hyperswitch_interfaces::connector_integration_interface::BoxedConnectorIntegrationInterface<
                    $flow,
                    RCD,
                    $request,
                    $response,
                >,
                router_data: &hyperswitch_domain_models::router_data::RouterData<
                    $flow,
                    $request,
                    $response,
                >,
                _call_connector_action: common_enums::CallConnectorAction,
                _connector_request: Option<common_utils::request::Request>,
                _return_raw_connector_response: Option<bool>,
                context: $crate::core::payments::gateway::context::RouterGatewayContext,
            ) -> common_utils::errors::CustomResult<
                hyperswitch_domain_models::router_data::RouterData<
                    $flow,
                    $request,
                    $response,
                >,
                hyperswitch_interfaces::errors::ConnectorError,
            > {
                let execution_context = $crate::core::payments::gateway::ucs_execution_context::RouterUcsExecutionContext::new(
                    &context.merchant_context,
                    &context.header_payload,
                    context.lineage_ids,
                    &context.merchant_connector_account,
                    context.execution_mode,
                );

                (|state, router_data, execution_context| async move {
                    $($routing)*
                })(state, router_data, execution_context).await
            }
        }

        impl<RCD>
            hyperswitch_interfaces::api::gateway::FlowGateway<
                $crate::routes::SessionState,
                RCD,
                $request,
                $response,
                $crate::core::payments::gateway::context::RouterGatewayContext,
            > for $flow
        where
            RCD: Clone
                + Send
                + Sync
                + 'static
                + hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<
                    $flow,
                    $request,
                    $response,
                >,
        {
            fn get_gateway(
                execution_path: common_enums::ExecutionPath,
            ) -> Box<
                dyn hyperswitch_interfaces::api::gateway::PaymentGateway<
                    $crate::routes::SessionState,
                    RCD,
                    Self,
                    $request,
                    $response,
                    $crate::core::payments::gateway::context::RouterGatewayContext,
                >,
            > {
                match execution_path {
                    common_enums::ExecutionPath::Direct => {
                        Box::new(hyperswitch_interfaces::api::gateway::DirectGateway)
                    }
                    common_enums::ExecutionPath::UnifiedConnectorService
                    | common_enums::ExecutionPath::ShadowUnifiedConnectorService => {
                        Box::new($flow_expr)
                    }
                }
            }
        }
    };
}