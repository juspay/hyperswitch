use error_stack::ResultExt;
use masking::PeekInterface;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    routes::SessionState,
    services::ApplicationResponse,
    types::{api, domain, storage, transformers::ForeignTryFrom},
    utils::{OptionExt, StringExt},
};

const INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_LIMIT: i64 = 100;

#[derive(Debug)]
enum MerchantAccountOrBusinessProfile {
    MerchantAccount(domain::MerchantAccount),
    BusinessProfile(domain::BusinessProfile),
}

#[instrument(skip(state))]
pub async fn list_initial_delivery_attempts(
    state: SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    constraints: api::webhook_events::EventListConstraints,
) -> RouterResponse<Vec<api::webhook_events::EventListItemResponse>> {
    let profile_id = constraints.profile_id.clone();
    let constraints =
        api::webhook_events::EventListConstraintsInternal::foreign_try_from(constraints)?;

    let store = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let (account, key_store) =
        get_account_and_key_store(state.clone(), merchant_id, profile_id).await?;

    let events = match constraints {
        api_models::webhook_events::EventListConstraintsInternal::ObjectIdFilter { object_id } => {
            match account {
                MerchantAccountOrBusinessProfile::MerchantAccount(merchant_account) => store
                .list_initial_events_by_merchant_id_primary_object_id(key_manager_state,
                   merchant_account.get_id(),
                    &object_id,
                    &key_store,
                )
                .await,
                MerchantAccountOrBusinessProfile::BusinessProfile(business_profile) => store
                .list_initial_events_by_profile_id_primary_object_id(key_manager_state,
                    &business_profile.profile_id,
                    &object_id,
                    &key_store,
                )
                .await,
            }
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

            match account {
                MerchantAccountOrBusinessProfile::MerchantAccount(merchant_account) => store
                .list_initial_events_by_merchant_id_constraints(key_manager_state,
                   merchant_account.get_id(),
                    created_after,
                    created_before,
                    limit,
                    offset,
                    &key_store,
                )
                .await,
                MerchantAccountOrBusinessProfile::BusinessProfile(business_profile) => store
                .list_initial_events_by_profile_id_constraints(key_manager_state,
                    &business_profile.profile_id,
                    created_after,
                    created_before,
                    limit,
                    offset,
                    &key_store,
                )
                .await,
            }
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
    state: SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    initial_attempt_id: String,
) -> RouterResponse<Vec<api::webhook_events::EventRetrieveResponse>> {
    let store = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let key_store = store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let events = store
        .list_events_by_merchant_id_initial_attempt_id(
            key_manager_state,
            &merchant_id,
            &initial_attempt_id,
            &key_store,
        )
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

#[instrument(skip(state))]
pub async fn retry_delivery_attempt(
    state: SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    event_id: String,
) -> RouterResponse<api::webhook_events::EventRetrieveResponse> {
    let store = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let key_store = store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let event_to_retry = store
        .find_event_by_merchant_id_event_id(
            key_manager_state,
            &key_store.merchant_id,
            &event_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::EventNotFound)?;

    let business_profile_id = event_to_retry
        .business_profile_id
        .get_required_value("business_profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to read business profile ID from event to retry")?;
    let business_profile = store
        .find_business_profile_by_profile_id(key_manager_state, &key_store, &business_profile_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to find business profile")?;

    let delivery_attempt = storage::enums::WebhookDeliveryAttempt::ManualRetry;
    let new_event_id = super::utils::generate_event_id();
    let idempotent_event_id = super::utils::get_idempotent_event_id(
        &event_to_retry.primary_object_id,
        event_to_retry.event_type,
        delivery_attempt,
    );

    let now = common_utils::date_time::now();
    let new_event = domain::Event {
        event_id: new_event_id.clone(),
        event_type: event_to_retry.event_type,
        event_class: event_to_retry.event_class,
        is_webhook_notified: false,
        primary_object_id: event_to_retry.primary_object_id,
        primary_object_type: event_to_retry.primary_object_type,
        created_at: now,
        merchant_id: Some(business_profile.merchant_id.clone()),
        business_profile_id: Some(business_profile.profile_id.clone()),
        primary_object_created_at: event_to_retry.primary_object_created_at,
        idempotent_event_id: Some(idempotent_event_id),
        initial_attempt_id: event_to_retry.initial_attempt_id,
        request: event_to_retry.request,
        response: None,
        delivery_attempt: Some(delivery_attempt),
        metadata: event_to_retry.metadata,
    };

    let event = store
        .insert_event(key_manager_state, new_event, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert event")?;

    // We only allow retrying deliveries for events with `request` populated.
    let request_content = event
        .request
        .as_ref()
        .get_required_value("request")
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .peek()
        .parse_struct("OutgoingWebhookRequestContent")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse webhook event request information")?;

    Box::pin(super::outgoing::trigger_webhook_and_raise_event(
        state.clone(),
        business_profile,
        &key_store,
        event,
        request_content,
        delivery_attempt,
        None,
        None,
    ))
    .await;

    let updated_event = store
        .find_event_by_merchant_id_event_id(
            key_manager_state,
            &key_store.merchant_id,
            &new_event_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::EventNotFound)?;

    Ok(ApplicationResponse::Json(
        api::webhook_events::EventRetrieveResponse::try_from(updated_event)?,
    ))
}

async fn get_account_and_key_store(
    state: SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    profile_id: Option<String>,
) -> errors::RouterResult<(MerchantAccountOrBusinessProfile, domain::MerchantKeyStore)> {
    let store = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let merchant_key_store = store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    match profile_id {
        // If profile ID is specified, return business profile, since a business profile is more
        // specific than a merchant account.
        Some(profile_id) => {
            let business_profile = store
                .find_business_profile_by_merchant_id_profile_id(
                    key_manager_state,
                    &merchant_key_store,
                    &merchant_id,
                    &profile_id,
                )
                .await
                .attach_printable_lazy(|| {
                    format!(
                        "Failed to find business profile by merchant_id `{merchant_id:?}` and profile_id `{profile_id}`. \
                        The merchant_id associated with the business profile `{profile_id}` may be \
                        different than the merchant_id specified (`{merchant_id:?}`)."
                    )
                })
                .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                    id: profile_id,
                })?;

            Ok((
                MerchantAccountOrBusinessProfile::BusinessProfile(business_profile),
                merchant_key_store,
            ))
        }

        None => {
            let merchant_account = store
                .find_merchant_account_by_merchant_id(
                    key_manager_state,
                    &merchant_id,
                    &merchant_key_store,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

            Ok((
                MerchantAccountOrBusinessProfile::MerchantAccount(merchant_account),
                merchant_key_store,
            ))
        }
    }
}
