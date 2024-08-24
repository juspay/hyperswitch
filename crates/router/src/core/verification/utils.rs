use common_utils::errors::CustomResult;
use error_stack::{Report, ResultExt};

use crate::{
    core::{
        errors::{self, utils::StorageErrorExt},
        utils,
    },
    logger,
    routes::SessionState,
    types,
    types::storage,
};

pub async fn check_existence_and_add_domain_to_db(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    profile_id_from_auth_layer: Option<common_utils::id_type::ProfileId>,
    merchant_connector_id: String,
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

    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "merchant_connector_account_v2")
    ))]
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

    #[cfg(all(feature = "v2", feature = "merchant_connector_account_v2"))]
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
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "merchant_connector_account_v2")
    ))]
    let updated_mca = storage::MerchantConnectorAccountUpdate::Update {
        connector_type: None,
        connector_name: None,
        connector_account_details: None,
        test_mode: None,
        disabled: None,
        merchant_connector_id: None,
        payment_methods_enabled: None,
        metadata: None,
        frm_configs: None,
        connector_webhook_details: None,
        applepay_verified_domains: Some(already_verified_domains.clone()),
        pm_auth_config: None,
        connector_label: None,
        status: None,
        connector_wallets_details: None,
    };
    #[cfg(all(feature = "v2", feature = "merchant_connector_account_v2"))]
    let updated_mca = storage::MerchantConnectorAccountUpdate::Update {
        connector_type: None,
        connector_account_details: None,
        disabled: None,
        payment_methods_enabled: None,
        metadata: None,
        frm_configs: None,
        connector_webhook_details: None,
        applepay_verified_domains: Some(already_verified_domains.clone()),
        pm_auth_config: None,
        connector_label: None,
        status: None,
        connector_wallets_details: None,
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
            format!("Failed while updating MerchantConnectorAccount: id: {merchant_connector_id}")
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
