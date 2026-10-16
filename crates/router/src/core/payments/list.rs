#[cfg(feature = "v2")]
use api_models::payments as payments_api;
#[cfg(all(feature = "v2", feature = "olap"))]
use common_utils::consts::LIST_MAX_LIMIT;
#[cfg(feature = "v1")]
use common_utils::id_type;
#[cfg(feature = "v1")]
use error_stack::report;
use error_stack::ResultExt;
use futures::future::join_all;
#[cfg(all(feature = "v2", feature = "olap"))]
use hyperswitch_domain_models::payments::payment_intent::PaymentIntentFetchConstraints;

#[cfg(feature = "v2")]
use crate::core::revenue_recovery::{
    candidate_intent_statuses_for_recovery_status, get_workflow_entries,
    map_to_recovery_payment_item,
};
use crate::{
    core::errors::{self, RouterResponse, RouterResult, StorageErrorExt},
    db::StorageInterface,
    routes::{metrics, SessionState},
    services,
    types::{api, domain, storage, transformers::ForeignFrom},
};
#[cfg(feature = "v1")]
use crate::{core::payments::helpers, logger, types::storage::enums as storage_enums};

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn list_payments(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<api::PaymentListResponse> {
    let processor_merchant_id = platform.get_processor().get_account().get_id();
    let db = state.store.as_ref();
    let payment_intents = helpers::filter_by_constraints(
        &state,
        &(constraints, profile_id_list).try_into()?,
        processor_merchant_id,
        platform.get_processor().get_key_store(),
        platform.get_processor().get_account().storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let collected_futures = payment_intents.into_iter().map(|pi| {
        async {
            match db
                .find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
                    &pi.payment_id,
                    processor_merchant_id,
                    &pi.active_attempt.get_id(),
                    // since OLAP doesn't have KV. Force to get the data from PSQL.
                    storage_enums::MerchantStorageScheme::PostgresOnly,
                    platform.get_processor().get_key_store(),
                )
                .await
            {
                Ok(pa) => Some(Ok((pi, pa))),
                Err(error) => {
                    if matches!(
                        error.current_context(),
                        errors::StorageError::ValueNotFound(_)
                    ) {
                        logger::warn!(
                            ?error,
                            "payment_attempts missing for payment_id : {:?}",
                            pi.payment_id,
                        );
                        return None;
                    }
                    Some(Err(error))
                }
            }
        }
    });

    //If any of the response are Err, we will get Result<Err(_)>
    let pi_pa_tuple_vec: Result<Vec<(storage::PaymentIntent, storage::PaymentAttempt)>, _> =
        join_all(collected_futures)
            .await
            .into_iter()
            .flatten() //Will ignore `None`, will only flatten 1 level
            .collect::<Result<Vec<(storage::PaymentIntent, storage::PaymentAttempt)>, _>>();
    //Will collect responses in same order async, leading to sorted responses

    //Converting Intent-Attempt array to Response if no error
    let data: Vec<api::PaymentsResponse> = pi_pa_tuple_vec
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .into_iter()
        .map(ForeignFrom::foreign_from)
        .collect();

    Ok(services::ApplicationResponse::Json(
        api::PaymentListResponse {
            size: data.len(),
            data,
        },
    ))
}

/// Lists payments across all connected merchants under a platform merchant.
///
/// Returns a slim, non-PII summary built from raw diesel rows, so no per-merchant key store
/// is fetched. Use the single-payment retrieve for full PII.
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn list_payments_for_platform(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    constraints: api::PlatformPaymentListConstraints,
) -> RouterResponse<api::PlatformPaymentListResponse> {
    common_utils::metrics::utils::record_operation_time(
        async {
            // This endpoint is exclusively for platform merchants - not connected, not standard.
            common_utils::fp_utils::when(
                !platform.get_provider().get_account().is_platform_account(),
                || {
                    Err(report!(errors::ApiErrorResponse::Unauthorized)).attach_printable(
                        "Platform payments list is only accessible to platform merchants",
                    )
                },
            )?;

            // `limit` is a `PageSize`, already validated at deserialize; no extra check needed.
            let platform_merchant_id = platform.get_provider().get_account().get_id();
            let db: &dyn StorageInterface = state.store.as_ref();

            let pi_fetch_constraints = (constraints, profile_id_list).try_into()?;

            let raw_rows = db
                .get_filtered_payment_intents_attempt_for_platform(
                    platform_merchant_id,
                    &pi_fetch_constraints,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

            let total_count = db
                .get_payment_intents_attempt_count_for_platform(
                    platform_merchant_id,
                    &pi_fetch_constraints,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

            let data: Vec<api::PlatformPaymentListItem> = raw_rows
                .into_iter()
                .map(ForeignFrom::foreign_from)
                .collect();

            Ok(services::ApplicationResponse::Json(
                api::PlatformPaymentListResponse {
                    count: data.len(),
                    total_count,
                    data,
                },
            ))
        },
        &metrics::PAYMENT_LIST_LATENCY,
        router_env::metric_attributes!((
            "merchant_id",
            platform.get_provider().get_account().get_id().clone()
        )),
    )
    .await
}

#[cfg(all(feature = "v2", feature = "olap"))]
pub async fn list_payments(
    state: SessionState,
    platform: domain::Platform,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<payments_api::PaymentListResponse> {
    common_utils::metrics::utils::record_operation_time(
        async {
            let db: &dyn StorageInterface = state.store.as_ref();
            let fetch_constraints = constraints.clone().into();
            let list: Vec<(storage::PaymentIntent, Option<storage::PaymentAttempt>)> = db
                .get_filtered_payment_intents_attempt(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let data: Vec<api_models::payments::PaymentsListResponseItem> =
                list.into_iter().map(ForeignFrom::foreign_from).collect();

            let active_attempt_ids = db
                .get_filtered_active_attempt_ids_for_total_count(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while retrieving active_attempt_ids for merchant")?;

            let total_count = if constraints.has_no_attempt_filters() {
                i64::try_from(active_attempt_ids.len())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while converting from usize to i64")
            } else {
                let active_attempt_ids = active_attempt_ids
                    .into_iter()
                    .flatten()
                    .collect::<Vec<String>>();

                db.get_total_count_of_filtered_payment_attempts(
                    platform.get_processor().get_account().get_id(),
                    &active_attempt_ids,
                    constraints.connector,
                    constraints.payment_method_type,
                    constraints.payment_method_subtype,
                    constraints.authentication_type,
                    constraints.merchant_connector_id,
                    constraints.card_network,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while retrieving total count of payment attempts")
            }?;

            Ok(services::ApplicationResponse::Json(
                api_models::payments::PaymentListResponse {
                    count: data.len(),
                    total_count,
                    data,
                },
            ))
        },
        &metrics::PAYMENT_LIST_LATENCY,
        router_env::metric_attributes!((
            "merchant_id",
            platform.get_processor().get_account().get_id().clone()
        )),
    )
    .await
}

/// Turns `starting_after` / `ending_before` into `created_gte` / `created_lte` bounds, so the page
/// and count queries filter on the same time range. Explicit time bounds take precedence.
#[cfg(all(feature = "v2", feature = "olap"))]
async fn resolve_pagination_cursors(
    state: &SessionState,
    platform: &domain::Platform,
    constraints: &mut payments_api::PaymentListConstraints,
) -> RouterResult<()> {
    let db: &dyn StorageInterface = state.store.as_ref();
    let has_lower_bound = constraints
        .created_gte
        .or(constraints.created_gt)
        .or(constraints.created)
        .is_some();
    let has_upper_bound = constraints
        .created_lte
        .or(constraints.created_lt)
        .or(constraints.created)
        .is_some();

    if let (false, Some(cursor)) = (has_lower_bound, constraints.starting_after.take()) {
        constraints.created_gte = Some(find_intent_created_at(db, platform, &cursor).await?);
    }
    if let (false, Some(cursor)) = (has_upper_bound, constraints.ending_before.take()) {
        constraints.created_lte = Some(find_intent_created_at(db, platform, &cursor).await?);
    }
    Ok(())
}

#[cfg(all(feature = "v2", feature = "olap"))]
async fn find_intent_created_at(
    db: &dyn StorageInterface,
    platform: &domain::Platform,
    payment_id: &common_utils::id_type::GlobalPaymentId,
) -> RouterResult<time::PrimitiveDateTime> {
    db.find_payment_intent_by_id(
        payment_id,
        platform.get_processor().get_key_store(),
        platform.get_processor().get_account().storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
    .map(|payment_intent| payment_intent.created_at)
}

/// Maps intents to recovery list items using their workflow entries and billing retry threshold.
#[cfg(all(feature = "v2", feature = "olap"))]
async fn build_recovery_list_items(
    state: &SessionState,
    platform: &domain::Platform,
    list: Vec<(storage::PaymentIntent, Option<storage::PaymentAttempt>)>,
) -> Vec<payments_api::RecoveryPaymentsListResponseItem> {
    let db: &dyn StorageInterface = state.store.as_ref();

    let billing_connector_ids: Vec<_> = list
        .iter()
        .map(|(payment_intent, _)| payment_intent.get_billing_merchant_connector_account_id())
        .collect();

    let workflow_futures: Vec<_> = list
        .iter()
        .map(|(payment_intent, _)| get_workflow_entries(state, &payment_intent.id))
        .collect();

    let billing_connector_futures: Vec<_> = billing_connector_ids
        .into_iter()
        .map(|billing_mca_id| {
            let platform_clone = platform.clone(); // Clone for each future
            async move {
                if let Some(billing_mca_id) = billing_mca_id {
                    db.find_merchant_connector_account_by_id(
                        &billing_mca_id,
                        platform_clone.get_processor().get_key_store(),
                    )
                    .await
                    .ok()
                } else {
                    None
                }
            }
        })
        .collect();

    let workflow_results = join_all(workflow_futures).await;
    let billing_connector_results = join_all(billing_connector_futures).await;

    list.into_iter()
        .zip(workflow_results)
        .zip(billing_connector_results)
        .map(
            |(((payment_intent, payment_attempt), workflow_result), billing_connector_account)| {
                let (calculate_workflow, execute_workflow) =
                    workflow_result.unwrap_or((None, None));

                let max_retry_threshold = billing_connector_account
                    .as_ref()
                    .and_then(|mca| mca.get_retry_threshold())
                    .unwrap_or(0); // Default fallback

                map_to_recovery_payment_item(
                    payment_intent,
                    payment_attempt,
                    calculate_workflow,
                    execute_workflow,
                    max_retry_threshold.try_into().unwrap_or(0),
                )
            },
        )
        .collect()
}

#[cfg(all(feature = "v2", feature = "olap"))]
pub async fn revenue_recovery_list_payments(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    constraints: payments_api::RecoveryPaymentListConstraints,
) -> RouterResponse<payments_api::RecoveryPaymentListResponse> {
    let (mut constraints, filters) = constraints.into_parts();
    let payments_api::RecoveryListFilters {
        recovery_status,
        search,
    } = filters;
    // The auth profile overrides a caller supplied `profile_id`.
    if profile_id.is_some() {
        constraints.profile_id = profile_id;
    }
    resolve_pagination_cursors(&state, &platform, &mut constraints).await?;
    common_utils::metrics::utils::record_operation_time(
        async {
            if let Some(recovery_status) = recovery_status {
                return list_recovery_payments_by_recovery_status(
                    &state,
                    &platform,
                    constraints,
                    search,
                    &recovery_status,
                )
                .await;
            }

            // `limit` is a `PageSize`, already validated at deserialize; no extra check needed.
            let db: &dyn StorageInterface = state.store.as_ref();
            let fetch_constraints =
                PaymentIntentFetchConstraints::from(constraints.clone()).with_search(search);
            let list: Vec<(storage::PaymentIntent, Option<storage::PaymentAttempt>)> = db
                .get_filtered_payment_intents_attempt(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

            let data = build_recovery_list_items(&state, &platform, list).await;

            let active_attempt_ids = db
                .get_filtered_active_attempt_ids_for_total_count(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while retrieving active_attempt_ids for merchant")?;

            let total_count = if constraints.has_no_attempt_filters() {
                i64::try_from(active_attempt_ids.len())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while converting from usize to i64")
            } else {
                let active_attempt_ids = active_attempt_ids
                    .into_iter()
                    .flatten()
                    .collect::<Vec<String>>();

                db.get_total_count_of_filtered_payment_attempts(
                    platform.get_processor().get_account().get_id(),
                    &active_attempt_ids,
                    constraints.connector,
                    constraints.payment_method_type,
                    constraints.payment_method_subtype,
                    constraints.authentication_type,
                    constraints.merchant_connector_id,
                    constraints.card_network,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while retrieving total count of payment attempts")
            }?;

            Ok(services::ApplicationResponse::Json(
                payments_api::RecoveryPaymentListResponse {
                    count: data.len(),
                    total_count,
                    data,
                },
            ))
        },
        &metrics::PAYMENT_LIST_LATENCY,
        router_env::metric_attributes!((
            "merchant_id",
            platform.get_processor().get_account().get_id().clone()
        )),
    )
    .await
}

/// Lists recovery invoices filtered by the derived recovery status.
///
/// The status is derived per row, so intents are narrowed by candidate intent status in the
/// database, then filtered, then paginated. `total_count` is the filtered size.
#[cfg(all(feature = "v2", feature = "olap"))]
async fn list_recovery_payments_by_recovery_status(
    state: &SessionState,
    platform: &domain::Platform,
    constraints: payments_api::PaymentListConstraints,
    search: Option<String>,
    recovery_status: &[common_enums::enums::RecoveryStatus],
) -> RouterResult<services::ApplicationResponse<payments_api::RecoveryPaymentListResponse>> {
    let db: &dyn StorageInterface = state.store.as_ref();
    let requested_limit = constraints.limit.as_usize();
    let requested_offset = constraints.offset.as_usize();

    let candidate_statuses = candidate_intent_statuses_for_recovery_status(
        recovery_status,
        constraints.status.as_deref(),
    );
    if candidate_statuses.is_empty() {
        return Ok(services::ApplicationResponse::Json(
            payments_api::RecoveryPaymentListResponse {
                count: 0,
                total_count: 0,
                data: Vec::new(),
            },
        ));
    }

    let scan_page_size = common_utils::types::list::PageSize::from(LIST_MAX_LIMIT);
    let scan_constraints = payments_api::PaymentListConstraints {
        status: Some(candidate_statuses),
        limit: scan_page_size,
        ..constraints
    };

    let mut matched: Vec<payments_api::RecoveryPaymentsListResponseItem> = Vec::new();
    let mut scan_offset: usize = 0;
    loop {
        let page_offset = common_utils::types::list::PageOffset::from(
            u32::try_from(scan_offset).unwrap_or(u32::MAX),
        );
        // `PageOffset` clamps at its maximum; stop rather than re-read the same page.
        if page_offset.as_usize() != scan_offset {
            break;
        }
        let page_constraints = payments_api::PaymentListConstraints {
            offset: page_offset,
            ..scan_constraints.clone()
        };
        let fetch_constraints =
            PaymentIntentFetchConstraints::from(page_constraints).with_search(search.clone());
        let list: Vec<(storage::PaymentIntent, Option<storage::PaymentAttempt>)> = db
            .get_filtered_payment_intents_attempt(
                platform.get_processor().get_account().get_id(),
                &fetch_constraints,
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let fetched = list.len();
        let items = build_recovery_list_items(state, platform, list).await;
        matched.extend(
            items
                .into_iter()
                .filter(|item| recovery_status.contains(&item.status)),
        );

        if fetched < scan_page_size.as_usize() {
            break;
        }
        scan_offset += fetched;
    }

    let total_count = i64::try_from(matched.len())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while converting from usize to i64")?;
    let data: Vec<_> = matched
        .into_iter()
        .skip(requested_offset)
        .take(requested_limit)
        .collect();

    Ok(services::ApplicationResponse::Json(
        payments_api::RecoveryPaymentListResponse {
            count: data.len(),
            total_count,
            data,
        },
    ))
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn apply_filters_on_payments(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    constraints: api::PaymentListFilterConstraints,
) -> RouterResponse<api::PaymentListResponseV2> {
    common_utils::metrics::utils::record_operation_time(
        async {
            let db: &dyn StorageInterface = state.store.as_ref();
            let fetch_constraints = (constraints.clone(), profile_id_list).try_into()?;
            let list: Vec<(storage::PaymentIntent, storage::PaymentAttempt)> = db
                .get_filtered_payment_intents_attempt(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let data: Vec<api::PaymentsResponse> =
                list.into_iter().map(ForeignFrom::foreign_from).collect();

            let active_attempt_ids = db
                .get_filtered_active_attempt_ids_for_total_count(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

            let total_count = if constraints.has_no_attempt_filters() {
                i64::try_from(active_attempt_ids.len())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while converting from usize to i64")
            } else {
                db.get_total_count_of_filtered_payment_attempts(
                    platform.get_processor().get_account().get_id(),
                    &active_attempt_ids,
                    constraints.connector,
                    constraints.payment_method,
                    constraints.payment_method_type,
                    constraints.authentication_type,
                    constraints.merchant_connector_id,
                    constraints.card_network,
                    constraints.card_discovery,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
            }?;

            Ok(services::ApplicationResponse::Json(
                api::PaymentListResponseV2 {
                    count: data.len(),
                    total_count,
                    data,
                },
            ))
        },
        &metrics::PAYMENT_LIST_LATENCY,
        router_env::metric_attributes!((
            "merchant_id",
            platform.get_processor().get_account().get_id().clone()
        )),
    )
    .await
}
