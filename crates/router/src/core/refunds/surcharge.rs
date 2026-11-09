//! InterPayments refund orchestration and balance validation.
//!
//! Partial refunds use Init/Complete here; full refunds use the old `SurchargeRefundSucceeded` call.
//!
//! Three parts: calculate the balances, validate an amount against them, persist the result.

use common_types::{payments::SurchargeErrorDetails, refunds::ExternalSurchargeRefundDetails};
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

/// True once Complete succeeded, or Init was rejected and the refund failed.
fn is_surcharge_refund_complete(details: &ExternalSurchargeRefundDetails, refund: &Refund) -> bool {
    details.refund_complete_notified
        || (details.surcharge_refund_id.is_none() && !consumes_balance(refund))
}

/// Whether this payment has a surcharge to refund.
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

/// Whether this refund still counts against the balances.
fn consumes_balance(refund: &Refund) -> bool {
    !refund.refund_status.is_failure()
}

/// True only for the full order amount with no prior non-failed refund.
fn is_full_refund(original: i64, requested: Option<MinorUnit>, previous: &[Refund]) -> bool {
    previous.iter().all(|refund| !consumes_balance(refund))
        && requested.is_none_or(|amount| amount.get_amount_as_i64() == original)
}

// --- calculate ---

/// Order and surcharge balances for a surcharge refund, read from stored amounts.
struct SurchargeRefundAmounts {
    /// Order amount of the original payment, excluding the surcharge fee.
    original_order_amount: i64,
    /// Order amount still refundable after previous refunds.
    remaining_order_amount: i64,
    /// Surcharge fee of the original payment.
    original_surcharge_amount: i64,
    /// Surcharge fee still refundable after previous refunds.
    remaining_surcharge_amount: i64,
}

/// Derives the order and surcharge balances from the attempt and the stored amounts on previous
/// refunds, excluding `exclude_refund_id` (the refund being created or retried) from the sums.
fn calculate_surcharge_refund_amounts(
    attempt: &storage::PaymentAttempt,
    previous_refunds: &[Refund],
    exclude_refund_id: Option<&str>,
) -> RouterResult<SurchargeRefundAmounts> {
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
        Some(refund.refund_id.as_str()) != exclude_refund_id && consumes_balance(refund)
    }) {
        if refund
            .surcharge_refund_details()
            .is_some_and(|details| details.error_details.is_some())
        {
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Surcharge refund reconciliation required".to_string(),
                },
            ));
        }
        refunded_order_amount += refund.refund_amount.get_amount_as_i64();
        refunded_surcharge_amount += refund
            .refund_surcharge_amount
            .unwrap_or_default()
            .get_amount_as_i64();
    }

    Ok(SurchargeRefundAmounts {
        original_order_amount,
        remaining_order_amount: original_order_amount - refunded_order_amount,
        original_surcharge_amount,
        remaining_surcharge_amount: original_surcharge_amount - refunded_surcharge_amount,
    })
}

// --- validate ---

