pub mod post_authn;
pub mod pre_authn;
pub(crate) mod utils;

pub mod transformers;
pub mod types;

use api_models::payments;
use common_enums::Currency;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;

use super::errors::{self, ConnectorErrorExt};
use crate::{
    core::{
        errors::ApiErrorResponse,
        payments::{self as payments_core, CallConnectorAction},
    },
    routes::AppState,
    services,
    types::{self as core_types, api, authentication::AuthenticationResponseData, storage},
};

#[allow(clippy::too_many_arguments)]
pub async fn perform_authentication(
    state: &AppState,
    authentication_connector: String,
    payment_method_data: payments::PaymentMethodData,
    payment_method: common_enums::PaymentMethod,
    billing_address: api_models::payments::Address,
    shipping_address: Option<api_models::payments::Address>,
    browser_details: Option<core_types::BrowserInformation>,
    merchant_account: core_types::domain::MerchantAccount,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    amount: Option<i64>,
    currency: Option<Currency>,
    message_category: api::authentication::MessageCategory,
    device_channel: payments::DeviceChannel,
    authentication_data: (types::AuthenticationData, storage::Authentication),
    return_url: Option<String>,
    sdk_information: Option<payments::SDKInformation>,
) -> CustomResult<core_types::api::authentication::AuthenticationResponse, ApiErrorResponse> {
    let connector_data =
        api::AuthenticationConnectorData::get_connector_by_name(&authentication_connector)?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::Authentication,
        core_types::ConnectorAuthenticationRequestData,
        AuthenticationResponseData,
    > = connector_data.connector.get_connector_integration();
    let router_data = transformers::construct_authentication_router_data(
        authentication_connector.clone(),
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
        authentication_data.clone(),
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
    let (_authentication, _authentication_data) =
        utils::update_trackers(state, response.clone(), authentication_data.1, None).await?;
    let authentication_response =
        response
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: authentication_connector,
                status_code: err.status_code,
                reason: err.reason,
            })?;
    match authentication_response {
        AuthenticationResponseData::AuthNResponse {
            authn_flow_type,
            trans_status,
            ..
        } => Ok(match authn_flow_type {
            core_types::authentication::AuthNFlowType::Challenge {
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                three_dsserver_trans_id,
                acs_signed_content,
            } => core_types::api::AuthenticationResponse {
                trans_status,
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                three_dsserver_trans_id,
                acs_signed_content,
            },
            core_types::authentication::AuthNFlowType::Frictionless => {
                core_types::api::AuthenticationResponse {
                    trans_status,
                    acs_url: None,
                    challenge_request: None,
                    acs_reference_number: None,
                    acs_trans_id: None,
                    three_dsserver_trans_id: None,
                    acs_signed_content: None,
                }
            }
        }),
        _ => Err(errors::ApiErrorResponse::InternalServerError.into())
            .attach_printable("unexpected response in authentication flow")?,
    }
}

pub async fn perform_post_authentication(
    state: &AppState,
    authentication_connector: String,
    merchant_account: core_types::domain::MerchantAccount,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    authentication_data: types::AuthenticationData,
) -> CustomResult<core_types::api::authentication::PostAuthenticationResponse, ApiErrorResponse> {
    let connector_data =
        api::AuthenticationConnectorData::get_connector_by_name(&authentication_connector)?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PostAuthentication,
        core_types::ConnectorPostAuthenticationRequestData,
        core_types::ConnectorPostAuthenticationResponse,
    > = connector_data.connector.get_connector_integration();
    let router_data = transformers::construct_post_authentication_router_data(
        authentication_connector.clone(),
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
    let post_authentication_response =
        response
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: authentication_connector,
                status_code: err.status_code,
                reason: err.reason,
            })?;
    Ok(core_types::api::PostAuthenticationResponse {
        trans_status: post_authentication_response.trans_status,
        authentication_value: post_authentication_response.authentication_value,
        eci: post_authentication_response.eci,
    })
}
