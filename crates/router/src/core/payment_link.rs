pub mod validator;
use actix_web::http::header;
use api_models::{
    admin::PaymentLinkConfig,
    payments::{PaymentLinkData, PaymentLinkStatusWrap},
};
use common_utils::{
    consts::{
        DEFAULT_ALLOWED_DOMAINS, DEFAULT_BACKGROUND_COLOR, DEFAULT_DISPLAY_SDK_ONLY,
        DEFAULT_ENABLE_SAVED_PAYMENT_METHOD, DEFAULT_LOCALE, DEFAULT_MERCHANT_LOGO,
        DEFAULT_PRODUCT_IMG, DEFAULT_SDK_LAYOUT, DEFAULT_SESSION_EXPIRY,
        DEFAULT_TRANSACTION_DETAILS,
    },
    ext_traits::{AsyncExt, OptionExt, ValueExt},
    types::{AmountConvertor, MinorUnit, StringMajorUnitForCore},
};
use error_stack::{report, ResultExt};
use futures::future;
use hyperswitch_domain_models::api::{GenericLinks, GenericLinksData};
use masking::{PeekInterface, Secret};
use router_env::logger;
use time::PrimitiveDateTime;

use super::{
    errors::{self, RouterResult, StorageErrorExt},
    payments::helpers,
};
use crate::{
    consts,
    errors::RouterResponse,
    get_payment_link_config_value, get_payment_link_config_value_based_on_priority,
    headers::ACCEPT_LANGUAGE,
    routes::SessionState,
    services::{self, authentication::get_header_value_by_key},
    types::{
        api::payment_link::PaymentLinkResponseExt,
        domain,
        storage::{enums as storage_enums, payment_link::PaymentLink},
        transformers::ForeignFrom,
    },
};

