//! External Vault implementation
//!
//! This module contains the implementation for external vault providers,
//! which use connector integrations to store and retrieve payment method data.
//!
//! # Design Pattern
//!
//! This module implements the **Strategy Pattern** as part of the vault strategy
//! hierarchy. The external vault uses connector integrations to communicate with
//! third-party vault providers (e.g., Stripe Vault, Spreedly, etc.).
//!
//! The external vault flow involves:
//! 1. Constructing router data for the connector
//! 2. Creating access tokens for authentication
//! 3. Executing connector-specific vault operations
//! 4. Parsing connector responses

#[cfg(feature = "v2")]
use common_utils::id_type;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

#[cfg(feature = "v2")]
use crate::types::payment_methods as pm_types;
#[cfg(feature = "v2")]
use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResult},
        payment_methods::access_token,
        payments::{self as payments_core},
        utils as core_utils,
    },
    routes::SessionState,
    services::{self, connector_integration_interface::RouterDataConversion},
    types::{self, api, domain},
};

use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::VaultConnectorFlowData,
    router_flow_types::ExternalVaultInsertFlow, types::VaultRouterData,
};

/// Vault a payment method using an external vault provider (v2)
///
/// This function performs the following steps:
/// 1. Constructs vault router data for the connector
/// 2. Gets the connector configuration
/// 3. Creates an access token for authentication
/// 4. Executes the connector processing step to vault the payment method
/// 5. Returns the parsed vault response
///
/// # Arguments
///
/// * `state` - The application session state
/// * `pmd` - The custom payment method data to vault (filtered fields based on token selector)
/// * `merchant_account` - The merchant account context
/// * `merchant_connector_account` - The merchant connector account for the vault provider
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn vault_payment_method(
    state: &SessionState,
    pmd: &domain::PaymentMethodCustomVaultingData,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: &domain::MerchantConnectorAccount,
) -> RouterResult<pm_types::AddVaultResponse> {
    // Step 1: Construct vault router data
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

    // Step 2: Get connector data
    let connector_data = api::ConnectorData::get_external_vault_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_account.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    // Step 3: Create access token for authentication
    access_token::create_access_token(
        state,
        &connector_data,
        merchant_account,
        &mut old_router_data,
    )
    .await?;

    if old_router_data.response.is_ok() {
        // Step 4: Execute connector processing step
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

        // Step 5: Parse response
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

/// Vault a payment method using an external vault provider (v1)
///
/// Similar to the v2 version but with v1-specific types.
#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn vault_payment_method_v1(
    state: &SessionState,
    pmd: &hyperswitch_domain_models::vault::PaymentMethodCustomVaultingData,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: MerchantConnectorAccountV1,
    should_generate_multiple_tokens: Option<bool>,
) -> RouterResult<pm_types::AddVaultResponse> {
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

    access_token::create_access_token(
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

/// Parse the vault response for insert payment method data
///
/// This function extracts the vault_id and fingerprint_id from the connector response
/// and returns them in a standardized format.
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

/// Retrieve a payment method from an external vault
#[cfg(feature = "v2")]
pub async fn retrieve_payment_method(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    pm: &domain::PaymentMethod,
    merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
) -> RouterResult<pm_types::VaultRetrieveResponse> {
    super::retrieve_payment_method_from_vault_external(
        state,
        merchant_account,
        pm,
        merchant_connector_account,
    )
    .await
}

/// Delete a payment method from an external vault
#[cfg(feature = "v2")]
pub async fn delete_payment_method(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
    vault_id: domain::VaultId,
    customer_id: &id_type::GlobalCustomerId,
) -> RouterResult<pm_types::VaultDeleteResponse> {
    super::delete_payment_method_data_from_vault_external(
        state,
        merchant_account,
        merchant_connector_account,
        vault_id,
        customer_id,
    )
    .await
}
