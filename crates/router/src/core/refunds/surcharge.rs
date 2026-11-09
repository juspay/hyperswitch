//! InterPayments surcharge refund orchestration.
//!
//! Partial refunds run the two-call Init/Complete protocol; a whole-order refund keeps the legacy
//! `SurchargeRefundSucceeded` single call. Every lifecycle step follows the same three stages:
//!
//! 1. **validate** — [`SurchargeRefundBalances`] says how much order amount and surcharge fee
//!    remain refundable.
//! 2. **trigger** — call InterPayments ([`SurchargeRefund::call_surcharge_refund_init`] /
//!    [`SurchargeRefund::notify_surcharge_refund_complete`]).
//! 3. **update** — persist the outcome ([`SurchargeRefund::persist_surcharge_details`] /
//!    [`SurchargeRefund::mark_refund_failed`]).
//!
//! The public entry points read as those three stages in order.

use common_types::refunds::ExternalSurchargeRefundDetails;
use common_utils::types::MinorUnit;
use diesel_models::refund::{Refund, RefundUpdate};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_request_types::{PaymentsSurchargeRefundCompleteData, PaymentsSurchargeRefundInitData},
    router_response_types::{SurchargeRefundCompleteResponseData, SurchargeRefundInitResponseData},
};
use router_env::logger;

use crate::{
    core::{
        errors::{self, RouterResult},
        payments::helpers,
        unified_connector_service as ucs,
        utils::refunds_validator,
    },
    routes::SessionState,
    types::{domain, storage, transformers::ForeignTryFrom},
};

/// Whether this payment has a surcharge that a refund must account for.
pub fn has_surcharge(attempt: &storage::PaymentAttempt) -> bool {
    attempt
        .external_surcharge_details
        .as_ref()
        .is_some_and(|details| {
            // An error-only record still counts.
            details.usable_connector_id().is_some()
                || attempt
                    .net_amount
                    .get_surcharge_amount()
                    .is_some_and(|amount| amount > MinorUnit::zero())
        })
}

// ------------------------------------------------------------------------------------------------
// validate — the balances a requested amount is checked against.
// ------------------------------------------------------------------------------------------------

/// Order and surcharge balances for a surcharge refund, derived from the original payment attempt
/// and the stored amounts on previous refunds. Carries no I/O; only answers how much order amount
/// and surcharge fee remain refundable.
#[derive(Clone, Copy, Debug)]
struct SurchargeRefundBalances {
    /// Order amount of the original payment, excluding the surcharge fee.
    original_order_amount: i64,
    /// Order amount still refundable after previous refunds.
    remaining_order_amount: i64,
    /// Surcharge fee of the original payment.
    original_surcharge_amount: i64,
    /// Surcharge fee still refundable after previous refunds.
    remaining_surcharge_amount: i64,
}

