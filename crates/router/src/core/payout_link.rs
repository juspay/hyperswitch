use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use actix_web::http::header;
use api_models::payouts;
use common_utils::{
    ext_traits::{AsyncExt, Encode, OptionExt},
    link_utils,
    types::{AmountConvertor, StringMajorUnitForConnector},
};
use diesel_models::PayoutLinkUpdate;
use error_stack::ResultExt;
use hyperswitch_domain_models::api::{GenericLinks, GenericLinksData};

use super::errors::{RouterResponse, StorageErrorExt};
use crate::{
    configs::settings::{PaymentMethodFilterKey, PaymentMethodFilters},
    core::{
        payments::helpers as payment_helpers,
        payouts::{helpers as payout_helpers, validator},
    },
    errors,
    routes::{app::StorageInterface, SessionState},
    services,
    types::{api, domain, transformers::ForeignFrom},
};

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub async fn initiate_payout_link(
    _state: SessionState,
    _merchant_account: domain::MerchantAccount,
    _key_store: domain::MerchantKeyStore,
    _req: payouts::PayoutLinkInitiateRequest,
    _request_headers: &header::HeaderMap,
    _locale: String,
) -> RouterResponse<services::GenericLinkFormData> {
    todo!()
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub async fn initiate_payout_link(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payouts::PayoutLinkInitiateRequest,
    request_headers: &header::HeaderMap,
) -> RouterResponse<services::GenericLinkFormData> {
    let db: &dyn StorageInterface = &*state.store;
    let merchant_id = merchant_account.get_id();
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

    let allowed_domains = validator::validate_payout_link_render_request_and_get_allowed_domains(
        request_headers,
        &payout_link,
    )?;

    // Check status and return form data accordingly
    let has_expired = common_utils::date_time::now() > payout_link.expiry;
    let status = payout_link.link_status.clone();
    let link_data = payout_link.link_data.clone();
    let default_config = &state.conf.generic_link.payout_link.clone();
    let default_ui_config = default_config.ui_config.clone();
    let ui_config_data = link_utils::GenericLinkUiConfigFormData {
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
                GenericLinks {
                    allowed_domains,
                    data: GenericLinksData::ExpiredLink(expired_link_data),
                    locale: state.locale,
                },
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
                    &(&state).into(),
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
            let address = payout
                .address_id
                .as_ref()
                .async_map(|address_id| async {
                    db.find_address_by_address_id(&(&state).into(), address_id, &key_store)
                        .await
                })
                .await
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable_lazy(|| {
                    format!(
                        "Failed while fetching address [id - {:?}] for payout [id - {}]",
                        payout.address_id, payout.payout_id
                    )
                })?;

            let enabled_payout_methods = filter_payout_methods(
                &state,
                &merchant_account,
                &key_store,
                &payout,
                address.as_ref(),
            )
            .await?;
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
            let mut enabled_payment_methods = link_data
                .enabled_payment_methods
                .unwrap_or(fallback_enabled_payout_methods.to_vec());

            // Sort payment methods (cards first)
            enabled_payment_methods.sort_by(|a, b| match (a.payment_method, b.payment_method) {
                (_, common_enums::PaymentMethod::Card) => Ordering::Greater,
                (common_enums::PaymentMethod::Card, _) => Ordering::Less,
                _ => Ordering::Equal,
            });

            let required_field_override = api::RequiredFieldsOverrideRequest {
                billing: address
                    .as_ref()
                    .map(hyperswitch_domain_models::address::Address::from)
                    .map(From::from),
            };

            let enabled_payment_methods_with_required_fields = ForeignFrom::foreign_from((
                &state.conf.payouts.required_fields,
                enabled_payment_methods.clone(),
                required_field_override,
            ));

            let js_data = payouts::PayoutLinkDetails {
                publishable_key: masking::Secret::new(merchant_account.publishable_key),
                client_secret: link_data.client_secret.clone(),
                payout_link_id: payout_link.link_id,
                payout_id: payout_link.primary_reference,
                customer_id: customer.customer_id,
                session_expiry: payout_link.expiry,
                return_url: payout_link
                    .return_url
                    .as_ref()
                    .map(|url| url::Url::parse(url))
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse payout status link's return URL")?,
                ui_config: ui_config_data,
                enabled_payment_methods,
                enabled_payment_methods_with_required_fields,
                amount,
                currency: payout.destination_currency,
                locale: state.locale.clone(),
                form_layout: link_data.form_layout,
                test_mode: link_data.test_mode.unwrap_or(false),
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
                sdk_url: default_config.sdk_url.to_string(),
                html_meta_tags: String::new(),
            };
            Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                GenericLinks {
                    allowed_domains,
                    data: GenericLinksData::PayoutLink(generic_form_data),
                    locale: state.locale.clone(),
                },
            )))
        }

        // Send back status page
        (_, link_utils::PayoutLinkStatus::Submitted) => {
            let translated_unified_message =
                payout_helpers::get_translated_unified_code_and_message(
                    &state,
                    payout_attempt.unified_code.as_ref(),
                    payout_attempt.unified_message.as_ref(),
                    &state.locale.clone(),
                )
                .await?;
            let js_data = payouts::PayoutLinkStatusDetails {
                payout_link_id: payout_link.link_id,
                payout_id: payout_link.primary_reference,
                customer_id: link_data.customer_id,
                session_expiry: payout_link.expiry,
                return_url: payout_link
                    .return_url
                    .as_ref()
                    .map(|url| url::Url::parse(url))
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse payout status link's return URL")?,
                status: payout.status,
                error_code: payout_attempt.unified_code,
                error_message: translated_unified_message,
                ui_config: ui_config_data,
                test_mode: link_data.test_mode.unwrap_or(false),
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
                GenericLinks {
                    allowed_domains,
                    data: GenericLinksData::PayoutLinkStatus(generic_status_data),
                    locale: state.locale.clone(),
                },
            )))
        }
    }
}

