pub(crate) mod utils;

pub mod transformers;
pub mod types;

use api_models::payments;
use common_enums::Currency;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use masking::ExposeInterface;

use super::errors::StorageErrorExt;
use crate::{
    core::{errors::ApiErrorResponse, payments as payments_core},
    routes::SessionState,
    types::{self as core_types, api, domain, storage},
    utils::check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata,
};

#[allow(clippy::too_many_arguments)]
pub async fn perform_authentication(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    authentication_connector: String,
    payment_method_data: domain::PaymentMethodData,
    payment_method: common_enums::PaymentMethod,
    billing_address: hyperswitch_domain_models::address::Address,
    shipping_address: Option<hyperswitch_domain_models::address::Address>,
    browser_details: Option<core_types::BrowserInformation>,
    merchant_connector_account: payments_core::helpers::MerchantConnectorAccountType,
    amount: Option<common_utils::types::MinorUnit>,
    currency: Option<Currency>,
    message_category: api::authentication::MessageCategory,
    device_channel: payments::DeviceChannel,
    authentication_data: storage::Authentication,
    return_url: Option<String>,
    sdk_information: Option<payments::SdkInformation>,
    threeds_method_comp_ind: payments::ThreeDsCompletionIndicator,
    email: Option<common_utils::pii::Email>,
    webhook_url: String,
    three_ds_requestor_url: String,
    psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
    payment_id: common_utils::id_type::PaymentId,
    force_3ds_challenge: bool,
) -> CustomResult<api::authentication::AuthenticationResponse, ApiErrorResponse> {
    let router_data = transformers::construct_authentication_router_data(
        state,
        merchant_id,
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
        merchant_connector_account,
        authentication_data.clone(),
        return_url,
        sdk_information,
        threeds_method_comp_ind,
        email,
        webhook_url,
        three_ds_requestor_url,
        psd2_sca_exemption_type,
        payment_id,
        force_3ds_challenge,
    )?;
    let response = Box::pin(utils::do_auth_connector_call(
        state,
        authentication_connector.clone(),
        router_data,
    ))
    .await?;
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
    api::authentication::AuthenticationResponse::try_from(authentication)
}

pub async fn perform_post_authentication(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    business_profile: domain::Profile,
    authentication_id: String,
    payment_id: &common_utils::id_type::PaymentId,
) -> CustomResult<storage::Authentication, ApiErrorResponse> {
    let (authentication_connector, three_ds_connector_account) =
        utils::get_authentication_connector_data(state, key_store, &business_profile).await?;
    let is_pull_mechanism_enabled =
        check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata(
            three_ds_connector_account
                .get_metadata()
                .map(|metadata| metadata.expose()),
        );
    let authentication = state
        .store
        .find_authentication_by_merchant_id_authentication_id(
            &business_profile.merchant_id,
            authentication_id.clone(),
        )
        .await
        .to_not_found_response(ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Error while fetching authentication record with authentication_id {authentication_id}"))?;
    if !authentication.authentication_status.is_terminal_status() && is_pull_mechanism_enabled {
        let router_data = transformers::construct_post_authentication_router_data(
            state,
            authentication_connector.to_string(),
            business_profile,
            three_ds_connector_account,
            &authentication,
            payment_id,
        )?;
        let router_data =
            utils::do_auth_connector_call(state, authentication_connector.to_string(), router_data)
                .await?;
        utils::update_trackers(state, router_data, authentication, None).await
    } else {
        Ok(authentication)
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_pre_authentication(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    card: hyperswitch_domain_models::payment_method_data::Card,
    token: String,
    business_profile: &domain::Profile,
    acquirer_details: Option<types::AcquirerDetails>,
    payment_id: common_utils::id_type::PaymentId,
    organization_id: common_utils::id_type::OrganizationId,
) -> CustomResult<storage::Authentication, ApiErrorResponse> {
    let (authentication_connector, three_ds_connector_account) =
        utils::get_authentication_connector_data(state, key_store, business_profile).await?;
    let authentication_connector_name = authentication_connector.to_string();
    let authentication = utils::create_new_authentication(
        state,
        business_profile.merchant_id.clone(),
        authentication_connector_name.clone(),
        token,
        business_profile.get_id().to_owned(),
        payment_id.clone(),
        three_ds_connector_account
            .get_mca_id()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Error while finding mca_id from merchant_connector_account")?,
        organization_id,
    )
    .await?;

    let authentication = if authentication_connector.is_separate_version_call_required() {
        let router_data: core_types::authentication::PreAuthNVersionCallRouterData =
            transformers::construct_pre_authentication_router_data(
                state,
                authentication_connector_name.clone(),
                card.clone(),
                &three_ds_connector_account,
                business_profile.merchant_id.clone(),
                payment_id.clone(),
            )?;
        let router_data = utils::do_auth_connector_call(
            state,
            authentication_connector_name.clone(),
            router_data,
        )
        .await?;

        let updated_authentication =
            utils::update_trackers(state, router_data, authentication, acquirer_details.clone())
                .await?;
        // from version call response, we will get to know the maximum supported 3ds version.
        // If the version is not greater than or equal to 3DS 2.0, We should not do the successive pre authentication call.
        if !updated_authentication.is_separate_authn_required() {
            return Ok(updated_authentication);
        }
        updated_authentication
    } else {
        authentication
    };

    let router_data: core_types::authentication::PreAuthNRouterData =
        transformers::construct_pre_authentication_router_data(
            state,
            authentication_connector_name.clone(),
            card,
            &three_ds_connector_account,
            business_profile.merchant_id.clone(),
            payment_id,
        )?;
    let router_data =
        utils::do_auth_connector_call(state, authentication_connector_name, router_data).await?;

    utils::update_trackers(state, router_data, authentication, acquirer_details).await
}
