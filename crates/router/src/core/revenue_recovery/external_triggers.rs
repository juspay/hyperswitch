use api_models::payments::RecoveryPaymentsCreate;
use base64::Engine;
use common_types::payments::{
    ExternalTriggerState, RecoveryConnector, RecoveryConnectorDetails, RecoveryNextRetry,
    RecoveryNoRetryReason, RecoveryTransactionStatus,
};
use common_utils::{
    consts::{BASE64_ENGINE, BASE64_ENGINE_URL_SAFE_NO_PAD},
    crypto::{self, Encryptable, GenerateDigest},
    encryption::Encryption,
    errors::CustomResult,
    id_type, type_name,
    types::keymanager,
};
use diesel_models::{business_profile::RevenueRecoveryConfig, process_tracker::business_status};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payments::{get_next_retry_for_intent_status, PaymentIntent, PaymentIntentData},
    platform::Platform,
    revenue_recovery::RecoveryPaymentAttempt,
    type_encryption::{crypto_operation, CryptoOperation},
    ApiModelToDieselModelConvertor,
};
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use router_env::{instrument, logger, tracing};
use scheduler::errors as sch_errors;
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
        errors::{self, RouterResult},
        payments::{
            self,
            operations::{
                payment_attempt_record::PaymentAttemptRecord, ValidateStatusForOperation,
            },
        },
        webhooks::{
            self,
            recovery_incoming::{RevenueRecoveryAttempt, RevenueRecoveryInvoice},
        },
    },
    routes::{app::ReqState, lock_utils, metrics, SessionState},
    types::{
        api as api_types, domain,
        storage::{
            self, revenue_recovery as pcr, revenue_recovery_redis_operation::RedisTokenManager,
        },
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

/// The recovery state of an invoice after a merchant-initiated report: its next retry and what the
/// report after it needs, with the A/B arm the decision assigned. The report markers are left for
/// the caller.
#[allow(clippy::too_many_arguments)]
async fn decide_reported_retry(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    retry_config: &RevenueRecoveryConfig,
    request: &RecoveryPaymentsCreate,
    recovery_attempt: &RecoveryPaymentAttempt,
    payment_intent: &PaymentIntent,
    previous_trigger_state: Option<&ExternalTriggerState>,
    is_existing_invoice: bool,
    is_same_card: bool,
) -> (
    ExternalTriggerState,
    Option<common_enums::RevenueRecoveryABAlgorithm>,
) {
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
        previous_trigger_state,
        is_existing_invoice,
        retry_count,
        max_hybrid_cascading_retry_count,
    );
    let last_failure_code = get_last_failure_code(request.transaction_status, recovery_attempt);
    let card_availability =
        get_card_availability(state, request.connector_customer_id.peek(), payment_intent).await;
    let retries_after_hard_decline = get_retries_after_hard_decline(
        previous_trigger_state.and_then(|trigger_state| trigger_state.retries_after_hard_decline),
        is_same_card,
        matches!(card_availability, CardAvailability::HardDecline),
    );

    let decision = Box::pin(decide_next_retry(
        state,
        platform,
        profile,
        NextRetryInputs {
            payment_intent,
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

    (
        ExternalTriggerState {
            last_failure_code,
            consumed_ladder_rungs: decision
                .next_ladder_progress
                .unwrap_or(ladder_progress)
                .consumed_rungs,
            retries_after_hard_decline,
            next_retry: decision.next_retry,
            last_report_id: None,
            last_transaction_created_at: None,
        },
        decision.assigned_ab_algorithm,
    )
}

/// A charged card is healthy again, so its retry history starts over, as it does for the retries
/// Hyperswitch makes.
async fn reset_card_history_after_a_charge(state: &SessionState, request: &RecoveryPaymentsCreate) {
    let Some(payment_method_data) = request.payment_method_data.as_ref() else {
        return;
    };
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
                        payment_method_data
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
            RecoveryNoRetryReason::GracePeriodExpired,
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

/// Keeps what the next report of the merchant needs on the invoice, through the update-intent
/// operation. A paid or cancelled invoice has ended, so it has no next retry to keep.
async fn save_trigger_state(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    payment_intent: PaymentIntent,
    trigger_state: ExternalTriggerState,
    assigned_ab_algorithm: Option<common_enums::RevenueRecoveryABAlgorithm>,
) -> RouterResult<()> {
    if matches!(
        payment_intent.status,
        common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
    ) {
        return Ok(());
    }
    let Some(mut feature_metadata) = payment_intent
        .feature_metadata
        .map(ApiModelToDieselModelConvertor::convert_back)
    else {
        return Ok(());
    };
    let Some(recovery_metadata) = feature_metadata.revenue_recovery.as_mut() else {
        return Ok(());
    };
    recovery_metadata.external_trigger_state = Some(trigger_state);
    recovery_metadata.revenue_recovery_ab_routing =
        assigned_ab_algorithm.or(recovery_metadata.revenue_recovery_ab_routing);

    Box::pin(payments::payments_intent_operation_core::<
        api_types::PaymentUpdateIntent,
        _,
        _,
        PaymentIntentData<api_types::PaymentUpdateIntent>,
    >(
        state,
        state.get_req_state(),
        platform.clone(),
        profile.clone(),
        payments::operations::PaymentUpdateIntent,
        api_models::payments::PaymentsUpdateIntentRequest::update_feature_metadata_and_active_attempt_with_api(
            feature_metadata,
            api_models::enums::UpdateActiveAttempt::NoAction,
        ),
        payment_intent.id,
        hyperswitch_domain_models::payments::HeaderPayload::default(),
    ))
    .await
    .map(|_| ())
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
/// it succeeded. Only a retry that was scheduled is counted, which leaves out the first transaction
/// of an invoice and the merchant's own retries before recovery starts, and partial charges are not
/// counted, as they are not for the retries Hyperswitch makes.
fn get_retry_outcome(
    previous_trigger_state: Option<&ExternalTriggerState>,
    transaction_status: RecoveryTransactionStatus,
) -> Option<(Option<common_enums::StandardisedCode>, bool)> {
    let trigger_state = previous_trigger_state
        .filter(|trigger_state| trigger_state.next_retry.schedule_time.is_some())?;

    match transaction_status {
        RecoveryTransactionStatus::Failure => Some((trigger_state.last_failure_code, false)),
        RecoveryTransactionStatus::Charged => Some((trigger_state.last_failure_code, true)),
        RecoveryTransactionStatus::PartialCharged => None,
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
    } else if matches!(
        invoice_status,
        common_enums::IntentStatus::Cancelled | common_enums::IntentStatus::CancelledPostCapture
    ) {
        Some(RecoveryNoRetryReason::Cancelled)
    } else if retry_algorithm_type == common_enums::RevenueRecoveryAlgorithmType::Monitoring {
        Some(RecoveryNoRetryReason::Monitoring)
    } else if limit_checks.is_retry_budget_spent {
        Some(RecoveryNoRetryReason::RetriesExhausted)
    } else if limit_checks.is_grace_window_over {
        Some(RecoveryNoRetryReason::GracePeriodExpired)
    } else if limit_checks.is_hard_decline_limit_reached {
        Some(RecoveryNoRetryReason::HardDeclined)
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
        RecoveryNextRetry::not_scheduled(retry_count, RecoveryNoRetryReason::GracePeriodExpired)
    } else {
        RecoveryNextRetry::scheduled(retry_count, schedule_time)
    }
}

/// Version of the request stored in a recovery report's tracking data.
const REPORT_REQUEST_VERSION: u8 = 1;

/// Seconds before a report runs again after a temporary error, by how many runs have failed. They
/// end well inside the 15 minutes after which the merchant falls back to its own retry timing.
const REPORT_RETRY_DELAYS_IN_SECONDS: [i64; 5] = [15, 30, 60, 120, 240];

/// How long after it was received a report keeps waiting for its invoice's lock.
const REPORT_LOCK_WAIT_IN_MINUTES: i64 = 10;

/// Seconds between tries for an invoice lock held by another report.
const REPORT_LOCK_RETRY_IN_SECONDS: i64 = 5;

/// How a recovery report ended.
#[derive(Clone, Copy, Debug)]
enum ReportOutcome {
    /// Recorded, and the next retry is scheduled.
    RetryScheduled,
    /// Recorded, and a limit of the profile stops the retries.
    LimitsExhausted(RecoveryNoRetryReason),
    /// Recorded, and no retry follows, for a reason that needs no webhook.
    NoWebhook(RecoveryNoRetryReason),
    /// Recorded and decided, with the next retry's reason, but the profile takes no webhooks.
    WebhookDisabled(Option<RecoveryNoRetryReason>),
    /// Made with the customer, so only recorded.
    CustomerInitiated,
    /// A newer report of the invoice was applied first, so only recorded.
    Stale,
    /// The transaction is recorded on the invoice, but not by this report.
    Duplicate,
    /// The invoice is closed, so nothing is recorded.
    InvoiceClosed,
    /// The invoice does not show every attempt recorded on it, so nothing is decided.
    InvoiceOutOfSync,
}

impl ReportOutcome {
    fn from_next_retry(next_retry: &RecoveryNextRetry) -> Self {
        match next_retry.reason {
            None => Self::RetryScheduled,
            Some(
                reason @ (RecoveryNoRetryReason::RetriesExhausted
                | RecoveryNoRetryReason::GracePeriodExpired
                | RecoveryNoRetryReason::HardDeclined),
            ) => Self::LimitsExhausted(reason),
            Some(
                reason @ (RecoveryNoRetryReason::Recovered
                | RecoveryNoRetryReason::Monitoring
                | RecoveryNoRetryReason::Cancelled
                | RecoveryNoRetryReason::BelowRetryThreshold),
            ) => Self::NoWebhook(reason),
        }
    }

    /// The webhook that tells the merchant the decision, for a decision that has one.
    fn webhook_event(self) -> Option<common_enums::EventType> {
        match self {
            Self::RetryScheduled => Some(common_enums::EventType::RecoveryRetryScheduled),
            Self::LimitsExhausted(_) => Some(common_enums::EventType::RecoveryLimitsExhausted),
            Self::NoWebhook(_)
            | Self::WebhookDisabled(_)
            | Self::CustomerInitiated
            | Self::Stale
            | Self::Duplicate
            | Self::InvoiceClosed
            | Self::InvoiceOutOfSync => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::RetryScheduled => "retry_scheduled",
            Self::LimitsExhausted(_) => "limits_exhausted",
            Self::NoWebhook(_) => "no_webhook",
            Self::WebhookDisabled(_) => "webhook_disabled",
            Self::CustomerInitiated => "customer_initiated",
            Self::Stale => "stale",
            Self::Duplicate => "duplicate",
            Self::InvoiceClosed => "invoice_closed",
            Self::InvoiceOutOfSync => "invoice_out_of_sync",
        }
    }

    fn reason(self) -> Option<RecoveryNoRetryReason> {
        match self {
            Self::LimitsExhausted(reason) | Self::NoWebhook(reason) => Some(reason),
            Self::WebhookDisabled(reason) => reason,
            Self::RetryScheduled
            | Self::CustomerInitiated
            | Self::Stale
            | Self::Duplicate
            | Self::InvoiceClosed
            | Self::InvoiceOutOfSync => None,
        }
    }

    fn business_status(self) -> &'static str {
        match self {
            Self::RetryScheduled => business_status::RECOVERY_REQUEST_RETRY_SCHEDULED,
            Self::LimitsExhausted(_) => business_status::RECOVERY_REQUEST_LIMITS_EXHAUSTED,
            Self::NoWebhook(_) => business_status::RECOVERY_REQUEST_NO_WEBHOOK,
            Self::WebhookDisabled(_) => business_status::RECOVERY_REQUEST_WEBHOOK_DISABLED,
            Self::CustomerInitiated => business_status::RECOVERY_REQUEST_CUSTOMER_INITIATED,
            Self::Stale => business_status::RECOVERY_REQUEST_STALE,
            Self::Duplicate => business_status::RECOVERY_REQUEST_DUPLICATE,
            Self::InvoiceClosed => business_status::RECOVERY_REQUEST_INVOICE_CLOSED,
            Self::InvoiceOutOfSync => business_status::RECOVERY_REQUEST_INVOICE_OUT_OF_SYNC,
        }
    }
}

fn reason_name(reason: RecoveryNoRetryReason) -> &'static str {
    match reason {
        RecoveryNoRetryReason::Recovered => "recovered",
        RecoveryNoRetryReason::RetriesExhausted => "retries_exhausted",
        RecoveryNoRetryReason::GracePeriodExpired => "grace_period_expired",
        RecoveryNoRetryReason::HardDeclined => "hard_declined",
        RecoveryNoRetryReason::Monitoring => "monitoring",
        RecoveryNoRetryReason::Cancelled => "cancelled",
        RecoveryNoRetryReason::BelowRetryThreshold => "below_retry_threshold",
    }
}

/// Accepts a transaction the merchant reported, unless its invoice takes no more transactions, and
/// queues it for the recovery request workflow, which records it. The workflow checks the invoice
/// again, as it can close while the report waits.
pub(super) async fn accept_report(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &RecoveryPaymentsCreate,
    request_body: &Secret<String>,
    connector: RecoveryConnector,
) -> RouterResult<api_models::payments::RecoveryReportAccepted> {
    let invoice = RevenueRecoveryInvoice::find_custom_recovery_intent(
        request.clone(),
        state,
        req_state,
        platform,
        profile,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to read the invoice of the recovery report")
    .inspect_err(|_| record_received_report("failed"))?;

    if invoice.is_none() && request.payment_method_data.is_none() {
        record_received_report("card_missing");
        return Err(report!(api::card_needed_for_a_new_invoice()));
    }
    if let Some(invoice) = invoice.filter(|invoice| {
        PaymentAttemptRecord
            .validate_status_for_operation(invoice.status)
            .is_err()
    }) {
        record_received_report("invoice_closed");
        logger::info!(
            invoice_status = %invoice.status,
            "Recovery report refused, as its invoice takes no more transactions"
        );
        return Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "The invoice is `{}` and takes no more transactions",
                invoice.status
            ),
        }));
    }

    queue_report(state, platform, profile, request, request_body, connector)
        .await
        .inspect_err(|error| {
            record_received_report(match error.current_context() {
                errors::ApiErrorResponse::GenericDuplicateError { .. } => "duplicate",
                _ => "failed",
            })
        })?;
    record_received_report("queued");

    Ok(api_models::payments::RecoveryReportAccepted {
        merchant_reference_id: request.merchant_reference_id.clone(),
        connector_transaction_id: request.connector_transaction_id.clone(),
    })
}