pub async fn retrieve_payment_link(
    state: SessionState,
    payment_link_id: String,
) -> RouterResponse<api_models::payments::RetrievePaymentLinkResponse> {
    let db = &*state.store;
    let payment_link_config = db
        .find_payment_link_by_payment_link_id(&payment_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    let session_expiry = payment_link_config.fulfilment_time.unwrap_or_else(|| {
        common_utils::date_time::now()
            .saturating_add(time::Duration::seconds(DEFAULT_SESSION_EXPIRY))
    });

    let status = check_payment_link_status(session_expiry);

    let response = api_models::payments::RetrievePaymentLinkResponse::foreign_from((
        payment_link_config,
        status,
    ));
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn form_payment_link_data(
    state: &SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    merchant_id: common_utils::id_type::MerchantId,
    payment_id: String,
    locale: Option<String>,
) -> RouterResult<(PaymentLink, PaymentLinkData, PaymentLinkConfig)> {
    let db = &*state.store;
    let key_manager_state = &state.into();

    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &(state).into(),
            &payment_id,
            &merchant_id,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_link_id = payment_intent
        .payment_link_id
        .get_required_value("payment_link_id")
        .change_context(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    let merchant_name_from_merchant_account = merchant_account
        .merchant_name
        .clone()
        .map(|merchant_name| merchant_name.into_inner().peek().to_owned())
        .unwrap_or_default();

    let payment_link = db
        .find_payment_link_by_payment_link_id(&payment_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    let payment_link_config =
        if let Some(pl_config_value) = payment_link.payment_link_config.clone() {
            extract_payment_link_config(pl_config_value)?
        } else {
            PaymentLinkConfig {
                theme: DEFAULT_BACKGROUND_COLOR.to_string(),
                logo: DEFAULT_MERCHANT_LOGO.to_string(),
                seller_name: merchant_name_from_merchant_account,
                sdk_layout: DEFAULT_SDK_LAYOUT.to_owned(),
                display_sdk_only: DEFAULT_DISPLAY_SDK_ONLY,
                enabled_saved_payment_method: DEFAULT_ENABLE_SAVED_PAYMENT_METHOD,
                allowed_domains: DEFAULT_ALLOWED_DOMAINS,
                transaction_details: DEFAULT_TRANSACTION_DETAILS,
            }
        };

    let profile_id = payment_link
        .profile_id
        .clone()
        .or(payment_intent.profile_id)
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Profile id missing in payment link and payment intent")?;

    let business_profile = db
        .find_business_profile_by_profile_id(key_manager_state, &key_store, &profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id.to_string(),
        })?;

    let return_url = if let Some(payment_create_return_url) = payment_intent.return_url.clone() {
        payment_create_return_url
    } else {
        business_profile
            .return_url
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "return_url",
            })?
    };

    let (currency, client_secret) = validate_sdk_requirements(
        payment_intent.currency,
        payment_intent.client_secret.clone(),
    )?;

    let required_conversion_type = StringMajorUnitForCore;

    let amount = required_conversion_type
        .convert(payment_intent.amount, currency)
        .change_context(errors::ApiErrorResponse::AmountConversionFailed {
            amount_type: "StringMajorUnit",
        })?;

    let order_details = validate_order_details(payment_intent.order_details.clone(), currency)?;

    let session_expiry = payment_link.fulfilment_time.unwrap_or_else(|| {
        payment_intent
            .created_at
            .saturating_add(time::Duration::seconds(DEFAULT_SESSION_EXPIRY))
    });

    // converting first letter of merchant name to upperCase
    let merchant_name = capitalize_first_char(&payment_link_config.seller_name);
    let payment_link_status = check_payment_link_status(session_expiry);

    let is_terminal_state = check_payment_link_invalid_conditions(
        &payment_intent.status,
        &[
            storage_enums::IntentStatus::Cancelled,
            storage_enums::IntentStatus::Failed,
            storage_enums::IntentStatus::Processing,
            storage_enums::IntentStatus::RequiresCapture,
            storage_enums::IntentStatus::RequiresMerchantAction,
            storage_enums::IntentStatus::Succeeded,
            storage_enums::IntentStatus::PartiallyCaptured,
        ],
    );
    if is_terminal_state || payment_link_status == api_models::payments::PaymentLinkStatus::Expired
    {
        let status = match payment_link_status {
            api_models::payments::PaymentLinkStatus::Active => {
                logger::info!("displaying status page as the requested payment link has reached terminal state with payment status as {:?}", payment_intent.status);
                PaymentLinkStatusWrap::IntentStatus(payment_intent.status)
            }
            api_models::payments::PaymentLinkStatus::Expired => {
                if is_terminal_state {
                    logger::info!("displaying status page as the requested payment link has reached terminal state with payment status as {:?}", payment_intent.status);
                    PaymentLinkStatusWrap::IntentStatus(payment_intent.status)
                } else {
                    logger::info!(
                        "displaying status page as the requested payment link has expired"
                    );
                    PaymentLinkStatusWrap::PaymentLinkStatus(
                        api_models::payments::PaymentLinkStatus::Expired,
                    )
                }
            }
        };

        let attempt_id = payment_intent.active_attempt.get_id().clone();
        let payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                &merchant_id,
                &attempt_id.clone(),
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        let payment_details = api_models::payments::PaymentLinkStatusDetails {
            amount,
            currency,
            payment_id: payment_intent.payment_id,
            merchant_name,
            merchant_logo: payment_link_config.logo.clone(),
            created: payment_link.created_at,
            status,
            error_code: payment_attempt.error_code,
            error_message: payment_attempt.error_message,
            redirect: false,
            theme: payment_link_config.theme.clone(),
            return_url: return_url.clone(),
            locale: locale.clone(),
            transaction_details: payment_link_config.transaction_details.clone(),
            unified_code: payment_attempt.unified_code,
            unified_message: payment_attempt.unified_message,
        };

        return Ok((
            payment_link,
            PaymentLinkData::PaymentLinkStatusDetails(Box::new(payment_details)),
            payment_link_config,
        ));
    };

    let payment_link_details = api_models::payments::PaymentLinkDetails {
        amount,
        currency,
        payment_id: payment_intent.payment_id,
        merchant_name,
        order_details,
        return_url,
        session_expiry,
        pub_key: merchant_account.publishable_key,
        client_secret,
        merchant_logo: payment_link_config.logo.clone(),
        max_items_visible_after_collapse: 3,
        theme: payment_link_config.theme.clone(),
        merchant_description: payment_intent.description,
        sdk_layout: payment_link_config.sdk_layout.clone(),
        display_sdk_only: payment_link_config.display_sdk_only,
        locale,
        transaction_details: payment_link_config.transaction_details.clone(),
    };

    Ok((
        payment_link,
        PaymentLinkData::PaymentLinkDetails(Box::new(payment_link_details)),
        payment_link_config,
    ))
}

pub async fn initiate_secure_payment_link_flow(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    merchant_id: common_utils::id_type::MerchantId,
    payment_id: String,
    request_headers: &header::HeaderMap,
) -> RouterResponse<services::PaymentLinkFormData> {
    let locale = get_header_value_by_key(ACCEPT_LANGUAGE.into(), request_headers)?
        .map(|val| val.to_string());
    let (payment_link, payment_link_details, payment_link_config) = form_payment_link_data(
        &state,
        merchant_account,
        key_store,
        merchant_id,
        payment_id,
        locale,
    )
    .await?;

    validator::validate_secure_payment_link_render_request(
        request_headers,
        &payment_link,
        &payment_link_config,
    )?;

    let css_script = get_color_scheme_css(&payment_link_config);

    match payment_link_details {
        PaymentLinkData::PaymentLinkStatusDetails(ref status_details) => {
            let js_script = get_js_script(&payment_link_details)?;
            let payment_link_error_data = services::PaymentLinkStatusData {
                js_script,
                css_script,
            };
            logger::info!(
                "payment link data, for building payment link status page {:?}",
                status_details
            );
            Ok(services::ApplicationResponse::PaymentLinkForm(Box::new(
                services::api::PaymentLinkAction::PaymentLinkStatus(payment_link_error_data),
            )))
        }
        PaymentLinkData::PaymentLinkDetails(link_details) => {
            let secure_payment_link_details = api_models::payments::SecurePaymentLinkDetails {
                enabled_saved_payment_method: payment_link_config.enabled_saved_payment_method,
                payment_link_details: *link_details.to_owned(),
            };
            let js_script = format!(
                "window.__PAYMENT_DETAILS = {}",
                serde_json::to_string(&secure_payment_link_details)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize PaymentLinkData")?
            );
            let html_meta_tags = get_meta_tags_html(&link_details);
            let payment_link_data = services::PaymentLinkFormData {
                js_script,
                sdk_url: state.conf.payment_link.sdk_url.clone(),
                css_script,
                html_meta_tags,
            };
            let allowed_domains = payment_link_config
                .allowed_domains
                .clone()
                .ok_or(report!(errors::ApiErrorResponse::InternalServerError))
                .attach_printable_lazy(|| {
                    format!(
                        "Invalid list of allowed_domains found - {:?}",
                        payment_link_config.allowed_domains.clone()
                    )
                })?;

            if allowed_domains.is_empty() {
                return Err(report!(errors::ApiErrorResponse::InternalServerError))
                    .attach_printable_lazy(|| {
                        format!(
                            "Invalid list of allowed_domains found - {:?}",
                            payment_link_config.allowed_domains.clone()
                        )
                    });
            }

            let link_data = GenericLinks {
                allowed_domains,
                data: GenericLinksData::SecurePaymentLink(payment_link_data),
                locale: DEFAULT_LOCALE.to_string(),
            };
            logger::info!(
                "payment link data, for building secure payment link {:?}",
                link_data
            );

            Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                link_data,
            )))
        }
    }
}

