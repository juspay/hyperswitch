pub mod post_authn;
pub mod pre_authn;
pub(crate) mod utils;

pub mod transformers;
pub mod types;

use api_models::payments;
use common_enums::Currency;
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use error_stack::ResultExt;
use masking::PeekInterface;

use super::errors;
use crate::{
    core::{errors::ApiErrorResponse, payments as payments_core},
    routes::AppState,
    types::{self as core_types, api, authentication::AuthenticationResponseData, storage},
    utils::OptionExt,
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
    email: Option<common_utils::pii::Email>,
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
        authentication_data.clone(),
        return_url,
        sdk_information,
        email,
    )?;
    let response =
        utils::do_auth_connector_call(state, authentication_connector.clone(), router_data).await?;
    let (_authentication, _authentication_data) =
        utils::update_trackers(state, response.clone(), authentication_data.1, None, None).await?;
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
            core_types::authentication::AuthNFlowType::Challenge(challenge_params) => {
                core_types::api::AuthenticationResponse {
                    trans_status,
                    acs_url: challenge_params.acs_url,
                    challenge_request: challenge_params.challenge_request,
                    acs_reference_number: challenge_params.acs_reference_number,
                    acs_trans_id: challenge_params.acs_trans_id,
                    three_dsserver_trans_id: challenge_params.three_dsserver_trans_id,
                    acs_signed_content: challenge_params.acs_signed_content,
                }
            }
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
                None,
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
    payment_connector_account: payments_core::helpers::MerchantConnectorAccountType,
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
            let acquirer_details: types::AcquirerDetails = payment_connector_account
                .get_metadata()
                .get_required_value("merchant_connector_account.metadata")?
                .peek()
                .clone()
                .parse_value("AcquirerDetails")
                .change_context(ApiErrorResponse::PreconditionFailed { message: "acquirer_bin and acquirer_merchant_id not found in Payment Connector's Metadata".to_string()})?;

            let (authentication, authentication_data) = utils::update_trackers(
                state,
                router_data,
                authentication,
                payment_data.token.clone(),
                Some(acquirer_details),
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
