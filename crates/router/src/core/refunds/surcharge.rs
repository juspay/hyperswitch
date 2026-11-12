//! InterPayments surcharge refund orchestration.
//!
//! Partial refunds run the two-call Init/Complete protocol; a whole-order refund keeps the legacy
//! single call. The refund-create flow runs two steps, and the webhook runs the third:
//!
//! - [`validate_partial_refund`] (create, step 1): validate the requested order amount against the
//!   remaining balance.
//! - [`init_surcharge_refund`] (create, step 2): calculate the surcharge fee via Init; the caller
//!   stores it on the new refund row (so the row carries the correct net amount for execution).
//! - [`complete_from_webhook`] (off the refund's outgoing webhook): call Complete and record it.

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
    },
    routes::SessionState,
    types::{domain, storage, transformers::ForeignTryFrom},
};

// ------------------------------------------------------------------------------------------------
// validate — the balances a requested amount is checked against.
// ------------------------------------------------------------------------------------------------

/// Error for an overflow in the surcharge balance arithmetic (should never happen for real amounts).
fn arithmetic_overflow() -> error_stack::Report<errors::ApiErrorResponse> {
    error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Arithmetic overflow while computing surcharge refund balances")
}

/// How much order amount and surcharge fee remain refundable, derived from the original attempt
/// and the stored amounts on previous refunds. No I/O.
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
    /// Pure calculation: derives the balances from the originally-charged surcharge amount and the
    /// stored amounts on previous (non-failed) refunds. The caller (`validate_partial_refund`) has
    /// already verified that a usable surcharge applies, so there is nothing to validate here.
    fn from_previous_refunds(
        attempt: &storage::PaymentAttempt,
        original_surcharge_amount: MinorUnit,
        previous_refunds: &[Refund],
    ) -> RouterResult<Self> {
        let original_surcharge_amount = original_surcharge_amount.get_amount_as_i64();
        let original_order_amount = attempt
            .get_total_amount()
            .get_amount_as_i64()
            .checked_sub(original_surcharge_amount)
            .ok_or_else(arithmetic_overflow)?;

        let mut refunded_order_amount: i64 = 0;
        let mut refunded_surcharge_amount: i64 = 0;
        for refund in previous_refunds
            .iter()
            .filter(|refund| !refund.refund_status.is_failure())
        {
            refunded_order_amount = refunded_order_amount
                .checked_add(refund.refund_amount.get_amount_as_i64())
                .ok_or_else(arithmetic_overflow)?;
            refunded_surcharge_amount = refunded_surcharge_amount
                .checked_add(
                    refund
                        .refund_surcharge_amount
                        .unwrap_or_default()
                        .get_amount_as_i64(),
                )
                .ok_or_else(arithmetic_overflow)?;
        }

        Ok(Self {
            original_order_amount,
            remaining_order_amount: original_order_amount
                .checked_sub(refunded_order_amount)
                .ok_or_else(arithmetic_overflow)?,
            original_surcharge_amount,
            remaining_surcharge_amount: original_surcharge_amount
                .checked_sub(refunded_surcharge_amount)
                .ok_or_else(arithmetic_overflow)?,
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

    /// Create-flow check of the requested order amount against the remaining order balance. The
    /// captured-funds check is left to the generic `validate_refund_amount`.
    fn validate_requested_order_amount(
        &self,
        requested_order_amount: MinorUnit,
    ) -> RouterResult<()> {
        hyperswitch_domain_models::refunds::validate_surcharge_order_amount(
            self.original_order_amount,
            self.remaining_order_amount,
            requested_order_amount.get_amount_as_i64(),
        )
    }

    /// Checks the Init-reported surcharge fee against the remaining surcharge balance and guards
    /// against an InterPayments inconsistency. Runs after Init (the fee only exists then), not in
    /// the create flow.
    fn validate_init_surcharge_fee(
        &self,
        response: &SurchargeRefundInitResponseData,
    ) -> RouterResult<()> {
        let refund_surcharge_fee = response.refund_surcharge_amount;
        if refund_surcharge_fee < MinorUnit::zero()
            || refund_surcharge_fee.get_amount_as_i64() > self.remaining_surcharge_amount
        {
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Surcharge refund exceeds remaining surcharge amount".to_string(),
                },
            ));
        }
        // Init echoes the original surcharge InterPayments recorded. If it disagrees with what we
        // charged, our records are out of sync — reject and flag for reconciliation.
        let original_fee_mismatch = response
            .original_transaction_fee
            .is_some_and(|amount| amount.get_amount_as_i64() != self.original_surcharge_amount);
        // This refund's fee plus everything InterPayments already refunded must not exceed the
        // original fee (checked add avoids overflow on the running total).
        let refunded_fee_exceeded = match response.previously_refunded_transaction_fees {
            Some(previously_refunded) => {
                refund_surcharge_fee
                    .get_amount_as_i64()
                    .checked_add(previously_refunded.get_amount_as_i64())
                    .ok_or_else(arithmetic_overflow)?
                    > self.original_surcharge_amount
            }
            None => false,
        };
        if original_fee_mismatch || refunded_fee_exceeded {
            return Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message:
                        "InterPayments surcharge balance is inconsistent; reconciliation required"
                            .to_string(),
                },
            ));
        }
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------
// Create — two steps the refund-create flow runs: `validate_partial_refund`, then
// `init_surcharge_refund`.
// ------------------------------------------------------------------------------------------------

