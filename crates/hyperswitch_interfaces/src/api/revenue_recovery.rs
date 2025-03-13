//! Revenue Recovery Interface

use hyperswitch_domain_models::{
    router_flow_types::{GetAdditionalRevenueRecoveryDetails, RecoveryRecordBack},
    router_request_types::revenue_recovery::{
        GetAdditionalRevenueRecoveryRequestData, RevenueRecoveryRecordBackRequest,
    },
    router_response_types::revenue_recovery::{
        GetAdditionalRevenueRecoveryResponseData, RevenueRecoveryRecordBackResponse,
    },
};

use super::{ConnectorCommon, ConnectorIntegration};
/// trait RevenueRecovery
pub trait RevenueRecovery:
    ConnectorCommon + AdditionalRevenueRecovery + RevenueRecoveryRecordBack
{
}

/// trait AdditionalRevenueRecovery
pub trait AdditionalRevenueRecovery:
    ConnectorIntegration<
    GetAdditionalRevenueRecoveryDetails,
    GetAdditionalRevenueRecoveryRequestData,
    GetAdditionalRevenueRecoveryResponseData,
>
{
}

/// trait RevenueRecoveryRecordBack
pub trait RevenueRecoveryRecordBack:
    ConnectorIntegration<
    RecoveryRecordBack,
    RevenueRecoveryRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
>
{
}