impl SurchargeRefundBalances {
    /// Derives the balances from the attempt and the stored amounts on previous refunds, excluding
    /// `exclude_refund_id` (the refund being created or retried) from the sums. Failed refunds are
    /// ignored. Returns a `PreconditionFailed` when the original surcharge record is missing or
    /// inconsistent and reconciliation is required.
    fn from_previous_refunds(
        attempt: &storage::PaymentAttempt,
        previous_refunds: &[Refund],
        exclude_refund_id: Option<&str>,
    ) -> RouterResult<Self> {
        let details = attempt.external_surcharge_details.as_ref().ok_or_else(|| {
            error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Missing original external surcharge details on payment attempt")
        })?;
        if details.usable_connector_id().is_none() {
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Invalid original surcharge ID; reconciliation required".to_string(),
                },
            ));
        }
        if attempt.net_amount.get_surcharge_amount() != Some(details.external_surcharge_amount) {
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Charged surcharge breakdown mismatch; reconciliation required"
                        .to_string(),
                },
            ));
        }

        let original_surcharge_amount = details.external_surcharge_amount.get_amount_as_i64();
        let original_order_amount =
            attempt.get_total_amount().get_amount_as_i64() - original_surcharge_amount;

        let mut refunded_order_amount = 0;
        let mut refunded_surcharge_amount = 0;
        for refund in previous_refunds.iter().filter(|refund| {
            Some(refund.refund_id.as_str()) != exclude_refund_id
                && !refund.refund_status.is_failure()
        }) {
            refunded_order_amount += refund.refund_amount.get_amount_as_i64();
            refunded_surcharge_amount += refund
                .refund_surcharge_amount
                .unwrap_or_default()
                .get_amount_as_i64();
        }

        Ok(Self {
            original_order_amount,
            remaining_order_amount: original_order_amount - refunded_order_amount,
            original_surcharge_amount,
            remaining_surcharge_amount: original_surcharge_amount - refunded_surcharge_amount,
        })
    }

    /// A whole-order refund (the entire order amount, with no prior non-failed refund) keeps the
    /// legacy single-call path rather than the partial Init/Complete protocol.
    fn is_whole_order_refund(
        &self,
        requested: Option<MinorUnit>,
        previous_refunds: &[Refund],
    ) -> bool {
        previous_refunds
            .iter()
            .all(|refund| refund.refund_status.is_failure())
            && requested
                .is_none_or(|amount| amount.get_amount_as_i64() == self.original_order_amount)
    }

    /// Checks a requested order amount, and (when present) the surcharge fee an Init response
    /// reported, against the remaining balances. The captured-funds check is intentionally left to
    /// the generic `validate_refund_amount` the refund-create path already runs.
    fn validate(
        &self,
        requested_order_amount: MinorUnit,
        init_response: Option<&SurchargeRefundInitResponseData>,
    ) -> RouterResult<()> {
        refunds_validator::validate_order_refund_amount(
            self.original_order_amount,
            self.remaining_order_amount,
            requested_order_amount.get_amount_as_i64(),
        )?;
        let refund_surcharge_fee = init_response
            .map(|response| response.refund_surcharge_amount)
            .unwrap_or_default();
        if refund_surcharge_fee < MinorUnit::zero()
            || refund_surcharge_fee.get_amount_as_i64() > self.remaining_surcharge_amount
        {
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Surcharge refund exceeds remaining surcharge amount".to_string(),
                },
            ));
        }
        if let Some(response) = init_response {
            let original_fee_mismatch = response
                .original_transaction_fee
                .is_some_and(|amount| amount.get_amount_as_i64() != self.original_surcharge_amount);
            let refunded_fee_exceeded =
                response
                    .previously_refunded_transaction_fees
                    .is_some_and(|amount| {
                        refund_surcharge_fee.get_amount_as_i64()
                            > self.original_surcharge_amount - amount.get_amount_as_i64()
                    });
            if original_fee_mismatch || refunded_fee_exceeded {
                return Err(error_stack::Report::new(
                    errors::ApiErrorResponse::PreconditionFailed {
                        message:
                            "InterPayments surcharge balance is inconsistent; reconciliation required"
                                .to_string(),
                    },
                ));
            }
        }
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------
// Entry points — each reads as validate -> trigger -> update.
// ------------------------------------------------------------------------------------------------

/// Pre-create validation: for a partial surcharge refund, validates and returns the order-only
/// amount to reserve; `None` for a plain or whole-order refund (which take the legacy path).
pub fn reserve_partial_refund_amount(
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
    refund_id: &str,
    requested_amount: Option<MinorUnit>,
    previous_refunds: &[Refund],
) -> RouterResult<Option<MinorUnit>> {
    if !has_surcharge(attempt) {
        // The attempt lost its surcharge details even though a surcharge was charged: reconcile.
        let surcharge_was_calculated = intent.external_surcharge_applicable == Some(true)
            && attempt
                .net_amount
                .get_surcharge_amount()
                .is_some_and(|surcharge| surcharge > MinorUnit::zero());
        if surcharge_was_calculated {
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Missing original external surcharge details; reconciliation required"
                        .to_string(),
                },
            ));
        }
        return Ok(None);
    }

    let balances =
        SurchargeRefundBalances::from_previous_refunds(attempt, previous_refunds, Some(refund_id))?;
    if balances.is_whole_order_refund(requested_amount, previous_refunds) {
        return Ok(None);
    }

    let reservation_amount =
        requested_amount.unwrap_or_else(|| MinorUnit::new(balances.remaining_order_amount));
    balances.validate(reservation_amount, None)?;
    Ok(Some(reservation_amount))
}

