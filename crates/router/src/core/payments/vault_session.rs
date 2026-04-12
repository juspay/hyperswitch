pub use common_enums::enums::CallConnectorAction;
use error_stack::{report, ResultExt};
#[cfg(feature = "v2")]
pub use hyperswitch_domain_models::payments::PaymentIntentData;
pub use hyperswitch_domain_models::{
    mandates::MandateData,
    payment_address::PaymentAddress,
    payments::HeaderPayload,
    router_data::{PaymentMethodToken, RouterData},
    router_data_v2::{flow_common_types::VaultConnectorFlowData, RouterDataV2},
    router_flow_types::ExternalVaultCreateFlow,
    router_request_types::CustomerDetails,
    types::{VaultRouterData, VaultRouterDataV2},
};
use hyperswitch_interfaces::api::Connector as ConnectorTrait;
#[cfg(feature = "v2")]
use hyperswitch_interfaces::connector_integration_v2::{ConnectorIntegrationV2, ConnectorV2};
use router_env::env::Env;

#[cfg(feature = "v2")]
use crate::core::payments::{customers, gateway::context as gateway_context};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{
            call_create_connector_customer_if_required,
            flows::{ConstructFlowSpecificData, Feature},
            gateway::context as gateway_context,
            helpers,
            operations::BoxedOperation,
            OperationSessionGetters, OperationSessionSetters,
        },
        utils as core_utils,
    },
    db::errors::ConnectorErrorExt,
    routes::{app::ReqState, SessionState},
    services::{self, connector_integration_interface::RouterDataConversion},
    types::{
        self as router_types,
        api::{self, enums as api_enums},
        domain,
    },
};
#[cfg(feature = "v2")]
use crate::{
    errors::RouterResponse,
    types::{api::ConnectorCommon, storage},
};

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn populate_vault_session_details<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    customer: &Option<domain::Customer>,
    platform: &domain::Platform,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    profile: &domain::Profile,
    payment_data: &mut D,
    header_payload: HeaderPayload,
) -> RouterResult<()>
where
    F: Send + Clone + Sync,
    RouterDReq: Send + Sync,

    // To create connector flow specific interface data
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, RouterDReq, router_types::PaymentsResponseData>,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>: Feature<F, RouterDReq> + Send,
    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, RouterDReq, router_types::PaymentsResponseData>,
{
    let is_external_vault_sdk_enabled = profile.is_vault_sdk_enabled();

    if is_external_vault_sdk_enabled {
        let external_vault_source = profile
            .external_vault_connector_details
            .as_ref()
            .map(|details| &details.vault_connector_id);

        let merchant_connector_account =
            domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
                helpers::get_merchant_connector_account_v2(
                    state,
                    platform.get_processor(),
                    external_vault_source,
                )
                .await?,
            ));

        let updated_customer = call_create_connector_customer_if_required(
            state,
            customer,
            platform.get_processor(),
            platform.get_initiator(),
            &merchant_connector_account,
            payment_data,
        )
        .await?;

        if let Some((customer, updated_customer)) = customer.clone().zip(updated_customer) {
            let db = &*state.store;
            let customer_id = customer.get_id().clone();
            let customer_merchant_id = customer.merchant_id.clone();

            let _updated_customer = db
                .update_customer_by_global_id(
                    &customer_id,
                    customer,
                    updated_customer,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update customer during Vault session")?;
        };

        let vault_session_details = generate_vault_session_details(
            state,
            platform,
            &merchant_connector_account,
            payment_data.get_connector_customer_id(),
        )
        .await?;

        payment_data.set_vault_session_details(vault_session_details);
    }
    Ok(())
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn populate_vault_session_details<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    _req_state: ReqState,
    customer: &Option<domain::Customer>,
    platform: &domain::Platform,
    _operation: &BoxedOperation<'_, F, ApiRequest, D>,
    profile: &domain::Profile,
    payment_data: &mut D,
    _header_payload: HeaderPayload,
) -> RouterResult<()>
where
    F: Send + Clone + Sync,
    RouterDReq: Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, RouterDReq, router_types::PaymentsResponseData>,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>: Feature<F, RouterDReq> + Send,
    dyn api::Connector:
        services::api::ConnectorIntegration<F, RouterDReq, router_types::PaymentsResponseData>,
{
    let is_external_vault_sdk_enabled = profile.external_vault_details.is_external_vault_enabled();

    if is_external_vault_sdk_enabled {
        // Extract the vault connector id from the profile
        let vault_connector_id = match &profile.external_vault_details {
            domain::ExternalVaultDetails::ExternalVaultEnabled(details) => {
                Some(&details.vault_connector_id)
            }
            domain::ExternalVaultDetails::Skip => None,
        };

        // Fetch the vault MCA using the vault_connector_id from profile
        let vault_mca_type = helpers::get_merchant_connector_account(
            state,
            platform.get_processor(),
            None,
            profile.get_id(),
            "",
            vault_connector_id,
        )
        .await?;

        let vault_mca = match vault_mca_type {
            helpers::MerchantConnectorAccountType::DbVal(ref mca) => mca.clone(),
            helpers::MerchantConnectorAccountType::CacheVal(_) => {
                return Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Vault MCA must be a DB-backed account")
            }
        };
        let connector_customer_map = customer
            .as_ref()
            .and_then(|customer| customer.connector_customer.as_ref());

        let default_gateway_context = gateway_context::RouterGatewayContext::direct(
            platform.get_processor().clone(),
            vault_mca_type.clone(),
            payment_data.get_payment_intent().merchant_id.clone(),
            profile.get_id().clone(),
            payment_data.get_creds_identifier().map(|id| id.to_string()),
        );
        println!("here111");

        let updated_customer = call_create_connector_customer_if_required(
            state,
            connector_customer_map,
            platform.get_processor(),
            platform.get_initiator(),
            &vault_mca_type,
            payment_data,
            None,
            &default_gateway_context,
        )
        .await?;

        if let Some((customer, updated_customer)) = customer.clone().zip(updated_customer) {
            let db = &*state.store;
            let customer_id = customer.get_id().clone();
            let customer_merchant_id = customer.merchant_id.clone();

            println!("customerrrr");

            let _updated_customer = db
                .update_customer_by_customer_id_merchant_id(
                    customer_id,
                    customer_merchant_id,
                    customer,
                    updated_customer,
                    platform.get_provider().get_key_store(),
                    platform.get_provider().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update customer during Vault session")?;
        };

        let connector_customer_id = payment_data.get_connector_customer_id();

        let vault_session_details = generate_vault_session_details_v1(
            state,
            platform.get_processor(),
            &vault_mca,
            connector_customer_id,
        )
        .await?;

        payment_data.set_vault_session_details(vault_session_details);
    }
    Ok(())
}

#[cfg(feature = "v2")]
pub async fn call_create_connector_customer_if_required<F, Req, D>(
    state: &SessionState,
    customer: &Option<domain::Customer>,
    processor: &domain::Processor,
    initiator: Option<&domain::Initiator>,
    merchant_connector_account_type: &domain::MerchantConnectorAccountTypeDetails,
    payment_data: &mut D,
) -> RouterResult<Option<storage::CustomerUpdate>>
where
    F: Send + Clone + Sync,
    Req: Send + Sync,

    // To create connector flow specific interface data
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, Req, router_types::PaymentsResponseData>,
    RouterData<F, Req, router_types::PaymentsResponseData>: Feature<F, Req> + Send,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, Req, router_types::PaymentsResponseData>,
{
    let db_merchant_connector_account =
        merchant_connector_account_type.get_inner_db_merchant_connector_account();
    let profile_id = payment_data.get_payment_intent().profile_id.clone();
    let default_gateway_context = gateway_context::RouterGatewayContext::direct(
        processor.clone(),
        merchant_connector_account_type.clone(),
        payment_data.get_payment_intent().merchant_id.clone(),
        profile_id,
        payment_data.get_creds_identifier().map(|id| id.to_string()),
    );
    match db_merchant_connector_account {
        Some(merchant_connector_account) => {
            let connector_name = merchant_connector_account.get_connector_name_as_string();
            let merchant_connector_id = merchant_connector_account.get_id();

            let connector = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &connector_name,
                api::GetToken::Connector,
                Some(merchant_connector_id.clone()),
            )?;

            let (should_call_connector, existing_connector_customer_id) =
                customers::should_call_connector_create_customer(
                    &connector,
                    customer,
                    payment_data.get_payment_attempt(),
                    merchant_connector_account_type,
                );

            if should_call_connector {
                // Create customer at connector and update the customer table to store this data
                let router_data = payment_data
                    .construct_router_data(
                        state,
                        connector.connector.id(),
                        processor,
                        customer,
                        merchant_connector_account_type,
                        None,
                        None,
                    )
                    .await?;

                let connector_customer_id = router_data
                    .create_connector_customer(state, &connector, &default_gateway_context)
                    .await?;

                let customer_update = customers::update_connector_customer_in_customers(
                    merchant_connector_account_type,
                    customer.as_ref(),
                    connector_customer_id.clone(),
                    initiator,
                )
                .await;

                payment_data.set_connector_customer_id(connector_customer_id);
                Ok(customer_update)
            } else {
                // Customer already created in previous calls use the same value, no need to update
                payment_data.set_connector_customer_id(
                    existing_connector_customer_id.map(ToOwned::to_owned),
                );
                Ok(None)
            }
        }
        None => {
            router_env::logger::error!(
                "Merchant connector account is missing, cannot create customer for vault session"
            );
            Err(errors::ApiErrorResponse::InternalServerError.into())
        }
    }
}

