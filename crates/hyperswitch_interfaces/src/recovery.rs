use api_models::webhooks::IncomingWebhookEvent;
use common_enums::TriggeredBy;
use hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse;
use time::PrimitiveDateTime;

/// Recovery payload is unified struct constructed from billing connectors
#[derive(Debug)]
pub struct RevenueRecoveryTransactionData {
    /// transaction amount against invoice, accepted in minor unit.
    pub amount: common_utils::types::MinorUnit,
    /// currency of the transaction
    pub currency: common_enums::enums::Currency,
    /// merchant reference id at billing connector. ex: invoice_id
    pub merchant_reference_id: String,
    /// transaction id reference at payment connector
    pub connector_transaction_id: Option<String>,
    /// error code sent by billing connector, should be mapped to gateway error if billing connector sends gateway error.
    pub error_code: Option<String>,
    /// error message sent by billing connector, should be mapped to issuer error if billing connector sends issuer error.
    pub error_message: Option<String>,
    /// mandate token at payment processor end.
    pub processor_payment_token: Option<String>,
    /// customer id at payment connector for which mandate is attached.
    pub connector_customer_id: Option<String>,
    /// payment merchant connnector account reference id at billing connector.
    pub connector_account_reference_id: Option<String>,
    /// timestamp at which transaction has been created at billing connector
    pub transaction_created_at: Option<PrimitiveDateTime>,
    /// transaction status at billing connector equivalent to payment attempt status.
    pub status: common_enums::enums::AttemptStatus,
    /// payment method of payment attempt.
    pub payment_method_type: common_enums::enums::PaymentMethod,
    /// payment method sub type of the payment attempt.
    pub payment_method_sub_type: common_enums::enums::PaymentMethodType,
}

/// This is unified struct for Revenue Recovery Invoice Data and it is constructed from billing connectors
#[derive(Debug)]
pub struct RevenueRecoveryInvoiceData {
    /// invoice amount at billing connector
    pub amount: common_utils::types::MinorUnit,
    /// currency of the amount.
    pub currency: common_enums::enums::Currency,
    /// merchant reference id at billing connector. ex: invoice_id
    pub merchant_reference_id: common_utils::id_type::PaymentReferenceId,
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
    /// Invalid event has been recieved.
    InvalidAction,
}

/// add docs
pub trait RevenueRecoveryAction {
    /// add docs
    fn find_action(
        event_type: IncomingWebhookEvent,
        attempt_triggered_by: Option<TriggeredBy>,
    ) -> Self;
}

impl RevenueRecoveryAction for RecoveryAction {
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
                Some(TriggeredBy::External) | None => Self::ScheduleFailedPayment,
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

impl From<&RevenueRecoveryInvoiceData> for api_models::payments::PaymentsCreateIntentRequest {
    fn from(data: &RevenueRecoveryInvoiceData) -> Self {
        let amount_details = api_models::payments::AmountDetails::from(data);
        Self {
            amount_details,
            merchant_reference_id: Some(data.merchant_reference_id.clone()),
            routing_algorithm_id: None,
            capture_method: Some(common_enums::CaptureMethod::Automatic),
            authentication_type: Some(common_enums::AuthenticationType::NoThreeDs),
            billing: None,
            shipping: None,
            customer_id: None,
            customer_present: Some(common_enums::PresenceOfCustomerDuringPayment::Absent),
            description: None,
            return_url: None,
            setup_future_usage: Some(common_enums::FutureUsage::OffSession),
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

impl From<&RevenueRecoveryInvoiceData> for api_models::payments::AmountDetails {
    fn from(data: &RevenueRecoveryInvoiceData) -> Self {
        let amount = api_models::payments::AmountDetailsSetter {
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