fn record_received_report(outcome: &'static str) {
    metrics::REVENUE_RECOVERY_REPORT_RECEIVED
        .add(1, router_env::metric_attributes!(("outcome", outcome)));
}

/// Queues a transaction the merchant reported, to be recorded and answered by the recovery request
/// workflow, under the report's id, which its job id is built from. A transaction already reported
/// in the profile is refused, as its job id is taken, whatever became of that job. The job keeps
/// `request_body`, the request exactly as the merchant sent it, and reads `request` back from it.
async fn queue_report(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &RecoveryPaymentsCreate,
    request_body: &Secret<String>,
    connector: RecoveryConnector,
) -> RouterResult<()> {
    let report_id = get_report_id(
        profile.get_id(),
        &request.merchant_reference_id,
        &request.connector_transaction_id,
    )?;
    let received_at = common_utils::date_time::now();
    let encrypted_request = encrypt_report_request(
        state,
        platform.get_processor().get_key_store(),
        request_body,
    )
    .await?;
    let tracking_data = pcr::RecoveryRequestTrackingData {
        report_id: report_id.clone(),
        merchant_id: platform.get_processor().get_account().get_id().clone(),
        profile_id: profile.get_id().clone(),
        merchant_reference_id: request.merchant_reference_id.clone(),
        connector_transaction_id: request.connector_transaction_id.clone(),
        connector,
        received_at,
        request_version: REPORT_REQUEST_VERSION,
        encrypted_request,
    };
    let runner = storage::ProcessTrackerRunner::RecoveryRequestWorkflow;
    let task = super::EXTERNAL_TRIGGER_HANDLER;
    let process = storage::ProcessTrackerNew::new(
        format!("{runner}_{task}_{report_id}"),
        task,
        runner,
        ["REVENUE_RECOVERY"],
        tracking_data,
        None,
        received_at,
        common_types::consts::API_VERSION,
        state.conf.application_source,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct the recovery report task")?;
    state.store.insert_process(process).await.map_err(|error| {
        if error.current_context().is_db_unique_violation() {
            error.change_context(errors::ApiErrorResponse::GenericDuplicateError {
                message: format!(
                    "The transaction {} is already reported for the invoice {}",
                    request.connector_transaction_id,
                    request.merchant_reference_id.get_string_repr()
                ),
            })
        } else {
            error
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to queue the recovery report")
        }
    })?;
    metrics::TASKS_ADDED_COUNT.add(
        1,
        router_env::metric_attributes!(("flow", "RecoveryRequest")),
    );
    logger::info!(
        report_id = %report_id,
        merchant_reference_id = %request.merchant_reference_id.get_string_repr(),
        connector_transaction_id = %request.connector_transaction_id,
        "Recovery report queued"
    );
    Ok(())
}

/// The id of a report, the same for every report of a transaction: the profile, the invoice and the
/// transaction, in that order, hashed with SHA-256 so that its job id always fits the job id column.
/// The format must not change, nor the runner and task names its job id starts with, or a
/// transaction reported before the change could be reported again.
fn get_report_id(
    profile_id: &id_type::ProfileId,
    merchant_reference_id: &id_type::PaymentReferenceId,
    connector_transaction_id: &str,
) -> RouterResult<String> {
    let key = format!(
        "{}:{}:{}",
        profile_id.get_string_repr(),
        merchant_reference_id.get_string_repr(),
        connector_transaction_id
    );
    let digest = crypto::Sha256
        .generate_digest(key.as_bytes())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to hash the recovery report's id")?;
    Ok(BASE64_ENGINE_URL_SAFE_NO_PAD.encode(digest))
}

/// The request body encrypted with the merchant's key and base64 encoded.
async fn encrypt_report_request(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    request_body: &Secret<String>,
) -> RouterResult<String> {
    let encrypted: Encryptable<Secret<String>> = crypto_operation(
        &state.into(),
        type_name!(pcr::RecoveryRequestTrackingData),
        CryptoOperation::Encrypt(request_body.clone()),
        keymanager::Identifier::Merchant(key_store.merchant_id.clone()),
        key_store.key.get_inner().peek(),
    )
    .await
    .and_then(|operation| operation.try_into_operation())
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to encrypt the recovery report")?;
    Ok(BASE64_ENGINE.encode(encrypted.into_encrypted().peek()))
}

/// The stored request decrypted, as JSON.
async fn decrypt_report_request(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    encrypted_request: Vec<u8>,
) -> RouterResult<String> {
    let decrypted: Encryptable<Secret<String>> = crypto_operation(
        &state.into(),
        type_name!(pcr::RecoveryRequestTrackingData),
        CryptoOperation::Decrypt(Encryption::new(Secret::new(encrypted_request))),
        keymanager::Identifier::Merchant(key_store.merchant_id.clone()),
        key_store.key.get_inner().peek(),
    )
    .await
    .and_then(|operation| operation.try_into_operation())
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to decrypt the recovery report")?;
    Ok(decrypted.into_inner().expose())
}

/// Records a transaction the merchant reported, decides the next retry of its invoice and tells the
/// merchant by webhook. Called by the recovery request workflow: every outcome finishes the job here,
/// apart from a wait for the invoice's lock, which puts the job back in the queue, and only a
/// temporary error is returned, for the workflow to run the report again.
#[instrument(
    skip_all,
    fields(
        process_id = %process.id,
        report_id = %tracking_data.report_id,
        merchant_id = %tracking_data.merchant_id.get_string_repr(),
        profile_id = %tracking_data.profile_id.get_string_repr(),
        merchant_reference_id = %tracking_data.merchant_reference_id.get_string_repr(),
        connector_transaction_id = %tracking_data.connector_transaction_id,
        payment_id = tracing::field::Empty,
        attempt_id = tracing::field::Empty,
    )
)]
pub(crate) async fn process_reported_transaction(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: pcr::RecoveryRequestTrackingData,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let now = common_utils::date_time::now();
    if tracking_data.request_version != REPORT_REQUEST_VERSION {
        logger::error!(
            request_version = tracking_data.request_version,
            "The stored recovery report has a version this release does not read"
        );
        return finish_unreadable_report(state, process).await;
    }
    let Ok(encrypted_request) = BASE64_ENGINE.decode(&tracking_data.encrypted_request) else {
        logger::error!("The stored recovery report is not base64");
        return finish_unreadable_report(state, process).await;
    };

    let (platform, profile) = load_report_context(state, &tracking_data)
        .await
        .map_err(|error| report_step_failed("load", error))?;
    let body = decrypt_report_request(
        state,
        platform.get_processor().get_key_store(),
        encrypted_request,
    )
    .await
    .map_err(|error| report_step_failed("decrypt", error))?;
    let request = match serde_json::from_str::<RecoveryPaymentsCreate>(&body) {
        Ok(request) => request,
        // Logged without its message, which can quote a value from the body.
        Err(error) => {
            logger::error!(
                category = ?error.classify(),
                line = error.line(),
                column = error.column(),
                "The stored recovery report cannot be read"
            );
            return finish_unreadable_report(state, process).await;
        }
    };
    let Some(retry_config) = profile.revenue_recovery_config.clone() else {
        logger::error!(
            "The profile has no revenue recovery config, so the report cannot be applied"
        );
        return finish_failed_report(state, process).await;
    };
    // A value sent in the request takes priority over the one on the profile.
    let request = RecoveryPaymentsCreate {
        enable_partial_authorization: request
            .enable_partial_authorization
            .or(Some(retry_config.enable_partial_authorization.into())),
        ..request
    };

    let merchant_id = tracking_data.merchant_id.clone();
    let invoice_lock = LockAction::Hold {
        input: LockingInput {
            unique_locking_key: tracking_data
                .merchant_reference_id
                .get_string_repr()
                .to_owned(),
            api_identifier: lock_utils::ApiIdentifier::Payments,
            override_lock_retries: Some(1),
        },
    };
    if let Err(error) = invoice_lock
        .clone()
        .perform_locking_action(state, merchant_id.clone())
        .await
    {
        return match error.current_context() {
            errors::ApiErrorResponse::ResourceBusy => {
                wait_for_invoice_lock(state, process, &tracking_data, now).await
            }
            _ => Err(report_step_failed("lock", error)),
        };
    }
    if process.retry_count == 0 {
        metrics::REVENUE_RECOVERY_REPORT_QUEUE_DELAY
            .record((now - tracking_data.received_at).as_seconds_f64(), &[]);
    }

    let outcome = Box::pin(apply_reported_transaction(
        state,
        &tracking_data,
        &platform,
        &profile,
        &request,
        &retry_config,
    ))
    .await;
    if let Err(error) = invoice_lock.free_lock_action(state, merchant_id).await {
        logger::error!(?error, "Failed to release the lock of the invoice");
    }
    finish_report(state, process, outcome?).await
}

