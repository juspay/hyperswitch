//! External Vault implementation
//!
//! This module contains the implementation for external vault connectors (PCI-compliant third-party vaults).
//! External vaults use connector-specific integrations to delegate tokenization to third-party services
//! like Stripe Vault, Braintree Vault, etc.
//!
//! # Design Pattern
//!
//! This module implements the **Strategy Pattern** as part of the vault strategy hierarchy.
//! External vaulting uses the standard connector integration framework to communicate with
//! external vault providers.

use common_utils::id_type;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use crate::db::errors::ConnectorErrorExt;
#[cfg(feature = "v2")]
use crate::types::payment_methods as pm_types;
#[cfg(feature = "v2")]
use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payments::{self as payments_core, helpers as payment_helpers},
        utils as core_utils,
    },
    routes::SessionState,
    services::{self, connector_integration_interface::RouterDataConversion},
    types::{
        self, api,
        domain::{self, MerchantConnectorAccountTypeDetails},
        storage::enums,
    },
    utils::{ext_traits::OptionExt, ConnectorResponseExt},
};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::types::VaultRouterData;

/// External vault strategy implementation
#[cfg(feature = "v2")]
pub struct ExternalVault {
    merchant_account: domain::MerchantAccount,
    merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
}

#[cfg(feature = "v2")]
impl ExternalVault {
    /// Create a new external vault strategy
    pub fn new(
        merchant_account: domain::MerchantAccount,
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    ) -> Self {
        Self {
            merchant_account,
            merchant_connector_account,
        }
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl crate::core::payment_methods::vault::VaultStrategy for ExternalVault {
    async fn vault_payment_method(
        &self,
        state: &SessionState,
        pmd: &domain::PaymentMethodVaultingData,
        _existing_vault_id: Option<domain::VaultId>,
        _customer_id: &id_type::GlobalCustomerId,
    ) -> RouterResult<pm_types::AddVaultResponse> {
        let mca = match &self.merchant_connector_account {
            domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(mca) => {
                Ok(mca.as_ref())
            }
            domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => Err(
                report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                    "MerchantConnectorDetails not supported for vault operations",
                ),
            ),
        }?;

        // Convert PaymentMethodVaultingData to PaymentMethodCustomVaultingData
        let custom_data: domain::PaymentMethodCustomVaultingData = pmd.clone().into();

        vault_payment_method(state, &custom_data, &self.merchant_account, mca).await
    }

    async fn retrieve_payment_method(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        _profile: &domain::Profile,
        pm: &domain::PaymentMethod,
    ) -> RouterResult<pm_types::VaultRetrieveResponse> {
        retrieve_payment_method(
            state,
            platform.get_provider().get_account(),
            pm,
            self.merchant_connector_account.clone(),
        )
        .await
    }

    async fn delete_payment_method(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        _profile: &domain::Profile,
        pm: &domain::PaymentMethod,
    ) -> RouterResult<pm_types::VaultDeleteResponse> {
        let vault_id = pm
            .locker_id
            .clone()
            .get_required_value("locker_id")
            .attach_printable("Missing locker_id in PaymentMethod")?;
        let customer_id = &pm
            .customer_id
            .clone()
            .get_required_value("GlobalCustomerId")?;

        delete_payment_method(
            state,
            platform.get_provider().get_account(),
            self.merchant_connector_account.clone(),
            vault_id,
            customer_id,
        )
        .await
    }
}

/// Vault a payment method using an external vault connector
///
/// This function:
/// 1. Constructs vault router data for the external connector
/// 2. Creates an access token for authentication
/// 3. Executes the connector processing step to vault the payment method
/// 4. Returns the vault response with connector-generated vault ID
///
/// # Arguments
///
/// * `state` - The application session state
/// * `pmd` - The payment method data to vault (in custom format for external vault)
/// * `merchant_account` - The merchant account context
/// * `merchant_connector_account` - The merchant connector account for the external vault
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn vault_payment_method(
    state: &SessionState,
    pmd: &domain::PaymentMethodCustomVaultingData,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: &domain::MerchantConnectorAccount,
) -> RouterResult<pm_types::AddVaultResponse> {
    use hyperswitch_domain_models::{
        router_data_v2::flow_common_types::VaultConnectorFlowData,
        router_flow_types::ExternalVaultInsertFlow,
    };

    let router_data = core_utils::construct_vault_router_data(
        state,
        merchant_account.get_id(),
        merchant_connector_account,
        Some(pmd.clone()),
        None,
        None,
        None,
    )
    .await?;

    let mut old_router_data = VaultConnectorFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the external vault insert api call",
        )?;

