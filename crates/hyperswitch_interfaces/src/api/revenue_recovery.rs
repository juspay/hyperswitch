//! Revenue Recovery Interface

use hyperswitch_domain_models::{
    router_flow_types::GetAdditionalRevenueRecoveryDetails,
    router_request_types::revenue_recovery::GetAdditionalRevenueRecoveryRequestData,
    router_response_types::revenue_recovery::GetAdditionalRevenueRecoveryResponseData,
};

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use super::ConnectorCommon;
use super::ConnectorIntegration;

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
/// trait RevenueRecovery
pub trait RevenueRecovery: ConnectorCommon + AdditionalRevenueRecovery {}

/// trait AdditionalRevenueRecovery
pub trait AdditionalRevenueRecovery:
    ConnectorIntegration<
    GetAdditionalRevenueRecoveryDetails,
    GetAdditionalRevenueRecoveryRequestData,
    GetAdditionalRevenueRecoveryResponseData,
>
{
}

#[cfg(not(all(feature = "v2", feature = "revenue_recovery")))]
/// trait RevenueRecovery
pub trait RevenueRecovery {}
