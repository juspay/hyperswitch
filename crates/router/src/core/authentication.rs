pub mod post_authn;
pub mod pre_authn;
pub(crate) mod utils;

pub mod transformers;
pub mod types;

use api_models::payments;
use common_enums::Currency;
use common_utils::errors::CustomResult;

use crate::{
    core::{errors::ApiErrorResponse, payments as payments_core},
    routes::AppState,
    types::{self as core_types, api, storage},
};

#[allow(clippy::too_many_arguments)]
pub async fn perform_authentication(
    state: &AppState,
    authentication_connector: String,
    payment_method_data: payments::PaymentMethodData,
    payment_method: common_enums::PaymentMethod,
    billing_address: api_models::payments::Address,
    shipping_address: Option<api_models::payments::Address>,
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
        authentication_data,
        return_url,
        sdk_information,
    )?;
    let response =
        utils::do_auth_connector_call(state, authentication_connector.clone(), router_data).await?;
    let submit_evidence_response =
        response
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: authentication_connector,
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

pub async fn perform_post_authentication<F: Clone + Send>(
    state: &AppState,
    authentication_connector: String,
    merchant_account: core_types::domain::MerchantAccount,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    authentication_flow_input: types::PostAuthenthenticationFlowInput<'_, F>,
) -> CustomResult<(), ApiErrorResponse> {
    match authentication_flow_input {
        types::PostAuthenthenticationFlowInput::PaymentAuthNFlow {
            payment_data,
            authentication_data: (authentication, authentication_data),
        } => {
            let router_data = transformers::construct_post_authentication_router_data(
                authentication_connector.clone(),
                merchant_account,
                merchant_connector_account,
                authentication_data,
            )?;
            let router_data =
                utils::do_auth_connector_call(state, authentication_connector, router_data).await?;
            let updated_authentication = utils::update_trackers(
                state,
                router_data,
                authentication,
                payment_data.token.clone(),
            )
            .await?;
            payment_data.authentication = Some(updated_authentication);
        }
        types::PostAuthenthenticationFlowInput::PaymentMethodAuthNFlow { other_fields: _ } => {
            // todo!("Payment method post authN operation");
        }
    }
    Ok(())
}

pub async fn perform_pre_authentication<F: Clone + Send>(
    state: &AppState,
    authentication_connector_name: String,
    authentication_flow_input: types::PreAuthenthenticationFlowInput<'_, F>,
    merchant_account: &core_types::domain::MerchantAccount,
    three_ds_connector_account: payments_core::helpers::MerchantConnectorAccountType,
) -> CustomResult<(), ApiErrorResponse> {
    let authentication = utils::create_new_authentication(
        state,
        merchant_account.merchant_id.clone(),
        authentication_connector_name.clone(),
    )
    .await?;
    match authentication_flow_input {
        types::PreAuthenthenticationFlowInput::PaymentAuthNFlow {
            payment_data,
            should_continue_confirm_transaction,
            card_number,
        } => {
            let router_data = transformers::construct_pre_authentication_router_data(
                authentication_connector_name.clone(),
                card_number,
                &three_ds_connector_account,
                merchant_account.merchant_id.clone(),
            )?;
            let router_data =
                utils::do_auth_connector_call(state, authentication_connector_name, router_data)
                    .await?;
            let (authentication, authentication_data) = utils::update_trackers(
                state,
                router_data,
                authentication,
                payment_data.token.clone(),
            )
            .await?;
            if authentication_data.is_separate_authn_required() {
                *should_continue_confirm_transaction = false;
            }
            payment_data.authentication = Some((authentication, authentication_data))
        }
        types::PreAuthenthenticationFlowInput::PaymentMethodAuthNFlow {
            card_number: _,
            other_fields: _,
        } => {
            // todo!("Payment method authN operation");
        }
    };
    Ok(())
}
