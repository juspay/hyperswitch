//! Vault V2 interface
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::VaultConnectorFlowData,
    router_flow_types::vault::{VaultDeleteFlow, VaultInsertFlow, VaultRetrieveFlow},
    router_request_types::VaultRequestData,
    router_response_types::VaultResponseData,
};

use super::ConnectorCommon;
use crate::api::ConnectorIntegrationV2;

/// trait VaultInsertV2
pub trait VaultInsertV2:
    ConnectorIntegrationV2<VaultInsertFlow, VaultConnectorFlowData, VaultRequestData, VaultResponseData>
{
}
/// trait VaultRetrieveV2
pub trait VaultRetrieveV2:
    ConnectorIntegrationV2<
    VaultRetrieveFlow,
    VaultConnectorFlowData,
    VaultRequestData,
    VaultResponseData,
>
{
}

/// trait VaultDeleteV2
pub trait VaultDeleteV2:
    ConnectorIntegrationV2<VaultDeleteFlow, VaultConnectorFlowData, VaultRequestData, VaultResponseData>
{
}

#[cfg(feature = "payouts")]
/// trait Payouts
pub trait VaultV2: ConnectorCommon + VaultInsertV2 + VaultRetrieveV2 + VaultDeleteV2 {}
