//! Revenue Recovery Interface

use hyperswitch_domain_models::{
    router_flow_types::{
        BillingConnectorInvoiceSync, BillingConnectorPaymentsSync, InvoiceRecordBack,
    },
    router_request_types::revenue_recovery::{
        BillingConnectorInvoiceSyncRequest, BillingConnectorPaymentsSyncRequest,
        InvoiceRecordBackRequest,
    },
    router_response_types::revenue_recovery::{
        BillingConnectorInvoiceSyncResponse, BillingConnectorPaymentsSyncResponse,
        InvoiceRecordBackResponse,
    },
};

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use super::ConnectorCommon;
use super::ConnectorIntegration;

/// trait RevenueRecovery
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
pub trait RevenueRecovery:
    ConnectorCommon
    + BillingConnectorPaymentsSyncIntegration
    + RevenueRecoveryRecordBack
    + BillingConnectorInvoiceSyncIntegration
{
}

/// trait BillingConnectorPaymentsSyncIntegration
pub trait BillingConnectorPaymentsSyncIntegration:
    ConnectorIntegration<
    BillingConnectorPaymentsSync,
    BillingConnectorPaymentsSyncRequest,
    BillingConnectorPaymentsSyncResponse,
>
{
}

/// trait RevenueRecoveryRecordBack
pub trait RevenueRecoveryRecordBack:
    ConnectorIntegration<InvoiceRecordBack, InvoiceRecordBackRequest, InvoiceRecordBackResponse>
{
}

/// trait BillingConnectorInvoiceSyncIntegration
pub trait BillingConnectorInvoiceSyncIntegration:
    ConnectorIntegration<
    BillingConnectorInvoiceSync,
    BillingConnectorInvoiceSyncRequest,
    BillingConnectorInvoiceSyncResponse,
>
{
}

#[cfg(not(all(feature = "v2", feature = "revenue_recovery")))]
/// trait RevenueRecovery
pub trait RevenueRecovery {}
