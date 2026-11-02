#[cfg(all(feature = "v2", feature = "olap"))]
use std::collections::{HashMap, HashSet};

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

/// Finds where a pagination cursor sits in the list. A cursor outside the caller's merchant or
/// profile is treated as not found.
#[cfg(all(feature = "v2", feature = "olap"))]
async fn find_recovery_list_cursor(
    db: &dyn StorageInterface,
    platform: &domain::Platform,
    profile_id: Option<&common_utils::id_type::ProfileId>,
    payment_id: &common_utils::id_type::GlobalPaymentId,
) -> RouterResult<(
    time::PrimitiveDateTime,
    common_utils::id_type::GlobalPaymentId,
)> {
    let payment_intent = db
        .find_payment_intent_by_id(
            payment_id,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let in_scope = &payment_intent.merchant_id == platform.get_processor().get_account().get_id()
        && profile_id.is_none_or(|profile_id| &payment_intent.profile_id == profile_id);
    if !in_scope {
        return Err(errors::ApiErrorResponse::PaymentNotFound.into());
    }
    Ok((payment_intent.created_at, payment_intent.id))
}

/// Maps intents to recovery list items using their workflow entries and billing retry threshold.
/// `retry_thresholds` keeps each billing connector's threshold, so it is fetched only once.
#[cfg(all(feature = "v2", feature = "olap"))]
async fn build_recovery_list_items(
    state: &SessionState,
    platform: &domain::Platform,
    list: Vec<(storage::PaymentIntent, Option<storage::PaymentAttempt>)>,
    retry_thresholds: &mut HashMap<common_utils::id_type::MerchantConnectorAccountId, u16>,
) -> Vec<payments_api::RecoveryPaymentsListResponseItem> {
    let db: &dyn StorageInterface = state.store.as_ref();

    let workflow_futures: Vec<_> = list
        .iter()
        .map(|(payment_intent, _)| get_workflow_entries(state, &payment_intent.id))
        .collect();

    let missing_billing_connector_ids: HashSet<_> = list
        .iter()
        .filter_map(|(payment_intent, _)| {
            payment_intent.get_billing_merchant_connector_account_id()
        })
        .filter(|billing_mca_id| !retry_thresholds.contains_key(billing_mca_id))
        .collect();

    let billing_connector_futures: Vec<_> = missing_billing_connector_ids
        .into_iter()
        .map(|billing_mca_id| async move {
            let retry_threshold = db
                .find_merchant_connector_account_by_id(
                    &billing_mca_id,
                    platform.get_processor().get_key_store(),
                )
                .await
                .ok()
                .and_then(|mca| mca.get_retry_threshold())
                .unwrap_or(0); // Default fallback
            (billing_mca_id, retry_threshold)
        })
        .collect();

    let workflow_results = join_all(workflow_futures).await;
    retry_thresholds.extend(join_all(billing_connector_futures).await);

    list.into_iter()
        .zip(workflow_results)
        .map(|((payment_intent, payment_attempt), workflow_result)| {
            let (calculate_workflow, execute_workflow) = workflow_result.unwrap_or((None, None));

            let max_retry_threshold = payment_intent
                .get_billing_merchant_connector_account_id()
                .and_then(|billing_mca_id| retry_thresholds.get(&billing_mca_id).copied())
                .unwrap_or(0); // Default fallback

            map_to_recovery_payment_item(
                payment_intent,
                payment_attempt,
                calculate_workflow,
                execute_workflow,
                max_retry_threshold.try_into().unwrap_or(0),
            )
        })
        .collect()
}

#[cfg(all(feature = "v2", feature = "olap"))]
pub async fn revenue_recovery_list_payments(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    constraints: payments_api::RecoveryPaymentListConstraints,
) -> RouterResponse<payments_api::RecoveryPaymentListResponse> {
    let (mut constraints, recovery_status) = constraints.into_parts();
    // The auth profile overrides a caller supplied `profile_id`.
    if profile_id.is_some() {
        constraints.profile_id = profile_id;
    }
    common_utils::metrics::utils::record_operation_time(
        list_recovery_page(&state, &platform, constraints, recovery_status),
        &metrics::PAYMENT_LIST_LATENCY,
        router_env::metric_attributes!((
            "merchant_id",
            platform.get_processor().get_account().get_id().clone()
        )),
    )
    .await
}

/// Fetches, page by page, the recovery invoices whose intent status is one of
/// `candidate_statuses`, with the recovery status derived for each of them.
#[cfg(all(feature = "v2", feature = "olap"))]
async fn scan_recovery_items(
    state: &SessionState,
    platform: &domain::Platform,
    constraints: payments_api::PaymentListConstraints,
    candidate_statuses: Vec<common_enums::enums::IntentStatus>,
) -> RouterResult<Vec<payments_api::RecoveryPaymentsListResponseItem>> {
    let db: &dyn StorageInterface = state.store.as_ref();
    let scan_page_size = common_utils::types::list::PageSize::from(LIST_MAX_LIMIT);
    let scan_constraints = payments_api::PaymentListConstraints {
        status: Some(candidate_statuses),
        limit: scan_page_size,
        ..constraints
    };

    let mut items: Vec<payments_api::RecoveryPaymentsListResponseItem> = Vec::new();
    let mut retry_thresholds = HashMap::new();
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
        let fetch_constraints = page_constraints.into();
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
        items.extend(build_recovery_list_items(state, platform, list, &mut retry_thresholds).await);

        if fetched < scan_page_size.as_usize() {
            break;
        }
        scan_offset += fetched;
    }
    Ok(items)
}

