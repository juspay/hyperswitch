pub mod approve_flow;
pub mod authorize_flow;
pub mod cancel_flow;
pub mod cancel_post_capture_flow;
pub mod cancel_post_capture_sync_flow;
pub mod capture_flow;
pub mod complete_authorize_flow;
pub mod extend_authorization_flow;
pub mod external_proxy_flow;
pub mod incremental_authorization_flow;
pub mod post_session_tokens_flow;
pub mod pre_authorize_void_flow;
pub mod psync_flow;
pub mod reject_flow;
pub mod session_flow;
pub mod session_update_flow;
pub mod setup_mandate_flow;
pub mod update_metadata_flow;
pub mod update_post_confirm_flow;

use async_trait::async_trait;
use common_enums;
use common_types::payments::CustomerAcceptance;
use error_stack::ResultExt;
use external_services::grpc_client;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::router_flow_types::{
    BillingConnectorInvoiceSync, BillingConnectorPaymentsSync, InvoiceRecordBack,
};
use hyperswitch_domain_models::{
    payments as domain_payments,
    router_request_types::{CurrentFlowInfo, PaymentsCaptureData},
};

use crate::{
    core::{
        errors::{ApiErrorResponse, RouterResult},
        payments::{self, gateway::context as gateway_context, helpers},
    },
    logger,
    routes::SessionState,
    services, types as router_types,
    types::{self, api, api::enums as api_enums, domain},
};

