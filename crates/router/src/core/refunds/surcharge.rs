//! InterPayments refund orchestration and balance validation.
//!
//! Full refunds keep using the old single-call `SurchargeRefundSucceeded` notification and never
//! enter this module (see `is_full_refund`). Only partial refunds use the Init/Complete flow here.
//!
//! No status enum: `surcharge_refund_id` set = Init done; `refund_complete_notified` = Complete
//! done; `error_details` set = needs reconciliation. The `SURCHARGE_REFUND` process-tracker task
//! retries an interrupted Init or Complete call.

use common_types::{payments::SurchargeErrorDetails, refunds::ExternalSurchargeRefundDetails};
use common_utils::types::MinorUnit;
use diesel_models::refund::{Refund, RefundUpdate};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    refunds::RefundAmountDetails,
    router_request_types::{PaymentsSurchargeRefundCompleteData, PaymentsSurchargeRefundInitData},
    router_response_types::{SurchargeRefundCompleteResponseData, SurchargeRefundInitResponseData},
};
use router_env::logger;

use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::helpers,
        unified_connector_service as ucs,
        utils::refunds_validator,
    },
    routes::SessionState,
    types::{domain, storage, transformers::ForeignTryFrom},
};

fn invalid(message: impl Into<String>) -> error_stack::Report<errors::ApiErrorResponse> {
    errors::ApiErrorResponse::InvalidRequestData {
        message: message.into(),
    }
    .into()
}

fn error(message: impl Into<String>) -> SurchargeErrorDetails {
    SurchargeErrorDetails {
        error_message: Some(message.into()),
        ..Default::default()
    }
}

pub fn operation(refund: &Refund) -> Option<ExternalSurchargeRefundDetails> {
    refund.external_surcharge_refund_details.clone()
}

/// Nothing more to do: Complete succeeded, or Init was rejected and the refund already failed.
fn is_resolved(details: &ExternalSurchargeRefundDetails, refund: &Refund) -> bool {
    details.refund_complete_notified
        || (details.surcharge_refund_id.is_none() && !consumes_balance(refund))
}

pub fn applies(attempt: &storage::PaymentAttempt) -> bool {
    attempt
        .external_surcharge_details
        .as_ref()
        .is_some_and(|details| {
            // A failed calculation still has an error-only record; that must be blocked too.
            details.usable_connector_id().is_some()
                || attempt
                    .net_amount
                    .get_surcharge_amount()
                    .is_some_and(|amount| amount > MinorUnit::zero())
        })
}

fn consumes_balance(refund: &Refund) -> bool {
    !matches!(
        refund.refund_status,
        common_enums::RefundStatus::Failure | common_enums::RefundStatus::TransactionFailure
    )
}

/// Read the original order/surcharge/captured breakdown from the payment.
fn payment_limits(
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
) -> RouterResult<(i64, i64, i64, i64)> {
    let details = attempt
        .external_surcharge_details
        .as_ref()
        .ok_or_else(|| invalid("Missing original external surcharge details"))?;
    if details.usable_connector_id().is_none() {
        return Err(invalid(
            "Invalid original surcharge ID; reconciliation required",
        ));
    }
    if attempt.net_amount.get_surcharge_amount() != Some(details.external_surcharge_amount) {
        return Err(invalid(
            "Charged surcharge breakdown mismatch; reconciliation required",
        ));
    }
    let original = attempt.net_amount.get_order_amount().get_amount_as_i64();
    let captured = intent
        .amount_captured
        .ok_or_else(|| invalid("Missing captured amount"))?
        .get_amount_as_i64();
    let captured_order = hyperswitch_domain_models::refunds::captured_order_amount(
        original,
        attempt.get_total_amount().get_amount_as_i64(),
        captured,
    )
    .ok_or_else(|| invalid("Unsupported payment breakdown; capture reconciliation required"))?;
    Ok((
        original,
        captured_order,
        details.external_surcharge_amount.get_amount_as_i64(),
        captured,
    ))
}

