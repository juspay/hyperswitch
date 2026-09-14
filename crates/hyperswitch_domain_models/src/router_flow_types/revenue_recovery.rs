#[derive(Debug, Clone)]
pub struct BillingConnectorPaymentsSync;
#[derive(Debug, Clone)]
pub struct InvoiceRecordBack;

/// Record a lost dispute back to the billing connector as an offline refund.
#[derive(Debug, Clone)]
pub struct DisputeRecordBack;

#[derive(Debug, Clone)]
pub struct BillingConnectorInvoiceSync;
