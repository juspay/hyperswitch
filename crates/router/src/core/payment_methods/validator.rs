use api_models::{admin, payment_methods::PaymentMethodCollectLinkRequest};
use common_utils::link_utils;
use diesel_models::generic_link::PaymentMethodCollectLinkData;
use error_stack::ResultExt;
use masking::Secret;

use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        utils as core_utils,
    },
    routes::{app::StorageInterface, SessionState},
    types::domain,
    utils,
};

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub async fn validate_request_and_initiate_payment_method_collect_link(
    _state: &SessionState,
    _merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
    _req: &PaymentMethodCollectLinkRequest,
) -> RouterResult<PaymentMethodCollectLinkData> {
    todo!()
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub async fn validate_request_and_initiate_payment_method_collect_link(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &PaymentMethodCollectLinkRequest,
) -> RouterResult<PaymentMethodCollectLinkData> {
    // Validate customer_id
    let db: &dyn StorageInterface = &*state.store;
    let customer_id = req.customer_id.clone();
    let merchant_id = merchant_account.get_id().clone();
    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    match db
        .find_customer_by_customer_id_merchant_id(
            &state.into(),
            &customer_id,
            &merchant_id,
            key_store,
            merchant_account.storage_scheme,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            if err.current_context().is_db_not_found() {
                Err(err).change_context(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "customer [{}] not found for merchant [{:?}]",
                        customer_id.get_string_repr(),
                        merchant_id
                    ),
                })
            } else {
                Err(err)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("database error while finding customer")
            }
        }
    }?;

    // Create payment method collect link ID
    let pm_collect_link_id = core_utils::get_or_generate_id(
        "pm_collect_link_id",
        &req.pm_collect_link_id,
        "pm_collect_link",
    )?;

    // Fetch all configs
    let default_config = &state.conf.generic_link.payment_method_collect;

    #[cfg(feature = "v1")]
    let merchant_config = merchant_account
        .pm_collect_link_config
        .as_ref()
        .map(|config| {
            common_utils::ext_traits::ValueExt::parse_value::<admin::BusinessCollectLinkConfig>(
                config.clone(),
                "BusinessCollectLinkConfig",
            )
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "pm_collect_link_config in merchant_account",
        })?;

    #[cfg(feature = "v2")]
    let merchant_config = Option::<admin::BusinessCollectLinkConfig>::None;

    let merchant_ui_config = merchant_config.as_ref().map(|c| c.config.ui_config.clone());
    let ui_config = req
        .ui_config
        .as_ref()
        .or(merchant_ui_config.as_ref())
        .cloned();

    // Form data to be injected in the link
    let (logo, merchant_name, theme) = match ui_config {
        Some(config) => (config.logo, config.merchant_name, config.theme),
        _ => (None, None, None),
    };
    let pm_collect_link_config = link_utils::GenericLinkUiConfig {
        logo,
        merchant_name,
        theme,
    };
    let client_secret = utils::generate_id(consts::ID_LENGTH, "pm_collect_link_secret");
    let domain = merchant_config
        .clone()
        .and_then(|c| c.config.domain_name)
        .map(|domain| format!("https://{}", domain))
        .unwrap_or(state.base_url.clone());
    let session_expiry = match req.session_expiry {
        Some(expiry) => expiry,
        None => default_config.expiry,
    };
    let link = Secret::new(format!(
        "{domain}/payment_methods/collect/{}/{pm_collect_link_id}",
        merchant_id.get_string_repr()
    ));
    let enabled_payment_methods = match (&req.enabled_payment_methods, &merchant_config) {
        (Some(enabled_payment_methods), _) => enabled_payment_methods.clone(),
        (None, Some(config)) => config.enabled_payment_methods.clone(),
        _ => {
            let mut default_enabled_payout_methods: Vec<link_utils::EnabledPaymentMethod> = vec![];
            for (payment_method, payment_method_types) in
                default_config.enabled_payment_methods.clone().into_iter()
            {
                let enabled_payment_method = link_utils::EnabledPaymentMethod {
                    payment_method,
                    payment_method_types: payment_method_types.into_iter().collect(),
                };
                default_enabled_payout_methods.push(enabled_payment_method);
            }

            default_enabled_payout_methods
        }
    };

    Ok(PaymentMethodCollectLinkData {
        pm_collect_link_id: pm_collect_link_id.clone(),
        customer_id,
        link,
        client_secret: Secret::new(client_secret),
        session_expiry,
        ui_config: pm_collect_link_config,
        enabled_payment_methods: Some(enabled_payment_methods),
    })
}