/// Sum order/fee/net amounts already refunded, excluding failed refunds and (optionally) one
/// refund ID currently in progress.
fn previous_amounts(
    previous: &[Refund],
    exclude_refund_id: Option<&str>,
) -> RouterResult<(i64, i64, i64)> {
    let (mut order, mut fee, mut net) = (0_i64, 0_i64, 0_i64);
    for refund in previous
        .iter()
        .filter(|r| Some(r.refund_id.as_str()) != exclude_refund_id)
    {
        if let Some(details) = operation(refund) {
            if details.error_details.is_some() {
                return Err(invalid("Surcharge refund reconciliation required"));
            }
        } else if consumes_balance(refund) {
            return Err(invalid(
                "Historical surcharge refund split is unknown; reconciliation required",
            ));
        }
        if consumes_balance(refund) {
            let amounts = RefundAmountDetails::from(refund);
            let refund_net = amounts
                .checked_net_refund_amount()
                .ok_or_else(|| invalid("Refund balance overflow"))?;
            order = order
                .checked_add(refund.refund_amount.get_amount_as_i64())
                .ok_or_else(|| invalid("Refund balance overflow"))?;
            fee = fee
                .checked_add(
                    refund
                        .refund_surcharge_amount
                        .unwrap_or_default()
                        .get_amount_as_i64(),
                )
                .ok_or_else(|| invalid("Refund balance overflow"))?;
            net = net
                .checked_add(
                    refund
                        .net_refund_amount
                        .unwrap_or(refund_net)
                        .get_amount_as_i64(),
                )
                .ok_or_else(|| invalid("Refund balance overflow"))?;
        }
    }
    Ok((order, fee, net))
}

/// True only for the entire order amount with no prior non-failed refund on this payment. The
/// last chunk of a series of partial refunds is still partial, not full.
fn is_full_refund(original: i64, requested: Option<MinorUnit>, previous: &[Refund]) -> bool {
    previous.iter().all(|refund| !consumes_balance(refund))
        && requested.is_none_or(|amount| amount.get_amount_as_i64() == original)
}

pub fn prepare_reservation(
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
    refund_id: &str,
    requested: Option<MinorUnit>,
    previous: &[Refund],
) -> RouterResult<Option<(MinorUnit, ExternalSurchargeRefundDetails)>> {
    if !applies(attempt) {
        if intent.external_surcharge_applicable == Some(true)
            && attempt
                .net_amount
                .get_surcharge_amount()
                .is_some_and(|a| a > MinorUnit::zero())
        {
            return Err(invalid(
                "Missing original external surcharge details; reconciliation required",
            ));
        }
        return Ok(None);
    }
    let (original, captured_order, _, _) = payment_limits(attempt, intent)?;
    if is_full_refund(original, requested, previous) {
        return Ok(None);
    }
    let (previous_order, _, _) = previous_amounts(previous, Some(refund_id))?;
    let remaining = captured_order
        .checked_sub(previous_order)
        .ok_or_else(|| invalid("Refund balance overflow"))?;
    let amount = requested
        .map(|a| a.get_amount_as_i64())
        .unwrap_or(remaining);
    refunds_validator::validate_order_refund_amount(original, remaining, amount)
        .map_err(invalid)?;
    Ok(Some((
        MinorUnit::new(amount),
        ExternalSurchargeRefundDetails {
            surcharge_refund_id: None,
            refund_complete_notified: false,
            error_details: None,
        },
    )))
}

