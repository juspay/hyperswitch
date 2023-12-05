use api_models::{connector_onboarding as api, enums};
use error_stack::ResultExt;
use masking::Secret;

use crate::{
    core::errors::{ApiErrorResponse, RouterResponse, RouterResult},
    services::{authentication as auth, ApplicationResponse},
    types::{self as oss_types},
    utils::connector_onboarding as utils,
    AppState,
};

pub mod paypal;

#[async_trait::async_trait]
pub trait AccessToken {
    async fn access_token(state: &AppState) -> RouterResult<oss_types::AccessToken>;
}

pub async fn get_action_url(
    state: AppState,
    request: api::ActionUrlRequest,
) -> RouterResponse<api::ActionUrlResponse> {
    let connector_onboarding_conf = state.conf.connector_onboarding.clone();
    let is_enabled = utils::is_enabled(request.connector, &connector_onboarding_conf);

    match (is_enabled, request.connector) {
        (Some(true), enums::Connector::Paypal) => {
            let action_url = Box::pin(paypal::get_action_url_from_paypal(
                state,
                request.connector_id,
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
    state: AppState,
    user_from_token: auth::UserFromToken,
    request: api::OnboardingSyncRequest,
) -> RouterResponse<api::OnboardingStatus> {
    let merchant_account = user_from_token
        .get_merchant_account(state.clone())
        .await
        .change_context(ApiErrorResponse::MerchantAccountNotFound)?;
    let connector_onboarding_conf = state.conf.connector_onboarding.clone();
    let is_enabled = utils::is_enabled(request.connector, &connector_onboarding_conf);

    match (is_enabled, request.connector) {
        (Some(true), enums::Connector::Paypal) => {
            let status = Box::pin(paypal::sync_merchant_onboarding_status(
                state.clone(),
                request.connector_id.clone(),
            ))
            .await?;
            if let api::OnboardingStatus::PayPal(api::PayPalOnboardingStatus::Success(
                ref inner_data,
            )) = status
            {
                let connector_onboarding_conf = state.conf.connector_onboarding.clone();
                let auth_details = oss_types::ConnectorAuthType::SignatureKey {
                    api_key: connector_onboarding_conf.paypal.client_secret,
                    key1: connector_onboarding_conf.paypal.client_id,
                    api_secret: Secret::new(inner_data.payer_id.clone()),
                };
                let some_data = paypal::update_mca(
                    &state,
                    &merchant_account,
                    request.connector_id.to_owned(),
                    auth_details,
                )
                .await?;

                return Ok(ApplicationResponse::Json(api::OnboardingStatus::PayPal(
                    api::PayPalOnboardingStatus::ConnectorIntegrated(some_data),
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