/// A validated partial surcharge refund: the order-only amount to refund, plus the balances the
/// surcharge fee is later checked against. Produced by [`validate_partial_refund`], consumed by
/// [`init_surcharge_refund`].
pub struct SurchargeRefundContext {
    order_amount: MinorUnit,
    balances: SurchargeRefundBalances,
}

impl SurchargeRefundContext {
    /// The order-only amount this refund will refund (what goes on the refund row).
    pub fn order_amount(&self) -> MinorUnit {
        self.order_amount
    }
}

/// Step 1 (validate): for a partial surcharge refund, validate the requested order amount against
/// the remaining order balance. Returns the validated context for [`init_surcharge_refund`];
/// `None` for a plain or whole-order refund, which takes the legacy path.
pub fn validate_partial_refund(
    attempt: &storage::PaymentAttempt,
    intent: &storage::PaymentIntent,
    requested_amount: Option<MinorUnit>,
    previous_refunds: &[Refund],
) -> RouterResult<Option<SurchargeRefundContext>> {
    // The usable surcharge charged on the original payment (same condition as `has_surcharge`).
    let usable_surcharge = attempt
        .external_surcharge_details
        .as_ref()
        .filter(|details| details.usable_connector_id().is_some());

    match usable_surcharge {
        // No usable surcharge: a plain refund (legacy path), unless one was actually charged and its
        // details are now lost — then reconciliation is required.
        None => {
            let surcharge_was_calculated = intent.external_surcharge_applicable == Some(true)
                && attempt
                    .net_amount
                    .get_surcharge_amount()
                    .is_some_and(|surcharge| surcharge > MinorUnit::zero());
            if surcharge_was_calculated {
                Err(error_stack::Report::new(
                    errors::ApiErrorResponse::PreconditionFailed {
                        message:
                            "Missing original external surcharge details; reconciliation required"
                                .to_string(),
                    },
                ))
            } else {
                Ok(None)
            }
        }
        // The charged surcharge on the attempt must match the stored breakdown.
        Some(details)
            if attempt.net_amount.get_surcharge_amount()
                != Some(details.external_surcharge_amount) =>
        {
            Err(error_stack::Report::new(
                errors::ApiErrorResponse::PreconditionFailed {
                    message: "Charged surcharge breakdown mismatch; reconciliation required"
                        .to_string(),
                },
            ))
        }
        Some(details) => {
            let balances = SurchargeRefundBalances::from_previous_refunds(
                attempt,
                details.external_surcharge_amount,
                previous_refunds,
            )?;
            // A whole-order refund keeps the legacy single-call path; only partial refunds use
            // Init/Complete.
            if balances.is_whole_order_refund(requested_amount, previous_refunds) {
                Ok(None)
            } else {
                let order_amount = requested_amount
                    .unwrap_or_else(|| MinorUnit::new(balances.remaining_order_amount));
                balances.validate_requested_order_amount(order_amount)?;
                Ok(Some(SurchargeRefundContext {
                    order_amount,
                    balances,
                }))
            }
        }
    }
}

/// Step 2 (initiate/calculate): call InterPayments Init for the validated order amount, validate the
/// returned fee, and return the surcharge tracking to store on the refund row. Runs before the row
/// is inserted, so any error simply fails the create (nothing is persisted).
pub async fn init_surcharge_refund(
    state: &SessionState,
    platform: &domain::Platform,
    attempt: &storage::PaymentAttempt,
    refund_id: &str,
    context: &SurchargeRefundContext,
) -> RouterResult<ExternalSurchargeRefundDetails> {
    let currency = attempt.currency.ok_or_else(|| {
        error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Missing currency on payment attempt")
    })?;
    let profile_id = &attempt.profile_id;
    let mca = resolve_interpayments_account(state, platform, profile_id).await?;
    let original_surcharge_id = attempt
        .external_surcharge_details
        .as_ref()
        .and_then(|details| details.usable_connector_id())
        .ok_or_else(|| {
            error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Missing original surcharge ID on payment attempt")
        })?
        .to_owned();

    let response = ucs::call_unified_connector_service_for_surcharge_refund_init(
        state,
        platform.get_processor(),
        mca,
        profile_id,
        PaymentsSurchargeRefundInitData {
            refund_id: refund_id.to_owned(),
            original_surcharge_id,
            order_amount: context.order_amount,
            currency,
        },
    )
    .await
    .attach_printable("SurchargeRefundInit call failed")?;

    let data = SurchargeRefundInitResponseData::foreign_try_from((&response, refund_id, currency))
        .map_err(|message| {
            error_stack::Report::new(errors::ApiErrorResponse::PreconditionFailed {
                message: message.to_string(),
            })
        })?;

    context.balances.validate_init_surcharge_fee(&data)?;

    let surcharge_refund_id = response
        .connector_surcharge_refund_id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            error_stack::Report::new(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "SurchargeRefundInit succeeded but returned no surcharge refund ID",
                )
        })?;

    Ok(ExternalSurchargeRefundDetails {
        surcharge_refund_id,
        refund_surcharge_amount: data.refund_surcharge_amount,
        refund_complete_notified: false,
    })
}