/// Persist the next surcharge details, and the surcharge/net amounts when known. `reject` also
/// marks the processor refund as failed (used only when Init cleanly refuses the refund).
async fn transition(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    details: ExternalSurchargeRefundDetails,
    reject: bool,
    amounts: Option<RefundAmountDetails>,
) -> RouterResult<Refund> {
    let (refund_surcharge_amount, net_refund_amount) = amounts
        .map(|a| {
            (
                a.get_refund_surcharge_amount(),
                Some(a.get_net_refund_amount()),
            )
        })
        .unwrap_or((None, None));
    let refund_status = reject.then_some(common_enums::RefundStatus::Failure);
    state
        .store
        .update_refund(
            refund.clone(),
            RefundUpdate::SurchargeUpdate {
                refund_status,
                refund_surcharge_amount,
                net_refund_amount,
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
}

/// Record a failure/reconciliation status, then return the original error.
async fn fail(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    details: ExternalSurchargeRefundDetails,
    reject: bool,
    error: error_stack::Report<errors::ApiErrorResponse>,
) -> error_stack::Report<errors::ApiErrorResponse> {
    if let Err(persist_error) = transition(state, platform, refund, details, reject, None).await {
        logger::error!(
            ?persist_error,
            refund_id = %refund.refund_id,
            "Failed to persist surcharge refund status"
        );
    }
    error
}

/// Validate against the existing amount columns before Init and again with the fee from Init.
async fn validate_amounts(
    state: &SessionState,
    platform: &domain::Platform,
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
    refund: &Refund,
    response: Option<&SurchargeRefundInitResponseData>,
) -> RouterResult<()> {
    let previous = state
        .store
        .find_refund_by_payment_id_processor_merchant_id(
            &refund.payment_id,
            platform.get_processor().get_account().get_id(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let (prior_order, prior_fee, prior_net) = previous_amounts(&previous, Some(&refund.refund_id))?;
    let (original, captured_order, surcharge, captured) = payment_limits(attempt, intent)?;
    refunds_validator::validate_order_refund_amount(
        original,
        captured_order - prior_order,
        refund.refund_amount.get_amount_as_i64(),
    )
    .map_err(invalid)?;
    let fee = response
        .map(|r| r.refund_surcharge_amount)
        .unwrap_or_default();
    if fee < MinorUnit::zero() || fee.get_amount_as_i64() > surcharge - prior_fee {
        return Err(invalid(
            "Surcharge refund exceeds remaining surcharge amount",
        ));
    }
    let net = RefundAmountDetails::new(refund.refund_amount, Some(fee))
        .checked_net_refund_amount()
        .ok_or_else(|| invalid("Refund amount overflow"))?;
    if net.get_amount_as_i64() > captured - prior_net {
        return Err(invalid("Refund exceeds remaining captured funds"));
    }
    if let Some(response) = response {
        if response
            .original_transaction_fee
            .is_some_and(|amount| amount.get_amount_as_i64() != surcharge)
            || response
                .previously_refunded_transaction_fees
                .is_some_and(|amount| {
                    fee.get_amount_as_i64() > surcharge - amount.get_amount_as_i64()
                })
        {
            return Err(invalid(
                "InterPayments surcharge balance is inconsistent; reconciliation required",
            ));
        }
    }
    Ok(())
}

/// Resolve the InterPayments account from the profile, same as the payment flow does.
async fn account(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
) -> RouterResult<helpers::MerchantConnectorAccountType> {
    let processor = platform.get_processor();
    let profile_id = refund
        .profile_id
        .as_ref()
        .ok_or_else(|| invalid("Missing profile ID"))?;
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
        .ok_or_else(|| invalid("Surcharge connector is not configured on the profile"))?;
    let account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            processor.get_account().get_id(),
            account_id,
            processor.get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: account_id.get_string_repr().to_owned(),
        })?;
    if account.connector_name != "interpayments" || account.disabled.unwrap_or(false) {
        return Err(invalid(
            "Surcharge connector must be an active InterPayments account",
        ));
    }
    Ok(helpers::MerchantConnectorAccountType::DbVal(Box::new(
        account,
    )))
}

/// Schedule recovery before making external calls, using the existing process tracker APIs.
pub async fn ensure_recovery_task(state: &SessionState, refund: &Refund) -> RouterResult<()> {
    let id = format!("SURCHARGE_REFUND_{}", refund.internal_reference_id);
    if state
        .store
        .find_process_by_id(&id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
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
    .change_context(errors::ApiErrorResponse::InternalServerError)?;
    match state.store.insert_process(task).await {
        Ok(_) => Ok(()),
        Err(error) if error.current_context().is_db_unique_violation() => Ok(()),
        Err(error) => Err(error).change_context(errors::ApiErrorResponse::InternalServerError),
    }
}

/// Call Init and persist the returned ID, or an error status if it fails/is rejected.
async fn init_surcharge_refund(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
) -> RouterResult<Refund> {
    validate_amounts(state, platform, attempt, intent, refund, None).await?;
    let mca = account(state, platform, refund).await?;
    let profile = refund
        .profile_id
        .as_ref()
        .ok_or_else(|| invalid("Missing profile ID"))?;
    let original_surcharge_id = attempt
        .external_surcharge_details
        .as_ref()
        .and_then(|d| d.usable_connector_id())
        .ok_or_else(|| invalid("Missing original surcharge ID"))?
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
                refund_complete_notified: false,
                error_details: Some(error("SurchargeRefundInit call failed")),
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
            // A clean 400/422 rejection means the refund is dead; anything else is uncertain.
            let reject = response.error.is_some() && matches!(response.status_code, 400 | 422);
            let details = ExternalSurchargeRefundDetails {
                surcharge_refund_id: surcharge_refund_id.clone(),
                refund_complete_notified: false,
                error_details: Some(error(message)),
            };
            return Err(fail(state, platform, refund, details, reject, invalid(message)).await);
        }
    };

    if let Err(err) = validate_amounts(state, platform, attempt, intent, refund, Some(&data)).await
    {
        let details = ExternalSurchargeRefundDetails {
            surcharge_refund_id: surcharge_refund_id.clone(),
            refund_complete_notified: false,
            error_details: Some(error(err.to_string())),
        };
        return Err(fail(state, platform, refund, details, false, err).await);
    }

    let amounts =
        RefundAmountDetails::new(refund.refund_amount, Some(data.refund_surcharge_amount));
    let details = ExternalSurchargeRefundDetails {
        surcharge_refund_id,
        refund_complete_notified: false,
        error_details: None,
    };
    transition(state, platform, refund, details, false, Some(amounts)).await
}

/// Returns the refund and whether the caller should now submit the processor refund.
pub async fn prepare_processor(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
) -> RouterResult<(Refund, bool)> {
    let Some(details) = operation(refund) else {
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
            let refund = init_surcharge_refund(state, platform, refund, attempt, intent).await?;
            Ok((refund, true))
        }
        Some(_) => Ok((refund.clone(), true)),
    }
}

