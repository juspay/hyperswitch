use std::collections::HashSet;

use common_utils::{self, errors::CustomResult, fp_utils};
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;
use router_env::{instrument, tracing};

use crate::{
    core::errors::{self, RouterResponse, StorageErrorExt},
    routes::SessionState,
    services::ApplicationResponse,
    types::{api, domain, storage, transformers::ForeignTryFrom},
    utils::{OptionExt, StringExt},
};

const INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_LIMIT: i64 = 100;
const INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_DAYS: i64 = 90;

#[instrument(skip(state))]
pub async fn list_initial_delivery_attempts(
    state: SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    api_constraints: api::webhook_events::EventListConstraints,
) -> RouterResponse<api::webhook_events::TotalEventsResponse> {
    let profile_id = api_constraints.profile_id.clone();
    let constraints = api::webhook_events::EventListConstraintsInternal::foreign_try_from(
        api_constraints.clone(),
    )?;

    let store = state.store.as_ref();
    let master_key = &store.get_master_key().to_vec().into();

    // The authenticated merchant is always the webhook recipient, so their
    // keystore is the correct one for decrypting any events that were sent
    // to them. Events are filtered by `initiator_merchant_id = merchant_id`
    // (with a fallback on `merchant_id` column for rows created before the
    // initiator column existed).
    let key_store = store
        .get_merchant_key_store_by_merchant_id(&merchant_id, master_key)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // Validate the profile_id if provided.
    if let Some(ref profile_id) = profile_id {
        store
            .find_business_profile_by_merchant_id_profile_id(&key_store, &merchant_id, profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;
    }

    let now = common_utils::date_time::now();
    let events_list_begin_time =
        (now.date() - time::Duration::days(INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_DAYS)).midnight();

    let (events, total_count) = match constraints {
        api_models::webhook_events::EventListConstraintsInternal::ObjectIdFilter { object_id } => {
            let events = store
                .list_initial_events_by_initiator_merchant_id_primary_object_id(
                    &merchant_id,
                    object_id.as_str(),
                    profile_id.clone(),
                    &key_store,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to list events with specified constraints")?;

            let total_count = i64::try_from(events.len())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while converting from usize to i64")?;
            (events, total_count)
        }
        api_models::webhook_events::EventListConstraintsInternal::EventIdFilter { event_id } => {
            let event_opt = match store
                .find_event_by_event_id(event_id.as_str(), &key_store)
                .await
            {
                Ok(event) => Some(event),
                Err(err) if err.current_context().is_db_not_found() => None,
                Err(err) => {
                    return Err(err)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to find event with specified event_id");
                }
            };

            // Scope the returned event to the calling merchant as the initiator and
            // (optionally) the provided profile_id. This prevents a merchant from
            // querying an event belonging to another merchant by guessing its event_id.
            let event_opt = event_opt.filter(|event| {
                let matches_initiator = event.initiator_merchant_id.as_ref() == Some(&merchant_id)
                    || (event.initiator_merchant_id.is_none()
                        && event.merchant_id.as_ref() == Some(&merchant_id));
                let matches_profile = profile_id
                    .as_ref()
                    .is_none_or(|pid| event.business_profile_id.as_ref() == Some(pid));
                matches_initiator
                    && matches_profile
                    && event.initial_attempt_id.as_deref() == Some(&event.event_id)
            });

            let (events, total_count) = event_opt.map_or((vec![], 0), |event| (vec![event], 1));
            (events, total_count)
        }
        api_models::webhook_events::EventListConstraintsInternal::GenericFilter {
            created_after,
            created_before,
            limit,
            offset,
            event_classes,
            event_types,
            is_delivered,
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

            let event_classes = event_classes.unwrap_or(HashSet::new());
            let mut event_types = event_types.unwrap_or(HashSet::new());
            if !event_classes.is_empty() {
                event_types = finalize_event_types(event_classes, event_types).await?;
            }

            fp_utils::when(
                !created_after
                    .zip(created_before)
                    .map(|(created_after, created_before)| created_after <= created_before)
                    .unwrap_or(true),
                || {
                    Err(errors::ApiErrorResponse::InvalidRequestData { message: "The `created_after` timestamp must be an earlier timestamp compared to the `created_before` timestamp".to_string() })
                },
            )?;

            let created_after = match created_after {
                Some(created_after) => {
                    if created_after < events_list_begin_time {
                        Err(errors::ApiErrorResponse::InvalidRequestData { message: format!("`created_after` must be a timestamp within the past {INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_DAYS} days.") })
                    } else {
                        Ok(created_after)
                    }
                }
                None => Ok(events_list_begin_time),
            }?;

            let created_before = match created_before {
                Some(created_before) => {
                    if created_before < events_list_begin_time {
                        Err(errors::ApiErrorResponse::InvalidRequestData { message: format!("`created_before` must be a timestamp within the past {INITIAL_DELIVERY_ATTEMPTS_LIST_MAX_DAYS} days.") })
                    } else {
                        Ok(created_before)
                    }
                }
                None => Ok(now),
            }?;

            let events = match profile_id.clone() {
                Some(profile_id) => {
                    store
                        .list_initial_events_by_profile_id_constraints(
                            &profile_id,
                            created_after,
                            created_before,
                            limit,
                            offset,
                            event_types.clone(),
                            is_delivered,
                            &key_store,
                        )
                        .await
                }
                None => {
                    store
                        .list_initial_events_by_initiator_merchant_id_constraints(
                            &merchant_id,
                            created_after,
                            created_before,
                            limit,
                            offset,
                            event_types.clone(),
                            is_delivered,
                            &key_store,
                        )
                        .await
                }
            }
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to list events with specified constraints")?;

            let total_count = match profile_id {
                Some(profile_id) => {
                    store
                        .count_initial_events_by_profile_id_constraints(
                            &profile_id,
                            created_after,
                            created_before,
                            event_types,
                            is_delivered,
                        )
                        .await
                }
                None => {
                    store
                        .count_initial_events_by_initiator_merchant_id_constraints(
                            &merchant_id,
                            None,
                            created_after,
                            created_before,
                            event_types,
                            is_delivered,
                        )
                        .await
                }
            }
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get total events count")?;

            (events, total_count)
        }
    };

    let events = events
        .into_iter()
        .map(api::webhook_events::EventListItemResponse::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ApplicationResponse::Json(
        api::webhook_events::TotalEventsResponse::new(total_count, events),
    ))
}

#[instrument(skip(state))]
pub async fn list_delivery_attempts(
    state: SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    initial_attempt_id: String,
) -> RouterResponse<Vec<api::webhook_events::EventRetrieveResponse>> {
    let store = state.store.as_ref();
    let master_key = &store.get_master_key().to_vec().into();

    // The calling merchant is the webhook recipient, so their keystore is the
    // one that was used to encrypt the event chain. Events in the chain are
    // scoped by the unique `initial_attempt_id`, so no merchant filter is
    // required in the query — but we verify ownership below to prevent a
    // merchant from reading another merchant's event chain.
    let key_store = store
        .get_merchant_key_store_by_merchant_id(&merchant_id, master_key)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let events = store
        .list_events_by_initiator_merchant_id_initial_attempt_id(
            &initial_attempt_id,
            &merchant_id,
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
#[cfg(feature = "v1")]
pub async fn retry_delivery_attempt(
    state: SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    event_id: String,
) -> RouterResponse<api::webhook_events::EventRetrieveResponse> {
    let store = state.store.as_ref();
    let master_key = &store.get_master_key().to_vec().into();

    // The calling merchant is the webhook recipient, so their keystore was used
    // to encrypt the original event. Use it for decryption.
    let key_store = store
        .get_merchant_key_store_by_merchant_id(&merchant_id, master_key)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    // Look up the event by its globally-unique event_id.
    let event_to_retry = store
        .find_event_by_event_id(&event_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::EventNotFound)?;

    // Ensure the calling merchant is the initiator (webhook recipient) of this
    // event. Fall back to `merchant_id` column for events created before the
    // `initiator_merchant_id` column existed.
    let caller_owns_event = event_to_retry.initiator_merchant_id.as_ref() == Some(&merchant_id)
        || (event_to_retry.initiator_merchant_id.is_none()
            && event_to_retry.merchant_id.as_ref() == Some(&merchant_id));
    if !caller_owns_event {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::EventNotFound
        ));
    }

    let provider_merchant_id = event_to_retry
        .merchant_id
        .clone()
        .get_required_value("merchant_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to read merchant ID from event to retry")?;
    let processor_merchant_id = event_to_retry
        .processor_merchant_id
        .clone()
        .unwrap_or_else(|| provider_merchant_id.clone());

    let business_profile_id = event_to_retry
        .business_profile_id
        .clone()
        .get_required_value("business_profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to read business profile ID from event to retry")?;
    let business_profile = store
        .find_business_profile_by_profile_id(&key_store, &business_profile_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to find business profile")?;

    let delivery_attempt = storage::enums::WebhookDeliveryAttempt::ManualRetry;
    let new_event_id = super::utils::generate_event_id();
    let idempotent_event_id = super::utils::get_idempotent_event_id(
        &event_to_retry.primary_object_id,
        event_to_retry.event_type,
        delivery_attempt,
    )
    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
    .attach_printable("Failed to generate idempotent event ID")?;

    let now = common_utils::date_time::now();
    let new_event = domain::Event {
        event_id: new_event_id.clone(),
        event_type: event_to_retry.event_type,
        event_class: event_to_retry.event_class,
        is_webhook_notified: false,
        primary_object_id: event_to_retry.primary_object_id,
        primary_object_type: event_to_retry.primary_object_type,
        created_at: now,
        merchant_id: Some(provider_merchant_id.clone()),
        business_profile_id: Some(business_profile.get_id().to_owned()),
        primary_object_created_at: event_to_retry.primary_object_created_at,
        idempotent_event_id: Some(idempotent_event_id),
        initial_attempt_id: event_to_retry.initial_attempt_id,
        request: event_to_retry.request,
        response: None,
        delivery_attempt: Some(delivery_attempt),
        metadata: event_to_retry.metadata,
        is_overall_delivery_successful: Some(false),
        processor_merchant_id: Some(processor_merchant_id.clone()),
        initiator_merchant_id: Some(merchant_id.clone()),
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

    Box::pin(super::outgoing::trigger_webhook_and_raise_event(
        state.clone(),
        business_profile,
        &key_store,
        provider_merchant_id,
        processor_merchant_id,
        event,
        request_content,
        delivery_attempt,
        None,
        None,
    ))
    .await;

    let updated_event = store
        .find_event_by_event_id(&new_event_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::EventNotFound)?;

    Ok(ApplicationResponse::Json(
        api::webhook_events::EventRetrieveResponse::try_from(updated_event)?,
    ))
}

async fn finalize_event_types(
    event_classes: HashSet<common_enums::EventClass>,
    mut event_types: HashSet<common_enums::EventType>,
) -> CustomResult<HashSet<common_enums::EventType>, errors::ApiErrorResponse> {
    // Examples:
    // 1. event_classes = ["payments", "refunds"], event_types = ["payment_succeeded"]
    // 2. event_classes = ["refunds"], event_types = ["payment_succeeded"]

    // Create possible_event_types based on event_classes
    // Example 1: possible_event_types = ["payment_*", "refund_*"]
    // Example 2: possible_event_types = ["refund_*"]
    let possible_event_types = event_classes
        .clone()
        .into_iter()
        .flat_map(common_enums::EventClass::event_types)
        .collect::<HashSet<_>>();

    if event_types.is_empty() {
        return Ok(possible_event_types);
    }

    // Extend event_types if disjoint with event_classes
    // Example 1: event_types = ["payment_succeeded", "refund_*"], is_disjoint is used to extend "refund_*" and ignore "payment_*".
    // Example 2: event_types = ["payment_succeeded", "refund_*"], is_disjoint is only used to extend "refund_*".
    event_classes.into_iter().for_each(|class| {
        let valid_event_types = class.event_types();
        if event_types.is_disjoint(&valid_event_types) {
            event_types.extend(valid_event_types);
        }
    });

    // Validate event_types is a subset of possible_event_types
    // Example 1: event_types is a subset of possible_event_types (valid)
    // Example 2: event_types is not a subset of possible_event_types (error due to "payment_succeeded")
    if !event_types.is_subset(&possible_event_types) {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "`event_types` must be a subset of `event_classes`".to_string(),
            }
        ));
    }

    Ok(event_types.clone())
}