/// Before the processor refund: for a partial surcharge refund awaiting Init, runs
/// validate -> Init -> persist. Returns the (possibly updated) refund and whether the caller should
/// now submit the processor refund.
pub async fn prepare_for_processor_refund(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    attempt: &storage::PaymentAttempt,
) -> RouterResult<(Refund, bool)> {
    // Init already succeeded (details are persisted only on success): proceed to the processor.
    if refund.surcharge_refund_details().is_some() {
        return Ok((refund.clone(), true));
    }
    // A plain refund, or one no longer awaiting Init, takes the normal path untouched.
    if !has_surcharge(attempt) || refund.refund_status != common_enums::RefundStatus::Pending {
        return Ok((refund.clone(), true));
    }

    let flow = SurchargeRefund::new(state, platform, refund);
    let previous = flow.previous_refunds().await?;
    // 1. validate
    let balances = SurchargeRefundBalances::from_previous_refunds(
        attempt,
        &previous,
        Some(&refund.refund_id),
    )?;
    // A whole-order refund keeps the legacy single-call path; only partial refunds use Init/Complete.
    if balances.is_whole_order_refund(Some(refund.refund_amount), &previous) {
        return Ok((refund.clone(), true));
    }
    balances.validate(refund.refund_amount, None)?;

    flow.schedule_recovery_task().await?;
    // 2. trigger: call Init (marks the refund failed on any error)
    let details = flow.call_surcharge_refund_init(attempt, &balances).await?;
    // 3. update: persist the Init result
    let refund = flow.persist_surcharge_details(details).await?;
    Ok((refund, true))
}

/// After the processor refund succeeds: runs Complete and records it. Failures are logged and left
/// to recovery so a refund whose money already moved never loses its success status.
pub async fn complete_after_processor_refund(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
) {
    // 1. validate: there is a Complete to do only for a succeeded refund not yet acknowledged.
    let Some(details) = refund.surcharge_refund_details() else {
        return;
    };
    if details.refund_complete_notified
        || refund.refund_status != common_enums::RefundStatus::Success
    {
        return;
    }
    let flow = SurchargeRefund::new(state, platform, refund);
    if let Err(error) = flow.notify_surcharge_refund_complete(details).await {
        logger::error!(
            ?error,
            refund_id = %refund.refund_id,
            "Surcharge completion deferred to recovery"
        );
    }
}

/// Process-tracker handler: re-drives the refund (via retrieve) and finishes the task once the
/// surcharge refund is settled, otherwise schedules a retry.
pub async fn resume_pending_surcharge_refund(
    state: &SessionState,
    task: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let tracking: diesel_models::refund::RefundCoreWorkflow =
        serde_json::from_value(task.tracking_data.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to deserialize surcharge refund process tracker data")?;
    let platform = crate::core::utils::build_platform_from_refund_core(state, &tracking).await?;
    let refund = Box::pin(super::refund_retrieve_core_with_internal_reference_id(
        state.clone(),
        platform,
        None,
        tracking.refund_internal_reference_id,
        Some(true),
    ))
    .await;
    if let Ok(refund) = refund {
        // Settled once the refund failed (so Complete never runs) or Complete was acknowledged.
        let settled = refund.refund_status.is_failure()
            || refund
                .surcharge_refund_details()
                .is_some_and(|details| details.refund_complete_notified);
        if settled {
            state
                .store
                .as_scheduler()
                .finish_process_with_business_status(task.clone(), "COMPLETED_BY_PT")
                .await?;
            return Ok(());
        }
    }
    state
        .store
        .as_scheduler()
        .retry_process(
            task.clone(),
            common_utils::date_time::now() + time::Duration::minutes(1),
        )
        .await?;
    Ok(())
}

// ------------------------------------------------------------------------------------------------
// Orchestrator — the trigger (InterPayments calls) and update (persistence) stages.
// ------------------------------------------------------------------------------------------------

/// Bundles the context every trigger/update step needs, so the steps read as intent, not plumbing.
struct SurchargeRefund<'a> {
    state: &'a SessionState,
    platform: &'a domain::Platform,
    refund: &'a Refund,
}