/// Counts the recovery invoices created in `time_range` in each recovery status, for the whole
/// merchant or, when `profile_id` is passed, for that profile only.
///
/// Like the list, the recovery status is derived from the intent status and the recovery
/// workflows rather than stored, so the invoices are scanned and counted by the derived status.
#[cfg(all(feature = "v2", feature = "olap"))]
pub async fn revenue_recovery_aggregates(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    time_range: common_utils::types::TimeRange,
) -> RouterResponse<payments_api::RecoveryAggregateResponse> {
    use strum::IntoEnumIterator;

    let constraints = payments_api::PaymentListConstraints {
        payment_id: None,
        profile_id,
        customer_id: None,
        starting_after: None,
        ending_before: None,
        limit: common_utils::types::list::PageSize::from(LIST_MAX_LIMIT),
        offset: common_utils::types::list::PageOffset::default(),
        created: None,
        created_lt: None,
        created_gt: None,
        created_lte: time_range.end_time,
        created_gte: Some(time_range.start_time),
        start_amount: None,
        end_amount: None,
        connector: None,
        currency: None,
        status: None,
        payment_method_type: None,
        payment_method_subtype: None,
        authentication_type: None,
        merchant_connector_id: None,
        order_on: Default::default(),
        order_by: Default::default(),
        card_network: None,
        merchant_order_reference_id: None,
    };

    let all_recovery_statuses: Vec<_> = common_enums::enums::RecoveryStatus::iter().collect();
    let candidate_statuses =
        candidate_intent_statuses_for_recovery_status(&all_recovery_statuses, None);

    let items = scan_recovery_items(&state, &platform, constraints, candidate_statuses).await?;

    let mut status_with_count: HashMap<_, i64> = all_recovery_statuses
        .into_iter()
        .map(|status| (status, 0))
        .collect();
    for item in &items {
        *status_with_count.entry(item.status).or_default() += 1;
    }
    let total_count = status_with_count.values().sum();

    Ok(services::ApplicationResponse::Json(
        payments_api::RecoveryAggregateResponse {
            status_with_count,
            total_count,
        },
    ))
}