/// The merchant and profile of a report, and the platform to act as, with the merchant's API key
/// as the initiator, as when the API acted on the request itself.
async fn load_report_context(
    state: &SessionState,
    tracking_data: &pcr::RecoveryRequestTrackingData,
) -> error_stack::Result<(Platform, domain::Profile), errors::StorageError> {
    let db = &*state.store;
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            &tracking_data.merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await?;
    let merchant_account = db
        .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
        .await?;
    let profile = db
        .find_business_profile_by_profile_id(&key_store, &tracking_data.profile_id)
        .await?;
    let initiator = domain::Initiator::Api {
        merchant_id: merchant_account.get_id().clone(),
        merchant_account_type: merchant_account.merchant_account_type,
        publishable_key: merchant_account.publishable_key.clone(),
    };
    let platform = Platform::new(
        merchant_account.clone(),
        key_store.clone(),
        merchant_account,
        key_store,
        Some(initiator),
    );
    Ok((platform, profile))
}

/// Records the reported transaction, decides the next retry and tells the merchant, under the
/// invoice's lock. An error is temporary, and the report runs again.
async fn apply_reported_transaction(
    state: &SessionState,
    tracking_data: &pcr::RecoveryRequestTrackingData,
    platform: &Platform,
    profile: &domain::Profile,
    request: &RecoveryPaymentsCreate,
    retry_config: &RevenueRecoveryConfig,
) -> Result<ReportOutcome, sch_errors::ProcessTrackerError> {
    let req_state = state.get_req_state();
    let invoice = api::get_or_create_invoice(state, &req_state, platform, profile, request)
        .await
        .map_err(|error| report_step_failed("invoice", error))?;
    tracing::Span::current().record("payment_id", invoice.payment_id.get_string_repr());

    let previous_recovery_metadata = invoice
        .feature_metadata
        .as_ref()
        .and_then(|feature_metadata| feature_metadata.revenue_recovery.as_ref());
    let previous_trigger_state = previous_recovery_metadata
        .and_then(|recovery_metadata| recovery_metadata.external_trigger_state.clone());
    let report_id = tracking_data.report_id.as_str();

    let attempts = RevenueRecoveryAttempt::find_reported_attempt(
        request, state, &req_state, platform, profile, &invoice,
    )
    .await
    .map_err(|error| report_step_failed("record", error))?;
    let recovery_attempt = match attempts.reported {
        Some(attempt) => {
            tracing::Span::current().record("attempt_id", attempt.attempt_id.get_string_repr());
            let recorded_by = attempt
                .feature_metadata
                .as_ref()
                .and_then(|feature_metadata| feature_metadata.revenue_recovery.as_ref())
                .and_then(|recovery_data| recovery_data.report_id.as_deref());
            if recorded_by != Some(report_id) {
                return Ok(ReportOutcome::Duplicate);
            }
            if let Some(saved_state) = previous_trigger_state
                .as_ref()
                .filter(|trigger_state| trigger_state.last_report_id.as_deref() == Some(report_id))
            {
                return Box::pin(send_next_retry(
                    state,
                    platform,
                    profile,
                    tracking_data,
                    &invoice.payment_id,
                    get_next_retry_for_intent_status(invoice.status, saved_state.next_retry),
                    true,
                ))
                .await;
            }
            let recorded_on_invoice = previous_recovery_metadata
                .map(|recovery_metadata| usize::from(recovery_metadata.total_retry_count));
            if recorded_on_invoice != Some(attempts.counted_as_retries) {
                logger::error!(
                    ?recorded_on_invoice,
                    attempts_counted_as_retries = ?attempts.counted_as_retries,
                    "The invoice does not show every attempt recorded on it"
                );
                return Ok(ReportOutcome::InvoiceOutOfSync);
            }
            attempt
        }
        None => {
            if PaymentAttemptRecord
                .validate_status_for_operation(invoice.status)
                .is_err()
            {
                return Ok(ReportOutcome::InvoiceClosed);
            }
            let attempt = RevenueRecoveryAttempt::record_reported_attempt(
                request,
                state,
                &req_state,
                platform,
                profile,
                &invoice,
                &RecoveryConnectorDetails::ReportedByMerchant {
                    connector: tracking_data.connector.into(),
                },
                tracking_data.report_id.clone(),
            )
            .await
            .map_err(|error| report_step_failed("record", error))?;
            tracing::Span::current().record("attempt_id", attempt.attempt_id.get_string_repr());
            attempt
        }
    };

    reset_card_history_after_a_charge(state, request).await;

    if !hyperswitch_domain_models::revenue_recovery::is_merchant_initiated(
        request.customer_present,
        request.setup_future_usage,
    ) {
        return Ok(ReportOutcome::CustomerInitiated);
    }
    if let Some(last_transaction_created_at) = previous_trigger_state
        .as_ref()
        .and_then(|trigger_state| trigger_state.last_transaction_created_at)
        .filter(|last_transaction_created_at| {
            *last_transaction_created_at > request.transaction_created_at
        })
    {
        logger::error!(
            ?last_transaction_created_at,
            transaction_created_at = ?request.transaction_created_at,
            "The invoice was last decided on a newer transaction, so this one is only recorded"
        );
        return Ok(ReportOutcome::Stale);
    }

    let is_existing_invoice = attempts.earlier > 0;
    let is_same_card = request
        .payment_method_data
        .as_ref()
        .is_some_and(|payment_method_data| {
            attempts.previous_retry_card.as_ref()
                == Some(
                    payment_method_data
                        .primary_processor_payment_method_token
                        .peek(),
                )
        });
    let payment_intent = read_invoice(state, platform, profile, &invoice.payment_id)
        .await
        .map_err(|error| report_step_failed("invoice", error))?;
    let (trigger_state, assigned_ab_algorithm) = Box::pin(decide_reported_retry(
        state,
        platform,
        profile,
        retry_config,
        request,
        &recovery_attempt,
        &payment_intent,
        previous_trigger_state.as_ref(),
        is_existing_invoice,
        is_same_card,
    ))
    .await;
    let next_retry = trigger_state.next_retry;
    let trigger_state = ExternalTriggerState {
        last_report_id: Some(tracking_data.report_id.clone()),
        last_transaction_created_at: Some(request.transaction_created_at),
        ..trigger_state
    };
    save_trigger_state(
        state,
        platform,
        profile,
        payment_intent,
        trigger_state,
        assigned_ab_algorithm,
    )
    .await
    .map_err(|error| report_step_failed("save_state", error))?;

    // The invoice's last error code is for its last card, so a retry on another card is not counted.
    if let Some((failure_code, success)) =
        get_retry_outcome(previous_trigger_state.as_ref(), request.transaction_status)
            .filter(|_| is_same_card)
    {
        RetryOutcomeEvent::from_retry_outcome(recovery_attempt.created_at, failure_code, success)
            .record(state)
            .await;
    }
    logger::info!(
        retry_count = next_retry.retry_count,
        schedule_time = ?next_retry.schedule_time,
        reason = ?next_retry.reason,
        "Next retry of the invoice decided"
    );
    Box::pin(send_next_retry(
        state,
        platform,
        profile,
        tracking_data,
        &invoice.payment_id,
        next_retry,
        false,
    ))
    .await
}

