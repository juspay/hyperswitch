use common_utils::{errors::CustomResult, id_type::PaymentId};
use error_stack::{Report, ResultExt};

use crate::{
    core::{
        errors::{self, utils::StorageErrorExt},
        utils,
    },
    logger,
    routes::SessionState,
    services::authentication::AuthenticationData,
    types::{self, storage},
};

pub async fn check_existence_and_add_domain_to_db(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    profile_id_from_auth_layer: Option<common_utils::id_type::ProfileId>,
    merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    domain_from_req: Vec<String>,
) -> CustomResult<Vec<String>, errors::ApiErrorResponse> {
    let key_manager_state = &state.into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    #[cfg(feature = "v1")]
    let merchant_connector_account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            key_manager_state,
            &merchant_id,
            &merchant_connector_id,
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    #[cfg(feature = "v2")]
    let merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount = {
        let _ = merchant_connector_id;
        let _ = key_store;
        let _ = domain_from_req;
        todo!()
    };
    utils::validate_profile_id_from_auth_layer(
        profile_id_from_auth_layer,
        &merchant_connector_account,
    )?;
    let mut already_verified_domains = merchant_connector_account
        .applepay_verified_domains
        .clone()
        .unwrap_or_default();

    let mut new_verified_domains: Vec<String> = domain_from_req
        .into_iter()
        .filter(|req_domain| !already_verified_domains.contains(req_domain))
        .collect();

    already_verified_domains.append(&mut new_verified_domains);
    #[cfg(feature = "v1")]
    let updated_mca = storage::MerchantConnectorAccountUpdate::Update {
        connector_type: None,
        connector_name: None,
        connector_account_details: Box::new(None),
        test_mode: None,
        disabled: None,
        merchant_connector_id: None,
        payment_methods_enabled: None,
        metadata: None,
        frm_configs: None,
        connector_webhook_details: Box::new(None),
        applepay_verified_domains: Some(already_verified_domains.clone()),
        pm_auth_config: Box::new(None),
        connector_label: None,
        status: None,
        connector_wallets_details: Box::new(None),
        additional_merchant_data: Box::new(None),
    };
    #[cfg(feature = "v2")]
    let updated_mca = storage::MerchantConnectorAccountUpdate::Update {
        connector_type: None,
        connector_account_details: Box::new(None),
        disabled: None,
        payment_methods_enabled: None,
        metadata: None,
        frm_configs: None,
        connector_webhook_details: Box::new(None),
        applepay_verified_domains: Some(already_verified_domains.clone()),
        pm_auth_config: Box::new(None),
        connector_label: None,
        status: None,
        connector_wallets_details: Box::new(None),
        additional_merchant_data: Box::new(None),
        feature_metadata: Box::new(None),
    };
    state
        .store
        .update_merchant_connector_account(
            key_manager_state,
            merchant_connector_account,
            updated_mca.into(),
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while updating MerchantConnectorAccount: id: {:?}",
                merchant_connector_id
            )
        })?;

    Ok(already_verified_domains.clone())
}

pub fn log_applepay_verification_response_if_error(
    response: &Result<Result<types::Response, types::Response>, Report<errors::ApiClientError>>,
) {
    if let Err(error) = response.as_ref() {
        logger::error!(applepay_domain_verification_error= ?error);
    };
    response.as_ref().ok().map(|res| {
        res.as_ref()
            .map_err(|error| logger::error!(applepay_domain_verification_error= ?error))
    });
}

#[cfg(feature = "v2")]
pub async fn check_if_profile_id_is_present_in_payment_intent(
    payment_id: PaymentId,
    state: &SessionState,
    auth_data: &AuthenticationData,
) -> CustomResult<(), errors::ApiErrorResponse> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn check_if_profile_id_is_present_in_payment_intent(
    payment_id: PaymentId,
    state: &SessionState,
    auth_data: &AuthenticationData,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let db = &*state.store;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &state.into(),
            &payment_id,
            auth_data.merchant_account.get_id(),
            &auth_data.key_store,
            auth_data.merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::Unauthorized)?;

    utils::validate_profile_id_from_auth_layer(auth_data.profile_id.clone(), &payment_intent)
}