/// Checks the refund against the remaining order and surcharge balances. The captured-funds check
/// is left to the caller's generic `validate_refund_amount`.
fn validate_surcharge_refund_amounts(
    amounts: &SurchargeRefundAmounts,
    refund_amount: MinorUnit,
    response: Option<&SurchargeRefundInitResponseData>,
) -> RouterResult<()> {
    refunds_validator::validate_order_refund_amount(
        amounts.original_order_amount,
        amounts.remaining_order_amount,
        refund_amount.get_amount_as_i64(),
    )?;
    let refund_surcharge_fee = response
        .map(|init_response| init_response.refund_surcharge_amount)
        .unwrap_or_default();
    if refund_surcharge_fee < MinorUnit::zero()
        || refund_surcharge_fee.get_amount_as_i64() > amounts.remaining_surcharge_amount
    {
        return Err(error_stack::Report::new(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Surcharge refund exceeds remaining surcharge amount".to_string(),
            },
        ));
    }
    if let Some(response) = response {
        if response
            .original_transaction_fee
            .is_some_and(|amount| amount.get_amount_as_i64() != amounts.original_surcharge_amount)
            || response
                .previously_refunded_transaction_fees
                .is_some_and(|amount| {
                    refund_surcharge_fee.get_amount_as_i64()
                        > amounts.original_surcharge_amount - amount.get_amount_as_i64()
                })
        {
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

// --- persistence ---

/// Persists the next surcharge tracking details.
async fn transition(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    details: ExternalSurchargeRefundDetails,
) -> RouterResult<Refund> {
    state
        .store
        .update_refund(
            refund.clone(),
            RefundUpdate::SurchargeUpdate {
                external_surcharge_refund_details: details,
                updated_by: platform
                    .get_processor()
                    .get_account()
                    .storage_scheme
                    .to_string(),
            },
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to persist surcharge refund tracking details")
}

/// Records failure/reconciliation, then returns the error. `reject` also fails the refund.
async fn fail(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    details: ExternalSurchargeRefundDetails,
    reject: bool,
    error: error_stack::Report<errors::ApiErrorResponse>,
) -> error_stack::Report<errors::ApiErrorResponse> {
    if let Err(persist_error) = transition(state, platform, refund, details).await {
        logger::error!(
            ?persist_error,
            refund_id = %refund.refund_id,
            "Failed to persist surcharge refund status"
        );
    }
    if reject {
        let storage_scheme = platform.get_processor().get_account().storage_scheme;
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
        if let Err(persist_error) = state
            .store
            .update_refund(refund.clone(), update, storage_scheme)
            .await
        {
            logger::error!(
                ?persist_error,
                refund_id = %refund.refund_id,
                "Failed to mark refund as failed after surcharge rejection"
            );
        }
    }
    error
}

/// Resolves the InterPayments account from the profile.
async fn account(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
) -> RouterResult<helpers::MerchantConnectorAccountType> {
    let processor = platform.get_processor();
    let profile_id = refund.profile_id.as_ref().ok_or_else(|| {
        error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing profile ID on refund")
    })?;
    let business_profile = state
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
        state,
        processor,
        None,
        profile_id,
        "interpayments",
        Some(account_id),
    )
    .await?;
    if account.get_connector_name().as_deref() != Some("interpayments") || account.is_disabled() {
        return Err(error_stack::Report::new(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Surcharge connector must be an active InterPayments account".to_string(),
            },
        ));
    }
    Ok(account)
}

/// Schedules a recovery task before external calls.
async fn ensure_recovery_task(state: &SessionState, refund: &Refund) -> RouterResult<()> {
    let id = format!("SURCHARGE_REFUND_{}", refund.internal_reference_id);
    if state
        .store
        .find_process_by_id(&id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to check for an existing SURCHARGE_REFUND process tracker task")?
        .is_some()
    {
        return Ok(());
    }
    let task = storage::ProcessTrackerNew::new(
        id,
        "SURCHARGE_REFUND",
        storage::ProcessTrackerRunner::RefundWorkflowRouter,
        ["REFUND"],
        super::refund_to_refund_core_workflow_model(refund),
        None,
        common_utils::date_time::now() + time::Duration::minutes(1),
        common_types::consts::API_VERSION,
        state.conf.application_source,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build the SURCHARGE_REFUND process tracker task")?;
    match state.store.insert_process(task).await {
        Ok(_) => Ok(()),
        Err(error) if error.current_context().is_db_unique_violation() => Ok(()),
        Err(error) => Err(error)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to insert the SURCHARGE_REFUND process tracker task"),
    }
}

// --- lifecycle entry points ---

/// Validates and reserves the amount for a partial refund; `None` otherwise.
pub fn validate_and_calculate_refund_amount(
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
    refund_id: &str,
    requested_amount: Option<MinorUnit>,
    previous_refunds: &[Refund],
) -> RouterResult<Option<(MinorUnit, ExternalSurchargeRefundDetails)>> {
    let surcharge_was_calculated = intent.external_surcharge_applicable == Some(true)
        && attempt
            .net_amount
            .get_surcharge_amount()
            .is_some_and(|surcharge| surcharge > MinorUnit::zero());
    if !has_surcharge(attempt) && surcharge_was_calculated {
        return Err(error_stack::Report::new(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Missing original external surcharge details; reconciliation required"
                    .to_string(),
            },
        ));
    }
    if !has_surcharge(attempt) {
        return Ok(None);
    }

    let amounts = calculate_surcharge_refund_amounts(attempt, previous_refunds, Some(refund_id))?;
    if is_full_refund(
        amounts.original_order_amount,
        requested_amount,
        previous_refunds,
    ) {
        return Ok(None);
    }

    let reservation_amount = requested_amount
        .map(|amount| amount.get_amount_as_i64())
        .unwrap_or(amounts.remaining_order_amount);
    validate_surcharge_refund_amounts(&amounts, MinorUnit::new(reservation_amount), None)?;

    Ok(Some((
        MinorUnit::new(reservation_amount),
        ExternalSurchargeRefundDetails {
            surcharge_refund_id: None,
            refund_surcharge_amount: None,
            refund_complete_notified: false,
            error_details: None,
        },
    )))
}

/// Calls Init and persists the result, or an error status if it fails/is rejected.
async fn init_surcharge_refund(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    attempt: &storage::PaymentAttempt,
) -> RouterResult<Refund> {
    let previous = state
        .store
        .find_refund_by_payment_id_processor_merchant_id(
            &refund.payment_id,
            platform.get_processor().get_account().get_id(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch previous refunds for this payment")?;
    let amounts = calculate_surcharge_refund_amounts(attempt, &previous, Some(&refund.refund_id))?;
    validate_surcharge_refund_amounts(&amounts, refund.refund_amount, None)?;

    let mca = account(state, platform, refund).await?;
    let profile = refund.profile_id.as_ref().ok_or_else(|| {
        error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing profile ID on refund")
    })?;
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
        state,
        platform.get_processor(),
        mca,
        profile,
        PaymentsSurchargeRefundInitData {
            refund_id: refund.refund_id.clone(),
            original_surcharge_id,
            order_amount: refund.refund_amount,
            currency: refund.currency,
        },
    )
    .await
    {
        Ok(response) => response,
        Err(err) => {
            let details = ExternalSurchargeRefundDetails {
                surcharge_refund_id: None,
                refund_surcharge_amount: None,
                refund_complete_notified: false,
                error_details: Some(SurchargeErrorDetails {
                    error_message: Some("SurchargeRefundInit call failed".to_string()),
                    ..Default::default()
                }),
            };
            return Err(fail(state, platform, refund, details, false, err).await);
        }
    };

    let surcharge_refund_id = response.connector_surcharge_refund_id.clone();
    let data = match SurchargeRefundInitResponseData::foreign_try_from((
        &response,
        refund.refund_id.as_str(),
        refund.currency,
    )) {
        Ok(data) => data,
        Err(message) => {
            // 400/422 means rejected; anything else is uncertain.
            let reject = response.error.is_some() && matches!(response.status_code, 400 | 422);
            let details = ExternalSurchargeRefundDetails {
                surcharge_refund_id: surcharge_refund_id.clone(),
                refund_surcharge_amount: None,
                refund_complete_notified: false,
                error_details: Some(SurchargeErrorDetails {
                    error_message: Some(message.to_string()),
                    ..Default::default()
                }),
            };
            let api_error = if reject {
                error_stack::Report::new(errors::ApiErrorResponse::PreconditionFailed {
                    message: message.to_string(),
                })
            } else {
                error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "SurchargeRefundInit response could not be validated; outcome is uncertain",
                    )
            };
            return Err(fail(state, platform, refund, details, reject, api_error).await);
        }
    };

    if let Err(err) = validate_surcharge_refund_amounts(&amounts, refund.refund_amount, Some(&data))
    {
        let details = ExternalSurchargeRefundDetails {
            surcharge_refund_id: surcharge_refund_id.clone(),
            refund_surcharge_amount: None,
            refund_complete_notified: false,
            error_details: Some(SurchargeErrorDetails {
                error_message: Some(err.to_string()),
                ..Default::default()
            }),
        };
        return Err(fail(state, platform, refund, details, false, err).await);
    }

    let details = ExternalSurchargeRefundDetails {
        surcharge_refund_id,
        refund_surcharge_amount: Some(data.refund_surcharge_amount),
        refund_complete_notified: false,
        error_details: None,
    };
    transition(state, platform, refund, details).await
}

/// Returns the refund and whether the caller should now submit the processor refund.
pub async fn prepare_processor(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    attempt: &storage::PaymentAttempt,
) -> RouterResult<(Refund, bool)> {
    let Some(details) = refund.surcharge_refund_details() else {
        return Ok((refund.clone(), true));
    };
    if refund.refund_status != common_enums::RefundStatus::Pending {
        return Ok((refund.clone(), false));
    }
    if details.error_details.is_some() {
        return Ok((refund.clone(), false));
    }
    ensure_recovery_task(state, refund).await?;
    match details.surcharge_refund_id {
        None => {
            let refund = init_surcharge_refund(state, platform, refund, attempt).await?;
            Ok((refund, true))
        }
        Some(_) => Ok((refund.clone(), true)),
    }
}

/// Calls Complete once the processor refund succeeds; recovery doesn't affect its status.
async fn complete(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
) -> RouterResult<()> {
    let Some(details) = refund.surcharge_refund_details() else {
        return Ok(());
    };
    if details.surcharge_refund_id.is_none()
        || details.refund_complete_notified
        || details.error_details.is_some()
    {
        // Not started, already done, or already flagged for reconciliation.
        return Ok(());
    }
    if !consumes_balance(refund) {
        // Processor refund failed after Init; flag for reconciliation.
        let next = ExternalSurchargeRefundDetails {
            error_details: Some(SurchargeErrorDetails {
                error_message: Some("Processor refund failed after Init succeeded".to_string()),
                ..Default::default()
            }),
            ..details
        };
        transition(state, platform, refund, next).await?;
        return Ok(());
    }
    if refund.refund_status != common_enums::RefundStatus::Success {
        return Ok(());
    }
    let mca = account(state, platform, refund).await?;
    let profile = refund.profile_id.as_ref().ok_or_else(|| {
        error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing profile ID on refund")
    })?;
    let id = details
        .surcharge_refund_id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Missing persisted surcharge refund ID before calling Complete")
        })?;

    let result = ucs::call_unified_connector_service_for_surcharge_refund_complete(
        state,
        platform.get_processor(),
        mca,
        profile,
        PaymentsSurchargeRefundCompleteData {
            refund_id: refund.refund_id.clone(),
            connector_surcharge_refund_id: id.clone(),
        },
    )
    .await;
    let successful = result.as_ref().is_ok_and(|response| {
        SurchargeRefundCompleteResponseData::foreign_try_from((
            response,
            refund.refund_id.as_str(),
            id.as_str(),
        ))
        .is_ok()
    });
    let next = if successful {
        ExternalSurchargeRefundDetails {
            refund_complete_notified: true,
            error_details: None,
            ..details
        }
    } else {
        ExternalSurchargeRefundDetails {
            error_details: Some(SurchargeErrorDetails {
                error_message: Some(
                    "SurchargeRefundComplete failed or returned an error".to_string(),
                ),
                ..Default::default()
            }),
            ..details
        }
    };
    transition(state, platform, refund, next).await?;
    Ok(())
}

/// Calls `complete`, deferring failures to recovery.
pub async fn try_complete_surcharge_refund(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
) {
    if let Err(error) = complete(state, platform, refund).await {
        logger::error!(
            ?error,
            refund_id = %refund.refund_id,
            "Surcharge completion deferred to recovery"
        );
    }
}

/// Process-tracker handler: resumes Init/Complete, or finishes once resolved.
pub async fn recover(
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
        if let Some(details) = refund.surcharge_refund_details() {
            let needs_reconciliation = details.error_details.is_some();
            if needs_reconciliation || is_surcharge_refund_complete(&details, &refund) {
                state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(
                        task.clone(),
                        if needs_reconciliation {
                            "SURCHARGE_RECONCILIATION_REQUIRED"
                        } else {
                            "COMPLETED_BY_PT"
                        },
                    )
                    .await?;
                return Ok(());
            }
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