pub async fn initiate_payment_link_flow(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    merchant_id: common_utils::id_type::MerchantId,
    payment_id: String,
    request_headers: &header::HeaderMap,
) -> RouterResponse<services::PaymentLinkFormData> {
    let locale = get_header_value_by_key(ACCEPT_LANGUAGE.into(), request_headers)?
        .map(|val| val.to_string());
    let (_, payment_details, payment_link_config) = form_payment_link_data(
        &state,
        merchant_account,
        key_store,
        merchant_id,
        payment_id,
        locale,
    )
    .await?;

    let css_script = get_color_scheme_css(&payment_link_config);
    let js_script = get_js_script(&payment_details)?;

    match payment_details {
        PaymentLinkData::PaymentLinkStatusDetails(status_details) => {
            let payment_link_error_data = services::PaymentLinkStatusData {
                js_script,
                css_script,
            };
            logger::info!(
                "payment link data, for building payment link status page {:?}",
                status_details
            );
            Ok(services::ApplicationResponse::PaymentLinkForm(Box::new(
                services::api::PaymentLinkAction::PaymentLinkStatus(payment_link_error_data),
            )))
        }
        PaymentLinkData::PaymentLinkDetails(payment_details) => {
            let html_meta_tags = get_meta_tags_html(&payment_details);
            let payment_link_data = services::PaymentLinkFormData {
                js_script,
                sdk_url: state.conf.payment_link.sdk_url.clone(),
                css_script,
                html_meta_tags,
            };
            logger::info!(
                "payment link data, for building open payment link {:?}",
                payment_link_data
            );
            Ok(services::ApplicationResponse::PaymentLinkForm(Box::new(
                services::api::PaymentLinkAction::PaymentLinkFormData(payment_link_data),
            )))
        }
    }
}