impl<'a> SurchargeRefund<'a> {
    fn new(state: &'a SessionState, platform: &'a domain::Platform, refund: &'a Refund) -> Self {
        Self {
            state,
            platform,
            refund,
        }
    }

    /// All refunds for this payment, used to compute balances.
    async fn previous_refunds(&self) -> RouterResult<Vec<Refund>> {
        self.state
            .store
            .find_refund_by_payment_id_processor_merchant_id(
                &self.refund.payment_id,
                self.platform.get_processor().get_account().get_id(),
                self.platform.get_processor().get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch previous refunds for this payment")
    }

    fn profile_id(&self) -> RouterResult<&common_utils::id_type::ProfileId> {
        self.refund.profile_id.as_ref().ok_or_else(|| {
            error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Missing profile ID on refund")
        })
    }

    // --- trigger ---

    /// Resolves the active InterPayments connector account configured on the profile.
    async fn resolve_interpayments_account(
        &self,
    ) -> RouterResult<helpers::MerchantConnectorAccountType> {
        let processor = self.platform.get_processor();
        let profile_id = self.profile_id()?;
        let business_profile = self
            .state
            .store
            .find_business_profile_by_profile_id(processor.get_key_store(), profile_id)
            .await
            .change_context(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;
        let account_id = business_profile
            .surcharge_connector_details
            .as_ref()
            .and_then(|details| details.surcharge_connector_id.as_ref())
            .ok_or_else(|| {
                error_stack::Report::new(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Surcharge connector is not configured on the profile".to_string(),
                })
            })?;
        let account = helpers::get_merchant_connector_account(
            self.state,
            processor,
            None,
            profile_id,
            "interpayments",
            Some(account_id),
        )
        .await?;
        if account.get_connector_name().as_deref() != Some("interpayments") || account.is_disabled()
        {
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Surcharge connector must be an active InterPayments account"
                        .to_string(),
                },
            ));
        }
        Ok(account)
    }

    /// Calls Init, validates its response against `balances`, and returns the tracking details to
    /// persist. On any failure the refund is marked failed (Init runs before any money moves, so
    /// this is safe) and the error is returned.
    async fn call_surcharge_refund_init(
        &self,
        attempt: &storage::PaymentAttempt,
        balances: &SurchargeRefundBalances,
    ) -> RouterResult<ExternalSurchargeRefundDetails> {
        let mca = self.resolve_interpayments_account().await?;
        let profile = self.profile_id()?;
        let original_surcharge_id = attempt
            .external_surcharge_details
            .as_ref()
            .and_then(|details| details.usable_connector_id())
            .ok_or_else(|| {
                error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Missing original surcharge ID on payment attempt")
            })?
            .to_owned();

        let response = match ucs::call_unified_connector_service_for_surcharge_refund_init(
            self.state,
            self.platform.get_processor(),
            mca,
            profile,
            PaymentsSurchargeRefundInitData {
                refund_id: self.refund.refund_id.clone(),
                original_surcharge_id,
                order_amount: self.refund.refund_amount,
                currency: self.refund.currency,
            },
        )
        .await
        {
            Ok(response) => response,
            Err(err) => {
                return Err(self
                    .mark_refund_failed(err.attach_printable("SurchargeRefundInit call failed"))
                    .await);
            }
        };

        let data = match SurchargeRefundInitResponseData::foreign_try_from((
            &response,
            self.refund.refund_id.as_str(),
            self.refund.currency,
        )) {
            Ok(data) => data,
            Err(message) => {
                let error =
                    error_stack::Report::new(errors::ApiErrorResponse::PreconditionFailed {
                        message: message.to_string(),
                    });
                return Err(self.mark_refund_failed(error).await);
            }
        };

        if let Err(error) = balances.validate(self.refund.refund_amount, Some(&data)) {
            return Err(self.mark_refund_failed(error).await);
        }

        let Some(surcharge_refund_id) = response
            .connector_surcharge_refund_id
            .clone()
            .filter(|id| !id.trim().is_empty())
        else {
            let error = error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "SurchargeRefundInit succeeded but returned no surcharge refund ID",
                );
            return Err(self.mark_refund_failed(error).await);
        };

        Ok(ExternalSurchargeRefundDetails {
            surcharge_refund_id,
            refund_surcharge_amount: data.refund_surcharge_amount,
            refund_complete_notified: false,
        })
    }

    /// Calls Complete and records the acknowledgement. On failure returns an error with
    /// `refund_complete_notified` left false, so recovery retries without losing refund success.
    async fn notify_surcharge_refund_complete(
        &self,
        details: ExternalSurchargeRefundDetails,
    ) -> RouterResult<()> {
        let mca = self.resolve_interpayments_account().await?;
        let profile = self.profile_id()?;
        let id = details.surcharge_refund_id.clone();
        let result = ucs::call_unified_connector_service_for_surcharge_refund_complete(
            self.state,
            self.platform.get_processor(),
            mca,
            profile,
            PaymentsSurchargeRefundCompleteData {
                refund_id: self.refund.refund_id.clone(),
                connector_surcharge_refund_id: id.clone(),
            },
        )
        .await;
        let acknowledged = result.as_ref().is_ok_and(|response| {
            SurchargeRefundCompleteResponseData::foreign_try_from((
                response,
                self.refund.refund_id.as_str(),
                id.as_str(),
            ))
            .is_ok()
        });
        if !acknowledged {
            return Err(
                error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("SurchargeRefundComplete failed or returned an error"),
            );
        }
        self.persist_surcharge_details(ExternalSurchargeRefundDetails {
            refund_complete_notified: true,
            ..details
        })
        .await?;
        Ok(())
    }

    // --- update ---

    /// Persists the surcharge tracking details on the refund row.
    async fn persist_surcharge_details(
        &self,
        details: ExternalSurchargeRefundDetails,
    ) -> RouterResult<Refund> {
        let storage_scheme = self.platform.get_processor().get_account().storage_scheme;
        self.state
            .store
            .update_refund(
                self.refund.clone(),
                RefundUpdate::SurchargeUpdate {
                    external_surcharge_refund_details: details,
                    updated_by: storage_scheme.to_string(),
                },
                storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to persist surcharge refund tracking details")
    }

    /// Marks the refund failed with `error`'s message, then returns the error. Safe because Init
    /// runs before any money moves; reconciliation, if needed, follows from that message.
    async fn mark_refund_failed(
        &self,
        error: error_stack::Report<errors::ApiErrorResponse>,
    ) -> error_stack::Report<errors::ApiErrorResponse> {
        let storage_scheme = self.platform.get_processor().get_account().storage_scheme;
        let update = RefundUpdate::ErrorUpdate {
            refund_status: Some(common_enums::RefundStatus::Failure),
            refund_error_message: Some(error.to_string()),
            refund_error_code: None,
            updated_by: storage_scheme.to_string(),
            connector_refund_id: None,
            processor_refund_data: None,
            unified_code: None,
            unified_message: None,
            issuer_error_code: None,
            issuer_error_message: None,
        };
        if let Err(persist_error) = self
            .state
            .store
            .update_refund(self.refund.clone(), update, storage_scheme)
            .await
        {
            logger::error!(
                ?persist_error,
                refund_id = %self.refund.refund_id,
                "Failed to mark surcharge refund as failed"
            );
        }
        error
    }

    /// Schedules the recovery process-tracker task (idempotently) before any external call.
    async fn schedule_recovery_task(&self) -> RouterResult<()> {
        let id = format!("SURCHARGE_REFUND_{}", self.refund.internal_reference_id);
        if self
            .state
            .store
            .find_process_by_id(&id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to check for an existing SURCHARGE_REFUND process tracker task",
            )?
            .is_some()
        {
            return Ok(());
        }
        let task = storage::ProcessTrackerNew::new(
            id,
            "SURCHARGE_REFUND",
            storage::ProcessTrackerRunner::RefundWorkflowRouter,
            ["REFUND"],
            super::refund_to_refund_core_workflow_model(self.refund),
            None,
            common_utils::date_time::now() + time::Duration::minutes(1),
            common_types::consts::API_VERSION,
            self.state.conf.application_source,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to build the SURCHARGE_REFUND process tracker task")?;
        match self.state.store.insert_process(task).await {
            Ok(_) => Ok(()),
            Err(error) if error.current_context().is_db_unique_violation() => Ok(()),
            Err(error) => Err(error)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to insert the SURCHARGE_REFUND process tracker task"),
        }
    }
}