/// Tells the merchant the next retry of the invoice by webhook, for a decision that has one, and
/// returns how the report ends. The webhook's event is keyed by the report. A later run answering
/// from the decision an earlier run saved (`is_saved_decision`) sends it only if that run did not.
async fn send_next_retry(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    tracking_data: &pcr::RecoveryRequestTrackingData,
    payment_id: &id_type::GlobalPaymentId,
    next_retry: RecoveryNextRetry,
    is_saved_decision: bool,
) -> Result<ReportOutcome, sch_errors::ProcessTrackerError> {
    let outcome = ReportOutcome::from_next_retry(&next_retry);
    let Some(event_type) = outcome.webhook_event() else {
        return Ok(outcome);
    };
    let webhook_recipient =
        webhooks::utils::resolve_webhook_recipient_from_created_by(state, platform, profile, None)
            .await
            .map_err(|error| report_step_failed("webhook", error))?;
    let idempotent_event_id = webhooks::utils::get_idempotent_event_id(
        &tracking_data.report_id,
        event_type,
        common_enums::WebhookDeliveryAttempt::InitialAttempt,
    )
    .map_err(|error| report_step_failed("webhook", error))?;
    let webhook_url = webhook_recipient
        .profile
        .get_webhook_url_from_profile()
        .change_context(errors::WebhooksFlowError::MerchantWebhookUrlNotConfigured);
    if webhooks::utils::is_outgoing_webhook_disabled(
        state,
        &webhook_url,
        &webhook_recipient.profile,
        &idempotent_event_id,
    ) {
        return Ok(ReportOutcome::WebhookDisabled(next_retry.reason));
    }
    if is_saved_decision
        && is_webhook_queued(state, platform, &webhook_recipient, &idempotent_event_id).await?
    {
        logger::info!(
            %event_type,
            %idempotent_event_id,
            "Recovery webhook already queued by an earlier run of the report"
        );
        return Ok(outcome);
    }

    Box::pin(webhooks::create_event_and_trigger_outgoing_webhook(
        state.clone(),
        platform.clone(),
        event_type,
        common_enums::EventClass::RevenueRecovery,
        tracking_data.report_id.clone(),
        common_enums::EventObjectType::RecoveryRetryDetails,
        api_models::webhooks::OutgoingWebhookContent::RecoveryRetryDetails(Box::new(
            api_models::payments::RecoveryRetryWebhookDetails {
                id: payment_id.clone(),
                merchant_reference_id: tracking_data.merchant_reference_id.clone(),
                next_retry,
            },
        )),
        tracking_data.received_at,
        webhook_recipient,
    ))
    .await
    .map_err(|error| report_step_failed("webhook", error))?;
    metrics::REVENUE_RECOVERY_REPORT_TIME_TO_WEBHOOK.record(
        (common_utils::date_time::now() - tracking_data.received_at).as_seconds_f64(),
        router_env::metric_attributes!(("outcome", outcome.name())),
    );
    logger::info!(%event_type, %idempotent_event_id, "Recovery webhook queued");
    Ok(outcome)
}

