use api_models::payments::RecoveryPaymentsCreate;
use common_types::payments::{
    ExternalTriggerState, RecoveryConnector, RecoveryConnectorDetails, RecoveryNextRetry,
    RecoveryNoRetryReason, RecoveryTransactionStatus,
};
use diesel_models::business_profile::RevenueRecoveryConfig;
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

/// What the next retry of an invoice is decided from.
struct NextRetryInputs<'a> {
    payment_intent: &'a PaymentIntent,
    dimensions: &'a DimensionsWithProcessorMerchantIdAndConnector,
    retry_config: &'a RevenueRecoveryConfig,
    ladder_progress: &'a StaticLadderProgress,
    retry_count: u16,
    last_failure_code: Option<common_enums::StandardisedCode>,
    card_availability: CardAvailability,
    retries_after_hard_decline: Option<u16>,
}

/// What the limits of the profile say about the next retry of an invoice.
#[derive(Clone, Copy, Debug, Default)]
struct RetryLimitChecks {
    is_retry_budget_spent: bool,
    is_grace_window_over: bool,
    is_hard_decline_limit_reached: bool,
    is_below_retry_threshold: bool,
}

/// What is decided about the next retry of an invoice, with the state to keep for the one after.
struct NextRetryDecision {
    next_retry: RecoveryNextRetry,
    next_ladder_progress: Option<StaticLadderProgress>,
    assigned_ab_algorithm: Option<common_enums::RevenueRecoveryABAlgorithm>,
}

impl NextRetryDecision {
    fn not_scheduled(retry_count: u16, reason: RecoveryNoRetryReason) -> Self {
        Self {
            next_retry: RecoveryNextRetry::not_scheduled(retry_count, reason),
            next_ladder_progress: None,
            assigned_ab_algorithm: None,
        }
    }
}

/// Records a transaction the merchant reports and, when it was made off session with the customer
/// absent, works out when they should trigger the next retry. The reports of an invoice are handled
/// one at a time, as each one builds on the state the last one left, and no connector sync is there
/// to correct a report handled out of turn.
pub(super) async fn record_transaction_and_get_next_retry(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &RecoveryPaymentsCreate,
    connector: RecoveryConnector,
    retry_config: &RevenueRecoveryConfig,
) -> RouterResult<(RecoveryPaymentIntent, Option<RecoveryNextRetry>)> {
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
        state,
        req_state,
        platform,
        profile,
        request,
        connector,
        retry_config,
    ))
    .await;

    if let Err(error) = invoice_lock.free_lock_action(state, merchant_id).await {
        logger::error!(?error, "Failed to release the lock of the invoice");
    }

    result
}

