use api_models::webhooks::IncomingWebhookEvent;
use common_enums::TriggeredBy;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
};
use time::PrimitiveDateTime;

/// Recovery payload is unified struct constructed from billing connectors
#[derive(Debug)]
pub struct RecoveryPayload {
    /// amount
    pub amount: common_utils::types::MinorUnit,
    /// currency
    pub currency: common_enums::enums::Currency,
    /// merchant reference id ex: invoice_id
    pub merchant_reference_id: String,
    /// connector transaction id
    pub connector_transaction_id: Option<String>,
    /// error code for failure payments
    pub error_code: Option<String>,
    /// error_message for failure messages
    pub error_message: Option<String>,
    /// mandate id of the connector
    pub connector_mandate_id: Option<String>,
    /// connnector customer id
    pub connector_customer_id: Option<String>,
    /// payment merchant connnector account reference id
    pub connector_account_reference_id: Option<String>,
    /// timestamp at which transaction has been created at billing connector
    pub transaction_created_at: Option<PrimitiveDateTime>,
    /// status of the transaction
    pub status: common_enums::enums::AttemptStatus,
    /// payment method of payment attempt
    pub payment_method_type: common_enums::enums::PaymentMethod,
    /// payment method sub type of the payment attempt
    pub payment_method_sub_type: common_enums::enums::PaymentMethodType,
}

/// Trait definition
pub trait RecoveryTrait {
    /// Get the payment intent
    fn get_intent(&self) -> Result<PaymentIntent, ApiErrorResponse>;
    /// Get the payment attempt
    fn get_attempt(&self) -> Result<PaymentAttempt, ApiErrorResponse>;
}

/// Implement the trait for RecoveryPayload
impl RecoveryTrait for RecoveryPayload {
    fn get_intent(&self) -> Result<PaymentIntent, ApiErrorResponse> {
        todo!("Implement the logic to retrieve the payment intent");
    }

    fn get_attempt(&self) -> Result<PaymentAttempt, ApiErrorResponse> {
        todo!("Implement the logic to retrieve the payment attempt");
    }
}

/// type of action that needs to taken after consuming recovery payload
#[derive(Debug)]
pub enum RecoveryAction {
    /// add docs
    CancelInvoice,
    /// add docs
    FailPaymentExternal,
    /// add docs
    SuccessPaymentExternal,
    /// add docs
    PendingPayment,
    /// add docs
    NoAction,
    /// add docs
    InvalidAction,
}

/// add docs
pub trait RecoveryActionTrait {
    /// add docs
    fn find_action(
        event_type: IncomingWebhookEvent,
        attempt_triggered_by: Option<TriggeredBy>,
    ) -> Self;
}

impl RecoveryActionTrait for RecoveryAction {
    fn find_action(
        event_type: IncomingWebhookEvent,
        attempt_triggered_by: Option<TriggeredBy>,
    ) -> Self {
        match event_type {
            IncomingWebhookEvent::PaymentIntentFailure
            | IncomingWebhookEvent::PaymentIntentSuccess
            | IncomingWebhookEvent::PaymentIntentProcessing
            | IncomingWebhookEvent::PaymentIntentPartiallyFunded
            | IncomingWebhookEvent::PaymentIntentCancelled
            | IncomingWebhookEvent::PaymentIntentCancelFailure
            | IncomingWebhookEvent::PaymentIntentAuthorizationSuccess
            | IncomingWebhookEvent::PaymentIntentAuthorizationFailure
            | IncomingWebhookEvent::PaymentIntentCaptureSuccess
            | IncomingWebhookEvent::PaymentIntentCaptureFailure
            | IncomingWebhookEvent::PaymentActionRequired
            | IncomingWebhookEvent::EventNotSupported
            | IncomingWebhookEvent::SourceChargeable
            | IncomingWebhookEvent::SourceTransactionCreated
            | IncomingWebhookEvent::RefundFailure
            | IncomingWebhookEvent::RefundSuccess
            | IncomingWebhookEvent::DisputeOpened
            | IncomingWebhookEvent::DisputeExpired
            | IncomingWebhookEvent::DisputeAccepted
            | IncomingWebhookEvent::DisputeCancelled
            | IncomingWebhookEvent::DisputeChallenged
            | IncomingWebhookEvent::DisputeWon
            | IncomingWebhookEvent::DisputeLost
            | IncomingWebhookEvent::MandateActive
            | IncomingWebhookEvent::MandateRevoked
            | IncomingWebhookEvent::EndpointVerification
            | IncomingWebhookEvent::ExternalAuthenticationARes
            | IncomingWebhookEvent::FrmApproved
            | IncomingWebhookEvent::FrmRejected
            | IncomingWebhookEvent::PayoutSuccess
            | IncomingWebhookEvent::PayoutFailure
            | IncomingWebhookEvent::PayoutProcessing
            | IncomingWebhookEvent::PayoutCancelled
            | IncomingWebhookEvent::PayoutCreated
            | IncomingWebhookEvent::PayoutExpired
            | IncomingWebhookEvent::PayoutReversed => Self::InvalidAction,
            IncomingWebhookEvent::RecoveryPaymentFailure => match attempt_triggered_by {
                Some(TriggeredBy::Internal) => Self::NoAction,
                Some(TriggeredBy::External) | None => Self::FailPaymentExternal,
            },
            IncomingWebhookEvent::RecoveryPaymentSuccess => match attempt_triggered_by {
                Some(TriggeredBy::Internal) => Self::NoAction,
                Some(TriggeredBy::External) | None => Self::SuccessPaymentExternal,
            },
            IncomingWebhookEvent::RecoveryPaymentPending => Self::PendingPayment,
            IncomingWebhookEvent::RecoveryInvoiceCancel => Self::CancelInvoice,
        }
    }
}
