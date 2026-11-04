//! InterPayments refund orchestration and balance validation.
use common_types::refunds::{ExternalSurchargeRefundDetails, SurchargeRefundStatus as Status};
use common_utils::types::MinorUnit;
use diesel_models::refund::Refund;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    refunds::{captured_order_amount, validate_surcharge_order_amount, RefundAmountDetails},
    router_request_types::{PaymentsSurchargeRefundCompleteData, PaymentsSurchargeRefundInitData},
    router_response_types::{SurchargeRefundCompleteResponseData, SurchargeRefundInitResponseData},
};

use crate::{
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payments::helpers,
        unified_connector_service as ucs,
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

pub fn operation(refund: &Refund) -> Option<ExternalSurchargeRefundDetails> {
    refund.external_surcharge_refund_details.clone()
}

pub fn applies(attempt: &storage::PaymentAttempt) -> bool {
    attempt
        .external_surcharge_details
        .as_ref()
        .is_some_and(|details| {
            // Failed calculations retain error details even though no surcharge was applied.
            // A positive charge with an invalid ID must still reach validation and be blocked.
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

/// Read the original breakdown from the payment, without copying it into the refund JSON.
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
    if details.surcharge_merchant_connector_id.is_none() {
        return Err(invalid(
            "Original surcharge account is unknown; reconciliation required",
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
    let captured_order = captured_order_amount(
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

/// Pending refunds reserve their amounts, as in the normal partial-refund flow.
fn previous_amounts(
    previous: &[Refund],
    current_id: Option<&str>,
) -> RouterResult<(i64, i64, i64)> {
    let (mut order, mut fee, mut net) = (0_i64, 0_i64, 0_i64);
    for refund in previous
        .iter()
        .filter(|r| Some(r.refund_id.as_str()) != current_id)
    {
        if let Some(details) = operation(refund) {
            if matches!(
                details.surcharge_refunds_status,
                Status::InitInFlight | Status::ReconciliationRequired
            ) {
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
            if refund
                .net_refund_amount
                .is_some_and(|stored| stored != refund_net)
            {
                return Err(invalid("Stored refund amount breakdown is inconsistent"));
            }
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

pub fn prepare_reservation(
    platform: &domain::Platform,
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
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
    if platform.get_processor().get_account().storage_scheme
        != common_enums::MerchantStorageScheme::PostgresOnly
    {
        return Err(invalid(
            "External surcharge refunds require Postgres storage",
        ));
    }
    let (original, captured_order, _, _) = payment_limits(attempt, intent)?;
    let (previous_order, _, _) = previous_amounts(previous, None)?;
    let remaining = captured_order
        .checked_sub(previous_order)
        .ok_or_else(|| invalid("Refund balance overflow"))?;
    let amount = requested
        .map(|a| a.get_amount_as_i64())
        .unwrap_or(remaining);
    validate_surcharge_order_amount(original, remaining, amount).map_err(invalid)?;
    Ok(Some((
        MinorUnit::new(amount),
        ExternalSurchargeRefundDetails {
            surcharge_refund_id: None,
            surcharge_refunds_status: Status::Reserved,
        },
    )))
}

async fn transition(
    state: &SessionState,
    refund: &Refund,
    details: &ExternalSurchargeRefundDetails,
    amounts: Option<RefundAmountDetails>,
) -> RouterResult<Option<Refund>> {
    let net = amounts
        .map(|a| {
            a.checked_net_refund_amount()
                .ok_or_else(|| invalid("Refund amount overflow"))
        })
        .transpose()?;
    state
        .store
        .compare_and_swap_surcharge_refund(
            refund,
            details.clone(),
            amounts.and_then(|a| a.get_refund_surcharge_amount()),
            net,
            details.surcharge_refunds_status == Status::Rejected,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
}

async fn payment_attempt(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
) -> RouterResult<storage::PaymentAttempt> {
    let processor = platform.get_processor();
    state
        .store
        .find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
            &refund.payment_id,
            &processor.get_processor_merchant_id(),
            &refund.attempt_id,
            processor.get_storage_scheme(),
            processor.get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
}

async fn account(
    state: &SessionState,
    platform: &domain::Platform,
    attempt: &storage::PaymentAttempt,
) -> RouterResult<helpers::MerchantConnectorAccountType> {
    let processor = platform.get_processor();
    let account_id = attempt
        .external_surcharge_details
        .as_ref()
        .and_then(|d| d.surcharge_merchant_connector_id.as_ref())
        .ok_or_else(|| invalid("Original surcharge account is unknown; reconciliation required"))?;
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
            "Original surcharge account must be an active InterPayments account",
        ));
    }
    Ok(helpers::MerchantConnectorAccountType::DbVal(Box::new(
        account,
    )))
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
    validate_surcharge_order_amount(
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

pub async fn prepare_processor(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
) -> RouterResult<(Refund, bool)> {
    let Some(mut details) = operation(refund) else {
        return Ok((refund.clone(), true));
    };
    if refund.refund_status != common_enums::RefundStatus::Pending {
        return Ok((refund.clone(), false));
    }
    ensure_recovery_task(state, refund).await?;
    let mut current = refund.clone();
    if details.surcharge_refunds_status == Status::Reserved {
        validate_amounts(state, platform, attempt, intent, refund, None).await?;
        let mca = account(state, platform, attempt).await?;
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
        details.surcharge_refunds_status = Status::InitInFlight;
        let Some(claimed) = transition(state, &current, &details, None).await? else {
            return Ok((current, false));
        };
        current = claimed;
        let result = ucs::call_unified_connector_service_for_surcharge_refund_init(
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
        .await;
        match result {
            Err(error) => {
                details.surcharge_refunds_status = Status::ReconciliationRequired;
                transition(state, &current, &details, None).await?;
                return Err(error);
            }
            Ok(response) => {
                details.surcharge_refund_id = response.connector_surcharge_refund_id.clone();
                let data = match SurchargeRefundInitResponseData::foreign_try_from((
                    &response,
                    refund.refund_id.as_str(),
                    refund.currency,
                )) {
                    Ok(data) => data,
                    Err(message) => {
                        details.surcharge_refunds_status = if response.error.is_some()
                            && matches!(response.status_code, 400 | 422)
                        {
                            Status::Rejected
                        } else {
                            Status::ReconciliationRequired
                        };
                        transition(state, &current, &details, None).await?;
                        return Err(invalid(message));
                    }
                };
                if let Err(error) =
                    validate_amounts(state, platform, attempt, intent, refund, Some(&data)).await
                {
                    details.surcharge_refunds_status = Status::ReconciliationRequired;
                    transition(state, &current, &details, None).await?;
                    return Err(error);
                }
                let amounts = RefundAmountDetails::new(
                    refund.refund_amount,
                    Some(data.refund_surcharge_amount),
                );
                details.surcharge_refunds_status = Status::Initialized;
                match transition(state, &current, &details, Some(amounts)).await? {
                    Some(saved) => current = saved,
                    None => return Ok((current, false)),
                }
            }
        }
    }
    if details.surcharge_refunds_status != Status::Initialized {
        return Ok((current, false));
    }
    details.surcharge_refunds_status = Status::ProcessorInFlight;
    match transition(state, &current, &details, None).await? {
        Some(claimed) => Ok((claimed, true)),
        None => Ok((current, false)),
    }
}

/// Keep the surcharge status separate from the processor refund's status.
pub async fn complete(
    state: &SessionState,
    platform: &domain::Platform,
    refund: &Refund,
) -> RouterResult<()> {
    let Some(mut details) = operation(refund) else {
        return Ok(());
    };
    if matches!(
        details.surcharge_refunds_status,
        Status::Initialized | Status::ProcessorInFlight
    ) && !consumes_balance(refund)
    {
        details.surcharge_refunds_status = Status::ReconciliationRequired;
        transition(state, refund, &details, None).await?;
        return Ok(());
    }
    if refund.refund_status != common_enums::RefundStatus::Success
        || details.surcharge_refunds_status != Status::ProcessorInFlight
    {
        return Ok(());
    }
    let attempt = payment_attempt(state, platform, refund).await?;
    let mca = account(state, platform, &attempt).await?;
    let profile = refund
        .profile_id
        .as_ref()
        .ok_or_else(|| invalid("Missing profile ID"))?;
    let id = details
        .surcharge_refund_id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| invalid("Missing persisted surcharge refund ID"))?;
    details.surcharge_refunds_status = Status::CompleteInFlight;
    let Some(claimed) = transition(state, refund, &details, None).await? else {
        return Ok(());
    };
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
    details.surcharge_refunds_status = if successful {
        Status::Completed
    } else {
        Status::ReconciliationRequired
    };
    transition(state, &claimed, &details, None).await?;
    Ok(())
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
        if let Some(mut details) = operation(&refund) {
            if matches!(
                details.surcharge_refunds_status,
                Status::InitInFlight | Status::CompleteInFlight
            ) && (common_utils::date_time::now() - refund.modified_at).whole_minutes() >= 5
            {
                details.surcharge_refunds_status = Status::ReconciliationRequired;
                transition(state, &refund, &details, None).await?;
            }
            if matches!(
                details.surcharge_refunds_status,
                Status::Completed | Status::Rejected | Status::ReconciliationRequired
            ) {
                state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(
                        task.clone(),
                        if details.surcharge_refunds_status == Status::ReconciliationRequired {
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