/// Records the transaction and decides the next retry. A transaction made on session or with the
/// customer present is only recorded: it is not a retry, so what the invoice last told the merchant
/// stays as it was.
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
    retry_config: &RevenueRecoveryConfig,
) -> RouterResult<(RecoveryPaymentIntent, Option<RecoveryNextRetry>)> {
    // A value sent in the request takes priority over the one on the profile.
    let request = &RecoveryPaymentsCreate {
        enable_partial_authorization: request
            .enable_partial_authorization
            .or(Some(retry_config.enable_partial_authorization.into())),
        ..request.clone()
    };

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
    let is_existing_invoice = previous_recovery_metadata.is_some();
    let is_same_card = previous_recovery_metadata.is_some_and(|recovery_metadata| {
        recovery_metadata
            .billing_connector_payment_details
            .payment_processor_token
            == *request
                .payment_method_data
                .primary_processor_payment_method_token
                .peek()
    });
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
            connector: connector.into(),
        },
    ))
    .await?;

    let is_merchant_initiated = request
        .customer_present
        .unwrap_or(common_enums::PresenceOfCustomerDuringPayment::Absent)
        == common_enums::PresenceOfCustomerDuringPayment::Absent
        && request
            .setup_future_usage
            .unwrap_or(common_enums::FutureUsage::OffSession)
            .is_off_session();

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

    if !is_merchant_initiated {
        return Ok((updated_invoice, None));
    }

    let dimensions = dimension_state::Dimensions::new()
        .with_processor_merchant_id(payment_intent.merchant_id.clone().into())
        .with_connector(common_enums::connector_enums::Connector::Custombilling);
    let retry_count = payment_intent
        .get_revenue_recovery_retry_count()
        .unwrap_or(1);
    let max_hybrid_cascading_retry_count = dimension_state::Dimensions::new()
        .with_processor_merchant_id(payment_intent.merchant_id.clone().into())
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
        .with_profile_id(payment_intent.profile_id.clone())
        .get_recovery_max_hybrid_cascading_retry_count(
            state.store.as_ref(),
            state.superposition_service.as_ref(),
            None,
        )
        .await;
    let ladder_progress = get_ladder_progress(
        previous_trigger_state.as_ref(),
        is_existing_invoice,
        retry_count,
        max_hybrid_cascading_retry_count,
    );
    let last_failure_code = get_last_failure_code(request.transaction_status, &recovery_attempt);
    let card_availability =
        get_card_availability(state, request.connector_customer_id.peek(), &payment_intent).await;
    let retries_after_hard_decline = get_retries_after_hard_decline(
        previous_trigger_state
            .as_ref()
            .and_then(|trigger_state| trigger_state.retries_after_hard_decline),
        is_same_card,
        matches!(card_availability, CardAvailability::HardDecline),
    );

    let decision = Box::pin(decide_next_retry(
        state,
        platform,
        profile,
        NextRetryInputs {
            payment_intent: &payment_intent,
            dimensions: &dimensions,
            retry_config,
            ladder_progress: &ladder_progress,
            retry_count,
            last_failure_code,
            card_availability,
            retries_after_hard_decline,
        },
    ))
    .await;

    let trigger_state = ExternalTriggerState {
        consumed_ladder_rungs: decision
            .next_ladder_progress
            .unwrap_or(ladder_progress)
            .consumed_rungs,
        retries_after_hard_decline,
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

    Ok((updated_invoice, Some(decision.next_retry)))
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

/// Decides the next retry of an invoice: first whatever the limits of the profile rule out, then
/// its time under the retry algorithm of the profile, then whatever the card allows.
async fn decide_next_retry(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    inputs: NextRetryInputs<'_>,
) -> NextRetryDecision {
    let NextRetryInputs {
        payment_intent,
        dimensions,
        retry_config,
        ladder_progress,
        retry_count,
        last_failure_code,
        card_availability,
        retries_after_hard_decline,
    } = inputs;

    let retry_algorithm_type = profile
        .revenue_recovery_retry_algorithm_type
        .unwrap_or(state.conf.revenue_recovery.retry_algorithm_type);
    let now = common_utils::date_time::now();

    let grace_window_end =
        get_recovery_grace_window_end(payment_intent, retry_config.grace_period_days)
            .inspect_err(|error| {
                logger::error!(?error, "Cannot bound the invoice by its grace window");
            })
            .ok()
            .filter(|grace_window_end| *grace_window_end > now);

    if let Some(reason) = get_no_retry_reason(
        payment_intent.status,
        retry_algorithm_type,
        RetryLimitChecks {
            is_retry_budget_spent: is_retry_budget_exhausted(
                retry_count,
                retry_config.max_retry_count,
            ),
            is_grace_window_over: grace_window_end.is_none(),
            is_hard_decline_limit_reached: retries_after_hard_decline
                .is_some_and(|retries| retries >= retry_config.max_hard_decline_retries),
            is_below_retry_threshold: retry_count < retry_config.billing_connector_retry_threshold,
        },
    ) {
        return NextRetryDecision::not_scheduled(retry_count, reason);
    }
    let Some(grace_window_end) = grace_window_end else {
        return NextRetryDecision::not_scheduled(
            retry_count,
            RecoveryNoRetryReason::GracePeriodExceeded,
        );
    };

    let retry_timing = Box::pin(revenue_recovery_workflow::get_retry_timing(
        state,
        revenue_recovery_workflow::RetryTimingParams {
            payment_intent,
            retry_algorithm_type,
            retry_count: u32::from(retry_count),
            prev_attempt_error_code: last_failure_code,
            static_ladder_progress: ladder_progress,
            remaining_grace_days: u32::try_from((grace_window_end - now).whole_days().max(0))
                .unwrap_or(0),
            remaining_budget: u32::from(retry_config.max_retry_count)
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
        Ok(RetryTiming::Decider) => {
            match revenue_recovery_workflow::get_schedule_time_to_retry_mit_payments(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                dimensions,
                u32::from(retry_count),
            )
            .await
            {
                Some(retry_time) => (retry_time, None, None),
                None => {
                    return NextRetryDecision::not_scheduled(
                        retry_count,
                        RecoveryNoRetryReason::RetriesExhausted,
                    )
                }
            }
        }
        Ok(RetryTiming::NoRetries) => {
            return NextRetryDecision::not_scheduled(retry_count, RecoveryNoRetryReason::Monitoring)
        }
        Ok(RetryTiming::LadderExhausted) => {
            return NextRetryDecision::not_scheduled(
                retry_count,
                RecoveryNoRetryReason::RetriesExhausted,
            )
        }
        Err(error) => {
            logger::error!(
                ?error,
                "No time is available for the next retry of the invoice"
            );
            return NextRetryDecision::not_scheduled(
                retry_count,
                RecoveryNoRetryReason::RetriesExhausted,
            );
        }
    };

    let next_retry = get_next_retry(
        retry_count,
        revenue_recovery_workflow::add_random_delay_to_schedule_time(state, retry_time),
        &card_availability,
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
        next_ladder_progress: next_ladder_progress.filter(|_| next_retry.schedule_time.is_some()),
        assigned_ab_algorithm,
    }
}

/// Where the card of the invoice stands. The merchant picks the card of a retry, so only that card
/// is considered. A failed lookup does not hold the retry back, the merchant is still given a time.
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
    is_existing_invoice: bool,
    retry_count: u16,
    max_hybrid_cascading_retry_count: u32,
) -> StaticLadderProgress {
    match previous_trigger_state {
        Some(ExternalTriggerState {
            next_retry:
                RecoveryNextRetry {
                    reason: Some(RecoveryNoRetryReason::BelowRetryThreshold),
                    ..
                },
            ..
        }) => StaticLadderProgress::seed_for_new_invoice(
            retry_count,
            max_hybrid_cascading_retry_count,
        ),
        Some(trigger_state) => StaticLadderProgress {
            consumed_rungs: trigger_state.consumed_ladder_rungs,
        },
        None if is_existing_invoice => StaticLadderProgress::seed_for_existing_invoice(
            retry_count,
            max_hybrid_cascading_retry_count,
        ),
        None => StaticLadderProgress::seed_for_new_invoice(
            retry_count,
            max_hybrid_cascading_retry_count,
        ),
    }
}

/// Retries made with the card of the invoice since it was first hard declined. A hard decline is
/// about the card, so a report on another card starts over.
fn get_retries_after_hard_decline(
    previous_retries: Option<u16>,
    is_same_card: bool,
    is_hard_declined: bool,
) -> Option<u16> {
    match previous_retries.filter(|_| is_same_card) {
        Some(retries) => Some(retries.saturating_add(1)),
        None => is_hard_declined.then_some(0),
    }
}

/// The standardised error code the next retry time is worked out from: that of this transaction,
/// if it failed.
fn get_last_failure_code(
    transaction_status: RecoveryTransactionStatus,
    recovery_attempt: &RecoveryPaymentAttempt,
) -> Option<common_enums::StandardisedCode> {
    match transaction_status {
        RecoveryTransactionStatus::Failure => recovery_attempt.standardised_error_code,
        RecoveryTransactionStatus::Charged | RecoveryTransactionStatus::PartialCharged => None,
    }
}

/// Why no retry should follow this transaction, before any retry time is worked out.
fn get_no_retry_reason(
    invoice_status: common_enums::IntentStatus,
    retry_algorithm_type: common_enums::RevenueRecoveryAlgorithmType,
    limit_checks: RetryLimitChecks,
) -> Option<RecoveryNoRetryReason> {
    if invoice_status == common_enums::IntentStatus::Succeeded {
        Some(RecoveryNoRetryReason::Recovered)
    } else if retry_algorithm_type == common_enums::RevenueRecoveryAlgorithmType::Monitoring {
        Some(RecoveryNoRetryReason::Monitoring)
    } else if limit_checks.is_retry_budget_spent {
        Some(RecoveryNoRetryReason::RetriesExhausted)
    } else if limit_checks.is_grace_window_over {
        Some(RecoveryNoRetryReason::GracePeriodExceeded)
    } else if limit_checks.is_hard_decline_limit_reached {
        Some(RecoveryNoRetryReason::HardDecline)
    } else if limit_checks.is_below_retry_threshold {
        Some(RecoveryNoRetryReason::BelowRetryThreshold)
    } else {
        None
    }
}

/// The next retry once its time is known, given where the card stands and when the grace window of
/// the invoice closes.
fn get_next_retry(
    retry_count: u16,
    retry_time: PrimitiveDateTime,
    card_availability: &CardAvailability,
    card_wait_buffer: Duration,
    grace_window_end: PrimitiveDateTime,
    now: PrimitiveDateTime,
) -> RecoveryNextRetry {
    let schedule_time = match card_availability {
        CardAvailability::Ready { .. }
        | CardAvailability::NoCard
        | CardAvailability::HardDecline => retry_time,
        CardAvailability::WaitHours(wait_hours) => retry_time.max(
            now.saturating_add(Duration::hours(*wait_hours))
                .saturating_add(card_wait_buffer),
        ),
    };

    if schedule_time >= grace_window_end {
        RecoveryNextRetry::not_scheduled(retry_count, RecoveryNoRetryReason::GracePeriodExceeded)
    } else {
        RecoveryNextRetry::scheduled(retry_count, schedule_time)
    }
}
