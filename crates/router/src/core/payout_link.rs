#[cfg(feature = "payouts")]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "payouts")]
use api_models::payouts;
#[cfg(feature = "payouts")]
use common_utils::{ext_traits::OptionExt, id_type::CustomerId};
#[cfg(feature = "payouts")]
use diesel_models::enums;
#[cfg(feature = "payouts")]
use error_stack::ResultExt;

#[cfg(feature = "payouts")]
use super::errors::{RouterResponse, StorageErrorExt};
#[cfg(feature = "payouts")]
use crate::{
    core::payments::helpers,
    errors,
    routes::{app::StorageInterface, SessionState},
    services::logger,
    services::{self, GenericLinks},
    types::domain,
};

#[cfg(feature = "payouts")]
pub async fn initiate_payout_link(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutLinkInitiateRequest,
) -> RouterResponse<services::GenericLinkFormData> {
    let db: &dyn StorageInterface = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    // Fetch payout
    let payout = db
        .find_payout_by_merchant_id_payout_id(
            merchant_id,
            &req.payout_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    let payout_link_id = payout
        .payout_link_id
        .get_required_value("payout link id")
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payout link not found".to_string(),
        })?;
    // Fetch payout link
    let payout_link = db
        .find_pm_collect_link_by_link_id(&payout_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payout link not found".to_string(),
        })?;

    // Check status and return form data accordingly
    let has_expired = common_utils::date_time::now() > payout_link.expiry;
    let status = payout_link.link_status;
    let link_data = payout_link.link_data;
    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &merchant_account.merchant_id,
            false,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    let filtered_mcas_on_profile = helpers::filter_mca_based_on_business_profile(
        all_mcas.clone(),
        Some(payout.profile_id.clone()),
    );
    let filtered_mca = helpers::filter_mca_based_on_connector_type(
        filtered_mcas_on_profile.clone(),
        common_enums::ConnectorType::PayoutProcessor,
    );
    let enabled_payout_methods = filter_payout_methods(filtered_mca);
    logger::debug!("enabled_payout_methods: {:?}", enabled_payout_methods);
    match status {
        enums::PaymentMethodCollectStatus::Initiated => {
            // if expired, send back expired status page
            if has_expired {
                let expired_link_data = services::GenericExpiredLinkData {
                    title: "Payout link has expired".to_string(),
                    message: "This payout link has expired.".to_string(),
                    theme: link_data.ui_config.theme,
                };
                Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                    GenericLinks::ExpiredLink(expired_link_data),
                )))

            // else, send back form link
            } else {
                let customer_id = CustomerId::from(payout_link.primary_reference.clone().into())
                    .change_context(errors::ApiErrorResponse::InvalidDataValue {
                        field_name: "customer_id",
                    })?;
                // Fetch customer
                let customer = db
                    .find_customer_by_customer_id_merchant_id(
                        &customer_id,
                        &req.merchant_id,
                        &key_store,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InvalidRequestData {
                        message: format!(
                            "Customer [{}] not found for link_id - {}",
                            payout_link.primary_reference, payout_link.link_id
                        ),
                    })
                    .attach_printable(format!(
                        "customer [{}] not found",
                        payout_link.primary_reference
                    ))?;
                let enabled_payment_methods = link_data
                    .enabled_payment_methods
                    .unwrap_or(enabled_payout_methods);

                let js_data = payouts::PayoutLinkDetails {
                    pub_key: merchant_account
                        .publishable_key
                        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "pub_key",
                        })?
                        .into(),
                    client_secret: link_data.client_secret.clone(),
                    payout_link_id: payout_link.link_id,
                    customer_id: customer.customer_id,
                    session_expiry: payout_link.expiry,
                    return_url: payout_link.return_url,
                    ui_config: link_data.ui_config,
                    enabled_payment_methods,
                    amount: payout.amount,
                    currency: payout.destination_currency,
                    flow: payouts::PayoutLinkFlow::PayoutLinkInitiate,
                };

                let serialized_css_content = "".to_string();

                let serialized_js_content =
                    format!("window.__PM_COLLECT_DETAILS = {}", serialize(&js_data)?);

                let generic_form_data = services::GenericLinkFormData {
                    js_data: serialized_js_content,
                    css_data: serialized_css_content,
                    sdk_url: link_data.sdk_host.clone(),
                    html_meta_tags: "".to_string(),
                };
                Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                    GenericLinks::PaymentMethodCollect(generic_form_data),
                )))
            }
        }

        // Send back status page
        status => {
            let js_data = api_models::payment_methods::PaymentMethodCollectLinkStatusDetails {
                pm_collect_link_id: payout_link.link_id,
                customer_id: link_data.customer_id,
                session_expiry: payout_link.expiry,
                return_url: payout_link.return_url,
                ui_config: link_data.ui_config,
                status,
            };

            let serialized_css_content = "".to_string();

            let serialized_js_content =
                format!("window.__PM_COLLECT_DETAILS = {}", serialize(&js_data)?);

            let generic_status_data = services::GenericLinkStatusData {
                js_data: serialized_js_content,
                css_data: serialized_css_content,
            };
            Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                GenericLinks::PaymentMethodCollectStatus(generic_status_data),
            )))
        }
    }
}
#[cfg(feature = "payouts")]
fn serialize<D>(data: &D) -> errors::RouterResult<String>
where
    D: serde::Serialize,
{
    serde_json::to_string(data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!(
            "Failed to serialize {}",
            std::any::type_name::<D>()
        ))
}

