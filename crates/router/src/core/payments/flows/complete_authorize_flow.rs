use std::str::FromStr;

use async_trait::async_trait;
use common_enums::connector_enums;
use common_utils::{id_type, ucs_types};
use error_stack::ResultExt;
use external_services::grpc_client;
use hyperswitch_domain_models::{router_request_types, router_response_types};
use hyperswitch_interfaces::{api as api_interface, api::ConnectorSpecifications};
use masking::{self, ExposeInterface};
use unified_connector_service_client::payments as payments_grpc;
use unified_connector_service_masking::ExposeInterface as UcsMaskingExposeInterface;

use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, RouterResult},
        payments::{self, access_token, helpers, transformers, PaymentData},
        unified_connector_service as ucs_core,
    },
    logger,
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
        platform: &domain::Platform,
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

    #[cfg(feature = "v2")]
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
        gateway_context: payments::flows::gateway_context::RouterGatewayContext,
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
                        gateway_context,
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
        platform: &domain::Platform,
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
                let updated_router_data =
                    Box::pin(handle_preprocessing_through_unified_connector_service(
                        self,
                        state,
                        header_payload,
                        lineage_ids.clone(),
                        merchant_connector_account.clone(),
                        platform,
                        connector_data,
                        unified_connector_service_execution_mode,
                        merchant_order_reference_id.clone(),
                        preprocessing_flow,
                    ))
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
        state: &SessionState,
        header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
        lineage_ids: grpc_client::LineageIds,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        platform: &domain::Platform,
        connector_data: &api::ConnectorData,
        unified_connector_service_execution_mode: common_enums::ExecutionMode,
        merchant_order_reference_id: Option<String>,
        _call_connector_action: common_enums::CallConnectorAction,
        _creds_identifier: Option<String>,
    ) -> RouterResult<()> {
        // Call UCS for Complete Authorize flow
        Box::pin(call_unified_connector_service_complete_authorize(
            self,
            state,
            header_payload,
            lineage_ids,
            merchant_connector_account,
            platform,
            connector_data.connector_name,
            unified_connector_service_execution_mode,
            merchant_order_reference_id,
        ))
        .await
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_preprocessing_through_unified_connector_service(
    mut router_data: types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    >,
    state: &SessionState,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    lineage_ids: grpc_client::LineageIds,
    #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    platform: &domain::Platform,
    connector_data: &api::ConnectorData,
    unified_connector_service_execution_mode: common_enums::ExecutionMode,
    merchant_order_reference_id: Option<String>,
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
            // Convert CompleteAuthorize to Authenticate for UCS call
            let mut complete_authorize_request_data = router_data.request.clone();
            let authenticate_request_data =
                types::PaymentsAuthenticateData::try_from(router_data.request.to_owned())?;
            let authenticate_response_data: Result<
                types::PaymentsResponseData,
                types::ErrorResponse,
            > = Err(types::ErrorResponse::default());
            let mut authenticate_router_data =
                helpers::router_data_type_conversion::<_, api::Authenticate, _, _, _, _>(
                    router_data.clone(),
                    authenticate_request_data,
                    authenticate_response_data,
                );

            // Call UCS for Authenticate flow and store authentication result for next step
            complete_authorize_request_data.authentication_data =
                call_unified_connector_service_authenticate(
                    &mut authenticate_router_data,
                    state,
                    header_payload,
                    lineage_ids,
                    merchant_connector_account,
                    platform,
                    connector_data.connector_name,
                    unified_connector_service_execution_mode,
                    merchant_order_reference_id,
                )
                .await?;

            // Convert back to CompleteAuthorize router data while preserving preprocessing response data
            let authenticate_response = authenticate_router_data.response.clone();
            let complete_authorize_router_data =
                helpers::router_data_type_conversion::<_, api::CompleteAuthorize, _, _, _, _>(
                    authenticate_router_data,
                    complete_authorize_request_data,
                    authenticate_response,
                );
            router_data = complete_authorize_router_data;

            Ok(router_data)
        }
        api_interface::PreProcessingFlowName::PostAuthenticate => {
            // Convert CompleteAuthorize to PostAuthenticate for UCS call
            let mut complete_authorize_request_data = router_data.request.clone();
            let post_authenticate_request_data =
                types::PaymentsPostAuthenticateData::try_from(router_data.request.to_owned())?;
            let post_authenticate_response_data: Result<
                types::PaymentsResponseData,
                types::ErrorResponse,
            > = Err(types::ErrorResponse::default());
            let mut post_authenticate_router_data =
                helpers::router_data_type_conversion::<_, api::PostAuthenticate, _, _, _, _>(
                    router_data.clone(),
                    post_authenticate_request_data,
                    post_authenticate_response_data,
                );

            // Call UCS for PostAuthenticate flow and store authentication result for next step
            complete_authorize_request_data.authentication_data =
                call_unified_connector_service_post_authenticate(
                    &mut post_authenticate_router_data,
                    state,
                    header_payload,
                    lineage_ids,
                    merchant_connector_account,
                    platform,
                    unified_connector_service_execution_mode,
                    merchant_order_reference_id,
                )
                .await?;

            // Convert back to CompleteAuthorize router data while preserving preprocessing response data
            let post_authenticate_response = post_authenticate_router_data.response.clone();
            let complete_authorize_router_data =
                helpers::router_data_type_conversion::<_, api::CompleteAuthorize, _, _, _, _>(
                    post_authenticate_router_data,
                    complete_authorize_request_data,
                    post_authenticate_response,
                );
            router_data = complete_authorize_router_data;

            Ok(router_data)
        }
    }
}

