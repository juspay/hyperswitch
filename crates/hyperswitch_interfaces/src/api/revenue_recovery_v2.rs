//! Revenue Recovery Interface V2

use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{
        BillingConnectorInvoiceSyncFlowData, BillingConnectorPaymentsSyncFlowData,
        InvoiceRecordBackData,
    },
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

use crate::connector_integration_v2::ConnectorIntegrationV2;

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
/// trait RevenueRecoveryV2
pub trait RevenueRecoveryV2:
    BillingConnectorPaymentsSyncIntegrationV2
    + RevenueRecoveryRecordBackV2
    + BillingConnectorInvoiceSyncIntegrationV2
{
}

#[cfg(not(all(feature = "v2", feature = "revenue_recovery")))]
/// trait RevenueRecoveryV2
pub trait RevenueRecoveryV2 {}

/// trait BillingConnectorPaymentsSyncIntegrationV2
pub trait BillingConnectorPaymentsSyncIntegrationV2:
    ConnectorIntegrationV2<
    BillingConnectorPaymentsSync,
    BillingConnectorPaymentsSyncFlowData,
    BillingConnectorPaymentsSyncRequest,
    BillingConnectorPaymentsSyncResponse,
>
{
}

/// trait RevenueRecoveryRecordBackV2
pub trait RevenueRecoveryRecordBackV2:
    ConnectorIntegrationV2<
    InvoiceRecordBack,
    InvoiceRecordBackData,
    InvoiceRecordBackRequest,
    InvoiceRecordBackResponse,
>
{
}

/// trait BillingConnectorInvoiceSyncIntegrationV2
pub trait BillingConnectorInvoiceSyncIntegrationV2:
    ConnectorIntegrationV2<
    BillingConnectorInvoiceSync,
    BillingConnectorInvoiceSyncFlowData,
    BillingConnectorInvoiceSyncRequest,
    BillingConnectorInvoiceSyncResponse,
>
{
}
