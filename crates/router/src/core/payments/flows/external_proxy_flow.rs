use std::str::FromStr;

use async_trait::async_trait;
use common_enums as enums;
use common_utils::{id_type, ucs_types, ucs_types::UcsReferenceId};
use error_stack::ResultExt;
use external_services::grpc_client;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentConfirmData;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse, payments as domain_payments,
};
use masking::ExposeInterface;
use unified_connector_service_client::payments as payments_grpc;
use unified_connector_service_masking::ExposeInterface as UcsMaskingExposeInterface;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, gateway::context as gateway_context, helpers,
            tokenization, transformers, PaymentData,
        },
        unified_connector_service::{self, ucs_logging_wrapper},
    },
    logger,
    routes::{metrics, SessionState},
    services::{self, api::ConnectorValidation},
    types::{
        self, api, domain,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::OptionExt,
};

#[cfg(feature = "v2")]
#[async_trait]
impl
    ConstructFlowSpecificData<
        api::ExternalVaultProxy,
        types::ExternalVaultProxyPaymentsData,
        types::PaymentsResponseData,
    > for PaymentConfirmData<api::ExternalVaultProxy>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        platform: &domain::Platform,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<domain_payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<
            api::ExternalVaultProxy,
            types::ExternalVaultProxyPaymentsData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(
            transformers::construct_external_vault_proxy_payment_router_data(
                state,
                self.clone(),
                connector_id,
                platform,
                customer,
                merchant_connector_account,
                merchant_recipient_data,
                header_payload,
            ),
        )
        .await
    }
}

