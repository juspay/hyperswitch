use std::str::FromStr;

use async_trait::async_trait;
use common_utils::{id_type, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client;
use hyperswitch_domain_models::payments as domain_payments;
use router_env::logger;
use unified_connector_service_client::payments as payments_grpc;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
        unified_connector_service::{
            build_unified_connector_service_auth_metadata,
            handle_unified_connector_service_response_for_payment_cancel, ucs_logging_wrapper,
        },
    },
    routes::{metrics, SessionState},
    services,
    types::{self, api, domain, transformers::ForeignTryFrom},
};
#[cfg(feature = "v1")]
#[async_trait]
impl ConstructFlowSpecificData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for PaymentData<api::Void>
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
    ) -> RouterResult<types::PaymentsCancelRouterData> {
        Box::pin(transformers::construct_payment_router_data::<
            api::Void,
            types::PaymentsCancelData,
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
impl ConstructFlowSpecificData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for hyperswitch_domain_models::payments::PaymentCancelData<api::Void>
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
    ) -> RouterResult<types::PaymentsCancelRouterData> {
        Box::pin(transformers::construct_router_data_for_cancel(
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
impl Feature<api::Void, types::PaymentsCancelData>
    for types::RouterData<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
{
    async fn decide_flows<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        _business_profile: &domain::Profile,
        _header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        _return_raw_connector_response: Option<bool>,
        _gateway_context: payments::gateway::context::RouterGatewayContext,
    ) -> RouterResult<Self> {
        metrics::PAYMENT_CANCEL_COUNT.add(
            1,
            router_env::metric_attributes!(("connector", connector.connector_name.to_string())),
        );

        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::Void,
            types::PaymentsCancelData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action,
            connector_request,
            None,
        )
        .await
        .to_payment_failed_response()?;

        Ok(resp)
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
                    api::Void,
                    types::PaymentsCancelData,
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

        let payment_void_request =
            payments_grpc::PaymentServiceVoidRequest::foreign_try_from(&*self)
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to construct Payment Void Request")?;

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
            payment_void_request,
            header_payload,
            |mut router_data, payment_void_request, grpc_headers| async move {
                let response = client
                    .payment_cancel(payment_void_request, connector_auth_metadata, grpc_headers)
                    .await
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to Cancel payment")?;

                let payment_void_response = response.into_inner();

                let (router_data_response, status_code) =
                    handle_unified_connector_service_response_for_payment_cancel(
                        payment_void_response.clone(),
                    )
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to deserialize UCS response")?;

                let router_data_response = router_data_response.map(|(response, status)| {
                    router_data.status = status;
                    response
                });
                router_data.response = router_data_response;
                router_data.connector_http_status_code = Some(status_code);

                Ok((router_data, (), payment_void_response))
            },
        ))
        .await?;

        *self = updated_router_data;
        Ok(())
    }
}