/// Whether an earlier run of the report already queued its webhook, which it stored as an event.
async fn is_webhook_queued(
    state: &SessionState,
    platform: &Platform,
    webhook_recipient: &webhooks::utils::WebhookRecipientContext,
    idempotent_event_id: &str,
) -> Result<bool, sch_errors::ProcessTrackerError> {
    match state
        .store
        .find_event_by_merchant_id_idempotent_event_id(
            platform.get_provider().get_account().get_id(),
            idempotent_event_id,
            &webhook_recipient.key_store,
        )
        .await
    {
        Ok(_) => Ok(true),
        Err(error) if error.current_context().is_db_not_found() => Ok(false),
        Err(error) => Err(report_step_failed("webhook", error)),
    }
}

/// The invoice as it is now, read through the get intent operation.
async fn read_invoice(
    state: &SessionState,
    platform: &Platform,
    profile: &domain::Profile,
    payment_id: &id_type::GlobalPaymentId,
) -> RouterResult<PaymentIntent> {
    let (payment_data, _, _) = Box::pin(payments::payments_intent_operation_core::<
        api_types::PaymentGetIntent,
        _,
        _,
        PaymentIntentData<api_types::PaymentGetIntent>,
    >(
        state,
        state.get_req_state(),
        platform.clone(),
        profile.clone(),
        payments::operations::PaymentGetIntent,
        api_models::payments::PaymentsGetIntentRequest {
            id: payment_id.clone(),
        },
        payment_id.clone(),
        hyperswitch_domain_models::payments::HeaderPayload::default(),
    ))
    .await?;
    Ok(payment_data.payment_intent)
}

