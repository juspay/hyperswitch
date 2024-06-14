use api_models::{connector_onboarding as api, enums};
use masking::Secret;

use crate::{
    core::errors::{ApiErrorResponse, RouterResponse, RouterResult},
    routes::app::ReqState,
    services::{authentication as auth, ApplicationResponse},
    types as oss_types,
    utils::connector_onboarding as utils,
    SessionState,
};

pub mod paypal;

#[async_trait::async_trait]
pub trait AccessToken {
    async fn access_token(state: &SessionState) -> RouterResult<oss_types::AccessToken>;
}

pub async fn get_action_url(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    request: api::ActionUrlRequest,
    _req_state: ReqState,
) -> RouterResponse<api::ActionUrlResponse> {
    utils::check_if_connector_exists(&state, &request.connector_id, &user_from_token.merchant_id)
        .await?;

    let connector_onboarding_conf = state.conf.connector_onboarding.get_inner();
    let is_enabled = utils::is_enabled(request.connector, connector_onboarding_conf);
    let tracking_id =
        utils::get_tracking_id_from_configs(&state, &request.connector_id, request.connector)
            .await?;

    match (is_enabled, request.connector) {
        (Some(true), enums::Connector::Paypal) => {
            let action_url = Box::pin(paypal::get_action_url_from_paypal(
                state,
                tracking_id,
                request.return_url,
            ))
            .await?;
            Ok(ApplicationResponse::Json(api::ActionUrlResponse::PayPal(
                api::PayPalActionUrlResponse { action_url },
            )))
        }
        _ => Err(ApiErrorResponse::FlowNotSupported {
            flow: "Connector onboarding".to_string(),
            connector: request.connector.to_string(),
        }
        .into()),
    }
}

pub async fn sync_onboarding_status(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    request: api::OnboardingSyncRequest,
    _req_state: ReqState,
) -> RouterResponse<api::OnboardingStatus> {
    utils::check_if_connector_exists(&state, &request.connector_id, &user_from_token.merchant_id)
        .await?;

    let connector_onboarding_conf = state.conf.connector_onboarding.get_inner();
    let is_enabled = utils::is_enabled(request.connector, connector_onboarding_conf);
    let tracking_id =
        utils::get_tracking_id_from_configs(&state, &request.connector_id, request.connector)
            .await?;

    match (is_enabled, request.connector) {
        (Some(true), enums::Connector::Paypal) => {
            let status = Box::pin(paypal::sync_merchant_onboarding_status(
                state.clone(),
                tracking_id,
            ))
            .await?;
            if let api::OnboardingStatus::PayPal(api::PayPalOnboardingStatus::Success(
                ref paypal_onboarding_data,
            )) = status
            {
                let connector_onboarding_conf = state.conf.connector_onboarding.get_inner();
                let auth_details = oss_types::ConnectorAuthType::SignatureKey {
                    api_key: connector_onboarding_conf.paypal.client_secret.clone(),
                    key1: connector_onboarding_conf.paypal.client_id.clone(),
                    api_secret: Secret::new(paypal_onboarding_data.payer_id.clone()),
                };
                let update_mca_data = paypal::update_mca(
                    &state,
                    user_from_token.merchant_id,
                    request.connector_id.to_owned(),
                    auth_details,
                )
                .await?;

                return Ok(ApplicationResponse::Json(api::OnboardingStatus::PayPal(
                    api::PayPalOnboardingStatus::ConnectorIntegrated(update_mca_data),
                )));
            }
            Ok(ApplicationResponse::Json(status))
        }
        _ => Err(ApiErrorResponse::FlowNotSupported {
            flow: "Connector onboarding".to_string(),
            connector: request.connector.to_string(),
        }
        .into()),
    }
}

pub async fn reset_tracking_id(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    request: api::ResetTrackingIdRequest,
    _req_state: ReqState,
) -> RouterResponse<()> {
    utils::check_if_connector_exists(&state, &request.connector_id, &user_from_token.merchant_id)
        .await?;
    utils::set_tracking_id_in_configs(&state, &request.connector_id, request.connector).await?;

    Ok(ApplicationResponse::StatusOk)
}
