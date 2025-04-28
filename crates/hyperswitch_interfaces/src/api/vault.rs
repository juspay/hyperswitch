//! Vault interface

use hyperswitch_domain_models::{
    router_flow_types::vault::{VaultDeleteFlow, VaultInsertFlow, VaultRetrieveFlow},
    router_request_types::VaultRequestData,
    router_response_types::VaultResponseData,
};

use super::ConnectorCommon;
use crate::api::ConnectorIntegration;

/// trait VaultInsert
pub trait VaultInsert:
    ConnectorIntegration<VaultInsertFlow, VaultRequestData, VaultResponseData>
{
}

/// trait VaultRetrieve
pub trait VaultRetrieve:
    ConnectorIntegration<VaultRetrieveFlow, VaultRequestData, VaultResponseData>
{
}

/// trait VaultDelete
pub trait VaultDelete:
    ConnectorIntegration<VaultDeleteFlow, VaultRequestData, VaultResponseData>
{
}

/// trait Payouts
pub trait Vault: ConnectorCommon + VaultInsert + VaultRetrieve + VaultDelete {}