/// Puts a report back in the queue while another report of its invoice holds the lock, without
/// spending its retries, until the wait runs out.
async fn wait_for_invoice_lock(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: &pcr::RecoveryRequestTrackingData,
    now: PrimitiveDateTime,
) -> Result<(), sch_errors::ProcessTrackerError> {
    if now < tracking_data.received_at + Duration::minutes(REPORT_LOCK_WAIT_IN_MINUTES) {
        logger::info!("The invoice is held by another report; trying again shortly");
        state
            .store
            .as_scheduler()
            .update_process(
                process.clone(),
                storage::ProcessTrackerUpdate::StatusRetryUpdate {
                    status: common_enums::ProcessTrackerStatus::Pending,
                    retry_count: process.retry_count,
                    schedule_time: now + Duration::seconds(REPORT_LOCK_RETRY_IN_SECONDS),
                },
            )
            .await?;
        return Ok(());
    }
    metrics::REVENUE_RECOVERY_REPORT_STEP_FAILED
        .add(1, router_env::metric_attributes!(("step", "lock")));
    logger::error!("The invoice stayed held by another report for too long");
    state
        .store
        .as_scheduler()
        .finish_process_with_business_status(
            process.clone(),
            business_status::RECOVERY_REQUEST_RECORDING_FAILED,
        )
        .await?;
    record_report_outcome("retries_exceeded", None);
    Ok(())
}