#[async_trait]
impl Feature<api::ExternalVaultProxy, types::ExternalVaultProxyPaymentsData>
    for types::ExternalVaultProxyPaymentsRouterData
{
    async fn decide_flows<'a>(
        mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        business_profile: &domain::Profile,
        header_payload: domain_payments::HeaderPayload,
        return_raw_connector_response: Option<bool>,
        gateway_context: gateway_context::RouterGatewayContext,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::ExternalVaultProxy,
            types::ExternalVaultProxyPaymentsData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        logger::debug!(auth_type=?self.auth_type);
        let mut auth_router_data = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action.clone(),
            connector_request,
            return_raw_connector_response,
        )
        .await
        .to_payment_failed_response()?;

        // External vault proxy doesn't use integrity checks
        auth_router_data.integrity_check = Ok(());
        metrics::PAYMENT_COUNT.add(1, &[]);

        Ok(auth_router_data)
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        _platform: &domain::Platform,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, self, creds_identifier).await
    }

    async fn add_session_token<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self>
    where
        Self: Sized,
    {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let authorize_data = &types::PaymentsAuthorizeSessionTokenRouterData::foreign_from((
            &self,
            types::AuthorizeSessionTokenData::foreign_from(&self),
        ));
        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            authorize_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_payment_failed_response()?;
        let mut router_data = self;
        router_data.session_token = resp.session_token;
        Ok(router_data)
    }

    async fn add_payment_method_token<'a>(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        tokenization_action: &payments::TokenizationAction,
        should_continue_payment: bool,
    ) -> RouterResult<types::PaymentMethodTokenResult> {
        let request = self.request.clone();
        tokenization::add_payment_method_token(
            state,
            connector,
            tokenization_action,
            self,
            types::PaymentMethodTokenizationData::try_from(request)?,
            should_continue_payment,
        )
        .await
    }

    async fn preprocessing_steps<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        todo!()
    }

    async fn postprocessing_steps<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        todo!()
    }

    async fn create_connector_customer<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<String>> {
        customers::create_connector_customer(
            state,
            connector,
            self,
            types::ConnectorCustomerData::try_from(self)?,
        )
        .await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                connector
                    .connector
                    .validate_connector_against_payment_request(
                        self.request.capture_method,
                        self.payment_method,
                        self.request.payment_method_type,
                    )
                    .to_payment_failed_response()?;

                // Check if the connector supports mandate payment
                // if the payment_method_type does not support mandate for the given connector, downgrade the setup future usage to on session
                if self.request.setup_future_usage
                    == Some(diesel_models::enums::FutureUsage::OffSession)
                    && !self
                        .request
                        .payment_method_type
                        .and_then(|payment_method_type| {
                            state
                                .conf
                                .mandates
                                .supported_payment_methods
                                .0
                                .get(&enums::PaymentMethod::from(payment_method_type))
                                .and_then(|supported_pm_for_mandates| {
                                    supported_pm_for_mandates.0.get(&payment_method_type).map(
                                        |supported_connector_for_mandates| {
                                            supported_connector_for_mandates
                                                .connector_list
                                                .contains(&connector.connector_name)
                                        },
                                    )
                                })
                        })
                        .unwrap_or(false)
                {
                    // downgrade the setup future usage to on session
                    self.request.setup_future_usage =
                        Some(diesel_models::enums::FutureUsage::OnSession);
                };

                // External vault proxy doesn't use regular payment method validation
                // Skip mandate payment validation for external vault proxy

                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                    api::ExternalVaultProxy,
                    types::ExternalVaultProxyPaymentsData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                metrics::EXECUTE_PRETASK_COUNT.add(
                    1,
                    router_env::metric_attributes!(
                        ("connector", connector.connector_name.to_string()),
                        ("flow", format!("{:?}", api::ExternalVaultProxy)),
                    ),
                );

                logger::debug!(completed_pre_tasks=?true);

                // External vault proxy always proceeds
                Ok((
                    connector_integration
                        .build_request(self, &state.conf.connectors)
                        .to_payment_failed_response()?,
                    true,
                ))
            }
            _ => Ok((None, true)),
        }
    }

    async fn create_order_at_connector(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        should_continue_payment: bool,
    ) -> RouterResult<Option<types::CreateOrderResult>> {
        if connector
            .connector_name
            .requires_order_creation_before_payment(self.payment_method)
            && should_continue_payment
        {
            let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                api::CreateOrder,
                types::CreateOrderRequestData,
                types::PaymentsResponseData,
            > = connector.connector.get_connector_integration();

            let request_data = types::CreateOrderRequestData::try_from(self.request.clone())?;

            let response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
                Err(types::ErrorResponse::default());

            let createorder_router_data =
                helpers::router_data_type_conversion::<_, api::CreateOrder, _, _, _, _>(
                    self.clone(),
                    request_data,
                    response_data,
                );

            let resp = services::execute_connector_processing_step(
                state,
                connector_integration,
                &createorder_router_data,
                payments::CallConnectorAction::Trigger,
                None,
                None,
            )
            .await
            .to_payment_failed_response()?;

            let create_order_resp = match resp.response {
                Ok(res) => {
                    if let types::PaymentsResponseData::PaymentsCreateOrderResponse { order_id } =
                        res
                    {
                        Ok(order_id)
                    } else {
                        Err(error_stack::report!(ApiErrorResponse::InternalServerError)
                            .attach_printable(format!(
                                "Unexpected response format from connector: {res:?}",
                            )))?
                    }
                }
                Err(error) => Err(error),
            };

            Ok(Some(types::CreateOrderResult {
                create_order_result: create_order_resp,
            }))
        } else {
            // If the connector does not require order creation, return None
            Ok(None)
        }
    }

    fn update_router_data_with_create_order_response(
        &mut self,
        create_order_result: types::CreateOrderResult,
    ) {
        match create_order_result.create_order_result {
            Ok(order_id) => {
                self.request.order_id = Some(order_id.clone()); // ? why this is assigned here and ucs also wants this to populate data
                self.response =
                    Ok(types::PaymentsResponseData::PaymentsCreateOrderResponse { order_id });
            }
            Err(err) => {
                self.response = Err(err.clone());
            }
        }
    }

    #[cfg(feature = "v2")]
    async fn call_unified_connector_service_with_external_vault_proxy<'a>(
        &mut self,
        state: &SessionState,
        header_payload: &domain_payments::HeaderPayload,
        lineage_ids: grpc_client::LineageIds,
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        external_vault_merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        platform: &domain::Platform,
        unified_connector_service_execution_mode: enums::ExecutionMode,
        merchant_order_reference_id: Option<String>,
    ) -> RouterResult<()> {
        let client = state
            .grpc_client
            .unified_connector_service_client
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch Unified Connector Service client")?;

        let payment_authorize_request =
            payments_grpc::PaymentServiceAuthorizeRequest::foreign_try_from(&*self)
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to construct Payment Authorize Request")?;

        let connector_auth_metadata =
            unified_connector_service::build_unified_connector_service_auth_metadata(
                merchant_connector_account,
                platform,
            )
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct request metadata")?;

        let external_vault_proxy_metadata =
            unified_connector_service::build_unified_connector_service_external_vault_proxy_metadata(
                external_vault_merchant_connector_account
            )
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct external vault proxy metadata")?;
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
        let headers_builder = state
            .get_grpc_headers_ucs(unified_connector_service_execution_mode)
            .external_vault_proxy_metadata(Some(external_vault_proxy_metadata))
            .merchant_reference_id(merchant_reference_id)
            .lineage_ids(lineage_ids);
        let (updated_router_data, _) = Box::pin(ucs_logging_wrapper(
            self.clone(),
            state,
            payment_authorize_request.clone(),
            headers_builder,
            |mut router_data, payment_authorize_request, grpc_headers| async move {
                let response = Box::pin(client
                    .payment_authorize(
                        payment_authorize_request,
                        connector_auth_metadata,
                        grpc_headers,
                    ))
                    .await
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to authorize payment")?;

                let payment_authorize_response = response.into_inner();

                let ucs_data =
                    unified_connector_service::handle_unified_connector_service_response_for_payment_authorize(
                        payment_authorize_response.clone(),
                    )
                    .change_context(ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to deserialize UCS response")?;

                let router_data_response = ucs_data.router_data_response.map(|(response, status)|{
                    router_data.status = status;
                    response
                });
                router_data.response = router_data_response;
                router_data.raw_connector_response = payment_authorize_response
                    .raw_connector_response
                    .clone()
                    .map(|raw_connector_response| raw_connector_response.expose().into());
                router_data.connector_http_status_code = Some(ucs_data.status_code);

                Ok((router_data,(), payment_authorize_response))
            }
        )).await?;

        // Copy back the updated data
        *self = updated_router_data;
        Ok(())
    }
}
