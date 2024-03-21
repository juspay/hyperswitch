use api_models::{admin as admin_types, payments::PaymentLinkStatusWrap};
use common_utils::{
    consts::{
        DEFAULT_BACKGROUND_COLOR, DEFAULT_MERCHANT_LOGO, DEFAULT_PRODUCT_IMG, DEFAULT_SDK_LAYOUT,
        DEFAULT_SESSION_EXPIRY,
    },
    ext_traits::{OptionExt, ValueExt},
};
use error_stack::{IntoReport, ResultExt};
use futures::future;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use super::errors::{self, RouterResult, StorageErrorExt};
use crate::{
    errors::RouterResponse,
    routes::AppState,
    services,
    types::{
        api::payment_link::PaymentLinkResponseExt, domain, storage::enums as storage_enums,
        transformers::ForeignFrom,
    },
};

pub async fn retrieve_payment_link(
    state: AppState,
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

pub async fn intiate_payment_link_flow(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    merchant_id: String,
    payment_id: String,
) -> RouterResponse<services::PaymentLinkFormData> {
    let db = &*state.store;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &payment_id,
            &merchant_id,
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
        admin_types::PaymentLinkConfig {
            theme: DEFAULT_BACKGROUND_COLOR.to_string(),
            logo: DEFAULT_MERCHANT_LOGO.to_string(),
            seller_name: merchant_name_from_merchant_account,
            sdk_layout: DEFAULT_SDK_LAYOUT.to_owned(),
        }
    };

    let profile_id = payment_link
        .profile_id
        .or(payment_intent.profile_id)
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Profile id missing in payment link and payment intent")?;

    let business_profile = db
        .find_business_profile_by_profile_id(&profile_id)
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

    let (pub_key, currency, client_secret) = validate_sdk_requirements(
        merchant_account.publishable_key,
        payment_intent.currency,
        payment_intent.client_secret.clone(),
    )?;
    let amount = currency
        .to_currency_base_unit(payment_intent.amount)
        .into_report()
        .change_context(errors::ApiErrorResponse::CurrencyConversionFailed)?;
    let order_details = validate_order_details(payment_intent.order_details.clone(), currency)?;

    let session_expiry = payment_link.fulfilment_time.unwrap_or_else(|| {
        payment_intent
            .created_at
            .saturating_add(time::Duration::seconds(DEFAULT_SESSION_EXPIRY))
    });

    // converting first letter of merchant name to upperCase
    let merchant_name = capitalize_first_char(&payment_link_config.seller_name);
    let css_script = get_color_scheme_css(payment_link_config.clone());
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
                PaymentLinkStatusWrap::IntentStatus(payment_intent.status)
            }
            api_models::payments::PaymentLinkStatus::Expired => {
                if is_terminal_state {
                    PaymentLinkStatusWrap::IntentStatus(payment_intent.status)
                } else {
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
        };
        let js_script = get_js_script(
            api_models::payments::PaymentLinkData::PaymentLinkStatusDetails(payment_details),
        )?;
        let payment_link_error_data = services::PaymentLinkStatusData {
            js_script,
            css_script,
        };
        return Ok(services::ApplicationResponse::PaymentLinkForm(Box::new(
            services::api::PaymentLinkAction::PaymentLinkStatus(payment_link_error_data),
        )));
    };

    let payment_details = api_models::payments::PaymentLinkDetails {
        amount,
        currency,
        payment_id: payment_intent.payment_id,
        merchant_name,
        order_details,
        return_url,
        session_expiry,
        pub_key,
        client_secret,
        merchant_logo: payment_link_config.logo.clone(),
        max_items_visible_after_collapse: 3,
        theme: payment_link_config.theme.clone(),
        merchant_description: payment_intent.description,
        sdk_layout: payment_link_config.sdk_layout.clone(),
    };

    let js_script = get_js_script(api_models::payments::PaymentLinkData::PaymentLinkDetails(
        payment_details,
    ))?;
    let payment_link_data = services::PaymentLinkFormData {
        js_script,
        sdk_url: state.conf.payment_link.sdk_url.clone(),
        css_script,
    };
    Ok(services::ApplicationResponse::PaymentLinkForm(Box::new(
        services::api::PaymentLinkAction::PaymentLinkFormData(payment_link_data),
    )))
}

/*
The get_js_script function is used to inject dynamic value to payment_link sdk, which is unique to every payment.
*/

fn get_js_script(payment_details: api_models::payments::PaymentLinkData) -> RouterResult<String> {
    let payment_details_str = serde_json::to_string(&payment_details)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize PaymentLinkData")?;
    Ok(format!("window.__PAYMENT_DETAILS = {payment_details_str};"))
}

fn get_color_scheme_css(payment_link_config: api_models::admin::PaymentLinkConfig) -> String {
    let background_primary_color = payment_link_config.theme;
    format!(
        ":root {{
      --primary-color: {background_primary_color};
    }}"
    )
}

fn validate_sdk_requirements(
    pub_key: Option<String>,
    currency: Option<api_models::enums::Currency>,
    client_secret: Option<String>,
) -> Result<(String, api_models::enums::Currency, String), errors::ApiErrorResponse> {
    let pub_key = pub_key.ok_or(errors::ApiErrorResponse::MissingRequiredField {
        field_name: "pub_key",
    })?;

    let currency = currency.ok_or(errors::ApiErrorResponse::MissingRequiredField {
        field_name: "currency",
    })?;

    let client_secret = client_secret.ok_or(errors::ApiErrorResponse::MissingRequiredField {
        field_name: "client_secret",
    })?;
    Ok((pub_key, currency, client_secret))
}

