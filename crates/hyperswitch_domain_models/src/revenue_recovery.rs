use api_models::{payments as api_payments, webhooks};
use common_enums::enums as common_enums;
use common_types::primitive_wrappers;
use common_utils::{id_type, pii, types as util_types};
use time::PrimitiveDateTime;

use crate::{
    payments,
    router_response_types::revenue_recovery::{
        BillingConnectorInvoiceSyncResponse, BillingConnectorPaymentsSyncResponse,
    },
    ApiModelToDieselModelConvertor,
};

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
    /// This field can be returned for both approved and refused Mastercard payments.
    /// This code provides additional information about the type of transaction or the reason why the payment failed.
    /// If the payment failed, the network advice code gives guidance on if and when you can retry the payment.
    pub network_advice_code: Option<String>,
    /// For card errors resulting from a card issuer decline, a brand specific 2, 3, or 4 digit code which indicates the reason the authorization failed.
    pub network_decline_code: Option<String>,
    /// A string indicating how to proceed with an network error if payment gateway provide one. This is used to understand the network error code better.
    pub network_error_message: Option<String>,
    /// Number of attempts made for an invoice
    pub retry_count: Option<u16>,
    /// Time when next invoice will be generated which will be equal to the end time of the current invoice
    pub invoice_next_billing_time: Option<PrimitiveDateTime>,
    /// Time at which the invoice created
    pub invoice_billing_started_at_time: Option<PrimitiveDateTime>,
    /// stripe specific id used to validate duplicate attempts in revenue recovery flow
    pub charge_id: Option<String>,
    /// Additional card details
    pub card_info: api_payments::AdditionalCardInfo,
}

/// This is unified struct for Revenue Recovery Invoice Data and it is constructed from billing connectors
#[derive(Debug, Clone)]
pub struct RevenueRecoveryInvoiceData {
    /// invoice amount at billing connector
    pub amount: util_types::MinorUnit,
    /// currency of the amount.
    pub currency: common_enums::Currency,
    /// merchant reference id at billing connector. ex: invoice_id
    pub merchant_reference_id: id_type::PaymentReferenceId,
    /// billing address id of the invoice
    pub billing_address: Option<api_payments::Address>,
    /// Retry count of the invoice
    pub retry_count: Option<u16>,
    /// Ending date of the invoice or the Next billing time of the Subscription
    pub next_billing_at: Option<PrimitiveDateTime>,
    /// Invoice Starting Time
    pub billing_started_at: Option<PrimitiveDateTime>,
    /// metadata of the merchant
    pub metadata: Option<pii::SecretSerdeValue>,
    /// Allow partial authorization for this payment
    pub enable_partial_authorization: Option<primitive_wrappers::EnablePartialAuthorizationBool>,
}

#[derive(Clone, Debug)]
pub struct RecoveryPaymentIntent {
    pub payment_id: id_type::GlobalPaymentId,
    pub status: common_enums::IntentStatus,
    pub feature_metadata: Option<api_payments::FeatureMetadata>,
    pub merchant_id: id_type::MerchantId,
    pub merchant_reference_id: Option<id_type::PaymentReferenceId>,
    pub invoice_amount: util_types::MinorUnit,
    pub invoice_currency: common_enums::Currency,
    pub created_at: Option<PrimitiveDateTime>,
    pub billing_address: Option<api_payments::Address>,
}

#[derive(Clone, Debug)]
pub struct RecoveryPaymentAttempt {
    pub attempt_id: id_type::GlobalAttemptId,
    pub attempt_status: common_enums::AttemptStatus,
    pub feature_metadata: Option<api_payments::PaymentAttemptFeatureMetadata>,
    pub amount: util_types::MinorUnit,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub error_code: Option<String>,
    pub created_at: PrimitiveDateTime,
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
            billing: data.billing_address.clone(),
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
            metadata: data.metadata.clone(),
            connector_metadata: None,
            feature_metadata: None,
            payment_link_enabled: None,
            payment_link_config: None,
            request_incremental_authorization: None,
            session_expiry: None,
            frm_metadata: None,
            request_external_three_ds_authentication: None,
            force_3ds_challenge: None,
            merchant_connector_details: None,
            enable_partial_authorization: data.enable_partial_authorization,
        }
    }
}