/*
The get_js_script function is used to inject dynamic value to payment_link sdk, which is unique to every payment.
*/

fn get_js_script(payment_details: &PaymentLinkData) -> RouterResult<String> {
    let payment_details_str = serde_json::to_string(payment_details)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize PaymentLinkData")?;
    Ok(format!("window.__PAYMENT_DETAILS = {payment_details_str};"))
}

fn get_color_scheme_css(payment_link_config: &PaymentLinkConfig) -> String {
    let background_primary_color = payment_link_config.theme.clone();
    format!(
        ":root {{
      --primary-color: {background_primary_color};
    }}"
    )
}

fn get_meta_tags_html(payment_details: &api_models::payments::PaymentLinkDetails) -> String {
    format!(
        r#"<meta property="og:title" content="Payment request from {0}"/>
        <meta property="og:description" content="{1}"/>"#,
        payment_details.merchant_name.clone(),
        payment_details
            .merchant_description
            .clone()
            .unwrap_or_default()
    )
}

fn validate_sdk_requirements(
    currency: Option<api_models::enums::Currency>,
    client_secret: Option<String>,
) -> Result<(api_models::enums::Currency, String), errors::ApiErrorResponse> {
    let currency = currency.ok_or(errors::ApiErrorResponse::MissingRequiredField {
        field_name: "currency",
    })?;

    let client_secret = client_secret.ok_or(errors::ApiErrorResponse::MissingRequiredField {
        field_name: "client_secret",
    })?;
    Ok((currency, client_secret))
}

pub async fn list_payment_link(
    state: SessionState,
    merchant: domain::MerchantAccount,
    constraints: api_models::payments::PaymentLinkListConstraints,
) -> RouterResponse<Vec<api_models::payments::RetrievePaymentLinkResponse>> {
    let db = state.store.as_ref();
    let payment_link = db
        .list_payment_link_by_merchant_id(merchant.get_id(), constraints)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to retrieve payment link")?;
    let payment_link_list = future::try_join_all(payment_link.into_iter().map(|payment_link| {
        api_models::payments::RetrievePaymentLinkResponse::from_db_payment_link(payment_link)
    }))
    .await?;
    Ok(services::ApplicationResponse::Json(payment_link_list))
}

pub fn check_payment_link_status(
    payment_link_expiry: PrimitiveDateTime,
) -> api_models::payments::PaymentLinkStatus {
    let curr_time = common_utils::date_time::now();

    if curr_time > payment_link_expiry {
        api_models::payments::PaymentLinkStatus::Expired
    } else {
        api_models::payments::PaymentLinkStatus::Active
    }
}

fn validate_order_details(
    order_details: Option<Vec<Secret<serde_json::Value>>>,
    currency: api_models::enums::Currency,
) -> Result<
    Option<Vec<api_models::payments::OrderDetailsWithStringAmount>>,
    error_stack::Report<errors::ApiErrorResponse>,
> {
    let required_conversion_type = StringMajorUnitForCore;
    let order_details = order_details
        .map(|order_details| {
            order_details
                .iter()
                .map(|data| {
                    data.to_owned()
                        .parse_value("OrderDetailsWithAmount")
                        .change_context(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "OrderDetailsWithAmount",
                        })
                        .attach_printable("Unable to parse OrderDetailsWithAmount")
                })
                .collect::<Result<Vec<api_models::payments::OrderDetailsWithAmount>, _>>()
        })
        .transpose()?;

    let updated_order_details = match order_details {
        Some(mut order_details) => {
            let mut order_details_amount_string_array: Vec<
                api_models::payments::OrderDetailsWithStringAmount,
            > = Vec::new();
            for order in order_details.iter_mut() {
                let mut order_details_amount_string : api_models::payments::OrderDetailsWithStringAmount = Default::default();
                if order.product_img_link.is_none() {
                    order_details_amount_string.product_img_link =
                        Some(DEFAULT_PRODUCT_IMG.to_string())
                } else {
                    order_details_amount_string
                        .product_img_link
                        .clone_from(&order.product_img_link)
                };
                order_details_amount_string.amount = required_conversion_type
                    .convert(MinorUnit::new(order.amount), currency)
                    .change_context(errors::ApiErrorResponse::AmountConversionFailed {
                        amount_type: "StringMajorUnit",
                    })?;
                order_details_amount_string.product_name =
                    capitalize_first_char(&order.product_name.clone());
                order_details_amount_string.quantity = order.quantity;
                order_details_amount_string_array.push(order_details_amount_string)
            }
            Some(order_details_amount_string_array)
        }
        None => None,
    };
    Ok(updated_order_details)
}