#[cfg(all(feature = "payouts", feature = "v1"))]
pub async fn filter_payout_methods(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payout: &hyperswitch_domain_models::payouts::payouts::Payouts,
    address: Option<&domain::Address>,
) -> errors::RouterResult<Vec<link_utils::EnabledPaymentMethod>> {
    use masking::ExposeInterface;

    let db = &*state.store;
    let key_manager_state = &state.into();
    //Fetch all merchant connector accounts
    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            key_manager_state,
            merchant_account.get_id(),
            false,
            key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    // Filter MCAs based on profile_id and connector_type
    let filtered_mcas = payment_helpers::filter_mca_based_on_profile_and_connector_type(
        all_mcas,
        &payout.profile_id,
        common_enums::ConnectorType::PayoutProcessor,
    );

    let mut response: Vec<link_utils::EnabledPaymentMethod> = vec![];
    let mut payment_method_list_hm: HashMap<
        common_enums::PaymentMethod,
        HashSet<common_enums::PaymentMethodType>,
    > = HashMap::new();
    let mut bank_transfer_hash_set: HashSet<common_enums::PaymentMethodType> = HashSet::new();
    let mut card_hash_set: HashSet<common_enums::PaymentMethodType> = HashSet::new();
    let mut wallet_hash_set: HashSet<common_enums::PaymentMethodType> = HashSet::new();
    let payout_filter_config = &state.conf.payout_method_filters.clone();
    for mca in &filtered_mcas {
        let payout_methods = match &mca.payment_methods_enabled {
            Some(pm) => pm,
            None => continue,
        };
        for payout_method in payout_methods.iter() {
            let parse_result = serde_json::from_value::<api_models::admin::PaymentMethodsEnabled>(
                payout_method.clone().expose(),
            );
            if let Ok(payment_methods_enabled) = parse_result {
                let payment_method = payment_methods_enabled.payment_method;
                let payment_method_types = match payment_methods_enabled.payment_method_types {
                    Some(payment_method_types) => payment_method_types,
                    None => continue,
                };
                let connector = mca.connector_name.clone();
                let payout_filter = payout_filter_config.0.get(&connector);
                for request_payout_method_type in &payment_method_types {
                    let currency_country_filter = check_currency_country_filters(
                        payout_filter,
                        request_payout_method_type,
                        payout.destination_currency,
                        address
                            .as_ref()
                            .and_then(|address| address.country)
                            .as_ref(),
                    )?;
                    if currency_country_filter.unwrap_or(true) {
                        match payment_method {
                            common_enums::PaymentMethod::Card => {
                                card_hash_set
                                    .insert(request_payout_method_type.payment_method_type);
                                payment_method_list_hm
                                    .insert(payment_method, card_hash_set.clone());
                            }
                            common_enums::PaymentMethod::Wallet => {
                                wallet_hash_set
                                    .insert(request_payout_method_type.payment_method_type);
                                payment_method_list_hm
                                    .insert(payment_method, wallet_hash_set.clone());
                            }
                            common_enums::PaymentMethod::BankTransfer => {
                                bank_transfer_hash_set
                                    .insert(request_payout_method_type.payment_method_type);
                                payment_method_list_hm
                                    .insert(payment_method, bank_transfer_hash_set.clone());
                            }
                            common_enums::PaymentMethod::CardRedirect
                            | common_enums::PaymentMethod::PayLater
                            | common_enums::PaymentMethod::BankRedirect
                            | common_enums::PaymentMethod::Crypto
                            | common_enums::PaymentMethod::BankDebit
                            | common_enums::PaymentMethod::Reward
                            | common_enums::PaymentMethod::RealTimePayment
                            | common_enums::PaymentMethod::MobilePayment
                            | common_enums::PaymentMethod::Upi
                            | common_enums::PaymentMethod::Voucher
                            | common_enums::PaymentMethod::OpenBanking
                            | common_enums::PaymentMethod::GiftCard => continue,
                        }
                    }
                }
            }
        }
    }
    for (payment_method, payment_method_types) in payment_method_list_hm {
        if !payment_method_types.is_empty() {
            let enabled_payment_method = link_utils::EnabledPaymentMethod {
                payment_method,
                payment_method_types,
            };
            response.push(enabled_payment_method);
        }
    }
    Ok(response)
}

