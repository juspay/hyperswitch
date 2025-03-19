use api_models::{payments as api_payments, webhooks};
use common_enums::enums as common_enums;
use common_utils::{id_type, types as util_types};
use time::PrimitiveDateTime;

/// Recovery payload is unified struct constructed from billing connectors
#[derive(Debug)]
pub struct RevenueRecoveryAttemptData {
    /// transaction amount against invoice, accepted in minor unit.
    pub amount: util_types::MinorUnit,
    /// currency of the transaction
    pub currency: common_enums::Currency,
    /// merchant reference id at billing connector. ex: invoice_id
    pub merchant_reference_id: id_type::PaymentReferenceId,
    /// transaction id reference at payment connector
    pub connector_transaction_id: Option<util_types::ConnectorTransactionId>,
    /// error code sent by billing connector.
    pub error_code: Option<String>,
    /// error message sent by billing connector.
    pub error_message: Option<String>,
    /// mandate token at payment processor end.
    pub processor_payment_method_token: String,
    /// customer id at payment connector for which mandate is attached.
    pub connector_customer_id: String,
    /// Payment gateway identifier id at billing processor.
    pub connector_account_reference_id: String,
    /// timestamp at which transaction has been created at billing connector
    pub transaction_created_at: Option<PrimitiveDateTime>,
    /// transaction status at billing connector equivalent to payment attempt status.
    pub status: common_enums::AttemptStatus,
    /// payment method of payment attempt.
    pub payment_method_type: common_enums::PaymentMethod,
    /// payment method sub type of the payment attempt.
    pub payment_method_sub_type: common_enums::PaymentMethodType,
}

/// This is unified struct for Revenue Recovery Invoice Data and it is constructed from billing connectors
#[derive(Debug)]
pub struct RevenueRecoveryInvoiceData {
    /// invoice amount at billing connector
    pub amount: util_types::MinorUnit,
    /// currency of the amount.
    pub currency: common_enums::Currency,
    /// merchant reference id at billing connector. ex: invoice_id
    pub merchant_reference_id: id_type::PaymentReferenceId,
}

/// type of action that needs to taken after consuming recovery payload
#[derive(Debug)]
pub enum RecoveryAction {
    /// Stops the process tracker and update the payment intent.
    CancelInvoice,
    /// Records the external transaction against payment intent.
    ScheduleFailedPayment,
    /// Records the external payment and stops the internal process tracker.
    SuccessPaymentExternal,
    /// Pending payments from billing processor.
    PendingPayment,
    /// No action required.
    NoAction,
    /// Invalid event has been received.
    InvalidAction,
}
pub struct RecoveryPaymentIntent {
    pub payment_id: id_type::GlobalPaymentId,
    pub status: common_enums::IntentStatus,
    pub feature_metadata: Option<api_payments::FeatureMetadata>,
}

pub struct RecoveryPaymentAttempt {
    pub attempt_id: id_type::GlobalAttemptId,
    pub attempt_status: common_enums::AttemptStatus,
    pub feature_metadata: Option<api_payments::PaymentAttemptFeatureMetadata>,
}

impl RecoveryPaymentAttempt {
    pub fn get_attempt_triggered_by(&self) -> Option<common_enums::TriggeredBy> {
        self.feature_metadata.as_ref().and_then(|metadata| {
            metadata
                .revenue_recovery
                .as_ref()
                .map(|recovery| recovery.attempt_triggered_by)
        })
    }
}

