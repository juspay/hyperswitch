use api_models::payment_methods::PaymentMethodCollectLinkRequest;
use diesel_models::generic_link::PaymentMethodCollectLinkData;
use error_stack::ResultExt;

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
        format!("pm_collect_link_secret").as_str(),
    );

    // Fetch SDK host
    let sdk_host = "".to_string();

    Ok(PaymentMethodCollectLinkData {
        pm_collect_link_id: pm_collect_link_id.clone(),
        customer_id,
        link: format!("https://host/payment_methods/collect/{pm_collect_link_id}"),
        sdk_host,
        client_secret,
    })
}