impl From<&BillingConnectorInvoiceSyncResponse> for RevenueRecoveryInvoiceData {
    fn from(data: &BillingConnectorInvoiceSyncResponse) -> Self {
        Self {
            amount: data.amount,
            currency: data.currency,
            merchant_reference_id: data.merchant_reference_id.clone(),
            billing_address: data.billing_address.clone(),
            retry_count: data.retry_count,
            next_billing_at: data.ends_at,
            billing_started_at: data.created_at,
            metadata: None,
            enable_partial_authorization: None,
        }
    }
}

impl
    From<(
        &BillingConnectorPaymentsSyncResponse,
        &RevenueRecoveryInvoiceData,
    )> for RevenueRecoveryAttemptData
{
    fn from(
        data: (
            &BillingConnectorPaymentsSyncResponse,
            &RevenueRecoveryInvoiceData,
        ),
    ) -> Self {
        let billing_connector_payment_details = data.0;
        let invoice_details = data.1;
        Self {
            amount: billing_connector_payment_details.amount,
            currency: billing_connector_payment_details.currency,
            merchant_reference_id: billing_connector_payment_details
                .merchant_reference_id
                .clone(),
            connector_transaction_id: billing_connector_payment_details
                .connector_transaction_id
                .clone(),
            error_code: billing_connector_payment_details.error_code.clone(),
            error_message: billing_connector_payment_details.error_message.clone(),
            processor_payment_method_token: billing_connector_payment_details
                .processor_payment_method_token
                .clone(),
            connector_customer_id: billing_connector_payment_details
                .connector_customer_id
                .clone(),
            connector_account_reference_id: billing_connector_payment_details
                .connector_account_reference_id
                .clone(),
            transaction_created_at: billing_connector_payment_details.transaction_created_at,
            status: billing_connector_payment_details.status,
            payment_method_type: billing_connector_payment_details.payment_method_type,
            payment_method_sub_type: billing_connector_payment_details.payment_method_sub_type,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            retry_count: invoice_details.retry_count,
            invoice_next_billing_time: invoice_details.next_billing_at,
            charge_id: billing_connector_payment_details.charge_id.clone(),
            invoice_billing_started_at_time: invoice_details.billing_started_at,
            card_info: billing_connector_payment_details.card_info.clone(),
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
                network_advice_code: data.network_advice_code.clone(),
                network_decline_code: data.network_decline_code.clone(),
                network_error_message: data.network_error_message.clone(),
            })
    }
}

impl From<&payments::PaymentIntent> for RecoveryPaymentIntent {
    fn from(payment_intent: &payments::PaymentIntent) -> Self {
        Self {
            payment_id: payment_intent.id.clone(),
            status: payment_intent.status,
            feature_metadata: payment_intent
                .feature_metadata
                .clone()
                .map(|feature_metadata| feature_metadata.convert_back()),
            merchant_reference_id: payment_intent.merchant_reference_id.clone(),
            invoice_amount: payment_intent.amount_details.order_amount,
            invoice_currency: payment_intent.amount_details.currency,
            billing_address: payment_intent
                .billing_address
                .clone()
                .map(|address| api_payments::Address::from(address.into_inner())),
            merchant_id: payment_intent.merchant_id.clone(),
            created_at: Some(payment_intent.created_at),
        }
    }
}

impl From<&payments::payment_attempt::PaymentAttempt> for RecoveryPaymentAttempt {
    fn from(payment_attempt: &payments::payment_attempt::PaymentAttempt) -> Self {
        Self {
            attempt_id: payment_attempt.id.clone(),
            attempt_status: payment_attempt.status,
            feature_metadata: payment_attempt
                .feature_metadata
                .clone()
                .map(
                    |feature_metadata| api_payments::PaymentAttemptFeatureMetadata {
                        revenue_recovery: feature_metadata.revenue_recovery.map(|recovery| {
                            api_payments::PaymentAttemptRevenueRecoveryData {
                                attempt_triggered_by: recovery.attempt_triggered_by,
                                charge_id: recovery.charge_id,
                            }
                        }),
                    },
                ),
            amount: payment_attempt.amount_details.get_net_amount(),
            network_advice_code: payment_attempt
                .error
                .clone()
                .and_then(|error| error.network_advice_code),
            network_decline_code: payment_attempt
                .error
                .clone()
                .and_then(|error| error.network_decline_code),
            error_code: payment_attempt
                .error
                .as_ref()
                .map(|error| error.code.clone()),
            created_at: payment_attempt.created_at,
        }
    }
}
