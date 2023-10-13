use api_models::enums::Currency;
use common_utils::{crypto::OptionalEncryptableName, ext_traits::AsyncExt, pii::SecretSerdeValue};
use error_stack::{IntoReport, ResultExt};
use time::PrimitiveDateTime;

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

    let order_details = payment_intent
        .order_details
        .get_required_value("order_details")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "order_details",
        })?;

    println!("order_details sahkal {:?}", order_details);

    let payment_details = api_models::payments::PaymentLinkDetails {
        amount: payment_intent.amount,
        currency: payment_intent.currency.unwrap_or_default(),
        payment_id: payment_intent.payment_id,
        merchant_name: merchant_account.merchant_name,
        order_details,
        return_url: payment_intent.return_url.unwrap_or_default(),
        expiry: fulfillment_time,
        pub_key: merchant_account.publishable_key.unwrap_or_default(),
        client_secret: payment_intent.client_secret.unwrap_or_default(),
        // TODO: Remove hardcoded values
        merchant_logo: "https://upload.wikimedia.org/wikipedia/commons/8/83/Steam_icon_logo.svg"
            .to_string(),
        max_items_visible_after_collapse: 3
    };

    let js_script = get_js_script(payment_details)?;
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


fn get_js_script(payment_details: api_models::payments::PaymentLinkDetails) -> RouterResult<String> {
    let payment_details_str = serde_json::to_string(&payment_details)
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize PaymentLinkDetails")?;
    Ok(format!("window.__PAYMENT_DETAILS = {payment_details_str};"))
}
