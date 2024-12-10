//! Contains functions of payment methods that are used in payments
//! one of such functions is `list_payment_methods`

use super::errors;
use crate::{routes, types::domain};
use common_utils::id_type;

#[cfg(all(
    feature = "v2",
    feature = "customer_v2",
    feature = "payment_methods_v2"
))]
pub async fn list_payment_methods(
    state: routes::SessionState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    payment_id: id_type::GlobalPaymentId,
    req: api_models::payments::PaymentMethodsListRequest,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
) -> errors::RouterResponse<api_models::payments::PaymentMethodListResponse> {
    // use common_utils::ext_traits::OptionExt;
    // use error_stack::ResultExt;

    // use crate::db::errors::StorageErrorExt;

    // let db = &*state.store;
    // let key_manager_state = &state.into();

    // let payment_intent = db
    //     .find_payment_intent_by_id(
    //         key_manager_state,
    //         &payment_id,
    //         &key_store,
    //         merchant_account.storage_scheme,
    //     )
    //     .await
    //     .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    // validate_payment_status(payment_intent.status)?;
    // let client_secret = header_payload
    //     .client_secret
    //     .as_ref()
    //     .get_required_value("client_secret header")?;
    // payment_intent.validate_client_secret(client_secret)?;

    // let payment_connector_accounts = db
    //     .list_enabled_connector_accounts_by_profile_id(
    //         state,
    //         profile.get_id(),
    //         &key_store,
    //         common_enums::ConnectorType::PaymentProcessor,
    //     )
    //     .await
    //     .change_context(errors::ApiErrorResponse::InternalServerError)
    //     .attach_printable("error when fetching merchant connector accounts")?;

    todo!()
}

/// Validate if payment methods list can be performed on the current status of payment intent
fn validate_payment_status(
    intent_status: common_enums::IntentStatus,
) -> Result<(), errors::ApiErrorResponse> {
    match intent_status {
        common_enums::IntentStatus::RequiresPaymentMethod => Ok(()),
        common_enums::IntentStatus::Succeeded
        | common_enums::IntentStatus::Failed
        | common_enums::IntentStatus::Cancelled
        | common_enums::IntentStatus::Processing
        | common_enums::IntentStatus::RequiresCustomerAction
        | common_enums::IntentStatus::RequiresMerchantAction
        | common_enums::IntentStatus::RequiresCapture
        | common_enums::IntentStatus::PartiallyCaptured
        | common_enums::IntentStatus::RequiresConfirmation
        | common_enums::IntentStatus::PartiallyCapturedAndCapturable => {
            Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                current_flow: "list_payment_methods".to_string(),
                field_name: "status".to_string(),
                current_value: intent_status.to_string(),
                states: ["requires_payment_method".to_string()].join(", "),
            })
        }
    }
}