/// Finishes a report's job with its outcome, counting and logging it.
async fn finish_report(
    state: &SessionState,
    process: &storage::ProcessTracker,
    outcome: ReportOutcome,
) -> Result<(), sch_errors::ProcessTrackerError> {
    state
        .store
        .as_scheduler()
        .finish_process_with_business_status(process.clone(), outcome.business_status())
        .await?;
    record_report_outcome(outcome.name(), outcome.reason());
    match outcome {
        ReportOutcome::Stale | ReportOutcome::Duplicate | ReportOutcome::InvoiceClosed => {
            logger::warn!(
                outcome = outcome.name(),
                "Recovery report ended without a decision"
            )
        }
        ReportOutcome::WebhookDisabled(reason) => logger::warn!(
            ?reason,
            "Recovery report decided, but the merchant is not told: outgoing webhooks are off or \
             the profile has no webhook URL"
        ),
        ReportOutcome::InvoiceOutOfSync => logger::error!(
            outcome = outcome.name(),
            "Recovery report not decided, as its invoice does not show every attempt recorded on \
             it; the invoice needs a manual check"
        ),
        ReportOutcome::RetryScheduled
        | ReportOutcome::LimitsExhausted(_)
        | ReportOutcome::NoWebhook(_)
        | ReportOutcome::CustomerInitiated => logger::info!(
            outcome = outcome.name(),
            reason = ?outcome.reason(),
            "Recovery report processed"
        ),
    }
    Ok(())
}

