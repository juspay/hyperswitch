use error_stack::ResultExt;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    routes::AppState,
    services::ApplicationResponse,
    types::{api, transformers::ForeignTryFrom},
};

const INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_LIMIT: i64 = 100;

#[instrument(skip(state))]
pub async fn list_initial_delivery_attempts(
    state: AppState,
    merchant_id: String,
    constraints: api::webhook_events::EventListConstraints,
) -> RouterResponse<Vec<api::webhook_events::EventListItemResponse>> {
    let constraints =
        api::webhook_events::EventListConstraintsInternal::foreign_try_from(constraints)?;

    let store = state.store.as_ref();

    // This would handle verifying that the merchant ID actually exists
    let key_store = store
        .get_merchant_key_store_by_merchant_id(
            &merchant_id,
            &store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let events = match constraints {
        api_models::webhook_events::EventListConstraintsInternal::ObjectIdFilter { object_id } => {
            store
                .list_initial_events_by_merchant_id_primary_object_id(
                    &merchant_id,
                    &object_id,
                    &key_store,
                )
                .await
        }
        api_models::webhook_events::EventListConstraintsInternal::GenericFilter {
            created_after,
            created_before,
            limit,
            offset,
        } => {
            let limit = match limit {
                Some(limit) if  limit <= INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_LIMIT => Ok(Some(limit)),
                Some(limit) if limit > INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_LIMIT => Err(
                    errors::ApiErrorResponse::InvalidRequestData{
                        message: format!("`limit` must be a number less than {INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_LIMIT}")
                    }
                ),
                _  => Ok(Some(INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_LIMIT)),
            }?;
            let offset = match offset {
                Some(offset) if offset > 0 => Some(offset),
                _ => None,
            };

            store
                .list_initial_events_by_merchant_id_constraints(
                    &merchant_id,
                    created_after,
                    created_before,
                    limit,
                    offset,
                    &key_store,
                )
                .await
        }
    }
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to list events with specified constraints")?;

    Ok(ApplicationResponse::Json(
        events
            .into_iter()
            .map(api::webhook_events::EventListItemResponse::try_from)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

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
