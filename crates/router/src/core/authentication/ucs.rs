use common_utils::{errors::CustomResult, ucs_types};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_flow_types::authentication as auth_flows,
    router_request_types::authentication as auth_request,
    router_response_types::{AuthenticationResponseData, PaymentsResponseData, RedirectForm},
};
use hyperswitch_interfaces::{
    errors as interface_errors,
    unified_connector_service::transformers::UnifiedConnectorServiceError,
};
use hyperswitch_masking::ExposeInterface;
use unified_connector_service_client::payments as payments_grpc;

use crate::{
    consts,
    core::{payments::gateway::context::RouterGatewayContext, unified_connector_service},
    routes::SessionState,
    types::{transformers::ForeignTryFrom, ErrorResponse, RouterData},
};

type AuthResult<F, Req> =
    CustomResult<RouterData<F, Req, AuthenticationResponseData>, interface_errors::ConnectorError>;

fn ucs_headers<F, Req>(
    state: &SessionState,
    router_data: &RouterData<F, Req, AuthenticationResponseData>,
    context: &RouterGatewayContext,
) -> external_services::grpc_client::GrpcHeadersUcsBuilderFinal {
    let merchant_reference_id = unified_connector_service::parse_merchant_reference_id(
        context
            .header_payload
            .x_reference_id
            .as_deref()
            .unwrap_or(router_data.payment_id.as_str()),
    )
    .map(ucs_types::UcsReferenceId::Payment);
    state
        .get_grpc_headers_ucs(context.execution_mode)
        .payment_method(Some(router_data.payment_method))
        .payment_method_type(router_data.payment_method_type)
        .external_vault_proxy_metadata(None)
        .merchant_reference_id(merchant_reference_id)
        .resource_id(None)
        .lineage_ids(context.lineage_ids.clone())
}

fn ucs_client(
    state: &SessionState,
) -> CustomResult<
    external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceClient,
    interface_errors::ConnectorError,
> {
    state
        .grpc_client
        .unified_connector_service_client
        .clone()
        .ok_or(interface_errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to fetch Unified Connector Service client")
}

fn ucs_auth_metadata<F, Req>(
    router_data: &RouterData<F, Req, AuthenticationResponseData>,
    context: &RouterGatewayContext,
) -> CustomResult<
    external_services::grpc_client::unified_connector_service::ConnectorAuthMetadata,
    interface_errors::ConnectorError,
> {
    unified_connector_service::build_unified_connector_service_auth_metadata(
        context.merchant_connector_account.clone(),
        context.processor.get_account().get_id(),
        router_data.connector.clone(),
    )
    .change_context(interface_errors::ConnectorError::RequestEncodingFailed)
    .attach_printable("Failed to construct request metadata")
}

fn transaction_response(
    response: PaymentsResponseData,
) -> (
    Option<hyperswitch_domain_models::router_request_types::UcsAuthenticationData>,
    Option<RedirectForm>,
) {
    match response {
        PaymentsResponseData::TransactionResponse {
            authentication_data,
            redirection_data,
            ..
        } => (authentication_data.map(|data| *data), *redirection_data),
        _ => (None, None),
    }
}

fn pre_authentication_response(
    response: PaymentsResponseData,
) -> CustomResult<AuthenticationResponseData, UnifiedConnectorServiceError> {
    let (authentication_data, redirection_data) = transaction_response(response);
    let (three_ds_method_data, three_ds_method_url) = match &redirection_data {
        Some(RedirectForm::Form { form_fields, .. }) => (
            form_fields.get(consts::UCS_DDC_METHOD_DATA_KEY).cloned(),
            form_fields.get(consts::UCS_DDC_METHOD_URL_KEY).cloned(),
        ),
        _ => (None, None),
    };
    let message_version = authentication_data
        .as_ref()
        .and_then(|data| data.message_version.clone())
        .unwrap_or_else(|| common_utils::types::SemanticVersion::new(2, 2, 0));
    let threeds_server_transaction_id = authentication_data
        .as_ref()
        .and_then(|data| data.threeds_server_transaction_id.clone())
        .ok_or(UnifiedConnectorServiceError::ResponseDeserializationFailed)
        .attach_printable("UCS pre-authenticate response missing threeds_server_transaction_id")?;

    Ok(AuthenticationResponseData::PreAuthNResponse {
        connector_authentication_id: threeds_server_transaction_id.clone(),
        threeds_server_transaction_id,
        maximum_supported_3ds_version: message_version.clone(),
        three_ds_method_data,
        three_ds_method_url,
        message_version,
        connector_metadata: None,
        directory_server_id: authentication_data
            .as_ref()
            .and_then(|data| data.ds_trans_id.clone()),
        scheme_id: None,
    })
}

fn post_authentication_response(response: PaymentsResponseData) -> AuthenticationResponseData {
    let (authentication_data, _) = transaction_response(response);
    AuthenticationResponseData::PostAuthNResponse {
        trans_status: authentication_data
            .as_ref()
            .and_then(|data| data.trans_status.clone())
            .unwrap_or(common_enums::TransactionStatus::VerificationNotPerformed),
        authentication_value: authentication_data
            .as_ref()
            .and_then(|data| data.cavv.clone()),
        eci: authentication_data
            .as_ref()
            .and_then(|data| data.eci.clone()),
        challenge_cancel: authentication_data
            .as_ref()
            .and_then(|data| data.challenge_cancel.clone()),
        challenge_code_reason: authentication_data
            .as_ref()
            .and_then(|data| data.challenge_code_reason.clone()),
    }
}

fn apply_response<F, Req>(
    router_data: &mut RouterData<F, Req, AuthenticationResponseData>,
    response: Result<AuthenticationResponseData, ErrorResponse>,
    status_code: u16,
    raw_connector_response: Option<hyperswitch_masking::Secret<String>>,
) {
    router_data.response = response;
    router_data.raw_connector_response = raw_connector_response.map(|raw| raw.expose().into());
    router_data.connector_http_status_code = Some(status_code);
}

pub async fn call_unified_connector_service_pre_authentication(
    router_data: &RouterData<
        auth_flows::PreAuthentication,
        auth_request::PreAuthNRequestData,
        AuthenticationResponseData,
    >,
    state: &SessionState,
    context: &RouterGatewayContext,
) -> AuthResult<auth_flows::PreAuthentication, auth_request::PreAuthNRequestData> {
    let client = ucs_client(state)?;
    let request =
        payments_grpc::PaymentMethodAuthenticationServicePreAuthenticateRequest::foreign_try_from(
            router_data,
        )
        .change_context(interface_errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to construct UCS pre-authenticate request")?;
    let auth_metadata = ucs_auth_metadata(router_data, context)?;

    Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
        router_data.clone(),
        state,
        request,
        ucs_headers(state, router_data, context),
        context.rollout_settings(),
        |mut router_data, request, grpc_headers| async move {
            let response = client
                .payment_pre_authenticate(request, auth_metadata, grpc_headers)
                .await
                .attach_printable("Failed to pre authenticate")?
                .into_inner();
            let (result, status_code) =
                unified_connector_service::handle_unified_connector_service_response_for_payment_pre_authenticate(
                    response.clone(),
                    router_data.status,
                )
                .attach_printable("Failed to deserialize UCS response")?;
            let result = match result {
                Ok((response, _)) => Ok(pre_authentication_response(response)?),
                Err(err) => Err(err),
            };
            apply_response(
                &mut router_data,
                result,
                status_code,
                response.raw_connector_response.clone(),
            );
            Ok((router_data, (), response))
        },
    ))
    .await
    .map(|(router_data, ())| router_data)
    .change_context(interface_errors::ConnectorError::ResponseHandlingFailed)
}

