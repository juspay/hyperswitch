use common_enums::AttemptStatus;
use masking::PeekInterface;

use crate::{
    core::revenue_recovery::types::RevenueRecoveryPaymentsAttemptStatus,
    types::transformers::ForeignFrom,
};

impl ForeignFrom<AttemptStatus> for RevenueRecoveryPaymentsAttemptStatus {
    fn foreign_from(s: AttemptStatus) -> Self {
        match s {
            AttemptStatus::Authorized
            | AttemptStatus::Charged
            | AttemptStatus::AutoRefunded
            | AttemptStatus::PartiallyAuthorized
            | AttemptStatus::PartialCharged
            | AttemptStatus::PartialChargedAndChargeable => Self::Succeeded,

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
            | AttemptStatus::VoidedPostCharge
            | AttemptStatus::ConfirmationAwaited
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
            retry_count: None,
            next_billing_at: None,
            billing_started_at: data.billing_started_at,
            metadata: data.metadata,
            enable_partial_authorization: data.enable_partial_authorization,
        }
    }
}

impl ForeignFrom<&api_models::payments::RecoveryPaymentsCreate>
    for hyperswitch_domain_models::revenue_recovery::RevenueRecoveryAttemptData
{
    fn foreign_from(data: &api_models::payments::RecoveryPaymentsCreate) -> Self {
        Self {
            amount: data.amount_details.order_amount().into(),
            currency: data.amount_details.currency(),
            merchant_reference_id: data.merchant_reference_id.to_owned(),
            connector_transaction_id: data.connector_transaction_id.as_ref().map(|txn_id| {
                common_utils::types::ConnectorTransactionId::TxnId(txn_id.peek().to_string())
            }),
            error_code: data.error.as_ref().map(|error| error.code.clone()),
            error_message: data.error.as_ref().map(|error| error.message.clone()),
            processor_payment_method_token: data
                .payment_method_data
                .primary_processor_payment_method_token
                .peek()
                .to_string(),
            connector_customer_id: data.connector_customer_id.peek().to_string(),
            connector_account_reference_id: data
                .payment_merchant_connector_id
                .get_string_repr()
                .to_string(),
            transaction_created_at: data.transaction_created_at.to_owned(),
            status: data.attempt_status,
            payment_method_type: data.payment_method_type,
            payment_method_sub_type: data.payment_method_sub_type,
            network_advice_code: data
                .error
                .as_ref()
                .and_then(|error| error.network_advice_code.clone()),
            network_decline_code: data
                .error
                .as_ref()
                .and_then(|error| error.network_decline_code.clone()),
            network_error_message: data
                .error
                .as_ref()
                .and_then(|error| error.network_error_message.clone()),
            // retry count will be updated whenever there is new attempt is created.
            retry_count: None,
            invoice_next_billing_time: None,
            invoice_billing_started_at_time: data.billing_started_at,
            card_info: data
                .payment_method_data
                .additional_payment_method_info
                .clone(),
            charge_id: None,
        }
    }
}
