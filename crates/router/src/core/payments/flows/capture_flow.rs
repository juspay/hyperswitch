use std::str::FromStr;

use async_trait::async_trait;
use common_utils::{id_type, types::MinorUnit, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse, payments as domain_payments,
};
use unified_connector_service_client::payments as payments_grpc;

use super::ConstructFlowSpecificData;
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, Feature, PaymentData},
        unified_connector_service::{
            build_unified_connector_service_auth_metadata,
            handle_unified_connector_service_response_for_payment_capture, ucs_logging_wrapper,
        },
    },
    routes::SessionState,
    services::{self, logger},
    types::{self, api, domain, transformers::ForeignTryFrom},
};

#[cfg(feature = "v1")]
#[async_trait]
impl
    ConstructFlowSpecificData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for PaymentData<api::Capture>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        platform: &domain::Platform,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
        _payment_method: Option<common_enums::PaymentMethod>,
        _payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<types::PaymentsCaptureRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::Capture,
            types::PaymentsCaptureData,
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
impl
    ConstructFlowSpecificData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for hyperswitch_domain_models::payments::PaymentCaptureData<api::Capture>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        platform: &domain::Platform,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>,
    > {
        Box::pin(transformers::construct_payment_router_data_for_capture(
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
impl Feature<api::Capture, types::PaymentsCaptureData>
    for types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        _business_profile: &domain::Profile,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        return_raw_connector_response: Option<bool>,
        _gateway_context: payments::gateway::context::RouterGatewayContext,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::Capture,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let mut new_router_data = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action,
            connector_request,
            return_raw_connector_response,
        )
        .await
        .to_payment_failed_response()?;

        // Initiating Integrity check
        let integrity_result = helpers::check_integrity_based_on_flow(
            &new_router_data.request,
            &new_router_data.response,
        );
        new_router_data.integrity_check = integrity_result;

        Ok(new_router_data)
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
                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                    api::Capture,
                    types::PaymentsCaptureData,
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
        unified_connector_service_execution_mode: common_enums::ExecutionMode,
        merchant_order_reference_id: Option<String>,
        _call_connector_action: common_enums::CallConnectorAction,
        _creds_identifier: Option<String>,
    ) -> RouterResult<()> {
        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        let payment_capture_request =
            payments_grpc::PaymentServiceCaptureRequest::foreign_try_from(&*self)
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to construct Payment Capture Request")?;

        let connector_auth_metadata =
            build_unified_connector_service_auth_metadata(merchant_connector_account, platform)
                .change_context(ApiErrorResponse::InternalServerError)
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
        let (updated_router_data, _) = Box::pin(ucs_logging_wrapper(
            self.clone(),
            state,
            payment_capture_request,
            header_payload,
            |mut router_data, payment_capture_request, grpc_headers| async move {
                let response = client
                    .payment_capture(
                        payment_capture_request,
                        connector_auth_metadata,
                        grpc_headers,
                    )
                    .await
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to capture payment")?;

                let payment_capture_response = response.into_inner();

                let (router_data_response, status_code) =
                    handle_unified_connector_service_response_for_payment_capture(
                        payment_capture_response.clone(),
                    )
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to deserialize UCS response")?;

                let router_data_response = router_data_response.map(|(response, status)| {
                    router_data.status = status;
                    response
                });
                router_data.response = router_data_response;
                router_data.amount_captured = payment_capture_response.captured_amount;
                router_data.minor_amount_captured = payment_capture_response
                    .minor_captured_amount
                    .map(MinorUnit::new);
                router_data.connector_http_status_code = Some(status_code);

                Ok((router_data, (), payment_capture_response))
            },
        ))
        .await?;

        // Copy back the updated data
        *self = updated_router_data;
        Ok(())
    }
}