pub fn extract_payment_link_config(
    pl_config: serde_json::Value,
) -> Result<PaymentLinkConfig, error_stack::Report<errors::ApiErrorResponse>> {
    serde_json::from_value::<PaymentLinkConfig>(pl_config).change_context(
        errors::ApiErrorResponse::InvalidDataValue {
            field_name: "payment_link_config",
        },
    )
}

pub fn get_payment_link_config_based_on_priority(
    payment_create_link_config: Option<api_models::payments::PaymentCreatePaymentLinkConfig>,
    business_link_config: Option<diesel_models::business_profile::BusinessPaymentLinkConfig>,
    merchant_name: String,
    default_domain_name: String,
    payment_link_config_id: Option<String>,
) -> Result<(PaymentLinkConfig, String), error_stack::Report<errors::ApiErrorResponse>> {
    let (domain_name, business_theme_configs, allowed_domains) =
        if let Some(business_config) = business_link_config {
            logger::info!(
                "domain name set to custom domain https://{:?}",
                business_config.domain_name
            );

            (
                business_config
                    .domain_name
                    .clone()
                    .map(|d_name| format!("https://{}", d_name))
                    .unwrap_or_else(|| default_domain_name.clone()),
                payment_link_config_id
                    .and_then(|id| {
                        business_config
                            .business_specific_configs
                            .as_ref()
                            .and_then(|specific_configs| specific_configs.get(&id).cloned())
                    })
                    .or(business_config.default_config),
                business_config.allowed_domains,
            )
        } else {
            (default_domain_name, None, None)
        };

    let (theme, logo, seller_name, sdk_layout, display_sdk_only, enabled_saved_payment_method) = get_payment_link_config_value!(
        payment_create_link_config,
        business_theme_configs,
        (theme, DEFAULT_BACKGROUND_COLOR.to_string()),
        (logo, DEFAULT_MERCHANT_LOGO.to_string()),
        (seller_name, merchant_name.clone()),
        (sdk_layout, DEFAULT_SDK_LAYOUT.to_owned()),
        (display_sdk_only, DEFAULT_DISPLAY_SDK_ONLY),
        (
            enabled_saved_payment_method,
            DEFAULT_ENABLE_SAVED_PAYMENT_METHOD
        )
    );
    let payment_link_config = PaymentLinkConfig {
        theme,
        logo,
        seller_name,
        sdk_layout,
        display_sdk_only,
        enabled_saved_payment_method,
        allowed_domains,
        transaction_details: payment_create_link_config.and_then(|payment_link_config| {
            payment_link_config
                .theme_config
                .transaction_details
                .and_then(|transaction_details| {
                    match serde_json::to_string(&transaction_details).change_context(
                        errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "transaction_details",
                        },
                    ) {
                        Ok(details) => Some(details),
                        Err(err) => {
                            logger::error!("Failed to serialize transaction details: {:?}", err);
                            None
                        }
                    }
                })
        }),
    };

    Ok((payment_link_config, domain_name))
}

fn capitalize_first_char(s: &str) -> String {
    if let Some(first_char) = s.chars().next() {
        let capitalized = first_char.to_uppercase();
        let mut result = capitalized.to_string();
        if let Some(remaining) = s.get(1..) {
            result.push_str(remaining);
        }
        result
    } else {
        s.to_owned()
    }
}

fn check_payment_link_invalid_conditions(
    intent_status: &storage_enums::IntentStatus,
    not_allowed_statuses: &[storage_enums::IntentStatus],
) -> bool {
    not_allowed_statuses.contains(intent_status)
}