/// Maps the response of a connector CreateOrder call, made ahead of a payment flow, to the
/// order id to carry forward and whether the payment flow should continue.
pub(crate) fn get_create_order_result(
    order_create_response: &Result<types::PaymentsResponseData, types::ErrorResponse>,
    should_continue_payment: bool,
) -> RouterResult<types::CreateOrderResult> {
    match order_create_response {
        Ok(types::PaymentsResponseData::PaymentsCreateOrderResponse {
            order_id,
            session_token,
        }) => {
            let should_continue_further = if session_token.is_some() {
                // if SDK session token is returned in order create response, do not continue and return control to SDK
                false
            } else {
                should_continue_payment
            };
            Ok(types::CreateOrderResult {
                create_order_result: Ok(order_id.clone()),
                should_continue_further,
            })
        }
        // Some connector return PreProcessingResponse and TransactionResponse response type
        // Rest of the match statements are temporary fixes for satisfying current connector side response handling
        // Create Order response must always be PaymentsResponseData::PaymentsCreateOrderResponse only
        Ok(types::PaymentsResponseData::PreProcessingResponse {
            pre_processing_id,
            session_token,
            ..
        }) => {
            let should_continue_further = if session_token.is_some() {
                // if SDK session token is returned in order create response, do not continue and return control to SDK
                false
            } else {
                should_continue_payment
            };
            Ok(types::CreateOrderResult {
                create_order_result: Ok(pre_processing_id.get_string_repr().clone()),
                should_continue_further,
            })
        }
        Ok(types::PaymentsResponseData::TransactionResponse {
            resource_id,
            redirection_data,
            ..
        }) => {
            let order_id = resource_id
                .get_connector_transaction_id()
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("unable to get connector_transaction_id during order create")?;
            let should_continue_further = if redirection_data.is_some() {
                // if redirection_data is returned in order create response, do not continue and return control to SDK
                false
            } else {
                should_continue_payment
            };
            Ok(types::CreateOrderResult {
                create_order_result: Ok(order_id),
                should_continue_further,
            })
        }
        Ok(res) => Err(error_stack::report!(ApiErrorResponse::InternalServerError)
            .attach_printable(format!(
                "Unexpected response format from connector: {res:?}",
            ))),
        Err(error) => Ok(types::CreateOrderResult {
            create_order_result: Err(error.clone()),
            should_continue_further: false,
        }),
    }
}

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait ConstructFlowSpecificData<F, Req, Res> {
    #[cfg(feature = "v1")]
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        processor: &domain::Processor,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<domain_payments::HeaderPayload>,
        payment_method: Option<common_enums::PaymentMethod>,
        payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<types::RouterData<F, Req, Res>>;

    #[cfg(feature = "v2")]
    async fn construct_router_data<'a>(
        &self,
        _state: &SessionState,
        _connector_id: &str,
        _processor: &domain::Processor,
        _customer: &Option<domain::Customer>,
        _merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        _merchant_recipient_data: Option<types::MerchantRecipientData>,
        _header_payload: Option<domain_payments::HeaderPayload>,
    ) -> RouterResult<types::RouterData<F, Req, Res>>;

    async fn get_merchant_recipient_data<'a>(
        &self,
        _state: &SessionState,
        _processor: &domain::Processor,
        _merchant_connector_account: &helpers::MerchantConnectorAccountType,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        Ok(None)
    }

    #[cfg(feature = "v2")]
    fn add_guest_customer(
        &self,
        _router_data: &mut types::RouterData<F, Req, Res>,
        _guest_customer: &Option<hyperswitch_domain_models::payments::GuestCustomer>,
    ) -> RouterResult<()> {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait Feature<F, T> {
    fn current_flow_info(&self) -> Option<CurrentFlowInfo> {
        None
    }

    async fn decide_flows<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        business_profile: &domain::Profile,
        header_payload: domain_payments::HeaderPayload,
        return_raw_connector_response: Option<bool>,
        gateway_context: gateway_context::RouterGatewayContext,
    ) -> RouterResult<Self>
    where
        Self: Sized,
        F: Clone,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;

    /// In case of payment methods like gifcard, some connectors might support balance check API
    /// This function can be overridden in those specific connectors to implement balance check flow
    /// Authorize must be called only if balance is sufficient
    async fn balance_check_flow<'a>(
        &self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<types::BalanceCheckResult>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(types::BalanceCheckResult {
            balance_check_result: Ok(None),
            should_continue_payment: true,
        })
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        processor: &domain::Processor,
        creds_identifier: Option<&str>,
        gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<types::AddAccessTokenResult>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;

    async fn add_session_token<'a>(
        &mut self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<()>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(())
    }

    async fn add_payment_method_token<'a>(
        &mut self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _tokenization_action: &payments::TokenizationAction,
        _should_continue_payment: bool,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<types::PaymentMethodTokenResult>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(types::PaymentMethodTokenResult {
            payment_method_token_result: Ok(None),
            is_payment_method_tokenization_performed: false,
            connector_response: None,
        })
    }

    async fn pre_authentication_step<'a>(
        self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<(Self, bool)>
    where
        F: Clone,
        Self: Sized,
    {
        Ok((self, true))
    }

    async fn authentication_step<'a>(
        self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<(Self, bool)>
    where
        F: Clone,
        Self: Sized,
    {
        Ok((self, true))
    }

    async fn post_authentication_step<'a>(
        self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<(Self, bool)>
    where
        F: Clone,
        Self: Sized,
    {
        Ok((self, true))
    }

    async fn push_notification_step<'a>(
        self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<(Self, bool)>
    where
        F: Clone,
        Self: Sized,
    {
        Ok((self, true))
    }

    async fn generate_qr_step<'a>(
        self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<(Self, bool)>
    where
        F: Clone,
        Self: Sized,
    {
        Ok((self, true))
    }

    async fn postprocessing_steps<'a>(
        self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Self>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(self)
    }

    async fn create_connector_customer<'a>(
        &self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<Option<String>>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(None)
    }

    /// Returns the connector request and a bool which specifies whether to proceed with further
    async fn build_flow_specific_connector_request(
        &mut self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        Ok((None, true))
    }

    async fn settlement_split_call<'a>(
        self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<(Self, bool)>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        // By default, settlement split call is not required
        Ok((self, true))
    }

    async fn create_order_at_connector(
        &mut self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _should_continue_payment: bool,
        _gateway_context: &gateway_context::RouterGatewayContext,
    ) -> RouterResult<Option<types::CreateOrderResult>>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(None)
    }

    fn update_router_data_with_create_order_response(
        &mut self,
        _create_order_result: types::CreateOrderResult,
    ) {
    }

    #[cfg(feature = "v2")]
    async fn call_unified_connector_service_with_external_vault_proxy<'a>(
        &mut self,
        _state: &SessionState,
        _header_payload: &domain_payments::HeaderPayload,
        _lineage_ids: grpc_client::LineageIds,
        _merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        _external_vault_merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        _processor: &domain::Processor,
        _unified_connector_service_execution_mode: common_enums::ExecutionMode,
    ) -> RouterResult<()>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(())
    }

    #[cfg(feature = "v1")]
    async fn call_unified_connector_service_with_external_vault_proxy_v1<'a>(
        &mut self,
        _state: &SessionState,
        _header_payload: &domain_payments::HeaderPayload,
        _lineage_ids: grpc_client::LineageIds,
        _merchant_connector_account: &'a helpers::MerchantConnectorAccountType,
        _external_vault_merchant_connector_account: &'a helpers::MerchantConnectorAccountType,
        _processor: &domain::Processor,
        _unified_connector_service_execution_mode: common_enums::ExecutionMode,
    ) -> RouterResult<()>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(())
    }
}

