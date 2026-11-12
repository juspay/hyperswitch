use api_models::payments::RecoveryPaymentsCreate;
use common_types::payments::{
    RecoveryConnector, RecoveryConnectorDetails, RecoveryTransactionStatus,
};
use common_utils::types::MinorUnit;
use diesel_models::business_profile::RevenueRecoveryConfig;
use error_stack::report;
use hyperswitch_domain_models::{
    payments::PaymentIntent, platform::Platform, revenue_recovery::RecoveryPaymentIntent,
};
use router_env::logger;

use super::api;
use crate::{
    core::{
        api_locking::{LockAction, LockingInput},
        errors::{self, RouterResult},
        payments::operations::{
            payment_attempt_record::PaymentAttemptRecord, ValidateStatusForOperation,
        },
    },
    routes::{app::ReqState, lock_utils, SessionState},
    types::domain,
};

/// Records a transaction the merchant reports against the invoice, with no connector account behind
/// it. Nothing is scheduled for it, as the merchant triggers the retries. The reports of an invoice
/// are handled one at a time, as what each one captures adds to what the earlier ones did, and no
/// connector sync is there to correct a report handled out of turn.
pub(super) async fn record_reported_transaction(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &RecoveryPaymentsCreate,
    connector: RecoveryConnector,
    retry_config: &RevenueRecoveryConfig,
) -> RouterResult<RecoveryPaymentIntent> {
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

async fn handle_merchant_report(
    state: &SessionState,
    req_state: &ReqState,
    platform: &Platform,
    profile: &domain::Profile,
    request: &RecoveryPaymentsCreate,
    connector: RecoveryConnector,
    retry_config: &RevenueRecoveryConfig,
) -> RouterResult<RecoveryPaymentIntent> {
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

    let invoice_before_report =
        api::get_payment_intent_api(state, req_state, platform, profile, &invoice.payment_id)
            .await?;
    let request = &RecoveryPaymentsCreate {
        amount_captured: get_amount_captured_of_report(request, &invoice_before_report)?,
        ..request.clone()
    };

    Box::pin(api::record_transaction(
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

    let payment_intent =
        api::get_payment_intent_api(state, req_state, platform, profile, &invoice.payment_id)
            .await?;

    Ok(RecoveryPaymentIntent::from(&payment_intent))
}

/// What a report collected. The order amount of every report is the amount of the invoice, and what
/// the transactions collect adds up towards it, so a charged transaction collects whatever is left.
/// A report that does not fit is refused here, before anything is recorded.
fn get_amount_captured_of_report(
    request: &RecoveryPaymentsCreate,
    invoice: &PaymentIntent,
) -> RouterResult<Option<MinorUnit>> {
    let invoice_amount = invoice.amount_details.order_amount;
    let invalid = |message: &str| {
        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: message.to_owned(),
        }))
    };

    if MinorUnit::from(request.amount_details.order_amount()) != invoice_amount {
        return invalid("`order_amount` must be the amount of the invoice on every transaction");
    }

    let amount_left = invoice_amount - invoice.amount_captured.unwrap_or(MinorUnit::zero());
    match (request.transaction_status, request.amount_captured) {
        (RecoveryTransactionStatus::Charged, _) => Ok(Some(amount_left)),
        (RecoveryTransactionStatus::PartialCharged, Some(amount_captured))
            if amount_captured > amount_left =>
        {
            invalid("`amount_captured` must not be more than what is left of the invoice")
        }
        (_, amount_captured) => Ok(amount_captured),
    }
}