fn transform_redirection_response_for_authenticate_flow(
    connector: connector_enums::Connector,
    response_data: router_response_types::RedirectForm,
) -> RouterResult<router_response_types::RedirectForm> {
    match (connector, &response_data) {
        (
            connector_enums::Connector::Cybersource,
            router_response_types::RedirectForm::Form {
                endpoint,
                method: _,
                ref form_fields,
            },
        ) => {
            let access_token = form_fields.get("access_token").cloned().ok_or(
                ApiErrorResponse::MissingRequiredField {
                    field_name: "access_token",
                },
            )?;
            let step_up_url = form_fields.get("step_up_url").unwrap_or(endpoint).clone();
            Ok(
                router_response_types::RedirectForm::CybersourceConsumerAuth {
                    access_token,
                    step_up_url,
                },
            )
        }
        _ => Ok(response_data),
    }
}
fn transform_response_for_authenticate_flow(
    connector: connector_enums::Connector,
    response_data: router_response_types::PaymentsResponseData,
) -> RouterResult<router_response_types::PaymentsResponseData> {
    match (connector, response_data.clone()) {
        (
            connector_enums::Connector::Cybersource,
            router_response_types::PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data,
                mandate_reference,
                connector_metadata,
                network_txn_id,
                connector_response_reference_id,
                incremental_authorization_allowed,
                charges,
            },
        ) => {
            let redirection_data = Box::new(
                (*redirection_data)
                    .clone()
                    .map(|redirection_data| {
                        transform_redirection_response_for_authenticate_flow(
                            connector,
                            redirection_data,
                        )
                    })
                    .transpose()?,
            );
            Ok(
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
                    mandate_reference,
                    connector_metadata,
                    network_txn_id,
                    connector_response_reference_id,
                    incremental_authorization_allowed,
                    charges,
                },
            )
        }
        _ => Ok(response_data),
    }
}

