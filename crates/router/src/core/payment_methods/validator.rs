use api_models::{admin, payment_methods::PaymentMethodCollectLinkRequest};
use common_utils::ext_traits::ValueExt;
use diesel_models::{enums::CollectLinkConfig, generic_link::PaymentMethodCollectLinkData};
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

pub async fn validate_request_and_initiate_payment_method_collect_link(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    req: &PaymentMethodCollectLinkRequest,
) -> RouterResult<PaymentMethodCollectLinkData> {
    // Validate customer_id
    let db: &dyn StorageInterface = &*state.store;
    let customer_id = req.customer_id.clone();
    let merchant_id = merchant_account.merchant_id.clone();
    match db
        .find_customer_by_customer_id_merchant_id(
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
                let message = format!(
                    "customer [{}] not found for merchant [{}]",
                    customer_id.get_string_repr(),
                    merchant_id
                );
                Err(err)
                    .change_context(errors::ApiErrorResponse::InvalidRequestData {
                        message: message.clone(),
                    })
                    .attach_printable(message)
            } else {
                Err(err)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("databaser error while finding customer")
            }
        }
    }?;

    // Create payment method collect link ID
    let pm_collect_link_id = core_utils::get_or_generate_id(
        "pm_collect_link_id",
        &req.pm_collect_link_id,
        "pm_collect_link",
    )?;

    // Create client secret
    let client_secret = utils::generate_id(consts::ID_LENGTH, "pm_collect_link_secret");

    // Fetch all configs
    let default_config = &state.conf.generic_link.payment_method_collect;
    let merchant_config = merchant_account
        .pm_collect_link_config
        .as_ref()
        .map(|config| {
            config
                .clone()
                .parse_value::<admin::MerchantCollectLinkConfig>("MerchantCollectLinkConfig")
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "pm_collect_link_config in merchant_account",
        })?;
    let ui_config = &req.ui_config;

    let fallback_ui_config = merchant_config
        .as_ref()
        .map(|config| config.ui_config.clone())
        .unwrap_or(default_config.ui_config.clone());

    // Form data to be injected in HTML
    let sdk_host = default_config.sdk_url.clone();

    let domain = merchant_config
        .clone()
        .and_then(|c| c.domain_name.clone())
        .unwrap_or_else(|| state.base_url.clone());

    let (collector_name, logo, theme) = match ui_config {
        Some(config) => (
            config.collector_name.clone(),
            config.logo.clone(),
            config.theme.clone(),
        ),
        None => (
            fallback_ui_config.collector_name.clone(),
            fallback_ui_config.logo.clone(),
            fallback_ui_config.theme.clone(),
        ),
    };

    let session_expiry = match req.session_expiry {
        Some(expiry) => expiry,
        None => default_config.expiry,
    };

    let link = Secret::new(format!(
        "{domain}/payment_methods/collect/{merchant_id}/{pm_collect_link_id}"
    ));

    let pm_collect_link_config = CollectLinkConfig {
        theme,
        logo,
        collector_name,
    };

    let enabled_payment_methods = match (&req.enabled_payment_methods, &merchant_config) {
        (Some(enabled_payment_methods), _) => enabled_payment_methods.clone(),
        (None, Some(config)) => config.enabled_payment_methods.clone(),
        _ => default_config.enabled_payment_methods.clone(),
    };

    Ok(PaymentMethodCollectLinkData {
        pm_collect_link_id: pm_collect_link_id.clone(),
        customer_id,
        link,
        sdk_host,
        client_secret: Secret::new(client_secret),
        session_expiry,
        ui_config: pm_collect_link_config,
        enabled_payment_methods: Some(enabled_payment_methods),
    })
}
