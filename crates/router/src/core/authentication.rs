pub(crate) mod utils;

pub mod transformers;
pub mod types;

use api_models::payments;
use common_enums::Currency;
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface};

use super::errors;
use crate::{
    core::{errors::ApiErrorResponse, payments as payments_core},
    routes::AppState,
    types::{self as core_types, api, authentication::AuthenticationResponseData, storage},
    utils::{check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata, OptionExt},
};

// 15 minutes = 900 seconds
const POLL_ID_TTL: i64 = 900;

#[allow(clippy::too_many_arguments)]
pub async fn perform_authentication(
    state: &AppState,
    authentication_connector: String,
    payment_method_data: payments::PaymentMethodData,
    payment_method: common_enums::PaymentMethod,
    billing_address: api_models::payments::Address,
    shipping_address: Option<api_models::payments::Address>,
    browser_details: Option<core_types::BrowserInformation>,
    business_profile: core_types::storage::BusinessProfile,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    amount: Option<i64>,
    currency: Option<Currency>,
    message_category: api::authentication::MessageCategory,
    device_channel: payments::DeviceChannel,
    authentication_data: storage::Authentication,
    return_url: Option<String>,
    sdk_information: Option<payments::SdkInformation>,
    threeds_method_comp_ind: api_models::payments::ThreeDsCompletionIndicator,
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
        business_profile,
        merchant_connector_account,
        authentication_data.clone(),
        return_url,
        sdk_information,
        threeds_method_comp_ind,
        email,
    )?;
    let response =
        utils::do_auth_connector_call(state, authentication_connector.clone(), router_data).await?;
    utils::update_trackers(state, response.clone(), authentication_data, None, None).await?;
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
        _ => Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable("unexpected response in authentication flow")?,
    }
}

pub async fn perform_post_authentication<F: Clone + Send>(
    state: &AppState,
    authentication_connector: String,
    business_profile: core_types::storage::BusinessProfile,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    authentication_flow_input: types::PostAuthenthenticationFlowInput<'_, F>,
) -> CustomResult<(), ApiErrorResponse> {
    match authentication_flow_input {
        types::PostAuthenthenticationFlowInput::PaymentAuthNFlow {
            payment_data,
            authentication,
            should_continue_confirm_transaction,
        } => {
            let is_pull_mechanism_enabled =
                check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata(
                    merchant_connector_account
                        .get_metadata()
                        .map(|metadata| metadata.expose()),
                );
            let authentication_status =
                if !authentication.authentication_status.is_terminal_status()
                    && is_pull_mechanism_enabled
                {
                    let router_data = transformers::construct_post_authentication_router_data(
                        authentication_connector.clone(),
                        business_profile.clone(),
                        merchant_connector_account,
                        &authentication,
                    )?;
                    let router_data =
                        utils::do_auth_connector_call(state, authentication_connector, router_data)
                            .await?;
                    let updated_authentication = utils::update_trackers(
                        state,
                        router_data,
                        authentication.clone(),
                        payment_data.token.clone(),
                        None,
                    )
                    .await?;
                    let authentication_status = updated_authentication.authentication_status;
                    payment_data.authentication = Some(updated_authentication);
                    authentication_status
                } else {
                    authentication.authentication_status
                };
            //If authentication is not successful, skip the payment connector flows and mark the payment as failure
            if !(authentication_status == api_models::enums::AuthenticationStatus::Success) {
                *should_continue_confirm_transaction = false;
            }
            // When authentication status is non-terminal, Set poll_id in redis to allow the fetch status of poll through retrieve_poll_status api from client
            if !authentication_status.is_terminal_status() {
                let req_poll_id = format!(
                    "external_authentication_{}",
                    payment_data.payment_intent.payment_id
                );
                let poll_id = super::utils::get_poll_id(
                    business_profile.merchant_id.clone(),
                    req_poll_id.clone(),
                );
                let redis_conn = state
                    .store
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to get redis connection")?;
                redis_conn
                    .set_key_with_expiry(
                        &poll_id,
                        api_models::poll::PollStatus::Pending.to_string(),
                        POLL_ID_TTL,
                    )
                    .await
                    .change_context(errors::StorageError::KVError)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to add poll_id in redis")?;
            }
        }
        types::PostAuthenthenticationFlowInput::PaymentMethodAuthNFlow { other_fields: _ } => {
            // todo!("Payment method post authN operation");
        }
    }
    Ok(())
}

fn get_payment_id_from_pre_authentication_flow_input<F: Clone + Send>(
    pre_authentication_flow_input: &types::PreAuthenthenticationFlowInput<'_, F>,
) -> Option<String> {
    match pre_authentication_flow_input {
        types::PreAuthenthenticationFlowInput::PaymentAuthNFlow { payment_data, .. } => {
            Some(payment_data.payment_intent.payment_id.clone())
        }
        _ => None,
    }
}

pub async fn perform_pre_authentication<F: Clone + Send>(
    state: &AppState,
    authentication_connector_name: String,
    authentication_flow_input: types::PreAuthenthenticationFlowInput<'_, F>,
    business_profile: &core_types::storage::BusinessProfile,
    three_ds_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    payment_connector_account: payments_core::helpers::MerchantConnectorAccountType,
) -> CustomResult<(), ApiErrorResponse> {
    let payment_id = get_payment_id_from_pre_authentication_flow_input(&authentication_flow_input);
    let authentication = utils::create_new_authentication(
        state,
        business_profile.merchant_id.clone(),
        authentication_connector_name.clone(),
        business_profile.profile_id.clone(),
        payment_id,
        three_ds_connector_account
            .get_mca_id()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while finding mca_id from merchant_connector_account")?,
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
                business_profile.merchant_id.clone(),
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

            let authentication = utils::update_trackers(
                state,
                router_data,
                authentication,
                payment_data.token.clone(),
                Some(acquirer_details),
            )
            .await?;
            if authentication.is_separate_authn_required()
                || authentication.authentication_status.is_failed()
            {
                *should_continue_confirm_transaction = false;
            }
            payment_data.authentication = Some(authentication);
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
