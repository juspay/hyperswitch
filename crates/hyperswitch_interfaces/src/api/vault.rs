//! Vault interface

use hyperswitch_domain_models::{
    router_flow_types::vault::{
        ExternalVaultCreateFlow, ExternalVaultDeleteFlow, ExternalVaultInsertFlow,
        ExternalVaultRetrieveFlow,
    },
    router_request_types::VaultRequestData,
    router_response_types::VaultResponseData,
};

use super::ConnectorCommon;
use crate::api::ConnectorIntegration;

/// trait ExternalVaultInsert
pub trait ExternalVaultInsert:
    ConnectorIntegration<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>
{
}

/// trait ExternalVaultRetrieve
pub trait ExternalVaultRetrieve:
    ConnectorIntegration<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>
{
}

/// trait ExternalVaultDelete
pub trait ExternalVaultDelete:
    ConnectorIntegration<ExternalVaultDeleteFlow, VaultRequestData, VaultResponseData>
{
}

/// trait ExternalVaultDelete
pub trait ExternalVaultCreate:
    ConnectorIntegration<ExternalVaultCreateFlow, VaultRequestData, VaultResponseData>
{
}

/// trait ExternalVault
pub trait ExternalVault:
    ConnectorCommon
    + ExternalVaultInsert
    + ExternalVaultRetrieve
    + ExternalVaultDelete
    + ExternalVaultCreate
{
}