    let connector_name = merchant_connector_account.get_connector_name_as_string();

    let connector_data = api::ConnectorData::get_external_vault_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_account.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    // Create access token for external vault authentication
    crate::core::payment_methods::access_token::create_access_token(
        state,
        &connector_data,
        merchant_account,
        &mut old_router_data,
    )
    .await?;

    if old_router_data.response.is_ok() {
        let connector_integration: services::BoxedVaultConnectorIntegrationInterface<
            ExternalVaultInsertFlow,
            types::VaultRequestData,
            types::VaultResponseData,
        > = connector_data.connector.get_connector_integration();

        let router_data_resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &old_router_data,
            payments_core::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_vault_failed_response()?;

        get_vault_response_for_insert_payment_method_data(router_data_resp)
    } else {
        router_env::logger::error!(
            "Error vaulting payment method: {:?}",
            old_router_data.response
        );
        Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to create access token for external vault"))
    }
}

/// Retrieve a payment method from an external vault
///
/// Uses connector integration to fetch payment method data from the external vault service.
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    pm: &domain::PaymentMethod,
    merchant_connector_account: MerchantConnectorAccountTypeDetails,
) -> RouterResult<pm_types::VaultRetrieveResponse> {
    use hyperswitch_domain_models::{
        router_data_v2::flow_common_types::VaultConnectorFlowData,
        router_flow_types::ExternalVaultRetrieveFlow,
    };

    let connector_vault_id = pm
        .locker_id
        .clone()
        .map(|id| id.get_string_repr().to_owned());

    let merchant_connector_account = match &merchant_connector_account {
        MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(mca) => Ok(mca.as_ref()),
        MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("MerchantConnectorDetails not supported for vault operations"))
        }
    }?;

    let router_data = core_utils::construct_vault_router_data(
        state,
        merchant_account.get_id(),
        merchant_connector_account,
        None,
        connector_vault_id,
        None,
        None,
    )
    .await?;

    let mut old_router_data = VaultConnectorFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the external vault retrieve api call",
        )?;

    let connector_name = merchant_connector_account.get_connector_name_as_string();

    let connector_data = api::ConnectorData::get_external_vault_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_account.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    let connector_integration: services::BoxedVaultConnectorIntegrationInterface<
        ExternalVaultRetrieveFlow,
        types::VaultRequestData,
        types::VaultResponseData,
    > = connector_data.connector.get_connector_integration();

    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &old_router_data,
        payments_core::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_vault_failed_response()?;

    get_vault_response_for_retrieve_payment_method_data::<ExternalVaultRetrieveFlow>(
        router_data_resp,
    )
}

