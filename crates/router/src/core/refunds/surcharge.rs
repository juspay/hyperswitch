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
        errors::{self, RouterResult},
        payments::helpers,
        unified_connector_service as ucs,
        utils::refunds_validator,
    },
    routes::SessionState,
    types::{domain, storage, transformers::ForeignTryFrom},
};

/// Combined order+surcharge amount to send the processor, once Init has returned the fee.
pub fn net_refund_amount(refund: &Refund) -> Option<MinorUnit> {
    refund
        .surcharge_refund_details()
        .and_then(|details| details.refund_surcharge_amount)
        .map(|surcharge| refund.refund_amount + surcharge)
}

/// Nothing more to do: Complete succeeded, or Init was rejected and the refund already failed.
fn is_resolved(details: &ExternalSurchargeRefundDetails, refund: &Refund) -> bool {
    details.refund_complete_notified
        || (details.surcharge_refund_id.is_none() && !consumes_balance(refund))
}

/// Whether this payment has an external surcharge, so its refunds go through this module.
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

/// Whether this refund still counts against the order/surcharge/captured balances.
fn consumes_balance(refund: &Refund) -> bool {
    !refund.refund_status.is_failure()
}

/// Read the original order/surcharge/captured breakdown from the payment.
fn payment_limits(
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
) -> RouterResult<(i64, i64, i64, i64)> {
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
    // Everything the merchant can refund except the surcharge fee itself: order amount plus
    // shipping/tax/installment components, not just the order amount.
    let original = attempt
        .get_total_amount()
        .get_amount_as_i64()
        .checked_sub(details.external_surcharge_amount.get_amount_as_i64())
        .ok_or_else(|| {
            error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Overflow computing original amount excluding surcharge")
        })?;
    let captured = intent
        .amount_captured
        .ok_or_else(|| {
            error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Missing captured amount on payment intent")
        })?
        .get_amount_as_i64();
    let captured_order = hyperswitch_domain_models::refunds::captured_order_amount(
        original,
        attempt.get_total_amount().get_amount_as_i64(),
        captured,
    )
    .ok_or_else(|| {
        error_stack::Report::new(errors::ApiErrorResponse::PreconditionFailed {
            message: "Unsupported payment breakdown; capture reconciliation required".to_string(),
        })
    })?;
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
        let details = refund.surcharge_refund_details();
        if let Some(details) = &details {
            if details.error_details.is_some() {
                return Err(error_stack::Report::new(
                    errors::ApiErrorResponse::PreconditionFailed {
                        message: "Surcharge refund reconciliation required".to_string(),
                    },
                ));
            }
        } else if consumes_balance(refund) {
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message:
                        "Historical surcharge refund split is unknown; reconciliation required"
                            .to_string(),
                },
            ));
        }
        if consumes_balance(refund) {
            // The fee lives on the tracking record, not the flat column, until the processor
            // call finishes persisting it there.
            let surcharge = details.and_then(|d| d.refund_surcharge_amount);
            let amounts = RefundAmountDetails::new(refund.refund_amount, surcharge);
            let refund_net = amounts.checked_net_refund_amount().ok_or_else(|| {
                error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Overflow computing net refund amount for a previous refund")
            })?;
            order = order
                .checked_add(refund.refund_amount.get_amount_as_i64())
                .ok_or_else(|| {
                    error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Overflow summing previous order refund amounts")
                })?;
            fee = fee
                .checked_add(surcharge.unwrap_or_default().get_amount_as_i64())
                .ok_or_else(|| {
                    error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Overflow summing previous surcharge refund amounts")
                })?;
            net = net
                .checked_add(refund_net.get_amount_as_i64())
                .ok_or_else(|| {
                    error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Overflow summing previous net refund amounts")
                })?;
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

/// Validate the requested amount and reserve it, for a partial refund only; `None` for a full
/// refund or a payment with no applicable surcharge.
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
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Missing original external surcharge details; reconciliation required"
                        .to_string(),
                },
            ));
        }
        return Ok(None);
    }
    let (original, captured_order, _, _) = payment_limits(attempt, intent)?;
    if is_full_refund(original, requested, previous) {
        return Ok(None);
    }
    let (previous_order, _, _) = previous_amounts(previous, Some(refund_id))?;
    let remaining = captured_order.checked_sub(previous_order).ok_or_else(|| {
        error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Overflow computing remaining order amount")
    })?;
    let amount = requested
        .map(|a| a.get_amount_as_i64())
        .unwrap_or(remaining);
    refunds_validator::validate_order_refund_amount(original, remaining, amount)?;
    Ok(Some((
        MinorUnit::new(amount),
        ExternalSurchargeRefundDetails {
            surcharge_refund_id: None,
            refund_surcharge_amount: None,
            refund_complete_notified: false,
            error_details: None,
        },
    )))
}

/// Persist the next surcharge tracking details through the normal update path.
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

/// Record a failure/reconciliation status, then return the original error. `reject` also marks
/// the processor refund as failed, through the normal error-update path (used only when Init
/// cleanly refuses the refund).
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch previous refunds for this payment")?;
    let (prior_order, prior_fee, prior_net) = previous_amounts(&previous, Some(&refund.refund_id))?;
    let (original, captured_order, surcharge, captured) = payment_limits(attempt, intent)?;
    refunds_validator::validate_order_refund_amount(
        original,
        captured_order - prior_order,
        refund.refund_amount.get_amount_as_i64(),
    )?;
    let fee = response
        .map(|r| r.refund_surcharge_amount)
        .unwrap_or_default();
    if fee < MinorUnit::zero() || fee.get_amount_as_i64() > surcharge - prior_fee {
        return Err(error_stack::Report::new(
            errors::ApiErrorResponse::PreconditionFailed {
                message: "Surcharge refund exceeds remaining surcharge amount".to_string(),
            },
        ));
    }
    let net = RefundAmountDetails::new(refund.refund_amount, Some(fee))
        .checked_net_refund_amount()
        .ok_or_else(|| {
            error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Overflow computing net refund amount for this refund")
        })?;
    if net.get_amount_as_i64() > captured - prior_net {
        // Same check and error as the existing combined captured-funds validation.
        return Err(error_stack::Report::new(
            errors::ApiErrorResponse::RefundAmountExceedsPaymentAmount,
        ));
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

/// Resolve the InterPayments account from the profile, same as the payment flow does.
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

/// Schedule recovery before making external calls, using the existing process tracker APIs.
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
    let profile = refund.profile_id.as_ref().ok_or_else(|| {
        error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing profile ID on refund")
    })?;
    let original_surcharge_id = attempt
        .external_surcharge_details
        .as_ref()
        .and_then(|d| d.usable_connector_id())
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
            // A clean 400/422 rejection means the refund is dead; anything else is uncertain.
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

    if let Err(err) = validate_amounts(state, platform, attempt, intent, refund, Some(&data)).await
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
    intent: &storage::PaymentIntent,
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
            let refund = init_surcharge_refund(state, platform, refund, attempt, intent).await?;
            Ok((refund, true))
        }
        Some(_) => Ok((refund.clone(), true)),
    }
}

/// Call Complete once the processor refund succeeds. A successful money refund stays successful
/// even if Complete needs recovery.
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
        // Not initialized yet, already completed, or already flagged for reconciliation.
        return Ok(());
    }
    if !consumes_balance(refund) {
        // Processor refund failed after Init succeeded; flag for reconciliation, skip Complete.
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

/// Call `complete`, logging and deferring to recovery on failure instead of erroring out.
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

/// `SURCHARGE_REFUND` process-tracker handler: resume Init/Complete, or finish once resolved.
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
