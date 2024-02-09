pub mod post_authn;
pub mod pre_authn;
pub(crate) mod utils;

pub mod transformers;
pub mod types;

use api_models::payments;
use common_enums::Currency;
use common_utils::errors::CustomResult;

use super::errors::ConnectorErrorExt;
use crate::{
    core::{
        errors::ApiErrorResponse,
        payments::{self as payments_core, CallConnectorAction},
    },
    routes::AppState,
    services,
    types::{self as core_types, api, storage},
};

#[allow(clippy::too_many_arguments)]
pub async fn perform_authentication(
    state: &AppState,
    authentication_provider: String,
    payment_method_data: payments::PaymentMethodData,
    payment_method: common_enums::PaymentMethod,
    billing_address: api_models::payments::Address,
    shipping_address: api_models::payments::Address,
    browser_details: core_types::BrowserInformation,
    merchant_account: core_types::domain::MerchantAccount,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    amount: Option<i64>,
    currency: Option<Currency>,
    message_category: api::authentication::MessageCategory,
    device_channel: String,
    authentication_data: (types::AuthenticationData, storage::Authentication),
    return_url: Option<String>,
    sdk_information: Option<payments::SDKInformation>,
) -> CustomResult<core_types::api::authentication::AuthenticationResponse, ApiErrorResponse> {
    let connector_data =
        api::AuthenticationConnectorData::get_connector_by_name(&authentication_provider)?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::Authentication,
        core_types::ConnectorAuthenticationRequestData,
        core_types::ConnectorAuthenticationResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = transformers::construct_authentication_router_data(
        authentication_provider.clone(),
        payment_method_data,
        payment_method,
        billing_address,
        shipping_address,
        browser_details,
        amount,
        currency,
        message_category,
        device_channel,
        merchant_account,
        merchant_connector_account,
        authentication_data,
        return_url,
        sdk_information,
    )?;
    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payment_failed_response()?;
    let submit_evidence_response =
        response
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: authentication_provider,
                status_code: err.status_code,
                reason: err.reason,
            })?;
    Ok(core_types::api::AuthenticationResponse {
        trans_status: submit_evidence_response.trans_status,
        acs_url: submit_evidence_response.acs_url,
        challenge_request: submit_evidence_response.challenge_request,
        acs_reference_number: submit_evidence_response.acs_reference_number,
        acs_trans_id: submit_evidence_response.acs_trans_id,
        three_dsserver_trans_id: submit_evidence_response.three_dsserver_trans_id,
        acs_signed_content: submit_evidence_response.acs_signed_content,
    })
}

pub async fn perform_post_authentication(
    state: &AppState,
    authentication_provider: String,
    merchant_account: core_types::domain::MerchantAccount,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    authentication_data: types::AuthenticationData,
) -> CustomResult<core_types::api::authentication::PostAuthenticationResponse, ApiErrorResponse> {
    let connector_data =
        api::AuthenticationConnectorData::get_connector_by_name(&authentication_provider)?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PostAuthentication,
        core_types::ConnectorPostAuthenticationRequestData,
        core_types::ConnectorPostAuthenticationResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = transformers::construct_post_authentication_router_data(
        authentication_provider.clone(),
        merchant_account,
        merchant_connector_account,
        authentication_data,
    )?;
    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payment_failed_response()?;
    let submit_evidence_response =
        response
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: authentication_provider,
                status_code: err.status_code,
                reason: err.reason,
            })?;
    Ok(core_types::api::PostAuthenticationResponse {
        trans_status: submit_evidence_response.trans_status,
        authentication_value: submit_evidence_response.authentication_value,
        eci: submit_evidence_response.eci,
    })
}