/// Determines whether a capture API call should be made for a payment attempt
/// This function evaluates whether an authorized payment should proceed with a capture API call
/// based on various payment parameters. It's primarily used in two-step (auth + capture) payment flows for CaptureMethod SequentialAutomatic
///
pub fn should_initiate_capture_flow(
    connector_name: &router_types::Connector,
    customer_acceptance: Option<CustomerAcceptance>,
    capture_method: Option<api_enums::CaptureMethod>,
    setup_future_usage: Option<api_enums::FutureUsage>,
    status: common_enums::AttemptStatus,
) -> bool {
    match status {
        common_enums::AttemptStatus::Authorized => match capture_method {
            Some(api_enums::CaptureMethod::SequentialAutomatic) => match connector_name {
                router_types::Connector::Paybox => {
                    // Check CIT conditions for Paybox
                    setup_future_usage == Some(api_enums::FutureUsage::OffSession)
                        && customer_acceptance.is_some()
                }
                _ => false,
            },
            // Affirm (BNPL) authorizes the loan on the transaction-create (CompleteAuthorize)
            // leg and needs a follow-up capture (POST /transactions/{id}/capture) to settle for
            // automatic capture, so chain the capture flow when it comes back Authorized.
            Some(api_enums::CaptureMethod::Automatic) => {
                matches!(connector_name, router_types::Connector::Affirm)
            }
            _ => false,
        },
        _ => false,
    }
}

/// Executes a capture request by building a connector-specific request and deciding
/// the appropriate flow to send it to the payment connector.
pub async fn call_capture_request(
    mut capture_router_data: types::RouterData<
        api::Capture,
        PaymentsCaptureData,
        types::PaymentsResponseData,
    >,
    state: &SessionState,
    connector: &api::ConnectorData,
    call_connector_action: payments::CallConnectorAction,
    business_profile: &domain::Profile,
    header_payload: domain_payments::HeaderPayload,
    context: gateway_context::RouterGatewayContext,
) -> RouterResult<types::RouterData<api::Capture, PaymentsCaptureData, types::PaymentsResponseData>>
{
    // Build capture-specific connector request
    let (connector_request, _should_continue_further) = capture_router_data
        .build_flow_specific_connector_request(state, connector, call_connector_action.clone())
        .await?;

    // Execute capture flow
    capture_router_data
        .decide_flows(
            state,
            connector,
            call_connector_action,
            connector_request,
            business_profile,
            header_payload.clone(),
            None,
            context, // gateway_context
        )
        .await
}

