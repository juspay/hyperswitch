use async_trait::async_trait;
use external_services::grpc_client;
use hyperswitch_interfaces::{api as api_interface, api::ConnectorSpecifications};
use masking::ExposeInterface;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
    },
    routes::{metrics, SessionState},
    services,
    types::{self, api, domain, transformers::ForeignTryFrom},
};

#[async_trait]
impl
    ConstructFlowSpecificData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentData<api::CompleteAuthorize>
{
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
        _payment_method: Option<common_enums::PaymentMethod>,
        _payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<
        types::RouterData<
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(transformers::construct_payment_router_data::<
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
        >(
            state,
            self.clone(),
            connector_id,
            merchant_context,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
            None,
            None,
        ))
        .await
    }

    #[cfg(feature = "v2")]
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_context: &domain::MerchantContext,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        todo!()
    }
}

#[async_trait]
impl Feature<api::CompleteAuthorize, types::CompleteAuthorizeData>
    for types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    >
{
    async fn decide_flows<'a>(
        mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        business_profile: &domain::Profile,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        _return_raw_connector_response: Option<bool>,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let mut complete_authorize_router_data = services::execute_connector_processing_step(
            state,
            connector_integration,
            &self,
            call_connector_action.clone(),
            connector_request,
            None,
        )
        .await
        .to_payment_failed_response()?;
        match complete_authorize_router_data.response.clone() {
            Err(_) => Ok(complete_authorize_router_data),
            Ok(complete_authorize_response) => {
                // Check if the Capture API should be called based on the connector and other parameters
                if super::should_initiate_capture_flow(
                    &connector.connector_name,
                    self.request.customer_acceptance,
                    self.request.capture_method,
                    self.request.setup_future_usage,
                    complete_authorize_router_data.status,
                ) {
                    complete_authorize_router_data = Box::pin(process_capture_flow(
                        complete_authorize_router_data,
                        complete_authorize_response,
                        state,
                        connector,
                        call_connector_action.clone(),
                        business_profile,
                        header_payload,
                    ))
                    .await?;
                }
                Ok(complete_authorize_router_data)
            }
        }
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        _merchant_context: &domain::MerchantContext,
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

    async fn add_payment_method_token<'a>(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        _tokenization_action: &payments::TokenizationAction,
        should_continue_payment: bool,
    ) -> RouterResult<types::PaymentMethodTokenResult> {
        // TODO: remove this and handle it in core
        if matches!(connector.connector_name, types::Connector::Payme) {
            let request = self.request.clone();
            payments::tokenization::add_payment_method_token(
                state,
                connector,
                &payments::TokenizationAction::TokenizeInConnector,
                self,
                types::PaymentMethodTokenizationData::try_from(request)?,
                should_continue_payment,
            )
            .await
        } else {
            Ok(types::PaymentMethodTokenResult {
                payment_method_token_result: Ok(None),
                is_payment_method_tokenization_performed: false,
                connector_response: None,
            })
        }
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
                    api::CompleteAuthorize,
                    types::CompleteAuthorizeData,
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

    async fn preprocessing_steps<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        complete_authorize_preprocessing_steps(state, &self, true, connector).await
    }

    async fn call_preprocessing_through_unified_connector_service<'a>(
        self,
        state: &SessionState,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        lineage_ids: &grpc_client::LineageIds,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        merchant_context: &domain::MerchantContext,
        connector_data: &api::ConnectorData,
        unified_connector_service_execution_mode: common_enums::ExecutionMode,
        merchant_order_reference_id: Option<String>,
    ) -> RouterResult<(Self, bool)> {
        let current_flow = api_interface::CurrentFlowInfo::CompleteAuthorize {
            request_data: &self.request,
        };
        let optional_preprocessing_flow = connector_data
            .connector
            .get_preprocessing_flow_if_needed(current_flow);
        match optional_preprocessing_flow {
            Some(preprocessing_flow) => {
                let updated_router_data = handle_preprocessing_through_unified_connector_service(
                    self,
                    state,
                    header_payload,
                    lineage_ids,
                    merchant_connector_account.clone(),
                    merchant_context,
                    connector_data,
                    unified_connector_service_execution_mode,
                    merchant_order_reference_id.clone(),
                    preprocessing_flow,
                )
                .await?;
                let pre_processing_flow_response = api_interface::PreProcessingFlowResponse {
                    response: &updated_router_data.response,
                    attempt_status: updated_router_data.status,
                };
                let current_flow = api_interface::CurrentFlowInfo::CompleteAuthorize {
                    request_data: &updated_router_data.request,
                };
                let should_continue = connector_data
                    .connector
                    .decide_should_continue_after_preprocessing(
                        current_flow,
                        preprocessing_flow,
                        pre_processing_flow_response,
                    );
                Ok((updated_router_data, should_continue))
            }
            None => Ok((self, true)),
        }
    }

    async fn call_unified_connector_service<'a>(
        &mut self,
        _state: &SessionState,
        _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        _lineage_ids: grpc_client::LineageIds,
        #[cfg(feature = "v1")] _merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        _merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        _merchant_context: &domain::MerchantContext,
        _connector_data: &api::ConnectorData,
        _unified_connector_service_execution_mode: common_enums::ExecutionMode,
        _merchant_order_reference_id: Option<String>,
    ) -> RouterResult<()> {
        // Call UCS for Authorize flow
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_preprocessing_through_unified_connector_service(
    router_data: types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    >,
    _state: &SessionState,
    _header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    _lineage_ids: &grpc_client::LineageIds,
    #[cfg(feature = "v1")] _merchant_connector_account: helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")] _merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    _merchant_context: &domain::MerchantContext,
    _connector_data: &api::ConnectorData,
    _unified_connector_service_execution_mode: common_enums::ExecutionMode,
    _merchant_order_reference_id: Option<String>,
    preprocessing_flow_name: api_interface::PreProcessingFlowName,
) -> RouterResult<
    types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    >,
> {
    match preprocessing_flow_name {
        api_interface::PreProcessingFlowName::Authenticate => {
            // Call UCS for Authenticate flow
            Ok(router_data)
        }
        api_interface::PreProcessingFlowName::PostAuthenticate => {
            // Call UCS for PostAuthenticate flow
            Ok(router_data)
        }
    }
}

pub async fn complete_authorize_preprocessing_steps<F: Clone>(
    state: &SessionState,
    router_data: &types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
    confirm: bool,
    connector: &api::ConnectorData,
) -> RouterResult<types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>> {
    if confirm {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let preprocessing_request_data =
            types::PaymentsPreProcessingData::try_from(router_data.request.to_owned())?;

        let preprocessing_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
            Err(types::ErrorResponse::default());

        let preprocessing_router_data =
            helpers::router_data_type_conversion::<_, api::PreProcessing, _, _, _, _>(
                router_data.clone(),
                preprocessing_request_data,
                preprocessing_response_data,
            );

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &preprocessing_router_data,
            payments::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_payment_failed_response()?;

        metrics::PREPROCESSING_STEPS_COUNT.add(
            1,
            router_env::metric_attributes!(
                ("connector", connector.connector_name.to_string()),
                ("payment_method", router_data.payment_method.to_string()),
            ),
        );

        let mut router_data_request = router_data.request.to_owned();

        if let Ok(types::PaymentsResponseData::TransactionResponse {
            connector_metadata, ..
        }) = &resp.response
        {
            connector_metadata.clone_into(&mut router_data_request.connector_meta);
        };

        let authorize_router_data = helpers::router_data_type_conversion::<_, F, _, _, _, _>(
            resp.clone(),
            router_data_request,
            resp.response,
        );

        Ok(authorize_router_data)
    } else {
        Ok(router_data.clone())
    }
}

impl<F>
    ForeignTryFrom<types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>>
    for types::PaymentsCaptureData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn foreign_try_from(
        item: types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: item.connector.clone().to_string(),
                status_code: err.status_code,
                reason: err.reason,
            })?;

        Ok(Self {
            amount_to_capture: item.request.amount,
            currency: item.request.currency,
            connector_transaction_id: types::PaymentsResponseData::get_connector_transaction_id(
                &response,
            )?,
            payment_amount: item.request.amount,
            multiple_capture_data: None,
            connector_meta: types::PaymentsResponseData::get_connector_metadata(&response)
                .map(|secret| secret.expose()),
            browser_info: None,
            metadata: None,
            capture_method: item.request.capture_method,
            minor_payment_amount: item.request.minor_amount,
            minor_amount_to_capture: item.request.minor_amount,
            integrity_object: None,
            split_payments: None,
            webhook_url: None,
        })
    }
}

async fn process_capture_flow(
    mut router_data: types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    >,
    complete_authorize_response: types::PaymentsResponseData,
    state: &SessionState,
    connector: &api::ConnectorData,
    call_connector_action: payments::CallConnectorAction,
    business_profile: &domain::Profile,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
) -> RouterResult<
    types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    >,
> {
    // Convert RouterData into Capture RouterData
    let capture_router_data = helpers::router_data_type_conversion(
        router_data.clone(),
        types::PaymentsCaptureData::foreign_try_from(router_data.clone())?,
        Err(types::ErrorResponse::default()),
    );

    // Call capture request
    let post_capture_router_data = super::call_capture_request(
        capture_router_data,
        state,
        connector,
        call_connector_action,
        business_profile,
        header_payload,
    )
    .await;

    // Process capture response
    let (updated_status, updated_response) =
        super::handle_post_capture_response(complete_authorize_response, post_capture_router_data)?;

    router_data.status = updated_status;
    router_data.response = Ok(updated_response);
    Ok(router_data)
}