#[cfg(feature = "v2")]
pub async fn generate_vault_session_details(
    state: &SessionState,
    platform: &domain::Platform,
    merchant_connector_account_type: &domain::MerchantConnectorAccountTypeDetails,
    connector_customer_id: Option<String>,
) -> RouterResult<Option<api::VaultSessionDetails>> {
    let connector =
        api_enums::VaultConnectors::try_from(merchant_connector_account_type.get_connector_name())
            .map_err(|error| {
                report!(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
                    "Failed to convert connector to vault connector: {}",
                    error
                ))
            })?;

    let connector_auth_type: router_types::ConnectorAuthType = merchant_connector_account_type
        .get_connector_account_details()
        .map_err(|err| {
            err.change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse connector auth type")
        })?;

    match (connector, connector_auth_type) {
        // create session for vgs vault
        (
            api_enums::VaultConnectors::Vgs,
            router_types::ConnectorAuthType::SignatureKey { api_secret, .. },
        ) => {
            let sdk_env = match state.conf.env {
                Env::Sandbox | Env::Development | Env::Integ => "sandbox",
                Env::Production => "live",
            }
            .to_string();
            Ok(Some(api::VaultSessionDetails::Vgs(
                api::VgsSessionDetails {
                    external_vault_id: api_secret,
                    sdk_env,
                },
            )))
        }
        // create session for hyperswitch vault
        (
            api_enums::VaultConnectors::HyperswitchVault,
            router_types::ConnectorAuthType::SignatureKey {
                key1, api_secret, ..
            },
        ) => {
            generate_hyperswitch_vault_session_details(
                state,
                platform,
                merchant_connector_account_type,
                connector_customer_id,
                merchant_connector_account_type
                    .get_connector_name()
                    .to_string(),
                key1,
                api_secret,
            )
            .await
        }
        _ => {
            router_env::logger::warn!(
                "External vault session creation is not supported for connector: {:?}",
                connector
            );
            Ok(None)
        }
    }
}

