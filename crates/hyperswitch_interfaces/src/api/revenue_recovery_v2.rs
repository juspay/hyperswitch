//! Revenue Recovery Interface V2

use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{
        BillingConnectorPaymentsSyncFlowData, RevenueRecoveryRecordBackData,
    },
    router_flow_types::{BillingConnectorPaymentsSync, RecoveryRecordBack},
    router_request_types::revenue_recovery::{
        BillingConnectorPaymentsSyncRequest, RevenueRecoveryRecordBackRequest,
    },
    router_response_types::revenue_recovery::{
        BillingConnectorPaymentsSyncResponse, RevenueRecoveryRecordBackResponse,
    },
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait RevenueRecoveryV2
pub trait RevenueRecoveryV2:
    BillingConnectorPaymentsSyncIntegrationV2 + RevenueRecoveryRecordBackV2
{
}

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
