pub mod post_authn;
pub mod pre_authn;
pub mod types;
pub(crate) mod utils;

use api_models::payments::PaymentMethodData;
use cards::CardNumber;

use crate::{
    core::payments,
    errors::RouterResult,
    types::{api::ConnectorCallType, domain},
    AppState,
};

pub async fn call_payment_3ds_service<F: Send + Clone>(
    state: &AppState,
    payment_data: &mut payments::PaymentData<F>,
    should_continue_confirm_transaction: &mut bool,
    connector_call_type: &ConnectorCallType,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<()> {
    let is_pre_authn_call = payment_data.authentication.is_none();
    let separate_authentication_requested = payment_data
        .payment_attempt
        .external_3ds_authentication_requested
        .unwrap_or(false);
    let connector_supports_separate_authn = utils::is_separate_authn_supported(connector_call_type);
    let card_number = payment_data.payment_method_data.as_ref().and_then(|pmd| {
        if let PaymentMethodData::Card(card) = pmd {
            Some(card.card_number.clone())
        } else {
            None
        }
    });
    if is_pre_authn_call {
        if separate_authentication_requested && connector_supports_separate_authn {
            if let Some(card_number) = card_number {
                let connector_account_for_3ds = "3d_secure_io".to_string();
                pre_authn::execute_pre_auth_flow(
                    state,
                    types::AuthenthenticationFlowInput::PaymentAuthNFlow {
                        payment_data,
                        should_continue_confirm_transaction,
                        card_number,
                    },
                    connector_account_for_3ds,
                    merchant_account,
                )
                .await;
            }
        }
        Ok(())
    } else {
        Ok(())
    }
}

async fn call_payment_method_3ds_service(_card_number: CardNumber) -> RouterResult<()> {
    Ok(())
}
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
    types::{self, api, domain},
};

pub async fn perform_authentication(
    state: &AppState,
    authentication_provider: String,
    payment_method_data: payments::PaymentMethodData,
    payment_method: common_enums::PaymentMethod,
    billing_address: domain::Address,
    shipping_address: domain::Address,
    browser_details: types::BrowserInformation,
    merchant_account: types::domain::MerchantAccount,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    acquirer_details: Option<types::api::authentication::AcquirerDetails>,
    amount: Option<i64>,
    currency: Option<Currency>,
    message_category: types::api::authentication::MessageCategory,
    device_channel: String,
    three_ds_server_trans_id: String,
) -> CustomResult<types::api::authentication::AuthenticationResponse, ApiErrorResponse> {
    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &authentication_provider,
        api::GetToken::Connector,
        merchant_connector_account.get_mca_id(),
    )?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::Authentication,
        types::ConnectorAuthenticationRequestData,
        types::ConnectorAuthenticationResponse,
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
    Ok(types::api::AuthenticationResponse {
        trans_status: submit_evidence_response.trans_status,
        acs_url: submit_evidence_response.acs_url,
        challenge_request: submit_evidence_response.challenge_request,
    })
}
