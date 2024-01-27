pub mod post_authn;
pub mod pre_authn;
pub mod types;
pub(crate) mod utils;

pub mod transformers;

use api_models::payments;
use common_enums::Currency;
use common_utils::errors::CustomResult;

use crate::{
    core::{
        errors::ApiErrorResponse,
        payments::{self as payments_core, CallConnectorAction},
    },
    routes::AppState,
    services,
    types::{self as core_types, api, domain},
};

pub async fn perform_authentication(
    state: &AppState,
    authentication_provider: String,
    payment_method_data: payments::PaymentMethodData,
    payment_method: common_enums::PaymentMethod,
    billing_address: domain::Address,
    shipping_address: domain::Address,
    browser_details: core_types::BrowserInformation,
    merchant_account: core_types::domain::MerchantAccount,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    acquirer_details: Option<core_types::api::authentication::AcquirerDetails>,
    amount: Option<i64>,
    currency: Option<Currency>,
    message_category: core_types::api::authentication::MessageCategory,
    device_channel: String,
    three_ds_server_trans_id: String,
) -> CustomResult<core_types::api::authentication::AuthenticationResponse, ApiErrorResponse> {
    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &authentication_provider,
        api::GetToken::Connector,
        merchant_connector_account.get_mca_id(),
    )?;
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
        acquirer_details,
        amount,
        currency,
        message_category,
        device_channel,
        merchant_account,
        merchant_connector_account,
        three_ds_server_trans_id,
    )?;
    let response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        CallConnectorAction::Trigger,
        None,
    )
    .await
    .map_err(|_err| ApiErrorResponse::InternalServerError)?;
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
    })
}
