use error_stack::ResultExt;
use masking::PeekInterface;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    routes::AppState,
    services::ApplicationResponse,
    types::{api, domain, storage, transformers::ForeignTryFrom},
    utils::{OptionExt, StringExt},
};

const INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_LIMIT: i64 = 100;

#[derive(Debug)]
enum MerchantAccountOrBusinessProfile {
    MerchantAccount(domain::MerchantAccount),
    BusinessProfile(storage::BusinessProfile),
}

#[instrument(skip(state))]
pub async fn list_initial_delivery_attempts(
    state: AppState,
    merchant_id_or_profile_id: String,
    constraints: api::webhook_events::EventListConstraints,
) -> RouterResponse<Vec<api::webhook_events::EventListItemResponse>> {
    let constraints =
        api::webhook_events::EventListConstraintsInternal::foreign_try_from(constraints)?;

    let store = state.store.as_ref();

    let (account, key_store) =
        determine_identifier_and_get_key_store(state.clone(), merchant_id_or_profile_id).await?;

    let events = match constraints {
        api_models::webhook_events::EventListConstraintsInternal::ObjectIdFilter { object_id } => {
            match account {
                MerchantAccountOrBusinessProfile::MerchantAccount(merchant_account) => store
                .list_initial_events_by_merchant_id_primary_object_id(
                    &merchant_account.merchant_id,
                    &object_id,
                    &key_store,
                )
                .await,
                MerchantAccountOrBusinessProfile::BusinessProfile(business_profile) => store
                .list_initial_events_by_profile_id_primary_object_id(
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
                .list_initial_events_by_merchant_id_constraints(
                    &merchant_account.merchant_id,
                    created_after,
                    created_before,
                    limit,
                    offset,
                    &key_store,
                )
                .await,
                MerchantAccountOrBusinessProfile::BusinessProfile(business_profile) => store
                .list_initial_events_by_profile_id_constraints(
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
    state: AppState,
    merchant_id_or_profile_id: String,
    initial_attempt_id: String,
) -> RouterResponse<Vec<api::webhook_events::EventRetrieveResponse>> {
    let store = state.store.as_ref();

    let (account, key_store) =
        determine_identifier_and_get_key_store(state.clone(), merchant_id_or_profile_id).await?;

    let events = match account {
        MerchantAccountOrBusinessProfile::MerchantAccount(merchant_account) => {
            store
                .list_events_by_merchant_id_initial_attempt_id(
                    &merchant_account.merchant_id,
                    &initial_attempt_id,
                    &key_store,
                )
                .await
        }
        MerchantAccountOrBusinessProfile::BusinessProfile(business_profile) => {
            store
                .list_events_by_profile_id_initial_attempt_id(
                    &business_profile.profile_id,
                    &initial_attempt_id,
                    &key_store,
                )
                .await
        }
    }
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
    state: AppState,
    merchant_id_or_profile_id: String,
    event_id: String,
) -> RouterResponse<api::webhook_events::EventRetrieveResponse> {
    let store = state.store.as_ref();

    let (account, key_store) =
        determine_identifier_and_get_key_store(state.clone(), merchant_id_or_profile_id).await?;

    let event_to_retry = store
        .find_event_by_merchant_id_event_id(&key_store.merchant_id, &event_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::EventNotFound)?;

    let business_profile = match account {
        MerchantAccountOrBusinessProfile::MerchantAccount(_) => {
            let business_profile_id = event_to_retry
                .business_profile_id
                .get_required_value("business_profile_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to read business profile ID from event to retry")?;
            store
                .find_business_profile_by_profile_id(&business_profile_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to find business profile")
        }
        MerchantAccountOrBusinessProfile::BusinessProfile(business_profile) => Ok(business_profile),
    }?;

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
    };

    let event = store
        .insert_event(new_event, &key_store)
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

    super::trigger_webhook_and_raise_event(
        state.clone(),
        business_profile,
        &key_store,
        event,
        request_content,
        delivery_attempt,
        None,
        None,
    )
    .await;

    let updated_event = store
        .find_event_by_merchant_id_event_id(&key_store.merchant_id, &new_event_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::EventNotFound)?;

    Ok(ApplicationResponse::Json(
        api::webhook_events::EventRetrieveResponse::try_from(updated_event)?,
    ))
}

async fn determine_identifier_and_get_key_store(
    state: AppState,
    merchant_id_or_profile_id: String,
) -> errors::RouterResult<(MerchantAccountOrBusinessProfile, domain::MerchantKeyStore)> {
    let store = state.store.as_ref();
    match store
        .get_merchant_key_store_by_merchant_id(
            &merchant_id_or_profile_id,
            &store.get_master_key().to_vec().into(),
        )
        .await
    {
        // Valid merchant ID
        Ok(key_store) => {
            let merchant_account = store
                .find_merchant_account_by_merchant_id(&merchant_id_or_profile_id, &key_store)
                .await
                .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

            Ok((
                MerchantAccountOrBusinessProfile::MerchantAccount(merchant_account),
                key_store,
            ))
        }

        // Invalid merchant ID, check if we can find a business profile with the identifier
        Err(error) if error.current_context().is_db_not_found() => {
            router_env::logger::debug!(
                ?error,
                %merchant_id_or_profile_id,
                "Failed to find merchant key store for the specified merchant ID or business profile ID"
            );

            let business_profile = store
                .find_business_profile_by_profile_id(&merchant_id_or_profile_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                    id: merchant_id_or_profile_id,
                })?;

            let key_store = store
                .get_merchant_key_store_by_merchant_id(
                    &business_profile.merchant_id,
                    &store.get_master_key().to_vec().into(),
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

            Ok((
                MerchantAccountOrBusinessProfile::BusinessProfile(business_profile),
                key_store,
            ))
        }

        Err(error) => Err(error)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to find merchant key store by merchant ID"),
    }
}