/// Delete a payment method from an external vault
///
/// Uses connector integration to delete payment method data from the external vault service.
#[cfg(feature = "v2")]
pub async fn delete_payment_method(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: MerchantConnectorAccountTypeDetails,
    vault_id: domain::VaultId,
    customer_id: &id_type::GlobalCustomerId,
) -> RouterResult<pm_types::VaultDeleteResponse> {
    use hyperswitch_domain_models::{
        router_data_v2::flow_common_types::VaultConnectorFlowData,
        router_flow_types::ExternalVaultDeleteFlow,
    };

    let connector_vault_id = vault_id.get_string_repr().to_owned();

    // Extract MerchantConnectorAccount from the enum
    let merchant_connector_account = match &merchant_connector_account {
        MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(mca) => Ok(mca.as_ref()),
        MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("MerchantConnectorDetails not supported for vault operations"))
        }
    }?;

    let router_data = core_utils::construct_vault_router_data(
        state,
        merchant_account.get_id(),
        merchant_connector_account,
        None,
        Some(connector_vault_id),
        None,
        None,
    )
    .await?;

    let mut old_router_data = VaultConnectorFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the external vault delete api call",
        )?;

    let connector_name = merchant_connector_account.get_connector_name_as_string();

    let connector_data = api::ConnectorData::get_external_vault_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_account.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    let connector_integration: services::BoxedVaultConnectorIntegrationInterface<
        ExternalVaultDeleteFlow,
        types::VaultRequestData,
        types::VaultResponseData,
    > = connector_data.connector.get_connector_integration();

    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &old_router_data,
        payments_core::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_vault_failed_response()?;

    get_vault_response_for_delete_payment_method_data::<ExternalVaultDeleteFlow>(
        router_data_resp,
        customer_id.to_owned(),
    )
}

