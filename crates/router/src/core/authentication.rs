pub(crate) mod utils;

pub mod transformers;
pub mod types;

use api_models::payments;
use common_enums::Currency;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use masking::ExposeInterface;

use super::errors::{self, StorageErrorExt};
use crate::{
    core::{errors::ApiErrorResponse, payments as payments_core},
    routes::AppState,
    types::{self as core_types, api, domain, storage},
    utils::check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata,
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
    let authentication =
        utils::update_trackers(state, response.clone(), authentication_data, None).await?;
    response
        .response
        .map_err(|err| ApiErrorResponse::ExternalConnectorError {
            code: err.code,
            message: err.message,
            connector: authentication_connector,
            status_code: err.status_code,
            reason: err.reason,
        })?;
    core_types::api::authentication::AuthenticationResponse::try_from(authentication)
}

pub async fn perform_post_authentication(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    business_profile: core_types::storage::BusinessProfile,
    authentication_id: String,
) -> CustomResult<storage::Authentication, ApiErrorResponse> {
    let (authentication_connector, three_ds_connector_account) =
        utils::get_authentication_connector_data(state, key_store, &business_profile).await?;
    let authentication = state
        .store
        .find_authentication_by_merchant_id_authentication_id(
            business_profile.merchant_id.clone(),
            authentication_id.clone(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Error while fetching authentication record with authentication_id {authentication_id}"))?;
    let is_pull_mechanism_enabled =
        check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata(
            three_ds_connector_account
                .get_metadata()
                .map(|metadata| metadata.expose()),
        );
    let authentication = if !authentication.authentication_status.is_terminal_status()
        && is_pull_mechanism_enabled
    {
        let router_data = transformers::construct_post_authentication_router_data(
            authentication_connector.to_string(),
            business_profile.clone(),
            three_ds_connector_account,
            &authentication,
        )?;
        let router_data =
            utils::do_auth_connector_call(state, authentication_connector.to_string(), router_data)
                .await?;
        utils::update_trackers(state, router_data, authentication.clone(), None).await?
    } else {
        authentication
    };
    Ok(authentication)
}

pub async fn perform_pre_authentication(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    card_number: cards::CardNumber,
    token: String,
    business_profile: &core_types::storage::BusinessProfile,
    acquirer_details: Option<types::AcquirerDetails>,
    payment_id: Option<String>,
) -> CustomResult<storage::Authentication, ApiErrorResponse> {
    let (authentication_connector, three_ds_connector_account) =
        utils::get_authentication_connector_data(state, key_store, business_profile).await?;
    let authentication_connector_name = authentication_connector.to_string();
    let authentication = utils::create_new_authentication(
        state,
        business_profile.merchant_id.clone(),
        authentication_connector_name.clone(),
        token,
        business_profile.profile_id.clone(),
        payment_id,
        three_ds_connector_account
            .get_mca_id()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while finding mca_id from merchant_connector_account")?,
    )
    .await?;

    let router_data = transformers::construct_pre_authentication_router_data(
        authentication_connector_name.clone(),
        card_number,
        &three_ds_connector_account,
        business_profile.merchant_id.clone(),
    )?;
    let router_data =
        utils::do_auth_connector_call(state, authentication_connector_name, router_data).await?;

    utils::update_trackers(state, router_data, authentication, acquirer_details).await
}
