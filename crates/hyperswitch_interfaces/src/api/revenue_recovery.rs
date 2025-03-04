//! Revenue Recovery Interface

use hyperswitch_domain_models::{
    router_flow_types::GetAdditionalRevenueRecoveryDetails,
    router_request_types::revenue_recovery::GetAdditionalRevenueRecoveryRequestData,
    router_response_types::revenue_recovery::GetAdditionalRevenueRecoveryResponseData,
};

use super::{ConnectorCommon, ConnectorIntegration};
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
