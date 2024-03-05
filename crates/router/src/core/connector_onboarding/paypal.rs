use api_models::{admin::MerchantConnectorUpdate, connector_onboarding as api};
use common_utils::ext_traits::Encode;
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};

use crate::{
    core::{
        admin,
        errors::{ApiErrorResponse, RouterResult},
    },
    services::{send_request, ApplicationResponse, Request},
    types::{self as oss_types, api as oss_api_types, api::connector_onboarding as types},
    utils::connector_onboarding as utils,
    AppState,
};

fn build_referral_url(state: AppState) -> String {
    format!(
        "{}v2/customer/partner-referrals",
        state.conf.connectors.paypal.base_url
    )
}

async fn build_referral_request(
    state: AppState,
    tracking_id: String,
    return_url: String,
) -> RouterResult<Request> {
    let access_token = utils::paypal::generate_access_token(state.clone()).await?;
    let request_body = types::paypal::PartnerReferralRequest::new(tracking_id, return_url);

    utils::paypal::build_paypal_post_request(
        build_referral_url(state),
        request_body,
        access_token.token.expose(),
    )
}

pub async fn get_action_url_from_paypal(
    state: AppState,
    tracking_id: String,
    return_url: String,
) -> RouterResult<String> {
    let referral_request = Box::pin(build_referral_request(
        state.clone(),
        tracking_id,
        return_url,
    ))
    .await?;
    let referral_response = send_request(&state, referral_request, None)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to send request to paypal referrals")?;

    let parsed_response: types::paypal::PartnerReferralResponse = referral_response
        .json()
        .await
        .into_report()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse paypal response")?;

    parsed_response.extract_action_url()
}

fn merchant_onboarding_status_url(state: AppState, tracking_id: String) -> String {
    let partner_id = state
        .conf
        .connector_onboarding
        .get_inner()
        .paypal
        .partner_id
        .to_owned();
    format!(
        "{}v1/customer/partners/{}/merchant-integrations?tracking_id={}",
        state.conf.connectors.paypal.base_url,
        partner_id.expose(),
        tracking_id
    )
}

pub async fn sync_merchant_onboarding_status(
    state: AppState,
    tracking_id: String,
) -> RouterResult<api::OnboardingStatus> {
    let access_token = utils::paypal::generate_access_token(state.clone()).await?;

    let Some(seller_status_response) =
        find_paypal_merchant_by_tracking_id(state.clone(), tracking_id, &access_token).await?
    else {
        return Ok(api::OnboardingStatus::PayPal(
            api::PayPalOnboardingStatus::AccountNotFound,
        ));
    };

    let merchant_details_url = seller_status_response
        .extract_merchant_details_url(&state.conf.connectors.paypal.base_url)?;

    let merchant_details_request =
        utils::paypal::build_paypal_get_request(merchant_details_url, access_token.token.expose())?;

    let merchant_details_response = send_request(&state, merchant_details_request, None)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to send request to paypal merchant details")?;

    let parsed_response: types::paypal::SellerStatusDetailsResponse = merchant_details_response
        .json()
        .await
        .into_report()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse paypal merchant details response")?;

    let eligibity = parsed_response.get_eligibility_status().await?;
    Ok(api::OnboardingStatus::PayPal(eligibity))
}

async fn find_paypal_merchant_by_tracking_id(
    state: AppState,
    tracking_id: String,
    access_token: &oss_types::AccessToken,
) -> RouterResult<Option<types::paypal::SellerStatusResponse>> {
    let seller_status_request = utils::paypal::build_paypal_get_request(
        merchant_onboarding_status_url(state.clone(), tracking_id),
        access_token.token.peek().to_string(),
    )?;
    let seller_status_response = send_request(&state, seller_status_request, None)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to send request to paypal onboarding status")?;

    if seller_status_response.status().is_success() {
        return Ok(Some(
            seller_status_response
                .json()
                .await
                .into_report()
                .change_context(ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse paypal onboarding status response")?,
        ));
    }
    Ok(None)
}

pub async fn update_mca(
    state: &AppState,
    merchant_id: String,
    connector_id: String,
    auth_details: oss_types::ConnectorAuthType,
) -> RouterResult<oss_api_types::MerchantConnectorResponse> {
    let connector_auth_json = auth_details
        .encode_to_value()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while deserializing connector_account_details")?;

    let request = MerchantConnectorUpdate {
        connector_type: common_enums::ConnectorType::PaymentProcessor,
        connector_account_details: Some(Secret::new(connector_auth_json)),
        disabled: Some(false),
        status: Some(common_enums::ConnectorStatus::Active),
        test_mode: None,
        connector_label: None,
        payment_methods_enabled: None,
        metadata: None,
        frm_configs: None,
        connector_webhook_details: None,
        pm_auth_config: None,
    };
    let mca_response =
        admin::update_payment_connector(state.clone(), &merchant_id, &connector_id, request)
            .await?;

    match mca_response {
        ApplicationResponse::Json(mca_data) => Ok(mca_data),
        _ => Err(ApiErrorResponse::InternalServerError.into()),
    }
}