#[allow(dead_code, clippy::too_many_arguments)]
async fn call_unified_connector_service_authenticate(
    router_data: &mut types::RouterData<
        api::Authenticate,
        types::PaymentsAuthenticateData,
        types::PaymentsResponseData,
    >,
    state: &SessionState,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    lineage_ids: grpc_client::LineageIds,
    #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    platform: &domain::Platform,
    connector: connector_enums::Connector,
    unified_connector_service_execution_mode: common_enums::ExecutionMode,
    merchant_order_reference_id: Option<String>,
) -> RouterResult<Option<router_request_types::UcsAuthenticationData>> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch Unified Connector Service client")?;

    let payment_authenticate_request =
        payments_grpc::PaymentServiceAuthenticateRequest::foreign_try_from(&*router_data)
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct Payment Authorize Request")?;

    let connector_auth_metadata = ucs_core::build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        platform,
    )
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
    let headers_builder = state
        .get_grpc_headers_ucs(unified_connector_service_execution_mode)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_reference_id)
        .lineage_ids(lineage_ids);
    let (updated_router_data, authentication_data) = Box::pin(ucs_core::ucs_logging_wrapper(
        router_data.clone(),
        state,
        payment_authenticate_request,
        headers_builder,
        |mut router_data, payment_authenticate_request, grpc_headers| async move {
            let response = client
                .payment_authenticate(
                    payment_authenticate_request,
                    connector_auth_metadata,
                    grpc_headers,
                )
                .await
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to authorize payment")?;

            let payment_authenticate_response = response.into_inner();

            let (router_data_response, status_code) =
                ucs_core::handle_unified_connector_service_response_for_payment_authenticate(
                    payment_authenticate_response.clone(),
                )
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to deserialize UCS response")?;

            let router_data_response = router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });
            let router_data_response = match router_data_response {
                Ok(response) => Ok(transform_response_for_authenticate_flow(
                    connector, response,
                )?),
                Err(err) => Err(err),
            };
            router_data.response = router_data_response;
            router_data.raw_connector_response = payment_authenticate_response
                .raw_connector_response
                .clone()
                .map(|raw_connector_response| raw_connector_response.expose().into());
            router_data.connector_http_status_code = Some(status_code);

            let domain_authentication_data = payment_authenticate_response
                .authentication_data
                .clone()
                .map(|grpc_authentication_data| {
                    router_request_types::UcsAuthenticationData::foreign_try_from(
                        grpc_authentication_data,
                    )
                })
                .transpose()
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to Convert to domain AuthenticationData")?;

            Ok((
                router_data,
                domain_authentication_data,
                payment_authenticate_response,
            ))
        },
    ))
    .await?;

    // Copy back the updated data
    *router_data = updated_router_data;
    Ok(authentication_data)
}

#[allow(dead_code, clippy::too_many_arguments)]
async fn call_unified_connector_service_post_authenticate(
    router_data: &mut types::RouterData<
        api::PostAuthenticate,
        types::PaymentsPostAuthenticateData,
        types::PaymentsResponseData,
    >,
    state: &SessionState,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    lineage_ids: grpc_client::LineageIds,
    #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    platform: &domain::Platform,
    unified_connector_service_execution_mode: common_enums::ExecutionMode,
    merchant_order_reference_id: Option<String>,
) -> RouterResult<Option<router_request_types::UcsAuthenticationData>> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch Unified Connector Service client")?;

    let payment_post_authenticate_request =
        payments_grpc::PaymentServicePostAuthenticateRequest::foreign_try_from(&*router_data)
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct Payment Authorize Request")?;

    let connector_auth_metadata = ucs_core::build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        platform,
    )
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
    let headers_builder = state
        .get_grpc_headers_ucs(unified_connector_service_execution_mode)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_reference_id)
        .lineage_ids(lineage_ids);
    let (updated_router_data, authentication_data) = Box::pin(ucs_core::ucs_logging_wrapper(
        router_data.clone(),
        state,
        payment_post_authenticate_request,
        headers_builder,
        |mut router_data, payment_post_authenticate_request, grpc_headers| async move {
            let response = client
                .payment_post_authenticate(
                    payment_post_authenticate_request,
                    connector_auth_metadata,
                    grpc_headers,
                )
                .await
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to authorize payment")?;

            let payment_post_authenticate_response = response.into_inner();

            let (router_data_response, status_code) =
                ucs_core::handle_unified_connector_service_response_for_payment_post_authenticate(
                    payment_post_authenticate_response.clone(),
                )
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to deserialize UCS response")?;

            let router_data_response = router_data_response.map(|(response, status)| {
                router_data.status = status;
                response
            });
            router_data.response = router_data_response;
            router_data.raw_connector_response = payment_post_authenticate_response
                .raw_connector_response
                .clone()
                .map(|raw_connector_response| raw_connector_response.expose().into());
            router_data.connector_http_status_code = Some(status_code);

            let domain_authentication_data = payment_post_authenticate_response
                .authentication_data
                .clone()
                .map(|grpc_authentication_data| {
                    router_request_types::UcsAuthenticationData::foreign_try_from(
                        grpc_authentication_data,
                    )
                })
                .transpose()
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to Convert to domain AuthenticationData")?;

            Ok((
                router_data,
                domain_authentication_data,
                payment_post_authenticate_response,
            ))
        },
    ))
    .await?;

    // Copy back the updated data
    *router_data = updated_router_data;
    Ok(authentication_data)
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

