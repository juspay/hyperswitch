//! Revenue Recovery Interface V2

use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::GetAdditionalRevenueRecoveryFlowCommonData,
    router_flow_types::GetAdditionalRevenueRecoveryDetails,
    router_request_types::revenue_recovery::GetAdditionalRevenueRecoveryRequestData,
    router_response_types::revenue_recovery::GetAdditionalRevenueRecoveryResponseData,
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait RevenueRecoveryV2
pub trait RevenueRecoveryV2: AdditionalRevenueRecoveryV2 {}

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
