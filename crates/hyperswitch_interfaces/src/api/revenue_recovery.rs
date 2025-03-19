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

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use super::ConnectorCommon;
use super::ConnectorIntegration;
/// trait RevenueRecovery
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
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

#[cfg(not(all(feature = "v2", feature = "revenue_recovery")))]
/// trait RevenueRecovery
pub trait RevenueRecovery {}