pub async fn get_payment_link_status(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    merchant_id: common_utils::id_type::MerchantId,
    payment_id: String,
    request_headers: &header::HeaderMap,
) -> RouterResponse<services::PaymentLinkFormData> {
    let locale = get_header_value_by_key(ACCEPT_LANGUAGE.into(), request_headers)?
        .map(|val| val.to_string());
    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            key_manager_state,
            &payment_id,
            &merchant_id,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let attempt_id = payment_intent.active_attempt.get_id().clone();
    let payment_attempt = db
        .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
            &payment_intent.payment_id,
            &merchant_id,
            &attempt_id.clone(),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_link_id = payment_intent
        .payment_link_id
        .get_required_value("payment_link_id")
        .change_context(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    let merchant_name_from_merchant_account = merchant_account
        .merchant_name
        .clone()
        .map(|merchant_name| merchant_name.into_inner().peek().to_owned())
        .unwrap_or_default();

    let payment_link = db
        .find_payment_link_by_payment_link_id(&payment_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    let payment_link_config = if let Some(pl_config_value) = payment_link.payment_link_config {
        extract_payment_link_config(pl_config_value)?
    } else {
        PaymentLinkConfig {
            theme: DEFAULT_BACKGROUND_COLOR.to_string(),
            logo: DEFAULT_MERCHANT_LOGO.to_string(),
            seller_name: merchant_name_from_merchant_account,
            sdk_layout: DEFAULT_SDK_LAYOUT.to_owned(),
            display_sdk_only: DEFAULT_DISPLAY_SDK_ONLY,
            enabled_saved_payment_method: DEFAULT_ENABLE_SAVED_PAYMENT_METHOD,
            allowed_domains: DEFAULT_ALLOWED_DOMAINS,
            transaction_details: DEFAULT_TRANSACTION_DETAILS,
        }
    };

    let currency =
        payment_intent
            .currency
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "currency",
            })?;

    let required_conversion_type = StringMajorUnitForCore;

    let amount = required_conversion_type
        .convert(payment_attempt.net_amount, currency)
        .change_context(errors::ApiErrorResponse::AmountConversionFailed {
            amount_type: "StringMajorUnit",
        })?;

    // converting first letter of merchant name to upperCase
    let merchant_name = capitalize_first_char(&payment_link_config.seller_name);
    let css_script = get_color_scheme_css(&payment_link_config);

    let profile_id = payment_link
        .profile_id
        .or(payment_intent.profile_id)
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Profile id missing in payment link and payment intent")?;

    let business_profile = db
        .find_business_profile_by_profile_id(key_manager_state, &key_store, &profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id.to_string(),
        })?;

    let return_url = if let Some(payment_create_return_url) = payment_intent.return_url.clone() {
        payment_create_return_url
    } else {
        business_profile
            .return_url
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "return_url",
            })?
    };
    let (unified_code, unified_message) = if let Some((code, message)) = payment_attempt
        .unified_code
        .as_ref()
        .zip(payment_attempt.unified_message.as_ref())
    {
        (code.to_owned(), message.to_owned())
    } else {
        (
            consts::DEFAULT_UNIFIED_ERROR_CODE.to_owned(),
            consts::DEFAULT_UNIFIED_ERROR_MESSAGE.to_owned(),
        )
    };
    let unified_translated_message = locale
        .as_ref()
        .async_and_then(|locale_str| async {
            helpers::get_unified_translation(
                &state,
                unified_code.to_owned(),
                unified_message.to_owned(),
                locale_str.to_owned(),
            )
            .await
        })
        .await
        .or(Some(unified_message));

    let payment_details = api_models::payments::PaymentLinkStatusDetails {
        amount,
        currency,
        payment_id: payment_intent.payment_id,
        merchant_name,
        merchant_logo: payment_link_config.logo.clone(),
        created: payment_link.created_at,
        status: PaymentLinkStatusWrap::IntentStatus(payment_intent.status),
        error_code: payment_attempt.error_code,
        error_message: payment_attempt.error_message,
        redirect: true,
        theme: payment_link_config.theme.clone(),
        return_url,
        locale,
        transaction_details: payment_link_config.transaction_details,
        unified_code: Some(unified_code),
        unified_message: unified_translated_message,
    };
    let js_script = get_js_script(&PaymentLinkData::PaymentLinkStatusDetails(Box::new(
        payment_details,
    )))?;
    let payment_link_status_data = services::PaymentLinkStatusData {
        js_script,
        css_script,
    };
    Ok(services::ApplicationResponse::PaymentLinkForm(Box::new(
        services::api::PaymentLinkAction::PaymentLinkStatus(payment_link_status_data),
    )))
}
