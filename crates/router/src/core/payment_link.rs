use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;

use super::errors::{self, StorageErrorExt};
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

    let expiry = fulfillment_time.assume_utc().unix_timestamp();

    let js_script = get_js_script(
        payment_intent.amount.to_string(),
        payment_intent.currency.unwrap_or_default().to_string(),
        merchant_account.publishable_key.unwrap_or_default(),
        payment_intent.client_secret.unwrap_or_default(),
        payment_intent.payment_id,
        expiry,
    );

    let payment_link_data = services::PaymentLinkFormData {
        js_script,
        sdk_url: state.conf.payment_link.sdk_url.clone(),
    };
    Ok(services::ApplicationResponse::PaymenkLinkForm(Box::new(
        payment_link_data,
    )))
}

/*
The get_js_script function is used to inject dynamic value to payment_link sdk, which is unique to every payment.
*/

fn get_js_script(
    amount: String,
    currency: String,
    pub_key: String,
    secret: String,
    payment_id: String,
    expiry: i64,
) -> String {
    format!(
        "window.__PAYMENT_DETAILS_STR = JSON.stringify({{
        client_secret: '{secret}',
        amount: '{amount}',
        currency: '{currency}',
        payment_id: '{payment_id}',
        expiry: {expiry},
        // TODO: Remove hardcoded values
        merchant_logo: 'https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg',
        return_url: 'http://localhost:5500/public/index.html',
        currency_symbol: '$',
        merchant: 'Steam',
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