#[cfg(feature = "v1")]
pub async fn generate_vault_session_details_v1(
    state: &SessionState,
    processor: &domain::Processor,
    vault_mca: &domain::MerchantConnectorAccount,
    connector_customer_id: Option<String>,
) -> RouterResult<Option<api::VaultSessionDetails>> {
    use std::str::FromStr;
    let connector_name_str = vault_mca.get_connector_name_as_string();
    let connector_enum = common_enums::connector_enums::Connector::from_str(&connector_name_str)
        .map_err(|_| {
            report!(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
                "Failed to parse connector name to enum: {}",
                connector_name_str
            ))
        })?;
    let connector = api_enums::VaultConnectors::try_from(connector_enum).map_err(|error| {
        report!(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
            "Failed to convert connector to vault connector: {}",
            error
        ))
    })?;

    let connector_auth_type: router_types::ConnectorAuthType = vault_mca
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to parse connector auth type")?;

    match (connector, connector_auth_type) {
        // create session for vgs vault
        (
            api_enums::VaultConnectors::Vgs,
            router_types::ConnectorAuthType::SignatureKey { api_secret, .. },
        ) => {
            let sdk_env = match state.conf.env {
                Env::Sandbox | Env::Development | Env::Integ => "sandbox",
                Env::Production => "live",
            }
            .to_string();
            Ok(Some(api::VaultSessionDetails::Vgs(
                api::VgsSessionDetails {
                    external_vault_id: api_secret,
                    sdk_env,
                },
            )))
        }
        // create session for hyperswitch vault
        (
            api_enums::VaultConnectors::HyperswitchVault,
            router_types::ConnectorAuthType::SignatureKey {
                key1, api_secret, ..
            },
        ) => {
            let connector_response = call_external_vault_create_v1(
                state,
                processor,
                connector_name_str,
                vault_mca,
                connector_customer_id,
            )
            .await?;

            match connector_response.response {
                Ok(router_types::VaultResponseData::ExternalVaultCreateResponse {
                    session_id,
                    client_secret,
                }) => Ok(Some(api::VaultSessionDetails::HyperswitchVault(
                    api::HyperswitchVaultSessionDetails {
                        payment_method_session_id: session_id,
                        client_secret,
                        publishable_key: key1,
                        profile_id: api_secret,
                    },
                ))),
                Ok(_) => {
                    router_env::logger::warn!("Unexpected response from external vault create API");
                    Err(errors::ApiErrorResponse::InternalServerError.into())
                }
                Err(err) => {
                    router_env::logger::error!(error_response_from_external_vault_create=?err);
                    Err(errors::ApiErrorResponse::InternalServerError.into())
                }
            }
        }
        _ => {
            router_env::logger::warn!(
                "External vault session creation is not supported for connector: {:?}",
                connector
            );
            Ok(None)
        }
    }
}

