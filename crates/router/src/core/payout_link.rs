use api_models::payouts;
use common_utils::{ext_traits::OptionExt, types};
use diesel_models::PayoutLinkUpdate;
use error_stack::ResultExt;

use super::errors::{RouterResponse, StorageErrorExt};
use crate::{
    errors,
    routes::{app::StorageInterface, SessionState},
    services::{self, GenericLinks},
    types::domain,
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
    let status = payout_link.link_status;
    let link_data = payout_link.link_data.clone();
    match (has_expired, status) {
        // Send back generic expired page
        (true, _) | (_, types::PayoutLinkStatus::Invalidated) => {
            let expired_link_data = services::GenericExpiredLinkData {
                title: "Payout Expired".to_string(),
                message: "This payout link has expired.".to_string(),
                theme: link_data.ui_config.theme,
            };

            if status != types::PayoutLinkStatus::Invalidated {
                let payout_link_update = PayoutLinkUpdate::StatusUpdate {
                    link_status: types::PayoutLinkStatus::Invalidated,
                };
                db.update_payout_link_by_merchant_id_link_id(payout_link, payout_link_update)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error updating payout links in db")?;
            }

            Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                GenericLinks::ExpiredLink(expired_link_data),
            )))
        }

        // Initiate Payout link flow
        (_, types::PayoutLinkStatus::Initiated) => {
            let customer_id = link_data.customer_id;
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

            let js_data = payouts::PayoutLinkDetails {
                pub_key: merchant_account
                    .publishable_key
                    .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                        field_name: "pub_key",
                    })?
                    .into(),
                client_secret: link_data.client_secret.clone(),
                payout_link_id: payout_link.link_id,
                payout_id: payout_link.primary_reference,
                customer_id: customer.customer_id,
                session_expiry: payout_link.expiry,
                return_url: payout_link.return_url,
                ui_config: link_data.ui_config,
                enabled_payment_methods: link_data.enabled_payment_methods,
                amount: payout.amount,
                currency: payout.destination_currency,
            };

            let serialized_css_content = "".to_string();

            let serialized_js_content =
                format!("window.__PAYOUT_DETAILS = {}", serialize(&js_data)?);

            let generic_form_data = services::GenericLinkFormData {
                js_data: serialized_js_content,
                css_data: serialized_css_content,
                sdk_url: link_data.sdk_host.clone(),
                html_meta_tags: "".to_string(),
            };
            Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                GenericLinks::PayoutLink(generic_form_data),
            )))
        }

        // Send back status page
        (_, types::PayoutLinkStatus::Submitted) => {
            let js_data = payouts::PayoutLinkStatusDetails {
                payout_link_id: payout_link.link_id,
                payout_id: payout_link.primary_reference,
                customer_id: link_data.customer_id,
                session_expiry: payout_link.expiry,
                return_url: payout_link.return_url,
                status: payout.status,
                error_code: payout_attempt.error_code,
                error_message: payout_attempt.error_message,
                ui_config: link_data.ui_config,
            };

            let serialized_css_content = "".to_string();

            let serialized_js_content =
                format!("window.__PAYOUT_DETAILS = {}", serialize(&js_data)?);

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
