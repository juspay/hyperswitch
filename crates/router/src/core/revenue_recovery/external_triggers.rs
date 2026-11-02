use api_models::payments::RecoveryPaymentsCreate;
use common_types::payments::{
    ExternalTriggerState, RecoveryConnector, RecoveryConnectorDetails, RecoveryNextRetry,
    RecoveryNoRetryReason, RecoveryTransactionStatus,
};
use error_stack::report;
use hyperswitch_domain_models::{
    payments::{payment_intent::PaymentIntentUpdate, PaymentIntent},
    platform::Platform,
    revenue_recovery::{RecoveryPaymentAttempt, RecoveryPaymentIntent},
};
use hyperswitch_masking::PeekInterface;
use router_env::logger;
use time::{Duration, PrimitiveDateTime};

use super::{
    api,
    card_selection::{self, CardAvailability, CardSelection},
    get_recovery_grace_window_end, is_retry_budget_exhausted,
    retry_stats::RetryOutcomeEvent,
    schedule::StaticLadderProgress,
};
use crate::{
    core::{
        api_locking::{LockAction, LockingInput},
        configs::dimension_state::{self, DimensionsWithProcessorMerchantIdAndConnector},
        errors::{self, RouterResult, StorageErrorExt},
        payments::operations::{
            payment_attempt_record::PaymentAttemptRecord, ValidateStatusForOperation,
        },
    },
    routes::{app::ReqState, lock_utils, SessionState},
    types::{
        domain,
        storage::{revenue_recovery as pcr, revenue_recovery_redis_operation::RedisTokenManager},
    },
    workflows::revenue_recovery::{self as revenue_recovery_workflow, RetryTiming},
};

/// The limits that bound how long an invoice is retried. There is no billing connector account to
/// hold them with external triggers, so they come from the configuration of the merchant.
struct RecoveryRetryLimits {
    max_retry_count: u16,
    grace_period_days: u16,
    max_hybrid_cascading_retry_count: u16,
}

impl RecoveryRetryLimits {
    async fn from_config(
        state: &SessionState,
        dimensions: &DimensionsWithProcessorMerchantIdAndConnector,
    ) -> Self {
        let store = state.store.as_ref();
        let superposition = state.superposition_service.as_ref();
        let to_limit = |value: i64| u16::try_from(value.max(0)).unwrap_or(u16::MAX);

        Self {
            max_retry_count: to_limit(
                dimensions
                    .get_recovery_max_retry_count(store, superposition, None)
                    .await,
            ),
            grace_period_days: to_limit(
                dimensions
                    .get_recovery_grace_period_days(store, superposition, None)
                    .await,
            ),
            max_hybrid_cascading_retry_count: to_limit(
                dimensions
                    .get_recovery_max_hybrid_cascading_retry_count(store, superposition, None)
                    .await,
            ),
        }
    }
}

/// What the next retry of an invoice is decided from.
struct NextRetryInputs<'a> {
    transaction_status: RecoveryTransactionStatus,
    connector_customer_id: &'a str,
    payment_intent: &'a PaymentIntent,
    dimensions: &'a DimensionsWithProcessorMerchantIdAndConnector,
    limits: &'a RecoveryRetryLimits,
    ladder_progress: &'a StaticLadderProgress,
    retry_count: u16,
    last_failure_code: Option<common_enums::StandardisedCode>,
}

/// What is decided about the next retry of an invoice, with the state to keep for the one after.
struct NextRetryDecision {
    next_retry: RecoveryNextRetry,
    next_ladder_progress: Option<StaticLadderProgress>,
    assigned_ab_algorithm: Option<common_enums::RevenueRecoveryABAlgorithm>,
}

impl NextRetryDecision {
    fn not_scheduled(reason: RecoveryNoRetryReason) -> Self {
        Self {
            next_retry: RecoveryNextRetry::NotScheduled { reason },
            next_ladder_progress: None,
            assigned_ab_algorithm: None,
        }
    }
}

