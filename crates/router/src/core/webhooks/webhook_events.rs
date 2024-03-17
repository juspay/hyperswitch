use error_stack::ResultExt;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    routes::AppState,
    services::ApplicationResponse,
    types::api,
};

#[instrument(skip(state))]
pub async fn list_delivery_attempts(
    state: AppState,
    merchant_id: &str,
    initial_attempt_id: &str,
) -> RouterResponse<Vec<api::webhook_events::EventRetrieveResponse>> {
    let store = state.store.as_ref();

    // This would handle verifying that the merchant ID actually exists
    let key_store = store
        .get_merchant_key_store_by_merchant_id(merchant_id, &store.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let events = store
        .list_events_by_merchant_id_initial_attempt_id(merchant_id, initial_attempt_id, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to list delivery attempts for initial event")?;

    if events.is_empty() {
        Err(error_stack::report!(
            errors::ApiErrorResponse::EventNotFound
        ))
        .attach_printable("No delivery attempts found with the specified `initial_attempt_id`")
    } else {
        Ok(ApplicationResponse::Json(
            events
                .into_iter()
                .map(api::webhook_events::EventRetrieveResponse::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
