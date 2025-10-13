use api_models::enums as api_enums;
use common_utils::id_type;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InvoiceSyncTrackingData {
    pub subscription_id: id_type::SubscriptionId,
    pub invoice_id: id_type::InvoiceId,
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub customer_id: id_type::CustomerId,
    // connector_invoice_id is optional because in some cases (Trial/Future), the invoice might not have been created in the connector yet.
    pub connector_invoice_id: Option<id_type::InvoiceId>,
    pub connector_name: api_enums::Connector, // The connector to which the invoice belongs
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InvoiceSyncRequest {
    pub subscription_id: id_type::SubscriptionId,
    pub invoice_id: id_type::InvoiceId,
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub customer_id: id_type::CustomerId,
    pub connector_invoice_id: Option<id_type::InvoiceId>,
    pub connector_name: api_enums::Connector,
}

impl From<InvoiceSyncRequest> for InvoiceSyncTrackingData {
    fn from(item: InvoiceSyncRequest) -> Self {
        Self {
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
        subscription_id: id_type::SubscriptionId,
        invoice_id: id_type::InvoiceId,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        customer_id: id_type::CustomerId,
        connector_invoice_id: Option<id_type::InvoiceId>,
        connector_name: api_enums::Connector,
    ) -> Self {
        Self {
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
        subscription_id: id_type::SubscriptionId,
        invoice_id: id_type::InvoiceId,
        merchant_id: id_type::MerchantId,
        profile_id: id_type::ProfileId,
        customer_id: id_type::CustomerId,
        connector_invoice_id: Option<id_type::InvoiceId>,
        connector_name: api_enums::Connector,
    ) -> Self {
        Self {
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

impl From<InvoiceSyncPaymentStatus> for common_enums::connector_enums::InvoiceStatus {
    fn from(value: InvoiceSyncPaymentStatus) -> Self {
        match value {
            InvoiceSyncPaymentStatus::PaymentSucceeded => Self::InvoicePaid,
            InvoiceSyncPaymentStatus::PaymentProcessing => Self::PaymentPending,
            InvoiceSyncPaymentStatus::PaymentFailed => Self::PaymentFailed,
        }
    }
}