fn filter_payout_methods(
    filtered_mca: Vec<domain::MerchantConnectorAccount>,
) -> Vec<enums::EnabledPaymentMethod> {
    let mut response: Vec<enums::EnabledPaymentMethod> = vec![];
    let mut payment_method_list_hm: HashMap<
        common_enums::PaymentMethod,
        HashSet<common_enums::PaymentMethodType>,
    > = HashMap::new();
    let mut bank_transfer_hs: HashSet<common_enums::PaymentMethodType> = HashSet::new();
    let mut card_hs: HashSet<common_enums::PaymentMethodType> = HashSet::new();
    let mut wallet_hs: HashSet<common_enums::PaymentMethodType> = HashSet::new();
    for mca in &filtered_mca {
        let payment_methods = match &mca.payment_methods_enabled {
            Some(pm) => pm,
            None => continue,
        };
        for payment_method in payment_methods.iter() {
            let parse_result = serde_json::from_value::<api_models::admin::PaymentMethodsEnabled>(
                payment_method.clone(),
            );
            if let Ok(payment_methods_enabled) = parse_result {
                let payment_method = payment_methods_enabled.payment_method;
                let payment_method_types = match payment_methods_enabled.payment_method_types {
                    Some(pmt) => pmt,
                    None => continue,
                };
                for pmts in &payment_method_types {
                    if payment_method == common_enums::PaymentMethod::Card {
                        card_hs.insert(pmts.payment_method_type);
                        payment_method_list_hm.insert(payment_method, card_hs.clone());
                    } else if payment_method == common_enums::PaymentMethod::Wallet {
                        wallet_hs.insert(pmts.payment_method_type);
                        payment_method_list_hm.insert(payment_method, wallet_hs.clone());
                    } else if payment_method == common_enums::PaymentMethod::BankTransfer {
                        bank_transfer_hs.insert(pmts.payment_method_type);
                        payment_method_list_hm.insert(payment_method, bank_transfer_hs.clone());
                    }
                }
            }
        }
    }
    for (pm, method_types) in payment_method_list_hm {
        if !method_types.is_empty() {
            let payment_method_types: Vec<enums::PaymentMethodType> =
                method_types.into_iter().collect();
            let enabled_payment_method = enums::EnabledPaymentMethod {
                payment_method: pm,
                payment_method_types,
            };
            response.push(enabled_payment_method);
        }
    }
    response
}