/// Drives `SurchargeRefundComplete` from the refund's outgoing webhook delivery.
///
/// `None` when the refund is not a partial surcharge refund (no Init ran) — caller takes the
/// ordinary notify path. Otherwise calls Complete and returns a `NotifyConnectorResponseData` that
/// plugs into the generic notify success/retry handling. A failure reschedules the webhook, never
/// the refund: the money is already refunded.
#[cfg(all(feature = "oltp", feature = "v1"))]
pub async fn complete_from_webhook(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    key_store: &domain::MerchantKeyStore,
    profile_id: &common_utils::id_type::ProfileId,
    surcharge_account: helpers::MerchantConnectorAccountType,
    refund_id: &str,
) -> error_stack::Result<
    Option<hyperswitch_domain_models::router_response_types::NotifyConnectorResponseData>,
    errors::ApiClientError,
> {
    let acknowledged =
        hyperswitch_domain_models::router_response_types::NotifyConnectorResponseData {
            status_code: 200,
            error_code: None,
            error_message: None,
        };

    let storage_scheme = state
        .store
        .find_merchant_account_by_merchant_id(merchant_id, key_store)
        .await
        .map(|account| account.storage_scheme)
        .unwrap_or_default();

    let refund = state
        .store
        .find_refund_by_processor_merchant_id_refund_id(merchant_id, refund_id, storage_scheme)
        .await
        .change_context(errors::ApiClientError::UnexpectedServerResponse)
        .attach_printable("Failed to fetch refund for SurchargeRefundComplete")?;

    // Not a partial surcharge refund (no Init ran): let the caller take the ordinary notify path.
    let Some(details) = refund.surcharge_refund_details() else {
        return Ok(None);
    };
    // Already completed: the gate that keeps Complete to exactly one successful call per refund.
    if details.refund_complete_notified {
        return Ok(Some(acknowledged));
    }

    let response = ucs::call_unified_connector_service_for_surcharge_refund_complete(
        state,
        merchant_id,
        surcharge_account,
        profile_id,
        PaymentsSurchargeRefundCompleteData {
            refund_id: refund.refund_id.clone(),
            connector_surcharge_refund_id: details.surcharge_refund_id.clone(),
        },
    )
    .await
    .change_context(errors::ApiClientError::UnexpectedServerResponse)
    .attach_printable("SurchargeRefundComplete call failed")?;

    SurchargeRefundCompleteResponseData::foreign_try_from((
        &response,
        refund.refund_id.as_str(),
        details.surcharge_refund_id.as_str(),
    ))
    .map_err(|message| {
        error_stack::Report::new(errors::ApiClientError::UnexpectedServerResponse)
            .attach_printable(message.to_string())
    })?;

    // Record the acknowledgement so a webhook retry can't re-call Complete. A persist failure is
    // only logged, since the connector has already acknowledged.
    let completed = ExternalSurchargeRefundDetails {
        refund_complete_notified: true,
        ..details
    };
    if let Err(persist_error) = state
        .store
        .update_refund(
            refund.clone(),
            RefundUpdate::SurchargeUpdate {
                external_surcharge_refund_details: completed,
                updated_by: storage_scheme.to_string(),
            },
            storage_scheme,
        )
        .await
    {
        logger::error!(
            ?persist_error,
            refund_id = %refund.refund_id,
            "SurchargeRefundComplete acknowledged but failed to persist refund_complete_notified"
        );
    }

    Ok(Some(acknowledged))
}

// ------------------------------------------------------------------------------------------------
// Shared — connector resolution used by initiate and complete.
// ------------------------------------------------------------------------------------------------

/// Resolves the surcharge connector account configured on the profile (the InterPayments MCA).
async fn resolve_interpayments_account(
    state: &SessionState,
    platform: &domain::Platform,
    profile_id: &common_utils::id_type::ProfileId,
) -> RouterResult<helpers::MerchantConnectorAccountType> {
    let processor = platform.get_processor();
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
    helpers::get_merchant_connector_account(
        state,
        processor,
        None,
        profile_id,
        "interpayments",
        Some(account_id),
    )
    .await
}
