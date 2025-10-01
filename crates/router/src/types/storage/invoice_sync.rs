use common_utils::{id_type, types::MinorUnit};

use api_models::enums as api_enums;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InvoiceSyncTrackingData {
    pub payment_id: id_type::PaymentId,
    pub subscription_id: id_type::SubscriptionId,
    pub invoice_id: id_type::InvoiceId,
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub customer_id: id_type::CustomerId,
    pub connector_invoice_id: String,
    pub connector_name: api_enums::Connector, // The connector to which the invoice belongs
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InvoiceSyncRequest {
    pub payment_id: id_type::PaymentId,
    pub subscription_id: id_type::SubscriptionId,
    pub invoice_id: id_type::InvoiceId,
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub customer_id: id_type::CustomerId,
    pub connector_invoice_id: String,
    pub connector_name: api_enums::Connector,
}

impl From<InvoiceSyncRequest> for InvoiceSyncTrackingData {
    fn from(item: InvoiceSyncRequest) -> Self {
        Self {
            payment_id: item.payment_id,
            subscription_id: item.subscription_id,
            invoice_id: item.invoice_id,
            merchant_id: item.merchant_id,
            profile_id: item.profile_id,
            customer_id: item.customer_id,
            connector_invoice_id: item.connector_invoice_id,
            connector_name: item.connector_name,
        }
    }
}

impl InvoiceSyncRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        payment_id: id_type::PaymentId,
        subscription_id: id_type::SubscriptionId,
        invoice_id: id_type::InvoiceId,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        customer_id: id_type::CustomerId,
        connector_invoice_id: String,
        connector_name: api_enums::Connector,
    ) -> Self {
        Self {
            payment_id,
            subscription_id,
            invoice_id,
            merchant_id,
            profile_id,
            customer_id,
            connector_invoice_id,
            connector_name,
        }
    }
}

impl InvoiceSyncTrackingData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        payment_id: id_type::PaymentId,
        subscription_id: id_type::SubscriptionId,
        invoice_id: id_type::InvoiceId,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        customer_id: id_type::CustomerId,
        connector_invoice_id: String,
        connector_name: api_enums::Connector,
    ) -> Self {
        Self {
            payment_id,
            subscription_id,
            invoice_id,
            merchant_id,
            profile_id,
            customer_id,
            connector_invoice_id,
            connector_name,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InvoiceSyncPaymentStatus {
    PaymentSucceeded,
    PaymentProcessing,
    PaymentFailed,
}

impl From<common_enums::IntentStatus> for InvoiceSyncPaymentStatus {
    fn from(value: common_enums::IntentStatus) -> Self {
        match value {
            common_enums::IntentStatus::Succeeded => Self::PaymentSucceeded,
            common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresCustomerAction
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::RequiresPaymentMethod => Self::PaymentProcessing,
            _ => Self::PaymentFailed,
        }
    }
}

/// Dummy Type for skeleton code, to be removed once Payments S2S call is merged
#[derive(Debug, Clone)]
pub struct PaymentsResponse {
    pub status: common_enums::IntentStatus,
    pub amount: MinorUnit,
    pub currency: api_enums::Currency,
}