/// Call Complete once the processor refund succeeds. A successful money refund stays successful
/// even if Complete needs recovery.
pub async fn complete(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
) -> RouterResult<()> {
    let Some(details) = operation(refund) else {
        return Ok(());
    };
    if details.surcharge_refund_id.is_none()
        || details.refund_complete_notified
        || details.error_details.is_some()
    {
        // Not initialized yet, already completed, or already flagged for reconciliation.
        return Ok(());
    }
    if !consumes_balance(refund) {
        // Processor refund failed after Init succeeded; flag for reconciliation, skip Complete.
        let next = ExternalSurchargeRefundDetails {
            error_details: Some(error("Processor refund failed after Init succeeded")),
            ..details
        };
        transition(state, platform, refund, next, false, None).await?;
        return Ok(());
    }
    if refund.refund_status != common_enums::RefundStatus::Success {
        return Ok(());
    }
    let mca = account(state, platform, refund).await?;
    let profile = refund
        .profile_id
        .as_ref()
        .ok_or_else(|| invalid("Missing profile ID"))?;
    let id = details
        .surcharge_refund_id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| invalid("Missing persisted surcharge refund ID"))?;

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
            error_details: Some(error("SurchargeRefundComplete failed or returned an error")),
            ..details
        }
    };
    transition(state, platform, refund, next, false, None).await?;
    Ok(())
}

/// Call `complete`, logging and deferring to recovery on failure instead of erroring out.
pub async fn complete_opportunistically(
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

pub async fn recover(
    state: &SessionState,
    task: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let tracking: diesel_models::refund::RefundCoreWorkflow =
        serde_json::from_value(task.tracking_data.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
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
        if let Some(details) = operation(&refund) {
            let needs_reconciliation = details.error_details.is_some();
            if needs_reconciliation || is_resolved(&details, &refund) {
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