/// Reads one page of recovery invoices, deriving the recovery status only for the rows read.
///
/// Rows are read in blocks of `limit` after the cursor until the page is full or no rows are left.
/// A sparse `recovery_status` filter can therefore read every candidate row in one request.
#[cfg(all(feature = "v2", feature = "olap"))]
async fn list_recovery_page(
    state: &SessionState,
    platform: &domain::Platform,
    mut constraints: payments_api::PaymentListConstraints,
    recovery_status: Option<Vec<common_enums::enums::RecoveryStatus>>,
) -> RouterResult<services::ApplicationResponse<payments_api::RecoveryPaymentListResponse>> {
    let db: &dyn StorageInterface = state.store.as_ref();

    if constraints.order_on != payments_api::SortOn::Created {
        return Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "recovery-list can only be sorted on `created`".to_string(),
        }
        .into());
    }

    let starting_after = constraints.starting_after.take();
    let ending_before = constraints.ending_before.take();
    let reading_backwards = ending_before.is_some();
    let cursor = match (starting_after, ending_before) {
        (Some(_), Some(_)) => {
            return Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "`starting_after` and `ending_before` cannot be used together".to_string(),
            }
            .into());
        }
        (cursor, None) | (None, cursor) => cursor,
    };
    let mut read_after = match &cursor {
        Some(cursor) => Some(
            find_recovery_list_cursor(db, platform, constraints.profile_id.as_ref(), cursor)
                .await?,
        ),
        None => None,
    };

    if let Some(recovery_status) = &recovery_status {
        let candidate_statuses = candidate_intent_statuses_for_recovery_status(
            recovery_status,
            constraints.status.as_deref(),
        );
        if candidate_statuses.is_empty() {
            return Ok(services::ApplicationResponse::Json(
                payments_api::RecoveryPaymentListResponse {
                    count: 0,
                    next_cursor: None,
                    data: Vec::new(),
                },
            ));
        }
        constraints.status = Some(candidate_statuses);
    }

    // Paging back reads away from the cursor in the opposite order; the page is flipped below.
    if reading_backwards {
        constraints.order_by = match constraints.order_by {
            payments_api::SortBy::Asc => payments_api::SortBy::Desc,
            payments_api::SortBy::Desc => payments_api::SortBy::Asc,
        };
    }

    let limit = constraints.limit.as_usize();
    let mut fetch_constraints: PaymentIntentFetchConstraints = constraints.into();
    let mut page = Vec::with_capacity(limit);
    let mut retry_thresholds = HashMap::new();

    let has_more = 'reads: loop {
        let PaymentIntentFetchConstraints::List(params) = &mut fetch_constraints;
        params.read_after = read_after.clone();

        let list: Vec<(storage::PaymentIntent, Option<storage::PaymentAttempt>)> = db
            .get_filtered_payment_intents_attempt(
                platform.get_processor().get_account().get_id(),
                &fetch_constraints,
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let rows_read = list.len();
        let is_last_block = rows_read < limit;
        let positions: Vec<_> = list
            .iter()
            .map(|(payment_intent, _)| (payment_intent.created_at, payment_intent.id.clone()))
            .collect();
        let items = build_recovery_list_items(state, platform, list, &mut retry_thresholds).await;

        for (row_index, (position, item)) in positions.into_iter().zip(items).enumerate() {
            read_after = Some(position);
            if recovery_status
                .as_ref()
                .is_none_or(|recovery_status| recovery_status.contains(&item.status))
            {
                page.push(item);
            }
            if page.len() == limit {
                break 'reads !(is_last_block && row_index + 1 == rows_read);
            }
        }

        if is_last_block {
            break false;
        }
    };

    if reading_backwards {
        page.reverse();
    }

    Ok(services::ApplicationResponse::Json(
        payments_api::RecoveryPaymentListResponse {
            count: page.len(),
            next_cursor: read_after.filter(|_| has_more).map(|(_, id)| id),
            data: page,
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