#[cfg(feature = "v1")]
async fn call_external_vault_create_v1(
    state: &SessionState,
    processor: &domain::Processor,
    connector_name: String,
    vault_mca: &domain::MerchantConnectorAccount,
    connector_customer_id: Option<String>,
) -> RouterResult<VaultRouterData<ExternalVaultCreateFlow>>
where
    dyn ConnectorTrait + Sync: services::api::ConnectorIntegration<
        ExternalVaultCreateFlow,
        router_types::VaultRequestData,
        router_types::VaultResponseData,
    >,
{
    let mca_id = vault_mca.get_id();
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api::GetToken::Connector,
        Some(mca_id),
    )?;

    let router_data = core_utils::construct_vault_router_data(
        state,
        processor.get_account().get_id(),
        vault_mca,
        None,
        None,
        connector_customer_id,
        None,
    )
    .await?;

    let old_router_data = VaultConnectorFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the external vault create api call",
        )?;

    println!("heyy, is it working???");

    let connector_integration: services::BoxedVaultConnectorIntegrationInterface<
        ExternalVaultCreateFlow,
        router_types::VaultRequestData,
        router_types::VaultResponseData,
    > = connector_data.connector.get_connector_integration();
    services::execute_connector_processing_step(
        state,
        connector_integration,
        &old_router_data,
        CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_vault_failed_response()
}

#[cfg(feature = "v2")]
async fn generate_hyperswitch_vault_session_details(
    state: &SessionState,
    platform: &domain::Platform,
    merchant_connector_account_type: &domain::MerchantConnectorAccountTypeDetails,
    connector_customer_id: Option<String>,
    connector_name: String,
    vault_publishable_key: hyperswitch_masking::Secret<String>,
    vault_profile_id: hyperswitch_masking::Secret<String>,
) -> RouterResult<Option<api::VaultSessionDetails>> {
    let connector_response = call_external_vault_create(
        state,
        platform,
        connector_name,
        merchant_connector_account_type,
        connector_customer_id,
    )
    .await?;

    match connector_response.response {
        Ok(router_types::VaultResponseData::ExternalVaultCreateResponse {
            session_id,
            client_secret,
        }) => Ok(Some(api::VaultSessionDetails::HyperswitchVault(
            api::HyperswitchVaultSessionDetails {
                payment_method_session_id: session_id,
                client_secret,
                publishable_key: vault_publishable_key,
                profile_id: vault_profile_id,
            },
        ))),
        Ok(_) => {
            router_env::logger::warn!("Unexpected response from external vault create API");
            Err(errors::ApiErrorResponse::InternalServerError.into())
        }
        Err(err) => {
            router_env::logger::error!(error_response_from_external_vault_create=?err);
            Err(errors::ApiErrorResponse::InternalServerError.into())
        }
    }
}

#[cfg(feature = "v2")]
async fn call_external_vault_create(
    state: &SessionState,
    platform: &domain::Platform,
    connector_name: String,
    merchant_connector_account_type: &domain::MerchantConnectorAccountTypeDetails,
    connector_customer_id: Option<String>,
) -> RouterResult<VaultRouterData<ExternalVaultCreateFlow>>
where
    dyn ConnectorTrait + Sync: services::api::ConnectorIntegration<
        ExternalVaultCreateFlow,
        router_types::VaultRequestData,
        router_types::VaultResponseData,
    >,
    dyn ConnectorV2 + Sync: ConnectorIntegrationV2<
        ExternalVaultCreateFlow,
        VaultConnectorFlowData,
        router_types::VaultRequestData,
        router_types::VaultResponseData,
    >,
{
    let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api::GetToken::Connector,
        merchant_connector_account_type.get_mca_id(),
    )?;
    let merchant_connector_account = match &merchant_connector_account_type {
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(mca) => {
            Ok(mca.as_ref())
        }
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("MerchantConnectorDetails not supported for vault operations"))
        }
    }?;

    let mut router_data = core_utils::construct_vault_router_data(
        state,
        platform.get_processor().get_account().get_id(),
        merchant_connector_account,
        None,
        None,
        connector_customer_id,
        None,
    )
    .await?;

    let mut old_router_data = VaultConnectorFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the external vault create api call",
        )?;

    let connector_integration: services::BoxedVaultConnectorIntegrationInterface<
        ExternalVaultCreateFlow,
        router_types::VaultRequestData,
        router_types::VaultResponseData,
    > = connector_data.connector.get_connector_integration();
    services::execute_connector_processing_step(
        state,
        connector_integration,
        &old_router_data,
        CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_vault_failed_response()
}
