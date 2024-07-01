use std::collections::{HashMap, HashSet};

use api_models::payouts;
use common_utils::{
    ext_traits::{Encode, OptionExt},
    link_utils,
    types::{AmountConvertor, StringMajorUnitForConnector},
};
use diesel_models::PayoutLinkUpdate;
use error_stack::ResultExt;

use super::errors::{RouterResponse, StorageErrorExt};
use crate::{
    core::payments::helpers,
    errors,
    routes::{app::StorageInterface, SessionState},
    services::{self, GenericLinks},
    types::{api::enums, domain},
};

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
    let payout_attempt = db
        .find_payout_attempt_by_merchant_id_payout_attempt_id(
            merchant_id,
            &format!("{}_{}", payout.payout_id, payout.attempt_count),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?;
    let payout_link_id = payout
        .payout_link_id
        .clone()
        .get_required_value("payout link id")
        .change_context(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payout link not found".to_string(),
        })?;
    // Fetch payout link
    let payout_link = db
        .find_payout_link_by_link_id(&payout_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payout link not found".to_string(),
        })?;

    // Check status and return form data accordingly
    let has_expired = common_utils::date_time::now() > payout_link.expiry;
    let status = payout_link.link_status.clone();
    let link_data = payout_link.link_data.clone();
    let default_config = &state.conf.generic_link.payout_link;
    let default_ui_config = default_config.ui_config.clone();
    let ui_config_data = link_utils::GenericLinkUIConfigFormData {
        merchant_name: link_data
            .ui_config
            .merchant_name
            .unwrap_or(default_ui_config.merchant_name),
        logo: link_data.ui_config.logo.unwrap_or(default_ui_config.logo),
        theme: link_data
            .ui_config
            .theme
            .clone()
            .unwrap_or(default_ui_config.theme.clone()),
    };
    match (has_expired, &status) {
        // Send back generic expired page
        (true, _) | (_, &link_utils::PayoutLinkStatus::Invalidated) => {
            let expired_link_data = services::GenericExpiredLinkData {
                title: "Payout Expired".to_string(),
                message: "This payout link has expired.".to_string(),
                theme: link_data.ui_config.theme.unwrap_or(default_ui_config.theme),
            };

            if status != link_utils::PayoutLinkStatus::Invalidated {
                let payout_link_update = PayoutLinkUpdate::StatusUpdate {
                    link_status: link_utils::PayoutLinkStatus::Invalidated,
                };
                db.update_payout_link(payout_link, payout_link_update)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error updating payout links in db")?;
            }

            Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                GenericLinks::ExpiredLink(expired_link_data),
            )))
        }

        // Initiate Payout link flow
        (_, link_utils::PayoutLinkStatus::Initiated) => {
            let customer_id = link_data.customer_id;
            let required_amount_type = StringMajorUnitForConnector;
            let amount = required_amount_type
                .convert(payout.amount, payout.destination_currency)
                .change_context(errors::ApiErrorResponse::CurrencyConversionFailed)?;
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
                .attach_printable_lazy(|| {
                    format!("customer [{}] not found", payout_link.primary_reference)
                })?;
            let enabled_payout_methods =
                filter_payout_methods(db, &merchant_account, &key_store, &payout).await?;
            // Fetch default enabled_payout_methods
            let mut default_enabled_payout_methods: Vec<link_utils::EnabledPaymentMethod> = vec![];
            for (payment_method, payment_method_types) in
                default_config.enabled_payment_methods.clone().into_iter()
            {
                let enabled_payment_method = link_utils::EnabledPaymentMethod {
                    payment_method,
                    payment_method_types,
                };
                default_enabled_payout_methods.push(enabled_payment_method);
            }
            let fallback_enabled_payout_methods = if enabled_payout_methods.is_empty() {
                &default_enabled_payout_methods
            } else {
                &enabled_payout_methods
            };
            // Fetch enabled payout methods from the request. If not found, fetch the enabled payout methods from MCA,
            // If none are configured for merchant connector accounts, fetch them from the default enabled payout methods.
            let enabled_payment_methods = link_data
                .enabled_payment_methods
                .unwrap_or(fallback_enabled_payout_methods.to_vec());

            let js_data = payouts::PayoutLinkDetails {
                publishable_key: merchant_account
                    .publishable_key
                    .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                        field_name: "publishable_key",
                    })?
                    .into(),
                client_secret: link_data.client_secret.clone(),
                payout_link_id: payout_link.link_id,
                payout_id: payout_link.primary_reference,
                customer_id: customer.customer_id,
                session_expiry: payout_link.expiry,
                return_url: payout_link.return_url,
                ui_config: ui_config_data,
                enabled_payment_methods,
                amount,
                currency: payout.destination_currency,
            };

            let serialized_css_content = String::new();

            let serialized_js_content = format!(
                "window.__PAYOUT_DETAILS = {}",
                js_data
                    .encode_to_string_of_json()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize PaymentMethodCollectLinkDetails")?
            );

            let generic_form_data = services::GenericLinkFormData {
                js_data: serialized_js_content,
                css_data: serialized_css_content,
                sdk_url: default_config.sdk_url.clone(),
                html_meta_tags: String::new(),
            };
            Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                GenericLinks::PayoutLink(generic_form_data),
            )))
        }

        // Send back status page
        (_, link_utils::PayoutLinkStatus::Submitted) => {
            let js_data = payouts::PayoutLinkStatusDetails {
                payout_link_id: payout_link.link_id,
                payout_id: payout_link.primary_reference,
                customer_id: link_data.customer_id,
                session_expiry: payout_link.expiry,
                return_url: payout_link.return_url,
                status: payout.status,
                error_code: payout_attempt.error_code,
                error_message: payout_attempt.error_message,
                ui_config: ui_config_data,
            };

            let serialized_css_content = String::new();

            let serialized_js_content = format!(
                "window.__PAYOUT_DETAILS = {}",
                js_data
                    .encode_to_string_of_json()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize PaymentMethodCollectLinkDetails")?
            );

            let generic_status_data = services::GenericLinkStatusData {
                js_data: serialized_js_content,
                css_data: serialized_css_content,
            };
            Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                GenericLinks::PayoutLinkStatus(generic_status_data),
            )))
        }
    }
}

#[cfg(feature = "payouts")]
pub async fn filter_payout_methods(
    db: &dyn StorageInterface,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payout: &hyperswitch_domain_models::payouts::payouts::Payouts,
) -> errors::RouterResult<Vec<link_utils::EnabledPaymentMethod>> {
    //Fetch all merchant connector accounts
    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &merchant_account.merchant_id,
            false,
            key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    // fetch all mca based on profile id
    let filtered_mca_on_profile =
        helpers::filter_mca_based_on_business_profile(all_mcas, Some(payout.profile_id.clone()));
    //Since we just need payout connectors here, filter mca based on connector type.
    let filtered_mca = helpers::filter_mca_based_on_connector_type(
        filtered_mca_on_profile.clone(),
        common_enums::ConnectorType::PayoutProcessor,
    );

    let mut response: Vec<link_utils::EnabledPaymentMethod> = vec![];
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
            let enabled_payment_method = link_utils::EnabledPaymentMethod {
                payment_method: pm,
                payment_method_types,
            };
            response.push(enabled_payment_method);
        }
    }
    Ok(response)
}
