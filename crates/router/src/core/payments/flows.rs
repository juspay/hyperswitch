pub mod approve_flow;
pub mod authorize_flow;
pub mod cancel_flow;
pub mod capture_flow;
pub mod complete_authorize_flow;
pub mod incremental_authorization_flow;
pub mod post_session_tokens_flow;
pub mod psync_flow;
pub mod reject_flow;
pub mod session_flow;
pub mod session_update_flow;
pub mod setup_mandate_flow;
pub mod update_metadata_flow;

use async_trait::async_trait;
use common_types::payments::CustomerAcceptance;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::router_flow_types::{
    BillingConnectorInvoiceSync, BillingConnectorPaymentsSync, RecoveryRecordBack,
};
use hyperswitch_domain_models::router_request_types::PaymentsCaptureData;

use crate::{
    core::{
        errors::{ApiErrorResponse, RouterResult},
        payments::{self, helpers},
    },
    logger,
    routes::SessionState,
    services, types as router_types,
    types::{self, api, api::enums as api_enums, domain},
};

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait ConstructFlowSpecificData<F, Req, Res> {
    #[cfg(feature = "v1")]
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_context: &domain::MerchantContext,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<types::RouterData<F, Req, Res>>;

    #[cfg(feature = "v2")]
    async fn construct_router_data<'a>(
        &self,
        _state: &SessionState,
        _connector_id: &str,
        _merchant_context: &domain::MerchantContext,
        _customer: &Option<domain::Customer>,
        _merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        _merchant_recipient_data: Option<types::MerchantRecipientData>,
        _header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<types::RouterData<F, Req, Res>>;

    async fn get_merchant_recipient_data<'a>(
        &self,
        _state: &SessionState,
        _merchant_context: &domain::MerchantContext,
        _merchant_connector_account: &helpers::MerchantConnectorAccountType,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait Feature<F, T> {
    async fn decide_flows<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        business_profile: &domain::Profile,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        return_raw_connector_response: Option<bool>,
    ) -> RouterResult<Self>
    where
        Self: Sized,
        F: Clone,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_context: &domain::MerchantContext,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;

    async fn add_session_token<'a>(
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

    async fn add_payment_method_token<'a>(
        &mut self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _tokenization_action: &payments::TokenizationAction,
        _should_continue_payment: bool,
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

    async fn preprocessing_steps<'a>(
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

    async fn create_order_at_connector(
        &mut self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
        _should_continue_payment: bool,
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

    async fn call_unified_connector_service<'a>(
        &mut self,
        _state: &SessionState,
        #[cfg(feature = "v1")] _merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        _merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        _merchant_context: &domain::MerchantContext,
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
        common_enums::AttemptStatus::Authorized => {
            if let Some(api_enums::CaptureMethod::SequentialAutomatic) = capture_method {
                match connector_name {
                    router_types::Connector::Paybox => {
                        // Check CIT conditions for Paybox
                        setup_future_usage == Some(api_enums::FutureUsage::OffSession)
                            && customer_acceptance.is_some()
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
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
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
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