/// Parse vault response for insert payment method data
pub fn get_vault_response_for_insert_payment_method_data<F>(
    router_data: VaultRouterData<F>,
) -> RouterResult<pm_types::AddVaultResponse> {
    match router_data.response {
        Ok(response) => match response {
            types::VaultResponseData::ExternalVaultInsertResponse {
                connector_vault_id,
                fingerprint_id,
            } => {
                #[cfg(feature = "v2")]
                let vault_id = domain::VaultId::generate(connector_vault_id.get_single_vault_id()?);
                #[cfg(not(feature = "v2"))]
                let vault_id = connector_vault_id;

                Ok(pm_types::AddVaultResponse {
                    vault_id,
                    fingerprint_id: Some(fingerprint_id),
                    entity_id: None,
                })
            }
            types::VaultResponseData::ExternalVaultRetrieveResponse { .. }
            | types::VaultResponseData::ExternalVaultDeleteResponse { .. }
            | types::VaultResponseData::ExternalVaultCreateResponse { .. } => {
                Err(report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Invalid Vault Response"))
            }
        },
        Err(err) => {
            router_env::logger::error!("Error vaulting payment method: {:?}", err);
            Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to vault payment method"))
        }
    }
}

/// Parse vault response for retrieve payment method data
#[cfg(feature = "v2")]
pub fn get_vault_response_for_retrieve_payment_method_data<F>(
    router_data: VaultRouterData<F>,
) -> RouterResult<pm_types::VaultRetrieveResponse> {
    match router_data.response {
        Ok(response) => match response {
            types::VaultResponseData::ExternalVaultRetrieveResponse { vault_data } => {
                Ok(pm_types::VaultRetrieveResponse { data: vault_data })
            }
            types::VaultResponseData::ExternalVaultInsertResponse { .. }
            | types::VaultResponseData::ExternalVaultDeleteResponse { .. }
            | types::VaultResponseData::ExternalVaultCreateResponse { .. } => {
                Err(report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Invalid Vault Response"))
            }
        },
        Err(err) => Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve payment method")),
    }
}

/// Parse vault response for delete payment method data
#[cfg(feature = "v2")]
pub fn get_vault_response_for_delete_payment_method_data<F>(
    router_data: VaultRouterData<F>,
    customer_id: id_type::GlobalCustomerId,
) -> RouterResult<pm_types::VaultDeleteResponse> {
    match router_data.response {
        Ok(response) => match response {
            types::VaultResponseData::ExternalVaultDeleteResponse { connector_vault_id } => {
                Ok(pm_types::VaultDeleteResponse {
                    vault_id: domain::VaultId::generate(connector_vault_id),
                    entity_id: customer_id,
                })
            }
            types::VaultResponseData::ExternalVaultInsertResponse { .. }
            | types::VaultResponseData::ExternalVaultRetrieveResponse { .. }
            | types::VaultResponseData::ExternalVaultCreateResponse { .. } => {
                Err(report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Invalid Vault Response"))
            }
        },
        Err(err) => Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve payment method")),
    }
}

/// V1 version of external vault - vault a payment method (legacy API support)
#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn vault_payment_method_v1(
    state: &SessionState,
    pmd: &hyperswitch_domain_models::vault::PaymentMethodCustomVaultingData,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    should_generate_multiple_tokens: Option<bool>,
) -> RouterResult<pm_types::AddVaultResponse> {
    use hyperswitch_domain_models::{
        router_data_v2::flow_common_types::VaultConnectorFlowData,
        router_flow_types::ExternalVaultInsertFlow,
    };

    let router_data = core_utils::construct_vault_router_data(
        state,
        merchant_account.get_id(),
        &merchant_connector_account,
        Some(pmd.clone()),
        None,
        None,
        should_generate_multiple_tokens,
    )
    .await?;

    let mut old_router_data = VaultConnectorFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the external vault insert api call",
        )?;

    let connector_name = merchant_connector_account.get_connector_name_as_string();

    let connector_data = api::ConnectorData::get_external_vault_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_account.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    crate::core::payment_methods::access_token::create_access_token(
        state,
        &connector_data,
        merchant_account,
        &mut old_router_data,
    )
    .await?;

    if old_router_data.response.is_ok() {
        let connector_integration: services::BoxedVaultConnectorIntegrationInterface<
            ExternalVaultInsertFlow,
            types::VaultRequestData,
            types::VaultResponseData,
        > = connector_data.connector.get_connector_integration();

        let router_data_resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &old_router_data,
            payments_core::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_vault_failed_response()?;

        get_vault_response_for_insert_payment_method_data(router_data_resp)
    } else {
        router_env::logger::error!(
            "Error vaulting payment method: {:?}",
            old_router_data.response
        );
        Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to create access token for external vault"))
    }
}

/// V1 version of external vault - retrieve a payment method (legacy API support)
#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn retrieve_payment_method_v1(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    pm: &domain::PaymentMethod,
    merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
) -> RouterResult<hyperswitch_domain_models::vault::PaymentMethodVaultingData> {
    use hyperswitch_domain_models::{
        router_data_v2::flow_common_types::VaultConnectorFlowData,
        router_flow_types::ExternalVaultRetrieveFlow,
        types::{VaultRequestData, VaultResponseData},
    };

    let connector_vault_id = pm.locker_id.clone().map(|id| id.to_string());

    let router_data = core_utils::construct_vault_router_data(
        state,
        merchant_id,
        &merchant_connector_account,
        None,
        connector_vault_id,
        None,
        None,
    )
    .await?;

    let old_router_data = VaultConnectorFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the external vault retrieve api call",
        )?;

    let connector_name = merchant_connector_account.get_connector_name_as_string();

    let connector_data = api::ConnectorData::get_external_vault_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_account.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    let connector_integration: services::BoxedVaultConnectorIntegrationInterface<
        ExternalVaultRetrieveFlow,
        VaultRequestData,
        VaultResponseData,
    > = connector_data.connector.get_connector_integration();

    let router_data_resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &old_router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_vault_failed_response()?;

    get_vault_response_for_retrieve_payment_method_data_v1(router_data_resp)
}

/// Parse V1 vault response for retrieve payment method data
pub fn get_vault_response_for_retrieve_payment_method_data_v1<F>(
    router_data: VaultRouterData<F>,
) -> RouterResult<hyperswitch_domain_models::vault::PaymentMethodVaultingData> {
    match router_data.response {
        Ok(response) => match response {
            types::VaultResponseData::ExternalVaultRetrieveResponse { vault_data } => {
                Ok(vault_data)
            }
            types::VaultResponseData::ExternalVaultInsertResponse { .. }
            | types::VaultResponseData::ExternalVaultDeleteResponse { .. }
            | types::VaultResponseData::ExternalVaultCreateResponse { .. } => {
                Err(report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Invalid Vault Response"))
            }
        },
        Err(err) => {
            router_env::logger::error!(
                "Failed to retrieve payment method from external vault: {:?}",
                err
            );
            Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to retrieve payment method from external vault"))
        }
    }
}