#[allow(clippy::too_many_arguments)]
async fn call_unified_connector_service_complete_authorize(
    router_data: &mut types::RouterData<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    >,
    state: &SessionState,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    lineage_ids: grpc_client::LineageIds,
    #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
    #[cfg(feature = "v2")] merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    platform: &domain::Platform,
    _connector: connector_enums::Connector,
    unified_connector_service_execution_mode: common_enums::ExecutionMode,
    merchant_order_reference_id: Option<String>,
) -> RouterResult<()> {
    let client = state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch Unified Connector Service client")?;

    let payment_authorize_request =
        payments_grpc::PaymentServiceAuthorizeRequest::foreign_try_from(&(*router_data))
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct Payment Complete Authorize Request")?;

    let connector_auth_metadata = ucs_core::build_unified_connector_service_auth_metadata(
        merchant_connector_account,
        platform,
    )
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

    let headers_builder = state
        .get_grpc_headers_ucs(unified_connector_service_execution_mode)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_reference_id)
        .lineage_ids(lineage_ids);

    let (updated_router_data, _) = Box::pin(ucs_core::ucs_logging_wrapper(
        router_data.clone(),
        state,
        payment_authorize_request,
        headers_builder,
        |mut router_data, payment_authorize_request, grpc_headers| async move {
            let response = Box::pin(client.payment_authorize(
                payment_authorize_request,
                connector_auth_metadata,
                grpc_headers,
            ))
            .await
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to complete authorize payment")?;

            let payment_authorize_response = response.into_inner();

            let ucs_authorize_response =
                ucs_core::handle_unified_connector_service_response_for_payment_authorize(
                    payment_authorize_response.clone(),
                )
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to deserialize UCS response")?;

            let router_data_response =
                ucs_authorize_response
                    .router_data_response
                    .map(|(response, status)| {
                        router_data.status = status;
                        response
                    });
            router_data.response = router_data_response;
            router_data.raw_connector_response = payment_authorize_response
                .raw_connector_response
                .clone()
                .map(|raw_connector_response| raw_connector_response.expose().into());
            router_data.connector_http_status_code = Some(ucs_authorize_response.status_code);

            // Populate connector_customer_id if present
            ucs_authorize_response
                .connector_customer_id
                .map(|connector_customer_id| {
                    router_data.connector_customer = Some(connector_customer_id);
                });

            ucs_authorize_response
                .connector_response
                .map(|customer_response| {
                    router_data.connector_response = Some(customer_response);
                });

            Ok((router_data, (), payment_authorize_response))
        },
    ))
    .await?;

    // Copy back the updated data
    *router_data = updated_router_data;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
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
    gateway_context: payments::flows::gateway_context::RouterGatewayContext,
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
        gateway_context,
    )
    .await;

    // Process capture response
    let (updated_status, updated_response) =
        super::handle_post_capture_response(complete_authorize_response, post_capture_router_data)?;

    router_data.status = updated_status;
    router_data.response = Ok(updated_response);
    Ok(router_data)
}