pub fn check_currency_country_filters(
    payout_method_filter: Option<&PaymentMethodFilters>,
    request_payout_method_type: &api_models::payment_methods::RequestPaymentMethodTypes,
    currency: common_enums::Currency,
    country: Option<&common_enums::CountryAlpha2>,
) -> errors::RouterResult<Option<bool>> {
    if matches!(
        request_payout_method_type.payment_method_type,
        common_enums::PaymentMethodType::Credit | common_enums::PaymentMethodType::Debit
    ) {
        Ok(Some(true))
    } else {
        let payout_method_type_filter =
            payout_method_filter.and_then(|payout_method_filter: &PaymentMethodFilters| {
                payout_method_filter
                    .0
                    .get(&PaymentMethodFilterKey::PaymentMethodType(
                        request_payout_method_type.payment_method_type,
                    ))
            });
        let country_filter = country.as_ref().and_then(|country| {
            payout_method_type_filter.and_then(|currency_country_filter| {
                currency_country_filter
                    .country
                    .as_ref()
                    .map(|country_hash_set| country_hash_set.contains(country))
            })
        });
        let currency_filter = payout_method_type_filter.and_then(|currency_country_filter| {
            currency_country_filter
                .currency
                .as_ref()
                .map(|currency_hash_set| currency_hash_set.contains(&currency))
        });
        Ok(currency_filter.or(country_filter))
    }
}
