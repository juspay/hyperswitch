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
pub struct BillingConnectorInvoiceSyncRequest {
    /// Invoice id
    pub billing_connector_invoice_id: String,
    /// connector params of the connector
    pub connector_params: connector_endpoints::ConnectorParams,
}
