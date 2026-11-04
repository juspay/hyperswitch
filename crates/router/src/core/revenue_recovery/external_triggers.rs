use api_models::payments::RecoveryPaymentsCreate;
use common_types::payments::{RecoveryConnector, RecoveryConnectorDetails};
use diesel_models::business_profile::RevenueRecoveryConfig;
use error_stack::report;
use hyperswitch_domain_models::{platform::Platform, revenue_recovery::RecoveryPaymentIntent};

use super::api;
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::operations::{
            payment_attempt_record::PaymentAttemptRecord, ValidateStatusForOperation,
        },
    },
    routes::{app::ReqState, SessionState},
    types::domain,
};

/// Records a transaction the merchant reports against the invoice, with no connector account behind
/// it. Nothing is scheduled for it, as the merchant triggers the retries.
pub(super) async fn record_reported_transaction(
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

    let (_, updated_invoice) = Box::pin(api::record_transaction(
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

    Ok(updated_invoice)
}
