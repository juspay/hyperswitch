use common_enums::AttemptStatus;

use crate::{
    core::revenue_recovery::types::RevenueRecoveryPaymentsAttemptStatus,
    types::transformers::ForeignFrom,
};

impl ForeignFrom<AttemptStatus> for RevenueRecoveryPaymentsAttemptStatus {
    fn foreign_from(s: AttemptStatus) -> Self {
        match s {
            AttemptStatus::Authorized | AttemptStatus::Charged | AttemptStatus::AutoRefunded => {
                Self::Succeeded
            }

            AttemptStatus::Started
            | AttemptStatus::AuthenticationSuccessful
            | AttemptStatus::Authorizing
            | AttemptStatus::CodInitiated
            | AttemptStatus::VoidInitiated
            | AttemptStatus::CaptureInitiated
            | AttemptStatus::Pending => Self::Processing,

            AttemptStatus::AuthenticationFailed
            | AttemptStatus::AuthorizationFailed
            | AttemptStatus::VoidFailed
            | AttemptStatus::RouterDeclined
            | AttemptStatus::CaptureFailed
            | AttemptStatus::Failure => Self::Failed,

            AttemptStatus::Voided
            | AttemptStatus::ConfirmationAwaited
            | AttemptStatus::PartialCharged
            | AttemptStatus::PartialChargedAndChargeable
            | AttemptStatus::PaymentMethodAwaited
            | AttemptStatus::AuthenticationPending
            | AttemptStatus::DeviceDataCollectionPending
            | AttemptStatus::Unresolved
            | AttemptStatus::IntegrityFailure
            | AttemptStatus::Expired => Self::InvalidStatus(s.to_string()),
        }
    }
}

impl ForeignFrom<api_models::payments::RecoveryPaymentsCreate>
    for hyperswitch_domain_models::revenue_recovery::RevenueRecoveryInvoiceData
{
    fn foreign_from(data: api_models::payments::RecoveryPaymentsCreate) -> Self {
        Self {
            amount: data.amount_details.order_amount().into(),
            currency: data.amount_details.currency(),
            merchant_reference_id: data.merchant_reference_id,
            billing_address: data.billing,
            retry_count: data.retry_count,
            next_billing_at: data.next_billing_date,
            billing_started_at: data.billing_started_at,
        }
    }
}
