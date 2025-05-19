//! Revenue Recovery Interface V2

use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{
        BillingConnectorInvoiceSyncFlowData, BillingConnectorPaymentsSyncFlowData,
        RevenueRecoveryRecordBackData,
    },
    router_flow_types::{
        BillingConnectorInvoiceSync, BillingConnectorPaymentsSync, RecoveryRecordBack,
    },
    router_request_types::revenue_recovery::{
        BillingConnectorInvoiceSyncRequest, BillingConnectorPaymentsSyncRequest,
        RevenueRecoveryRecordBackRequest,
    },
    router_response_types::revenue_recovery::{
        BillingConnectorInvoiceSyncResponse, BillingConnectorPaymentsSyncResponse,
        RevenueRecoveryRecordBackResponse,
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
    RecoveryRecordBack,
    RevenueRecoveryRecordBackData,
    RevenueRecoveryRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
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