/// Records a transaction the merchant reports and works out when they should trigger the next
/// retry. The reports of an invoice are handled one at a time, as each one builds on the state the
/// last one left, and no connector sync is there to correct a report handled out of turn.
pub(super) async fn record_transaction_and_get_next_retry(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &RecoveryPaymentsCreate,
    connector: RecoveryConnector,
) -> RouterResult<(RecoveryPaymentIntent, RecoveryNextRetry)> {
    let invoice_lock = LockAction::Hold {
        input: LockingInput {
            unique_locking_key: request.merchant_reference_id.get_string_repr().to_owned(),
            api_identifier: lock_utils::ApiIdentifier::Payments,
            override_lock_retries: None,
        },
    };
    let merchant_id = platform.get_processor().get_account().get_id().to_owned();

    invoice_lock
        .clone()
        .perform_locking_action(state, merchant_id.clone())
        .await?;

    let result = Box::pin(handle_merchant_report(
        state, req_state, platform, profile, request, connector,
    ))
    .await;

    if let Err(error) = invoice_lock.free_lock_action(state, merchant_id).await {
        logger::error!(?error, "Failed to release the lock of the invoice");
    }

    result
}

/// Records the transaction, keeps the cluster data up to date and decides the next retry.
///
/// Once the transaction is recorded the merchant must get an answer, because reporting it again is
/// refused as a duplicate. So the steps after recording log their failures rather than fail the
/// request, apart from reading the invoice back, which fails only if the database just went away.
async fn handle_merchant_report(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &RecoveryPaymentsCreate,
    connector: RecoveryConnector,
) -> RouterResult<(RecoveryPaymentIntent, RecoveryNextRetry)> {
    let invoice = api::get_or_create_invoice(state, req_state, platform, profile, request).await?;

    PaymentAttemptRecord
        .validate_status_for_operation(invoice.status)
        .map_err(|_| {
            report!(errors::ApiErrorResponse::PreconditionFailed {
                message: format!(
                    "The invoice is `{}` and takes no more transactions",
                    invoice.status
                ),
            })
        })?;

    let previous_recovery_metadata = invoice
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.revenue_recovery.as_ref());
    let is_retry = previous_recovery_metadata.is_some();
    let previous_trigger_state = previous_recovery_metadata
        .and_then(|recovery_metadata| recovery_metadata.external_trigger_state.clone());

    let (recovery_attempt, _) = Box::pin(api::record_transaction(
        state,
        req_state,
        platform,
        profile,
        request,
        &invoice,
        &RecoveryConnectorDetails::ReportedByMerchant {
            connector: connector.get_known_connector(),
        },
    ))
    .await?;

    let previous_failure_code = previous_trigger_state
        .as_ref()
        .and_then(|trigger_state| trigger_state.last_failure_code);
    if let Some((failure_code, success)) =
        get_retry_outcome(is_retry, request.transaction_status, previous_failure_code)
    {
        RetryOutcomeEvent::from_retry_outcome(recovery_attempt.created_at, failure_code, success)
            .record(state)
            .await;
    }

    reset_card_history_after_a_charge(state, request).await;

    let payment_intent = state
        .store
        .find_payment_intent_by_id(
            &invoice.payment_id,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_storage_scheme(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
    let updated_invoice = RecoveryPaymentIntent::from(&payment_intent);

    let dimensions = dimension_state::Dimensions::new()
        .with_processor_merchant_id(payment_intent.merchant_id.clone().into())
        // The merchant's own system plays the billing connector, as with a custom billing account.
        .with_connector(common_enums::connector_enums::Connector::Custombilling);
    let limits = RecoveryRetryLimits::from_config(state, &dimensions).await;
    let retry_count = payment_intent
        .get_revenue_recovery_retry_count()
        .unwrap_or(1);
    let ladder_progress = get_ladder_progress(
        previous_trigger_state.as_ref(),
        is_retry,
        retry_count,
        limits.max_hybrid_cascading_retry_count,
    );
    let last_failure_code = get_last_failure_code(request.transaction_status, &recovery_attempt);

    let decision = Box::pin(decide_next_retry(
        state,
        platform,
        profile,
        NextRetryInputs {
            transaction_status: request.transaction_status,
            connector_customer_id: request.connector_customer_id.peek(),
            payment_intent: &payment_intent,
            dimensions: &dimensions,
            limits: &limits,
            ladder_progress: &ladder_progress,
            retry_count,
            last_failure_code,
        },
    ))
    .await;

    let trigger_state = ExternalTriggerState {
        last_failure_code,
        consumed_ladder_rungs: decision
            .next_ladder_progress
            .unwrap_or(ladder_progress)
            .consumed_rungs,
        next_retry: decision.next_retry,
    };
    save_trigger_state(
        state,
        platform,
        payment_intent,
        trigger_state,
        decision.assigned_ab_algorithm,
    )
    .await;

    Ok((updated_invoice, decision.next_retry))
}

/// A charged card is healthy again, so its retry history starts over, as it does for the retries
/// Hyperswitch makes.
async fn reset_card_history_after_a_charge(state: &SessionState, request: &RecoveryPaymentsCreate) {
    match request.transaction_status {
        RecoveryTransactionStatus::Failure => {}
        RecoveryTransactionStatus::Charged | RecoveryTransactionStatus::PartialCharged => {
            let reset =
                RedisTokenManager::update_payment_processor_token_error_code_from_process_tracker(
                    state,
                    request.connector_customer_id.peek(),
                    &None,
                    &None,
                    Some(
                        request
                            .payment_method_data
                            .primary_processor_payment_method_token
                            .peek()
                            .as_str(),
                    ),
                )
                .await;
            if let Err(error) = reset {
                logger::error!(
                    ?error,
                    "Failed to reset the retry history of a charged card"
                );
            }
        }
    }
}

/// Decides the next retry of an invoice: first whatever rules a retry out, then its time under the
/// retry algorithm of the profile, then whatever the card allows.
async fn decide_next_retry(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    inputs: NextRetryInputs<'_>,
) -> NextRetryDecision {
    let NextRetryInputs {
        transaction_status,
        connector_customer_id,
        payment_intent,
        dimensions,
        limits,
        ladder_progress,
        retry_count,
        last_failure_code,
    } = inputs;

    let retry_algorithm_type = profile
        .revenue_recovery_retry_algorithm_type
        .unwrap_or(state.conf.revenue_recovery.retry_algorithm_type);
    let now = common_utils::date_time::now();

    // An invoice that cannot be bounded by its grace window is not retried.
    let grace_window_end = get_recovery_grace_window_end(payment_intent, limits.grace_period_days)
        .inspect_err(|error| {
            logger::error!(?error, "Cannot bound the invoice by its grace window");
        })
        .ok()
        .filter(|grace_window_end| *grace_window_end > now);

    if let Some(reason) = get_no_retry_reason(
        transaction_status,
        retry_algorithm_type,
        is_retry_budget_exhausted(i32::from(retry_count), limits.max_retry_count),
        grace_window_end.is_none(),
    ) {
        return NextRetryDecision::not_scheduled(reason);
    }
    let Some(grace_window_end) = grace_window_end else {
        return NextRetryDecision::not_scheduled(RecoveryNoRetryReason::GraceWindowExpired);
    };

    let retry_timing = Box::pin(revenue_recovery_workflow::get_retry_timing(
        state,
        revenue_recovery_workflow::RetryTimingParams {
            payment_intent,
            retry_algorithm_type,
            retry_count: i32::from(retry_count),
            prev_attempt_error_code: last_failure_code,
            static_ladder_progress: ladder_progress,
            max_hybrid_cascading_retry_count: Some(limits.max_hybrid_cascading_retry_count),
            remaining_grace_days: u32::try_from((grace_window_end - now).whole_days().max(0))
                .unwrap_or(0),
            remaining_budget: u32::from(limits.max_retry_count)
                .saturating_sub(u32::from(retry_count)),
            provider_merchant_id: platform.get_provider().get_provider_merchant_id(),
            merchant_context: pcr::RecoveryMerchantContext {
                merchant_account: platform.get_processor().get_account(),
                key_store: platform.get_processor().get_key_store().inner(),
                profile,
            },
        },
        dimensions,
    ))
    .await;

    let (retry_time, next_ladder_progress, assigned_ab_algorithm) = match retry_timing {
        Ok(RetryTiming::At {
            retry_time,
            next_static_ladder_progress,
            assigned_ab_algorithm,
        }) => (
            retry_time,
            next_static_ladder_progress,
            assigned_ab_algorithm,
        ),
        // The decider works card by card under a lock on the customer, which is held while
        // Hyperswitch makes the retry, so the merchant gets the cascading ladder instead.
        Ok(RetryTiming::Decider) => {
            match revenue_recovery_workflow::get_schedule_time_to_retry_mit_payments(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                dimensions,
                i32::from(retry_count),
            )
            .await
            {
                Some(retry_time) => (retry_time, None, None),
                None => {
                    return NextRetryDecision::not_scheduled(
                        RecoveryNoRetryReason::RetriesExhausted,
                    )
                }
            }
        }
        Ok(RetryTiming::NoRetries) => {
            return NextRetryDecision::not_scheduled(RecoveryNoRetryReason::Monitoring)
        }
        Ok(RetryTiming::LadderExhausted) => {
            return NextRetryDecision::not_scheduled(RecoveryNoRetryReason::RetriesExhausted)
        }
        Err(error) => {
            logger::error!(
                ?error,
                "No time is available for the next retry of the invoice"
            );
            return NextRetryDecision::not_scheduled(RecoveryNoRetryReason::RetriesExhausted);
        }
    };

    let next_retry = get_next_retry(
        revenue_recovery_workflow::add_random_delay_to_schedule_time(state, retry_time),
        &get_card_availability(state, connector_customer_id, payment_intent).await,
        Duration::seconds(
            state
                .conf
                .revenue_recovery
                .recovery_timestamp
                .job_schedule_buffer_time_in_seconds,
        ),
        grace_window_end,
        now,
    );

    NextRetryDecision {
        next_retry,
        next_ladder_progress: next_ladder_progress
            .filter(|_| matches!(next_retry, RecoveryNextRetry::Scheduled { .. })),
        assigned_ab_algorithm,
    }
}

/// Where the card of the invoice stands. The merchant picks the card of a retry, so only that card
/// is considered, whatever the card switching flag says. A failed lookup does not hold the retry
/// back, the merchant is still given a time.
async fn get_card_availability(
    state: &SessionState,
    connector_customer_id: &str,
    payment_intent: &PaymentIntent,
) -> CardAvailability {
    let invoice_token =
        revenue_recovery_workflow::get_invoice_payment_processor_token(payment_intent);

    match RedisTokenManager::get_payment_processor_metadata_for_connector_customer(
        state,
        connector_customer_id,
    )
    .await
    {
        Ok(tokens) => card_selection::pick_card_for_retry_time(
            CardSelection::InvoiceCard,
            &tokens,
            invoice_token.as_deref(),
        ),
        Err(error) => {
            logger::error!(?error, "Failed to read the retry history of the card");
            CardAvailability::NoCard
        }
    }
}

/// Keeps what the next report of the merchant needs on the invoice. Failing here costs the next
/// retry its cluster and ladder position, not the answer to this report.
async fn save_trigger_state(
    state: &SessionState,
    platform: &Platform,
    payment_intent: PaymentIntent,
    trigger_state: ExternalTriggerState,
    assigned_ab_algorithm: Option<common_enums::RevenueRecoveryABAlgorithm>,
) {
    let mut feature_metadata = payment_intent.feature_metadata.clone();
    if let Some(recovery_metadata) = feature_metadata
        .as_mut()
        .and_then(|feature_metadata| feature_metadata.payment_revenue_recovery_metadata.as_mut())
    {
        recovery_metadata.external_trigger_state = Some(trigger_state);
        recovery_metadata.revenue_recovery_ab_routing =
            assigned_ab_algorithm.or(recovery_metadata.revenue_recovery_ab_routing);
    }

    let storage_scheme = platform.get_processor().get_storage_scheme();
    // The status is left out, so that a cancel made since the transaction was recorded stands.
    let intent_update = PaymentIntentUpdate::FeatureMetadataUpdate {
        feature_metadata: Box::new(feature_metadata),
        updated_by: storage_scheme.inner().to_string(),
    };

    if let Err(error) = state
        .store
        .update_payment_intent(
            payment_intent,
            intent_update,
            platform.get_processor().get_key_store(),
            storage_scheme,
        )
        .await
    {
        logger::error!(?error, "Failed to save the recovery state of the invoice");
    }
}

/// Where the invoice stands on the static ladder: what the last report saved, or else a position
/// worked out from the retries already made.
fn get_ladder_progress(
    previous_trigger_state: Option<&ExternalTriggerState>,
    is_retry: bool,
    retry_count: u16,
    max_hybrid_cascading_retry_count: u16,
) -> StaticLadderProgress {
    match previous_trigger_state {
        Some(trigger_state) => StaticLadderProgress {
            consumed_rungs: trigger_state.consumed_ladder_rungs,
        },
        // The last report could not save its state.
        None if is_retry => StaticLadderProgress::seed_for_existing_invoice(
            retry_count,
            max_hybrid_cascading_retry_count,
        ),
        None => StaticLadderProgress::seed_for_new_invoice(
            retry_count,
            max_hybrid_cascading_retry_count,
        ),
    }
}

/// The standardised error code the next retry is counted against: that of this transaction when it
/// failed. A partial charge leaves none, as it does for the retries Hyperswitch makes.
fn get_last_failure_code(
    transaction_status: RecoveryTransactionStatus,
    recovery_attempt: &RecoveryPaymentAttempt,
) -> Option<common_enums::StandardisedCode> {
    match transaction_status {
        RecoveryTransactionStatus::Failure => recovery_attempt.standardised_error_code,
        RecoveryTransactionStatus::Charged | RecoveryTransactionStatus::PartialCharged => None,
    }
}

/// The result to count in the cluster data, as the error code the retry was made after and whether
/// it succeeded. The first transaction of an invoice is not a retry, and partial charges are not
/// counted, as they are not for the retries Hyperswitch makes.
fn get_retry_outcome(
    is_retry: bool,
    transaction_status: RecoveryTransactionStatus,
    previous_failure_code: Option<common_enums::StandardisedCode>,
) -> Option<(Option<common_enums::StandardisedCode>, bool)> {
    if !is_retry {
        return None;
    }

    match transaction_status {
        RecoveryTransactionStatus::Failure => Some((previous_failure_code, false)),
        RecoveryTransactionStatus::Charged => Some((previous_failure_code, true)),
        RecoveryTransactionStatus::PartialCharged => None,
    }
}

/// Why no retry should follow this transaction, before any retry time is worked out.
fn get_no_retry_reason(
    transaction_status: RecoveryTransactionStatus,
    retry_algorithm_type: common_enums::RevenueRecoveryAlgorithmType,
    is_retry_budget_spent: bool,
    is_grace_window_over: bool,
) -> Option<RecoveryNoRetryReason> {
    if matches!(transaction_status, RecoveryTransactionStatus::Charged) {
        Some(RecoveryNoRetryReason::Recovered)
    } else if retry_algorithm_type == common_enums::RevenueRecoveryAlgorithmType::Monitoring {
        Some(RecoveryNoRetryReason::Monitoring)
    } else if is_retry_budget_spent {
        Some(RecoveryNoRetryReason::RetriesExhausted)
    } else if is_grace_window_over {
        Some(RecoveryNoRetryReason::GraceWindowExpired)
    } else {
        None
    }
}

/// The next retry once its time is known, given where the card stands and when the grace window of
/// the invoice closes.
fn get_next_retry(
    retry_time: PrimitiveDateTime,
    card_availability: &CardAvailability,
    card_wait_buffer: Duration,
    grace_window_end: PrimitiveDateTime,
    now: PrimitiveDateTime,
) -> RecoveryNextRetry {
    let schedule_time = match card_availability {
        CardAvailability::HardDecline => {
            return RecoveryNextRetry::NotScheduled {
                reason: RecoveryNoRetryReason::HardDecline,
            }
        }
        CardAvailability::Ready { .. } | CardAvailability::NoCard => retry_time,
        // The card is at its network's retry limit until then, and nothing works out the time
        // again before the merchant retries, so the retry has to wait past the limit here.
        CardAvailability::WaitHours(wait_hours) => retry_time.max(
            now.saturating_add(Duration::hours(*wait_hours))
                .saturating_add(card_wait_buffer),
        ),
    };

    if schedule_time >= grace_window_end {
        RecoveryNextRetry::NotScheduled {
            reason: RecoveryNoRetryReason::GraceWindowExpired,
        }
    } else {
        RecoveryNextRetry::Scheduled { schedule_time }
    }
}

#[cfg(test)]
mod tests {
    use common_enums::{RevenueRecoveryAlgorithmType, StandardisedCode};

    use super::*;

    const FAILURE_CODE: StandardisedCode = StandardisedCode::DoNotHonor;

    /// `hours` past a fixed time, so that tests read as a timeline.
    fn at(hours: i64) -> PrimitiveDateTime {
        PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::November, 2).expect("valid date"),
            time::Time::MIDNIGHT,
        ) + Duration::hours(hours)
    }

    fn ready_card() -> CardAvailability {
        CardAvailability::Ready {
            payment_processor_token: "token".to_owned(),
        }
    }

    fn scheduled(schedule_time: PrimitiveDateTime) -> RecoveryNextRetry {
        RecoveryNextRetry::Scheduled { schedule_time }
    }

    fn not_scheduled(reason: RecoveryNoRetryReason) -> RecoveryNextRetry {
        RecoveryNextRetry::NotScheduled { reason }
    }

    // ---- where the invoice stands on the static ladder --------------------

    fn saved_state(consumed_ladder_rungs: i32) -> ExternalTriggerState {
        ExternalTriggerState {
            last_failure_code: None,
            consumed_ladder_rungs,
            next_retry: scheduled(at(1)),
        }
    }

    #[test]
    fn the_saved_ladder_position_is_used() {
        assert_eq!(
            get_ladder_progress(Some(&saved_state(2)), true, 9, 5).consumed_rungs,
            2
        );
    }

    #[test]
    fn a_new_invoice_may_open_the_ladder_fully_consumed() {
        assert_eq!(get_ladder_progress(None, false, 5, 5).consumed_rungs, 5);
    }

    #[test]
    fn an_invoice_whose_state_was_not_saved_keeps_a_ladder_position_in_hand() {
        assert_eq!(get_ladder_progress(None, true, 5, 5).consumed_rungs, 4);
    }

    // ---- what is counted in the cluster data ------------------------------

    #[test]
    fn the_first_transaction_of_an_invoice_is_not_a_retry() {
        for status in [
            RecoveryTransactionStatus::Failure,
            RecoveryTransactionStatus::Charged,
            RecoveryTransactionStatus::PartialCharged,
        ] {
            assert_eq!(get_retry_outcome(false, status, None), None);
        }
    }

    #[test]
    fn a_failed_retry_counts_against_the_previous_failure() {
        assert_eq!(
            get_retry_outcome(true, RecoveryTransactionStatus::Failure, Some(FAILURE_CODE)),
            Some((Some(FAILURE_CODE), false))
        );
    }

    #[test]
    fn a_charged_retry_counts_for_the_previous_failure() {
        assert_eq!(
            get_retry_outcome(true, RecoveryTransactionStatus::Charged, Some(FAILURE_CODE)),
            Some((Some(FAILURE_CODE), true))
        );
    }

    #[test]
    fn a_retry_after_an_unknown_failure_is_still_counted() {
        assert_eq!(
            get_retry_outcome(true, RecoveryTransactionStatus::Failure, None),
            Some((None, false))
        );
    }

    #[test]
    fn a_partial_charge_is_not_counted() {
        assert_eq!(
            get_retry_outcome(
                true,
                RecoveryTransactionStatus::PartialCharged,
                Some(FAILURE_CODE)
            ),
            None
        );
    }

    // ---- what rules a retry out -------------------------------------------

    #[test]
    fn a_failure_within_every_limit_can_be_retried() {
        assert_eq!(
            get_no_retry_reason(
                RecoveryTransactionStatus::Failure,
                RevenueRecoveryAlgorithmType::Cascading,
                false,
                false,
            ),
            None
        );
    }

    #[test]
    fn a_partial_charge_within_every_limit_can_be_retried() {
        assert_eq!(
            get_no_retry_reason(
                RecoveryTransactionStatus::PartialCharged,
                RevenueRecoveryAlgorithmType::Smart,
                false,
                false,
            ),
            None
        );
    }

    #[test]
    fn a_charged_invoice_is_recovered_whatever_else_holds() {
        assert_eq!(
            get_no_retry_reason(
                RecoveryTransactionStatus::Charged,
                RevenueRecoveryAlgorithmType::Monitoring,
                true,
                true,
            ),
            Some(RecoveryNoRetryReason::Recovered)
        );
    }

    #[test]
    fn monitoring_never_retries() {
        assert_eq!(
            get_no_retry_reason(
                RecoveryTransactionStatus::Failure,
                RevenueRecoveryAlgorithmType::Monitoring,
                true,
                true,
            ),
            Some(RecoveryNoRetryReason::Monitoring)
        );
    }

    #[test]
    fn a_spent_retry_budget_stops_the_retries() {
        assert_eq!(
            get_no_retry_reason(
                RecoveryTransactionStatus::Failure,
                RevenueRecoveryAlgorithmType::Cascading,
                true,
                true,
            ),
            Some(RecoveryNoRetryReason::RetriesExhausted)
        );
    }

    #[test]
    fn a_closed_grace_window_stops_the_retries() {
        assert_eq!(
            get_no_retry_reason(
                RecoveryTransactionStatus::Failure,
                RevenueRecoveryAlgorithmType::Cascading,
                false,
                true,
            ),
            Some(RecoveryNoRetryReason::GraceWindowExpired)
        );
    }

    // ---- the next retry once its time is known ----------------------------

    #[test]
    fn a_ready_card_is_retried_at_the_retry_time() {
        assert_eq!(
            get_next_retry(at(3), &ready_card(), Duration::hours(1), at(100), at(0)),
            scheduled(at(3))
        );
    }

    #[test]
    fn a_card_with_no_history_does_not_hold_the_retry_back() {
        assert_eq!(
            get_next_retry(
                at(3),
                &CardAvailability::NoCard,
                Duration::hours(1),
                at(100),
                at(0)
            ),
            scheduled(at(3))
        );
    }

    #[test]
    fn a_hard_declined_card_is_not_retried() {
        assert_eq!(
            get_next_retry(
                at(3),
                &CardAvailability::HardDecline,
                Duration::hours(1),
                at(100),
                at(0)
            ),
            not_scheduled(RecoveryNoRetryReason::HardDecline)
        );
    }

    #[test]
    fn a_card_at_its_network_limit_moves_the_retry_past_the_limit() {
        assert_eq!(
            get_next_retry(
                at(3),
                &CardAvailability::WaitHours(24),
                Duration::hours(1),
                at(100),
                at(0)
            ),
            scheduled(at(25))
        );
    }

    #[test]
    fn a_card_free_before_the_retry_time_leaves_the_retry_time() {
        assert_eq!(
            get_next_retry(
                at(30),
                &CardAvailability::WaitHours(24),
                Duration::hours(1),
                at(100),
                at(0)
            ),
            scheduled(at(30))
        );
    }

    #[test]
    fn a_retry_time_past_the_grace_window_is_not_scheduled() {
        assert_eq!(
            get_next_retry(at(101), &ready_card(), Duration::hours(1), at(100), at(0)),
            not_scheduled(RecoveryNoRetryReason::GraceWindowExpired)
        );
    }

    #[test]
    fn a_card_free_only_after_the_grace_window_is_not_scheduled() {
        assert_eq!(
            get_next_retry(
                at(3),
                &CardAvailability::WaitHours(100),
                Duration::hours(1),
                at(100),
                at(0)
            ),
            not_scheduled(RecoveryNoRetryReason::GraceWindowExpired)
        );
    }
}
