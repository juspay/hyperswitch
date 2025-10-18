//! Unified Connector Service Gateway Implementation
//!
//! This gateway executes payment operations through the UCS gRPC service.
//! Each flow type has a specialized implementation that handles the transformation
//! between RouterData and gRPC requests/responses.

use std::marker::PhantomData;

use async_trait::async_trait;
use common_enums::CallConnectorAction;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::RouterData,
    types::{
        PaymentsAuthorizeData, PaymentsResponseData, PaymentsSyncData, SetupMandateRequestData,
    },
};
use hyperswitch_interfaces::api::gateway as gateway_interface;

use super::PaymentGateway;
use crate::{
    core::{
        errors::{self, RouterResult},
        unified_connector_service::{
            self as ucs,
            transformers::{ForeignTryFrom, PaymentServiceAuthorizeRequest},
        },
    },
    routes::SessionState,
    types::{api, MerchantConnectorAccountType},
};

/// Gateway that executes through Unified Connector Service
///
/// This gateway transforms RouterData to gRPC requests, calls the UCS service,
/// and transforms responses back to RouterData.
pub struct UnifiedConnectorServiceGateway<F> {
    flow_type: PhantomData<F>,
}

impl<F> UnifiedConnectorServiceGateway<F> {
    /// Create a new UCS gateway for the given flow type
    pub fn new() -> Self {
        Self {
            flow_type: PhantomData,
        }
    }
}

impl<F> Default for UnifiedConnectorServiceGateway<F> {
    fn default() -> Self {
        Self::new()
    }
}

