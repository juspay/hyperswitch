use common_enums::enums;

use crate::connector_endpoints;

#[derive(Debug, Clone)]
pub struct BillingConnectorPaymentsSyncRequest {
    /// unique id for making billing connector psync call
    pub billing_connector_psync_id: String,
    /// connector params of the connector
    pub connector_params: connector_endpoints::ConnectorParams,
}

#[derive(Debug, Clone)]
pub struct InvoiceRecordBackRequest {
    pub merchant_reference_id: common_utils::id_type::PaymentReferenceId,
    pub amount: common_utils::types::MinorUnit,
    pub currency: enums::Currency,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub attempt_status: common_enums::AttemptStatus,
    pub connector_transaction_id: Option<common_utils::types::ConnectorTransactionId>,
    pub connector_params: connector_endpoints::ConnectorParams,
}

#[derive(Debug, Clone)]
pub struct DisputeRecordBackRequest {
    /// Invoice the disputed payment was recorded against. The refund is recorded on the
    /// invoice rather than on the payment transaction: the transaction-level endpoint can
    /// only refund a payment's unapplied balance, which is zero once it has paid an invoice.
    pub merchant_reference_id: common_utils::id_type::PaymentReferenceId,
    /// Transaction id at the billing connector, captured at record-back time. Sent as the
    /// refund's reference number so the record points back at the payment it reverses.
    pub billing_connector_transaction_id: String,
    /// The disputed amount, in minor units.
    pub amount: common_utils::types::MinorUnit,
    /// When the refund occurred.
    pub refund_date: time::PrimitiveDateTime,
    /// Free-text note stored against the billing connector's record.
    pub comment: Option<String>,
    pub connector_params: connector_endpoints::ConnectorParams,
}

#[derive(Debug, Clone)]
pub struct BillingConnectorInvoiceSyncRequest {
    /// Invoice id
    pub billing_connector_invoice_id: String,
    /// connector params of the connector
    pub connector_params: connector_endpoints::ConnectorParams,
}
