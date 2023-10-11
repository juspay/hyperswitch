use api_models::admin as admin_types;
use error_stack::{IntoReport, ResultExt};

use super::errors::{self, StorageErrorExt};
use crate::{
    core::payments::helpers,
    errors::RouterResponse,
    routes::AppState,
    services,
    types::{domain, storage::enums as storage_enums, transformers::ForeignFrom},
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

    let payment_link_metadata = merchant_account
        .payment_link_metadata
        .map(|pl_metadata| {
            serde_json::from_value::<admin_types::PaymentLinkMetadata>(pl_metadata)
                .into_report()
                .change_context(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "payment_link_metadata",
                })
        })
        .transpose()?;

    let js_script = get_js_script(
        payment_intent.amount.to_string(),
        payment_intent.currency.unwrap_or_default().to_string(),
        merchant_account.publishable_key.unwrap_or_default(),
        payment_intent.client_secret.unwrap_or_default(),
        payment_intent.payment_id,
        payment_intent.return_url,
        merchant_account.merchant_id,
        payment_link_metadata.clone(),
    );

    let css_script = get_color_scheme_css(payment_link_metadata.clone());
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
#[allow(clippy::too_many_arguments)]
fn get_js_script(
    amount: String,
    currency: String,
    pub_key: String,
    secret: String,
    payment_id: String,
    return_url: Option<String>,
    merchant_id: String,
    payment_link_metadata: Option<api_models::admin::PaymentLinkMetadata>,
) -> String {
    let merchant_logo = if let Some(pl_metadata) = payment_link_metadata {
        pl_metadata.merchant_logo.unwrap_or(
            "https://images.softwaresuggest.com/software_logo/1680595424_Hyperswitch_(2).png"
                .to_string(),
        )
    } else {
        "https://images.softwaresuggest.com/software_logo/1680595424_Hyperswitch_(2).png"
            .to_string()
    };
    let return_url = return_url.unwrap_or("https://hyperswitch.io/".to_string());
    format!(
        "window.__PAYMENT_DETAILS_STR = JSON.stringify({{
        client_secret: '{secret}',
        amount: '{amount}',
        currency: '{currency}',
        payment_id: '{payment_id}',
        // TODO: Remove hardcoded values
        merchant_logo: '{merchant_logo}',
        return_url: '{return_url}',
        currency_symbol: '{currency}',
        merchant: '{merchant_id}',
        max_items_visible_after_collapse: 3,
        order_details: [
            {{
              product_name:
                'dskjghbdsiuh sagfvbsajd ugbfiusedg fiudshgiu sdhgvishd givuhdsifu gnb gidsug biuesbdg iubsedg bsduxbg jhdxbgv jdskfbgi sdfgibuh ew87t54378 ghdfjbv jfdhgvb dufhvbfidu hg5784ghdfbjnk f (taxes incl.)',
              quantity: 2,
              amount: 100,
              product_image:
                'https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg',
            }},
            {{
              product_name: \"F1 '23\",
              quantity: 4,
              amount: 500,
              product_image:
                'https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg',
            }},
            {{
              product_name: \"Motosport '24\",
              quantity: 4,
              amount: 500,
              product_image:
                'https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg',
            }},
            {{
              product_name: 'Trackmania',
              quantity: 4,
              amount: 500,
              product_image:
                'https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg',
            }},
            {{
              product_name: 'Ghost Recon',
              quantity: 4,
              amount: 500,
              product_image:
                'https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg',
            }},
            {{
              product_name: 'Cup of Tea',
              quantity: 4,
              amount: 500,
              product_image:
                'https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg',
            }},
            {{
              product_name: 'Tea cups',
              quantity: 4,
              amount: 500,
              product_image:
                'https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg',
            }},
          ]
    }});

    const hyper = Hyper(\"{pub_key}\");"
    )
}

fn get_color_scheme_css(
    payment_link_metadata: Option<api_models::admin::PaymentLinkMetadata>,
) -> String {
    let (primary_color, primary_accent_color, secondary_color) = payment_link_metadata
        .and_then(|pl_metadata| {
            pl_metadata.color_scheme.map(|color| {
                (
                    color.primary_color.unwrap_or("#6A8EF5".to_string()),
                    color.primary_accent_color.unwrap_or("#6A8EF5".to_string()),
                    color.secondary_color.unwrap_or("#0C48F6".to_string()),
                )
            })
        })
        .unwrap_or((
            "#6A8EF5".to_string(),
            "#6A8EF5".to_string(),
            "#0C48F6".to_string(),
        ));

    format!(
        ":root {{
      --primary-color: {primary_color};
      --primary-accent-color: {primary_accent_color};
      --secondary-color: {secondary_color};
    }}"
    )
}
