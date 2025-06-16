//! Vault V2 interface
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::VaultConnectorFlowData,
    router_flow_types::vault::{
        ExternalVaultCreateFlow, ExternalVaultDeleteFlow, ExternalVaultInsertFlow,
        ExternalVaultRetrieveFlow,
    },
    router_request_types::VaultRequestData,
    router_response_types::VaultResponseData,
};

use super::ConnectorCommon;
use crate::api::ConnectorIntegrationV2;

/// trait ExternalVaultInsertV2
pub trait ExternalVaultInsertV2:
    ConnectorIntegrationV2<
    ExternalVaultInsertFlow,
    VaultConnectorFlowData,
    VaultRequestData,
    VaultResponseData,
>
{
}
/// trait ExternalVaultRetrieveV2
pub trait ExternalVaultRetrieveV2:
    ConnectorIntegrationV2<
    ExternalVaultRetrieveFlow,
    VaultConnectorFlowData,
    VaultRequestData,
    VaultResponseData,
>
{
}

/// trait ExternalVaultDeleteV2
pub trait ExternalVaultDeleteV2:
    ConnectorIntegrationV2<
    ExternalVaultDeleteFlow,
    VaultConnectorFlowData,
    VaultRequestData,
    VaultResponseData,
>
{
}

/// trait ExternalVaultDeleteV2
pub trait ExternalVaultCreateV2:
    ConnectorIntegrationV2<
    ExternalVaultCreateFlow,
    VaultConnectorFlowData,
    VaultRequestData,
    VaultResponseData,
>
{
}

/// trait ExternalVaultV2
pub trait ExternalVaultV2:
    ConnectorCommon
    + ExternalVaultInsertV2
    + ExternalVaultRetrieveV2
    + ExternalVaultDeleteV2
    + ExternalVaultCreateV2
{
}