/// Processes the response from the capture flow and determines the final status and the response.
fn handle_post_capture_response(
    authorize_router_data_response: types::PaymentsResponseData,
    post_capture_router_data: Result<
        types::RouterData<api::Capture, PaymentsCaptureData, types::PaymentsResponseData>,
        error_stack::Report<ApiErrorResponse>,
    >,
) -> RouterResult<(common_enums::AttemptStatus, types::PaymentsResponseData)> {
    match post_capture_router_data {
        Err(err) => {
            logger::error!(
                "Capture flow encountered an error: {:?}. Proceeding without updating.",
                err
            );
            Ok((
                common_enums::AttemptStatus::Authorized,
                authorize_router_data_response,
            ))
        }
        Ok(post_capture_router_data) => {
            match (
                &post_capture_router_data.response,
                post_capture_router_data.status,
            ) {
                (Ok(post_capture_resp), common_enums::AttemptStatus::Charged) => Ok((
                    common_enums::AttemptStatus::Charged,
                    types::PaymentsResponseData::merge_transaction_responses(
                        &authorize_router_data_response,
                        post_capture_resp,
                    )?,
                )),
                _ => {
                    logger::error!(
                        "Error in post capture_router_data response: {:?}, Current Status: {:?}. Proceeding without updating.",
                        post_capture_router_data.response,
                        post_capture_router_data.status,
                    );
                    Ok((
                        common_enums::AttemptStatus::Authorized,
                        authorize_router_data_response,
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod create_order_result_tests {
    use api_models::payments::SessionToken;
    use hyperswitch_domain_models::{
        router_data::ErrorResponse,
        router_request_types::ResponseId,
        router_response_types::{PaymentsResponseData, PreprocessingResponseId, RedirectForm},
    };

    use super::get_create_order_result;
    use crate::core::errors::ApiErrorResponse;

    fn transaction_response(
        resource_id: ResponseId,
        redirection_data: Option<RedirectForm>,
    ) -> PaymentsResponseData {
        PaymentsResponseData::TransactionResponse {
            resource_id,
            redirection_data: Box::new(redirection_data),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            network_txn_link_id: None,
            connector_response_reference_id: None,
            payment_account_reference: None,
            incremental_authorization_allowed: None,
            authentication_data: None,
            charges: None,
        }
    }

    /// Asserts `Ok(CreateOrderResult { create_order_result: Ok(order_id), should_continue_further })`.
    fn assert_order(
        response: Result<PaymentsResponseData, ErrorResponse>,
        should_continue_payment: bool,
        expected_order_id: &str,
        expected_continue: bool,
    ) {
        let result = get_create_order_result(&response, should_continue_payment);
        assert!(
            matches!(
                &result,
                Ok(create_order)
                    if matches!(&create_order.create_order_result, Ok(order_id) if order_id == expected_order_id)
                        && create_order.should_continue_further == expected_continue
            ),
            "response {response:?}, should_continue_payment {should_continue_payment}"
        );
    }

    #[test]
    fn create_order_response_passes_should_continue_through_without_session_token() {
        let response = || {
            Ok(PaymentsResponseData::PaymentsCreateOrderResponse {
                order_id: "order_1".to_string(),
                session_token: None,
            })
        };
        assert_order(response(), true, "order_1", true);
        assert_order(response(), false, "order_1", false);
    }

    #[test]
    fn create_order_response_with_session_token_stops_the_flow() {
        let response = Ok(PaymentsResponseData::PaymentsCreateOrderResponse {
            order_id: "order_1".to_string(),
            session_token: Some(SessionToken::NoSessionTokenReceived),
        });
        assert_order(response, true, "order_1", false);
    }

    #[test]
    fn pre_processing_response_uses_pre_processing_id() {
        let response = |session_token| {
            Ok(PaymentsResponseData::PreProcessingResponse {
                pre_processing_id: PreprocessingResponseId::PreProcessingId("pre_1".to_string()),
                connector_metadata: None,
                session_token,
                connector_response_reference_id: None,
            })
        };
        assert_order(response(None), true, "pre_1", true);
        assert_order(
            response(Some(SessionToken::NoSessionTokenReceived)),
            true,
            "pre_1",
            false,
        );
    }

    #[test]
    fn transaction_response_with_redirection_data_stops_the_flow() {
        let txn_id = || ResponseId::ConnectorTransactionId("txn_1".to_string());
        assert_order(
            Ok(transaction_response(txn_id(), None)),
            true,
            "txn_1",
            true,
        );
        assert_order(
            Ok(transaction_response(
                txn_id(),
                Some(RedirectForm::Html {
                    html_data: "<html></html>".to_string(),
                }),
            )),
            true,
            "txn_1",
            false,
        );
    }

    #[test]
    fn transaction_response_without_connector_transaction_id_is_an_error() {
        let response = Ok(transaction_response(ResponseId::NoResponseId, None));
        let result = get_create_order_result(&response, true);
        assert!(matches!(
            &result,
            Err(report) if matches!(report.current_context(), ApiErrorResponse::InternalServerError)
        ));
    }

    #[test]
    fn unexpected_response_variant_is_an_error() {
        let response = Ok(PaymentsResponseData::TokenizationResponse {
            token: "token_1".to_string(),
        });
        let result = get_create_order_result(&response, true);
        assert!(matches!(
            &result,
            Err(report) if matches!(report.current_context(), ApiErrorResponse::InternalServerError)
        ));
    }

    #[test]
    fn connector_error_stops_the_flow_and_keeps_the_error() {
        let response = Err(ErrorResponse {
            code: "E1".to_string(),
            ..ErrorResponse::default()
        });
        let result = get_create_order_result(&response, true);
        assert!(matches!(
            &result,
            Ok(create_order) if matches!(&create_order.create_order_result, Err(error) if error.code == "E1")
                && !create_order.should_continue_further
        ));
    }
}
