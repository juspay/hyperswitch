use api_models::{admin, payment_methods::PaymentMethodCollectLinkRequest};
use common_utils::ext_traits::OptionExt;
use diesel_models::{enums::CollectLinkConfig, generic_link::PaymentMethodCollectLinkData};
use error_stack::ResultExt;
use masking::Secret;

use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        utils as core_utils,
    },
    routes::{app::StorageInterface, AppState},
    types::domain,
    utils,
};

pub async fn validate_request_and_initiate_payment_method_collect_link(
    state: &AppState,
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
                    customer_id, merchant_id
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
    let client_secret = utils::generate_id(
        consts::ID_LENGTH,
        "pm_collect_link_secret",
    );

    // Fetch SDK host
    let sdk_host = state
        .conf
        .generic_link
        .payment_method_collect
        .sdk_url
        .clone();

    let requested_config = &req.config;
    let merchant_config: admin::MerchantCollectLinkConfig = merchant_account
        .pm_collect_link_config
        .clone()
        .parse_value("MerchantCollectLinkConfig")
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "pm_collect_link_config in merchant_account",
        })?;

    let domain = merchant_config
        .domain_name
        .unwrap_or_else(|| state.conf.server.base_url.clone());

    let theme = requested_config
        .as_ref()
        .and_then(|config| config.theme.clone())
        .or_else(|| merchant_config.config.theme.clone())
        .unwrap_or_else(|| common_utils::consts::DEFAULT_PM_COLLECT_LINK_THEME.to_string());

    let logo = requested_config
        .as_ref()
        .and_then(|config| config.logo.clone())
        .or_else(|| merchant_config.config.logo.clone())
        .unwrap_or_else(|| common_utils::consts::DEFAULT_PM_COLLECT_LINK_LOGO.to_string());

    let collector_name = requested_config
        .as_ref()
        .and_then(|config| config.collector_name.clone())
        .or_else(|| merchant_config.config.collector_name.clone())
        .map_or(
            merchant_account
                .merchant_name
                .clone()
                .get_required_value("merchant_name")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "collector_name",
                })?
                .get_inner()
                .clone(),
            |c| c,
        )
        .clone();

    let pm_collect_link_config = CollectLinkConfig {
        theme,
        logo,
        collector_name,
    };

    Ok(PaymentMethodCollectLinkData {
        pm_collect_link_id: pm_collect_link_id.clone(),
        customer_id,
        link: Secret::new(format!(
            "{domain}/payment_methods/collect/{merchant_id}/{pm_collect_link_id}"
        )),
        sdk_host,
        client_secret: Secret::new(client_secret),
        session_expiry: req
            .session_expiry
            .unwrap_or(common_utils::consts::DEFAULT_PM_COLLECT_LINK_EXPIRY),
        config: pm_collect_link_config,
    })
}
