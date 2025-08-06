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