pub async fn list_payment_link(
    state: AppState,
    merchant: domain::MerchantAccount,
    constraints: api_models::payments::PaymentLinkListConstraints,
) -> RouterResponse<Vec<api_models::payments::RetrievePaymentLinkResponse>> {
    let db = state.store.as_ref();
    let payment_link = db
        .list_payment_link_by_merchant_id(&merchant.merchant_id, constraints)
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
                    order_details_amount_string.product_img_link = order.product_img_link.clone()
                };
                order_details_amount_string.amount = currency
                    .to_currency_base_unit(order.amount)
                    .into_report()
                    .change_context(errors::ApiErrorResponse::CurrencyConversionFailed)?;
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
) -> Result<api_models::admin::PaymentLinkConfig, error_stack::Report<errors::ApiErrorResponse>> {
    serde_json::from_value::<api_models::admin::PaymentLinkConfig>(pl_config)
        .into_report()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "payment_link_config",
        })
}

pub fn get_payment_link_config_based_on_priority(
    payment_create_link_config: Option<api_models::payments::PaymentCreatePaymentLinkConfig>,
    business_link_config: Option<serde_json::Value>,
    merchant_name: String,
    default_domain_name: String,
) -> Result<(admin_types::PaymentLinkConfig, String), error_stack::Report<errors::ApiErrorResponse>>
{
    let (domain_name, business_config) = if let Some(business_config) = business_link_config {
        let extracted_value: api_models::admin::BusinessPaymentLinkConfig = business_config
            .parse_value("BusinessPaymentLinkConfig")
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "payment_link_config",
            })
            .attach_printable("Invalid payment_link_config given in business config")?;

        (
            extracted_value
                .domain_name
                .clone()
                .map(|d_name| format!("https://{}", d_name))
                .unwrap_or_else(|| default_domain_name.clone()),
            Some(extracted_value.config),
        )
    } else {
        (default_domain_name, None)
    };

    let theme = payment_create_link_config
        .as_ref()
        .and_then(|pc_config| pc_config.config.theme.clone())
        .or_else(|| {
            business_config
                .as_ref()
                .and_then(|business_config| business_config.theme.clone())
        })
        .unwrap_or(DEFAULT_BACKGROUND_COLOR.to_string());

    let logo = payment_create_link_config
        .as_ref()
        .and_then(|pc_config| pc_config.config.logo.clone())
        .or_else(|| {
            business_config
                .as_ref()
                .and_then(|business_config| business_config.logo.clone())
        })
        .unwrap_or(DEFAULT_MERCHANT_LOGO.to_string());

    let seller_name = payment_create_link_config
        .as_ref()
        .and_then(|pc_config| pc_config.config.seller_name.clone())
        .or_else(|| {
            business_config
                .as_ref()
                .and_then(|business_config| business_config.seller_name.clone())
        })
        .unwrap_or(merchant_name);

    let sdk_layout = payment_create_link_config
        .as_ref()
        .and_then(|pc_config| pc_config.config.sdk_layout.clone())
        .or_else(|| {
            business_config
                .as_ref()
                .and_then(|business_config| business_config.sdk_layout.clone())
        })
        .unwrap_or(DEFAULT_SDK_LAYOUT.to_owned());

    let payment_link_config = admin_types::PaymentLinkConfig {
        theme,
        logo,
        seller_name,
        sdk_layout,
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
    state: AppState,
    merchant_account: domain::MerchantAccount,
    merchant_id: String,
    payment_id: String,
) -> RouterResponse<services::PaymentLinkFormData> {
    let db = &*state.store;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &payment_id,
            &merchant_id,
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
        admin_types::PaymentLinkConfig {
            theme: DEFAULT_BACKGROUND_COLOR.to_string(),
            logo: DEFAULT_MERCHANT_LOGO.to_string(),
            seller_name: merchant_name_from_merchant_account,
            sdk_layout: DEFAULT_SDK_LAYOUT.to_owned(),
        }
    };

    let currency =
        payment_intent
            .currency
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "currency",
            })?;

    let amount = currency
        .to_currency_base_unit(payment_attempt.net_amount)
        .into_report()
        .change_context(errors::ApiErrorResponse::CurrencyConversionFailed)?;

    // converting first letter of merchant name to upperCase
    let merchant_name = capitalize_first_char(&payment_link_config.seller_name);
    let css_script = get_color_scheme_css(payment_link_config.clone());

    let profile_id = payment_link
        .profile_id
        .or(payment_intent.profile_id)
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Profile id missing in payment link and payment intent")?;

    let business_profile = db
        .find_business_profile_by_profile_id(&profile_id)
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
    };
    let js_script = get_js_script(
        api_models::payments::PaymentLinkData::PaymentLinkStatusDetails(payment_details),
    )?;
    let payment_link_status_data = services::PaymentLinkStatusData {
        js_script,
        css_script,
    };
    Ok(services::ApplicationResponse::PaymentLinkForm(Box::new(
        services::api::PaymentLinkAction::PaymentLinkStatus(payment_link_status_data),
    )))
}