impl RecoveryAction {
    pub fn get_action(
        event_type: webhooks::IncomingWebhookEvent,
        attempt_triggered_by: Option<common_enums::TriggeredBy>,
    ) -> Self {
        match event_type {
            webhooks::IncomingWebhookEvent::PaymentIntentFailure
            | webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            | webhooks::IncomingWebhookEvent::PaymentIntentProcessing
            | webhooks::IncomingWebhookEvent::PaymentIntentPartiallyFunded
            | webhooks::IncomingWebhookEvent::PaymentIntentCancelled
            | webhooks::IncomingWebhookEvent::PaymentIntentCancelFailure
            | webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationSuccess
            | webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationFailure
            | webhooks::IncomingWebhookEvent::PaymentIntentCaptureSuccess
            | webhooks::IncomingWebhookEvent::PaymentIntentCaptureFailure
            | webhooks::IncomingWebhookEvent::PaymentActionRequired
            | webhooks::IncomingWebhookEvent::EventNotSupported
            | webhooks::IncomingWebhookEvent::SourceChargeable
            | webhooks::IncomingWebhookEvent::SourceTransactionCreated
            | webhooks::IncomingWebhookEvent::RefundFailure
            | webhooks::IncomingWebhookEvent::RefundSuccess
            | webhooks::IncomingWebhookEvent::DisputeOpened
            | webhooks::IncomingWebhookEvent::DisputeExpired
            | webhooks::IncomingWebhookEvent::DisputeAccepted
            | webhooks::IncomingWebhookEvent::DisputeCancelled
            | webhooks::IncomingWebhookEvent::DisputeChallenged
            | webhooks::IncomingWebhookEvent::DisputeWon
            | webhooks::IncomingWebhookEvent::DisputeLost
            | webhooks::IncomingWebhookEvent::MandateActive
            | webhooks::IncomingWebhookEvent::MandateRevoked
            | webhooks::IncomingWebhookEvent::EndpointVerification
            | webhooks::IncomingWebhookEvent::ExternalAuthenticationARes
            | webhooks::IncomingWebhookEvent::FrmApproved
            | webhooks::IncomingWebhookEvent::FrmRejected
            | webhooks::IncomingWebhookEvent::PayoutSuccess
            | webhooks::IncomingWebhookEvent::PayoutFailure
            | webhooks::IncomingWebhookEvent::PayoutProcessing
            | webhooks::IncomingWebhookEvent::PayoutCancelled
            | webhooks::IncomingWebhookEvent::PayoutCreated
            | webhooks::IncomingWebhookEvent::PayoutExpired
            | webhooks::IncomingWebhookEvent::PayoutReversed => Self::InvalidAction,
            webhooks::IncomingWebhookEvent::RecoveryPaymentFailure => match attempt_triggered_by {
                Some(common_enums::TriggeredBy::Internal) => Self::NoAction,
                Some(common_enums::TriggeredBy::External) | None => Self::ScheduleFailedPayment,
            },
            webhooks::IncomingWebhookEvent::RecoveryPaymentSuccess => match attempt_triggered_by {
                Some(common_enums::TriggeredBy::Internal) => Self::NoAction,
                Some(common_enums::TriggeredBy::External) | None => Self::SuccessPaymentExternal,
            },
            webhooks::IncomingWebhookEvent::RecoveryPaymentPending => Self::PendingPayment,
            webhooks::IncomingWebhookEvent::RecoveryInvoiceCancel => Self::CancelInvoice,
        }
    }
}

impl From<&RevenueRecoveryInvoiceData> for api_payments::AmountDetails {
    fn from(data: &RevenueRecoveryInvoiceData) -> Self {
        let amount = api_payments::AmountDetailsSetter {
            order_amount: data.amount.into(),
            currency: data.currency,
            shipping_cost: None,
            order_tax_amount: None,
            skip_external_tax_calculation: common_enums::TaxCalculationOverride::Skip,
            skip_surcharge_calculation: common_enums::SurchargeCalculationOverride::Skip,
            surcharge_amount: None,
            tax_on_surcharge: None,
        };
        Self::new(amount)
    }
}

impl From<&RevenueRecoveryInvoiceData> for api_payments::PaymentsCreateIntentRequest {
    fn from(data: &RevenueRecoveryInvoiceData) -> Self {
        let amount_details = api_payments::AmountDetails::from(data);
        Self {
            amount_details,
            merchant_reference_id: Some(data.merchant_reference_id.clone()),
            routing_algorithm_id: None,
            // Payments in the revenue recovery flow are always recurring transactions,
            // so capture method will be always automatic.
            capture_method: Some(common_enums::CaptureMethod::Automatic),
            authentication_type: Some(common_enums::AuthenticationType::NoThreeDs),
            billing: None,
            shipping: None,
            customer_id: None,
            customer_present: Some(common_enums::PresenceOfCustomerDuringPayment::Absent),
            description: None,
            return_url: None,
            setup_future_usage: None,
            apply_mit_exemption: None,
            statement_descriptor: None,
            order_details: None,
            allowed_payment_method_types: None,
            metadata: None,
            connector_metadata: None,
            feature_metadata: None,
            payment_link_enabled: None,
            payment_link_config: None,
            request_incremental_authorization: None,
            session_expiry: None,
            frm_metadata: None,
            request_external_three_ds_authentication: None,
        }
    }
}

impl From<&RevenueRecoveryAttemptData> for api_payments::PaymentAttemptAmountDetails {
    fn from(data: &RevenueRecoveryAttemptData) -> Self {
        Self {
            net_amount: data.amount,
            amount_to_capture: None,
            surcharge_amount: None,
            tax_on_surcharge: None,
            amount_capturable: data.amount,
            shipping_cost: None,
            order_tax_amount: None,
        }
    }
}

impl From<&RevenueRecoveryAttemptData> for Option<api_payments::RecordAttemptErrorDetails> {
    fn from(data: &RevenueRecoveryAttemptData) -> Self {
        data.error_code
            .as_ref()
            .zip(data.error_message.clone())
            .map(|(code, message)| api_payments::RecordAttemptErrorDetails {
                code: code.to_string(),
                message: message.to_string(),
            })
    }
}
