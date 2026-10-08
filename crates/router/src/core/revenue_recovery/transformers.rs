use common_enums::IntentStatus;
use hyperswitch_masking::PeekInterface;

use crate::{
    core::revenue_recovery::types::RevenueRecoveryPaymentIntentStatus,
    types::transformers::ForeignFrom,
};

impl ForeignFrom<IntentStatus> for RevenueRecoveryPaymentIntentStatus {
    fn foreign_from(status: IntentStatus) -> Self {
        match status {
            IntentStatus::Succeeded => Self::Succeeded,
            IntentStatus::PartiallyCapturedAndProcessing | IntentStatus::Processing => {
                Self::Processing
            }
            IntentStatus::Failed => Self::Failed,
            IntentStatus::PartiallyCaptured | IntentStatus::PartiallyCapturedAndCapturable => {
                Self::PartialCharged
            }
            IntentStatus::Cancelled
            | IntentStatus::CancelledPostCapture
            | IntentStatus::RequiresCustomerAction
            | IntentStatus::RequiresMerchantAction
            | IntentStatus::RequiresPaymentMethod
            | IntentStatus::RequiresConfirmation
            | IntentStatus::RequiresCapture
            | IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | IntentStatus::Conflicted
            | IntentStatus::Expired
            | IntentStatus::Review => Self::InvalidStatus(status.to_string()),
        }
    }
}

impl From<RevenueRecoveryPaymentIntentStatus> for common_enums::EventType {
    fn from(status: RevenueRecoveryPaymentIntentStatus) -> Self {
        match status {
            RevenueRecoveryPaymentIntentStatus::Succeeded => Self::PaymentSucceeded,
            RevenueRecoveryPaymentIntentStatus::PartialCharged => Self::PaymentCaptured,
            RevenueRecoveryPaymentIntentStatus::Processing => Self::PaymentProcessing,
            RevenueRecoveryPaymentIntentStatus::Failed => Self::PaymentFailed,
            RevenueRecoveryPaymentIntentStatus::InvalidStatus(_) => Self::ActionRequired,
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
            billing_started_at: Some(data.billing_started_at),
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
            connector_transaction_id: Some(common_utils::types::ConnectorTransactionId::TxnId(
                data.connector_transaction_id.to_owned(),
            )),
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
            transaction_created_at: Some(data.transaction_created_at),
            status: data.transaction_status.into(),
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
            invoice_billing_started_at_time: Some(data.billing_started_at),
            card_info: data.payment_method_data.payment_method_metadata.clone(),
            charge_id: None,
        }
    }
}