/// Ends a report whose stored request cannot be read, which no later run can read either.
async fn finish_unreadable_report(
    state: &SessionState,
    process: &storage::ProcessTracker,
) -> Result<(), sch_errors::ProcessTrackerError> {
    metrics::REVENUE_RECOVERY_REPORT_STEP_FAILED
        .add(1, router_env::metric_attributes!(("step", "decode")));
    finish_failed_report(state, process).await
}

/// Ends a report's job for good, for an error a later run would hit again.
pub(crate) async fn finish_failed_report(
    state: &SessionState,
    process: &storage::ProcessTracker,
) -> Result<(), sch_errors::ProcessTrackerError> {
    state
        .store
        .as_scheduler()
        .finish_process_with_business_status(
            process.clone(),
            business_status::RECOVERY_REQUEST_RECORDING_FAILED,
        )
        .await?;
    record_report_outcome("failed", None);
    Ok(())
}

/// Runs a report again after a temporary error, sooner first, until its retries run out.
pub(crate) async fn retry_report(
    state: &SessionState,
    process: storage::ProcessTracker,
    error: sch_errors::ProcessTrackerError,
) -> CustomResult<(), sch_errors::ProcessTrackerError> {
    let delay_in_seconds = usize::try_from(process.retry_count)
        .ok()
        .and_then(|failed_runs| REPORT_RETRY_DELAYS_IN_SECONDS.get(failed_runs).copied());
    match delay_in_seconds {
        Some(delay_in_seconds) => {
            logger::warn!(
                ?error,
                retry_count = process.retry_count,
                delay_in_seconds,
                "Recovery report failed; running it again"
            );
            state
                .store
                .as_scheduler()
                .retry_process(
                    process,
                    common_utils::date_time::now() + Duration::seconds(delay_in_seconds),
                )
                .await
                .change_context(sch_errors::ProcessTrackerError::ProcessUpdateFailed)
        }
        None => {
            logger::error!(
                ?error,
                retry_count = process.retry_count,
                "Recovery report failed and has no retries left"
            );
            state
                .store
                .as_scheduler()
                .finish_process_with_business_status(
                    process,
                    business_status::RECOVERY_REQUEST_RECORDING_FAILED,
                )
                .await
                .change_context(sch_errors::ProcessTrackerError::ProcessUpdateFailed)?;
            record_report_outcome("retries_exceeded", None);
            Ok(())
        }
    }
}

fn record_report_outcome(outcome: &'static str, reason: Option<RecoveryNoRetryReason>) {
    metrics::REVENUE_RECOVERY_REPORT_OUTCOME.add(
        1,
        router_env::metric_attributes!(
            ("outcome", outcome),
            ("reason", reason.map_or("none", reason_name))
        ),
    );
}

/// Counts and logs a temporary error in a step of a report, for the report to run again.
fn report_step_failed<E: std::fmt::Debug>(
    step: &'static str,
    error: error_stack::Report<E>,
) -> sch_errors::ProcessTrackerError {
    metrics::REVENUE_RECOVERY_REPORT_STEP_FAILED
        .add(1, router_env::metric_attributes!(("step", step)));
    logger::error!(?error, step, "A step of the recovery report failed");
    sch_errors::ProcessTrackerError::FlowExecutionError { flow: step }
}
