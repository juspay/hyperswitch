use api_models::admin as admin_types;
use common_utils::ext_traits::AsyncExt;
use error_stack::{IntoReport, ResultExt};

use super::errors::{self, RouterResult, StorageErrorExt};
use crate::{
    core::payments::helpers,
    errors::RouterResponse,
    routes::AppState,
    services,
    types::{domain, storage::enums as storage_enums, transformers::ForeignFrom},
    utils::OptionExt,
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

    let response =
        api_models::payments::RetrievePaymentLinkResponse::foreign_from(payment_link_object);
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

    helpers::validate_payment_status_against_not_allowed_statuses(
        &payment_intent.status,
        &[
            storage_enums::IntentStatus::Cancelled,
            storage_enums::IntentStatus::Succeeded,
            storage_enums::IntentStatus::Processing,
            storage_enums::IntentStatus::RequiresCapture,
            storage_enums::IntentStatus::RequiresMerchantAction,
        ],
        "create payment link",
    )?;

    let fulfillment_time = payment_intent
        .payment_link_id
        .as_ref()
        .async_and_then(|pli| async move {
            db.find_payment_link_by_payment_link_id(pli)
                .await
                .ok()?
                .fulfilment_time
                .ok_or(errors::ApiErrorResponse::PaymentNotFound)
                .ok()
        })
        .await
        .get_required_value("fulfillment_time")
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

    let payment_link_config = merchant_account
        .payment_link_config
        .map(|pl_config| {
            serde_json::from_value::<admin_types::PaymentLinkConfig>(pl_config)
                .into_report()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "payment_link_config",
                })
        })
        .transpose()?;

    let order_details = payment_intent
        .order_details
        .get_required_value("order_details")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "order_details",
        })?;

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

    let payment_details = api_models::payments::PaymentLinkDetails {
        amount: payment_intent.amount,
        currency,
        payment_id: payment_intent.payment_id,
        merchant_name: merchant_account.merchant_name,
        order_details,
        return_url,
        expiry: fulfillment_time,
        pub_key,
        client_secret,
        merchant_logo: payment_link_config
            .clone()
            .map(|pl_metadata| pl_metadata.merchant_logo.unwrap_or_default())
            .unwrap_or_default(),
        max_items_visible_after_collapse: 3,
    };

    let js_script = get_js_script(payment_details)?;
    let css_script = get_color_scheme_css(payment_link_config.clone());
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
) -> String {
    let (default_primary_color, default_accent_color, default_secondary_color) = (
        "#C6C7C8".to_string(),
        "#6A8EF5".to_string(),
        "#0C48F6".to_string(),
    );

    let (primary_color, primary_accent_color, secondary_color) = payment_link_config
        .and_then(|pl_config| {
            pl_config.color_scheme.map(|color| {
                (
                    color.primary_color.unwrap_or(default_primary_color.clone()),
                    color
                        .primary_accent_color
                        .unwrap_or(default_accent_color.clone()),
                    color
                        .secondary_color
                        .unwrap_or(default_secondary_color.clone()),
                )
            })
        })
        .unwrap_or((
            default_primary_color,
            default_accent_color,
            default_secondary_color,
        ));

    format!(
        ":root {{
      --primary-color: {primary_color};
      --primary-accent-color: {primary_accent_color};
      --secondary-color: {secondary_color};
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
