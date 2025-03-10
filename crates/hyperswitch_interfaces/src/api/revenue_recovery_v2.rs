//! Revenue Recovery Interface V2

use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::{
        GetAdditionalRevenueRecoveryFlowCommonData, RevenueRecoveryRecordBackData,
    },
    router_flow_types::{GetAdditionalRevenueRecoveryDetails, RecoveryRecordBack},
    router_request_types::revenue_recovery::{
        GetAdditionalRevenueRecoveryRequestData, RevenueRecoveryRecordBackRequest,
    },
    router_response_types::revenue_recovery::{
        GetAdditionalRevenueRecoveryResponseData, RevenueRecoveryRecordBackResponse,
    },
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait RevenueRecoveryV2
pub trait RevenueRecoveryV2: AdditionalRevenueRecoveryV2 + RevenueRecoveryRecordBackV2 {}

/// trait AdditionalRevenueRecoveryV2
pub trait AdditionalRevenueRecoveryV2:
    ConnectorIntegrationV2<
    GetAdditionalRevenueRecoveryDetails,
    GetAdditionalRevenueRecoveryFlowCommonData,
    GetAdditionalRevenueRecoveryRequestData,
    GetAdditionalRevenueRecoveryResponseData,
>
{
}

/// trait ConnectorAdditionalRevenueRecoveryDetailsCallV2
pub trait RevenueRecoveryRecordBackV2:
    ConnectorIntegrationV2<
    RecoveryRecordBack,
    RevenueRecoveryRecordBackData,
    RevenueRecoveryRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
>
{
}
