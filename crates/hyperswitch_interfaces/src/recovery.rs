use api_models::webhooks::IncomingWebhookEvent;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    payment_methods::PaymentMethod,
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
};
use time::PrimitiveDateTime;

/// Recovery payload is unified struct constructed from billing connectors
#[derive(Debug, Clone)]
pub struct RecoveryPayload {
    /// amount
    pub amount: common_utils::types::MinorUnit,
    /// currency
    pub currency: common_enums::enums::Currency,
    /// merchant reference id ex: invoice_id
    pub merchant_reference_id: common_utils::id_type::PaymentReferenceId,
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
    /// created_at
    pub created_at: PrimitiveDateTime,
    /// status of the transaction
    pub status: common_enums::enums::AttemptStatus,
    /// payment method of payment attempt
    pub payment_method_type: common_enums::enums::PaymentMethod,
    /// payment method sub type of the payment attempt
    pub payment_method_sub_type: common_enums::enums::PaymentMethodType,
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
        triggered_by: Option<common_enums::TriggeredBy>,
    ) -> Self;
}

impl RecoveryActionTrait for RecoveryAction {
    fn find_action(
        event_type: IncomingWebhookEvent,
        triggered_by: Option<common_enums::TriggeredBy>,
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
            IncomingWebhookEvent::RecoveryPaymentFailure => match triggered_by {
                Some(common_enums::TriggeredBy::Internal) => Self::NoAction,
                Some(common_enums::TriggeredBy::External) | None => Self::FailPaymentExternal,
            },
            IncomingWebhookEvent::RecoveryPaymentSuccess => match triggered_by {
                Some(common_enums::TriggeredBy::Internal) => Self::NoAction,
                Some(common_enums::TriggeredBy::External) | None => Self::SuccessPaymentExternal,
            },
            IncomingWebhookEvent::RecoveryPaymentPending => Self::PendingPayment,
            IncomingWebhookEvent::RecoveryInvoiceCancel => Self::CancelInvoice,
        }
    }
}