pub async fn call_unified_connector_service_authentication(
    router_data: &RouterData<
        auth_flows::Authentication,
        auth_request::ConnectorAuthenticationRequestData,
        AuthenticationResponseData,
    >,
    state: &SessionState,
    context: &RouterGatewayContext,
) -> AuthResult<auth_flows::Authentication, auth_request::ConnectorAuthenticationRequestData> {
    let client = ucs_client(state)?;
    let request =
        payments_grpc::PaymentMethodAuthenticationServiceAuthenticateRequest::foreign_try_from(
            router_data,
        )
        .change_context(interface_errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to construct UCS authenticate request")?;
    let auth_metadata = ucs_auth_metadata(router_data, context)?;

    Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
        router_data.clone(),
        state,
        request,
        ucs_headers(state, router_data, context),
        context.rollout_settings(),
        |mut router_data, request, grpc_headers| async move {
            let response = client
                .payment_authenticate(request, auth_metadata, grpc_headers)
                .await
                .attach_printable("Failed to authenticate")?
                .into_inner();
            let (result, status_code) =
                unified_connector_service::handle_unified_connector_service_response_for_payment_authenticate(
                    response.clone(),
                    router_data.status,
                )
                .attach_printable("Failed to deserialize UCS response")?;
            let result = match result {
                Ok((response, _)) => Ok(super::parse_ucs_authenticate_response(&response)
                    .change_context(UnifiedConnectorServiceError::ResponseDeserializationFailed)?
                    .authenticate_response_data),
                Err(err) => Err(err),
            };
            apply_response(
                &mut router_data,
                result,
                status_code,
                response.raw_connector_response.clone(),
            );
            Ok((router_data, (), response))
        },
    ))
    .await
    .map(|(router_data, ())| router_data)
    .change_context(interface_errors::ConnectorError::ResponseHandlingFailed)
}

pub async fn call_unified_connector_service_post_authentication(
    router_data: &RouterData<
        auth_flows::PostAuthentication,
        auth_request::ConnectorPostAuthenticationRequestData,
        AuthenticationResponseData,
    >,
    state: &SessionState,
    context: &RouterGatewayContext,
) -> AuthResult<auth_flows::PostAuthentication, auth_request::ConnectorPostAuthenticationRequestData>
{
    let client = ucs_client(state)?;
    let request =
        payments_grpc::PaymentMethodAuthenticationServicePostAuthenticateRequest::foreign_try_from(
            router_data,
        )
        .change_context(interface_errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failed to construct UCS post-authenticate request")?;
    let auth_metadata = ucs_auth_metadata(router_data, context)?;

    Box::pin(unified_connector_service::ucs_logging_wrapper_granular(
        router_data.clone(),
        state,
        request,
        ucs_headers(state, router_data, context),
        context.rollout_settings(),
        |mut router_data, request, grpc_headers| async move {
            let response = client
                .payment_post_authenticate(request, auth_metadata, grpc_headers)
                .await
                .attach_printable("Failed to post authenticate")?
                .into_inner();
            let (result, status_code) =
                unified_connector_service::handle_unified_connector_service_response_for_payment_post_authenticate(
                    response.clone(),
                    router_data.status,
                )
                .attach_printable("Failed to deserialize UCS response")?;
            let result = result.map(|(response, _)| post_authentication_response(response));
            apply_response(
                &mut router_data,
                result,
                status_code,
                response.raw_connector_response.clone(),
            );
            Ok((router_data, (), response))
        },
    ))
    .await
    .map(|(router_data, ())| router_data)
    .change_context(interface_errors::ConnectorError::ResponseHandlingFailed)
}