// Implementation for Authorize flow (CIT and MIT)
#[async_trait]
impl
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        api::Authorize,
        PaymentsAuthorizeData,
        PaymentsResponseData,
    > for UnifiedConnectorServiceGateway<api::Authorize>
{
    async fn execute(
        &self,
        state: &SessionState,
        mut router_data: RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        connector: &api::ConnectorData,
        merchant_connector_account: &MerchantConnectorAccountType,
        call_connector_action: CallConnectorAction,
    ) -> Result<
        RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        hyperswitch_interfaces::errors::ConnectorError,
    >
    {
        // Only handle Trigger action for UCS
        match call_connector_action {
            CallConnectorAction::Trigger => {
                // Get UCS client
                let client = state
                    .grpc_client
                    .unified_connector_service_client
                    .as_ref()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("UCS client not available")?;

                // Determine if this is MIT (recurring) or CIT (first payment)
                let is_mandate_payment = router_data.request.mandate_id.is_some();

                if is_mandate_payment {
                    // MIT flow - use payment_repeat
                    let grpc_request =
                        ucs::transformers::PaymentServiceRepeatEverythingRequest::foreign_try_from(
                            &router_data,
                        )
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to transform RouterData to gRPC request")?;

                    let auth_metadata = ucs::build_unified_connector_service_auth_metadata(
                        merchant_connector_account,
                        &connector.connector_name,
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to build auth metadata")?;

                    let headers = state
                        .get_grpc_headers_ucs(ucs::types::ExecutionMode::Primary)
                        .lineage_ids(router_data.get_lineage_ids());

                    let response = client
                        .payment_repeat(grpc_request, auth_metadata, headers)
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("UCS payment_repeat call failed")?;

                    let (payments_response, attempt_status, http_status_code) =
                        ucs::handle_unified_connector_service_response_for_payment_repeat(
                            response.into_inner(),
                        )
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to handle UCS response")?;

                    router_data.response = payments_response;
                    router_data.status = attempt_status;
                    router_data.connector_http_status_code = Some(http_status_code);
                } else {
                    // CIT flow - use payment_authorize
                    let grpc_request = PaymentServiceAuthorizeRequest::foreign_try_from(&router_data)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to transform RouterData to gRPC request")?;

                    let auth_metadata = ucs::build_unified_connector_service_auth_metadata(
                        merchant_connector_account,
                        &connector.connector_name,
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to build auth metadata")?;

                    let headers = state
                        .get_grpc_headers_ucs(ucs::types::ExecutionMode::Primary)
                        .lineage_ids(router_data.get_lineage_ids());

                    let response = client
                        .payment_authorize(grpc_request, auth_metadata, headers)
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("UCS payment_authorize call failed")?;

                    let (payments_response, attempt_status, http_status_code) =
                        ucs::handle_unified_connector_service_response_for_payment_authorize(
                            response.into_inner(),
                        )
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to handle UCS response")?;

                    router_data.response = payments_response;
                    router_data.status = attempt_status;
                    router_data.connector_http_status_code = Some(http_status_code);
                }

                Ok(router_data)
            }
            // For non-Trigger actions, return router_data as-is
            // These are typically handled differently (webhooks, status updates, etc.)
            _ => Ok(router_data),
        }
        .map_err(|e| {
            hyperswitch_interfaces::errors::ConnectorError::ProcessingStepFailed(Some(
                e.to_string(),
            ))
        })
    }
}

// Implementation for PSync flow
#[async_trait]
impl
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        api::PSync,
        PaymentsSyncData,
        PaymentsResponseData,
    > for UnifiedConnectorServiceGateway<api::PSync>
{
    async fn execute(
        &self,
        state: &SessionState,
        mut router_data: RouterData<api::PSync, PaymentsSyncData, PaymentsResponseData>,
        connector: &api::ConnectorData,
        merchant_connector_account: &MerchantConnectorAccountType,
        call_connector_action: CallConnectorAction,
    ) -> Result<
        RouterData<api::PSync, PaymentsSyncData, PaymentsResponseData>,
        hyperswitch_interfaces::errors::ConnectorError,
    > {
        match call_connector_action {
            CallConnectorAction::Trigger => {
                let client = state
                    .grpc_client
                    .unified_connector_service_client
                    .as_ref()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("UCS client not available")?;

                let grpc_request = ucs::transformers::PaymentServiceGetRequest::foreign_try_from(
                    &router_data,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to transform RouterData to gRPC request")?;

                let auth_metadata = ucs::build_unified_connector_service_auth_metadata(
                    merchant_connector_account,
                    &connector.connector_name,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to build auth metadata")?;

                let headers = state
                    .get_grpc_headers_ucs(ucs::types::ExecutionMode::Primary)
                    .lineage_ids(router_data.get_lineage_ids());

                let response = client
                    .payment_get(grpc_request, auth_metadata, headers)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("UCS payment_get call failed")?;

                let (payments_response, attempt_status, http_status_code) =
                    ucs::handle_unified_connector_service_response_for_payment_get(
                        response.into_inner(),
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to handle UCS response")?;

                router_data.response = payments_response;
                router_data.status = attempt_status;
                router_data.connector_http_status_code = Some(http_status_code);

                Ok(router_data)
            }
            _ => Ok(router_data),
        }
        .map_err(|e| {
            hyperswitch_interfaces::errors::ConnectorError::ProcessingStepFailed(Some(
                e.to_string(),
            ))
        })
    }
}

// Implementation for SetupMandate flow
#[async_trait]
impl
    gateway_interface::PaymentGateway<
        SessionState,
        api::ConnectorData,
        MerchantConnectorAccountType,
        api::SetupMandate,
        SetupMandateRequestData,
        PaymentsResponseData,
    > for UnifiedConnectorServiceGateway<api::SetupMandate>
{
    async fn execute(
        &self,
        state: &SessionState,
        mut router_data: RouterData<api::SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        connector: &api::ConnectorData,
        merchant_connector_account: &MerchantConnectorAccountType,
        call_connector_action: CallConnectorAction,
    ) -> Result<
        RouterData<api::SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        hyperswitch_interfaces::errors::ConnectorError,
    >
    {
        match call_connector_action {
            CallConnectorAction::Trigger => {
                let client = state
                    .grpc_client
                    .unified_connector_service_client
                    .as_ref()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("UCS client not available")?;

                let grpc_request =
                    ucs::transformers::PaymentServiceRegisterRequest::foreign_try_from(
                        &router_data,
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to transform RouterData to gRPC request")?;

                let auth_metadata = ucs::build_unified_connector_service_auth_metadata(
                    merchant_connector_account,
                    &connector.connector_name,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to build auth metadata")?;

                let headers = state
                    .get_grpc_headers_ucs(ucs::types::ExecutionMode::Primary)
                    .lineage_ids(router_data.get_lineage_ids());

                let response = client
                    .payment_setup_mandate(grpc_request, auth_metadata, headers)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("UCS payment_setup_mandate call failed")?;

                let (payments_response, attempt_status, http_status_code) =
                    ucs::handle_unified_connector_service_response_for_payment_setup_mandate(
                        response.into_inner(),
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to handle UCS response")?;

                router_data.response = payments_response;
                router_data.status = attempt_status;
                router_data.connector_http_status_code = Some(http_status_code);

                Ok(router_data)
            }
            _ => Ok(router_data),
        }
        .map_err(|e| {
            hyperswitch_interfaces::errors::ConnectorError::ProcessingStepFailed(Some(
                e.to_string(),
            ))
        })
    }
}