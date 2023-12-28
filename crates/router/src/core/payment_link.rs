use api_models::admin as admin_types;
use common_utils::{
    consts::{
        DEFAULT_BACKGROUND_COLOR, DEFAULT_MERCHANT_LOGO, DEFAULT_PRODUCT_IMG, DEFAULT_SDK_THEME,
    },
    ext_traits::{OptionExt, ValueExt},
};
use error_stack::{IntoReport, ResultExt};
use futures::future;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use super::errors::{self, RouterResult, StorageErrorExt};
use crate::{
    core::payments::helpers,
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
    let payment_link_object = db
        .find_payment_link_by_payment_link_id(&payment_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    let status = check_payment_link_status(payment_link_object.fulfilment_time);

    let response = api_models::payments::RetrievePaymentLinkResponse::foreign_from((
        payment_link_object,
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

    helpers::validate_payment_status_against_not_allowed_statuses(
        &payment_intent.status,
        &[
            storage_enums::IntentStatus::Cancelled,
            storage_enums::IntentStatus::Succeeded,
            storage_enums::IntentStatus::Processing,
            storage_enums::IntentStatus::RequiresCapture,
            storage_enums::IntentStatus::RequiresMerchantAction,
        ],
        "use payment link for",
    )?;

    let payment_link = db
        .find_payment_link_by_payment_link_id(&payment_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    let payment_link_config = if let Some(pl_config) = payment_link.payment_link_config.clone() {
        extract_payment_link_config(Some(pl_config))?
    } else {
        extract_payment_link_config(merchant_account.payment_link_config.clone())?
    };

    let return_url = if let Some(payment_create_return_url) = payment_intent.return_url {
        payment_create_return_url
    } else {
        merchant_account
            .return_url
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "return_url",
            })?
    };

    let (pub_key, currency, client_secret) = validate_sdk_requirements(
        merchant_account.publishable_key,
        payment_intent.currency,
        payment_intent.client_secret,
    )?;
    let order_details = validate_order_details(payment_intent.order_details, currency)?;

    let (default_sdk_theme, default_background_color) =
        (DEFAULT_SDK_THEME, DEFAULT_BACKGROUND_COLOR);

    let payment_details = api_models::payments::PaymentLinkDetails {
        amount: currency
            .to_currency_base_unit(payment_intent.amount)
            .into_report()
            .change_context(errors::ApiErrorResponse::CurrencyConversionFailed)?,
        currency,
        payment_id: payment_intent.payment_id,
        merchant_name: payment_link.custom_merchant_name.unwrap_or(
            merchant_account
                .merchant_name
                .map(|merchant_name| merchant_name.into_inner().peek().to_owned())
                .unwrap_or_default(),
        ),
        order_details,
        return_url,
        expiry: payment_link.fulfilment_time,
        pub_key,
        client_secret,
        merchant_logo: payment_link_config
            .clone()
            .map(|pl_config| {
                pl_config
                    .merchant_logo
                    .unwrap_or(DEFAULT_MERCHANT_LOGO.to_string())
            })
            .unwrap_or_default(),
        max_items_visible_after_collapse: 3,
        sdk_theme: payment_link_config.clone().and_then(|pl_config| {
            pl_config
                .color_scheme
                .map(|color| color.sdk_theme.unwrap_or(default_sdk_theme.to_string()))
        }),
    };

    let js_script = get_js_script(payment_details)?;
    let css_script = get_color_scheme_css(
        payment_link_config.clone(),
        default_background_color.to_string(),
    );
    let payment_link_data = services::PaymentLinkFormData {
        js_script,
        sdk_url: state.conf.payment_link.sdk_url.clone(),
        css_script,
    };
    Ok(services::ApplicationResponse::PaymenkLinkForm(Box::new(
        payment_link_data,
    )))
}

/*
The get_js_script function is used to inject dynamic value to payment_link sdk, which is unique to every payment.
*/

fn get_js_script(
    payment_details: api_models::payments::PaymentLinkDetails,
) -> RouterResult<String> {
    let payment_details_str = serde_json::to_string(&payment_details)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize PaymentLinkDetails")?;
    Ok(format!("window.__PAYMENT_DETAILS = {payment_details_str};"))
}

fn get_color_scheme_css(
    payment_link_config: Option<api_models::admin::PaymentLinkConfig>,
    default_primary_color: String,
) -> String {
    let background_primary_color = payment_link_config
        .and_then(|pl_config| {
            pl_config.color_scheme.map(|color| {
                color
                    .background_primary_color
                    .unwrap_or(default_primary_color.clone())
            })
        })
        .unwrap_or(default_primary_color);

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

pub fn check_payment_link_status(fulfillment_time: Option<PrimitiveDateTime>) -> String {
    let curr_time = Some(common_utils::date_time::now());

    if curr_time > fulfillment_time {
        "expired".to_string()
    } else {
        "active".to_string()
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
                order_details_amount_string.product_name = order.product_name.clone();
                order_details_amount_string.quantity = order.quantity;
                order_details_amount_string_array.push(order_details_amount_string)
            }
            Some(order_details_amount_string_array)
        }
        None => None,
    };
    Ok(updated_order_details)
}

fn extract_payment_link_config(
    pl_config: Option<serde_json::Value>,
) -> Result<Option<admin_types::PaymentLinkConfig>, error_stack::Report<errors::ApiErrorResponse>> {
    pl_config
        .map(|config| {
            serde_json::from_value::<admin_types::PaymentLinkConfig>(config)
                .into_report()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "payment_link_config",
                })
        })
        .transpose()
}
