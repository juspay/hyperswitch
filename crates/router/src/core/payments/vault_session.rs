#[cfg(feature = "v2")]
use api_models::enums::VaultConnectors;
pub use common_enums::enums::CallConnectorAction;
use common_utils::id_type;
#[cfg(feature = "v2")]
use error_stack::report;
use error_stack::ResultExt;
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
#[cfg(feature = "v2")]
use hyperswitch_interfaces::{
    api::Connector as ConnectorTrait,
    connector_integration_interface::RouterDataConversion,
    connector_integration_v2::{ConnectorIntegrationV2, ConnectorV2},
};
use hyperswitch_masking::ExposeInterface;
#[cfg(feature = "v1")]
use hyperswitch_masking::Mask;

#[cfg(feature = "v2")]
use crate::core::{
    errors::utils::ConnectorErrorExt,
    payments::{customers, gateway::context as gateway_context, helpers},
    utils as core_utils,
};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{
            flows::{ConstructFlowSpecificData, Feature},
            operations::BoxedOperation,
            OperationSessionGetters, OperationSessionSetters,
        },
    },
    routes::{app::ReqState, SessionState},
    services,
    types::{self as router_types, api, domain},
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
    // V2 gates on `profile.is_vault_sdk_enabled()`; the modular-service flag is V1-only and ignored
    // here, but kept in the signature so the shared call site compiles under both feature flags.
    _is_modular_service_enabled: bool,
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
        // Guest flow (no customer) uses volatile storage; a known customer uses persistent.
        let storage_type = if customer.is_some() {
            common_enums::StorageType::Persistent
        } else {
            common_enums::StorageType::Volatile
        };
        let external_vault_details =
            fetch_external_vault_details(state, platform, profile, customer, storage_type).await?;
        let vault_details = external_vault_details.map(|evd| api::VaultDetails {
            internal_vault: None,
            external_vault_details: Some(evd),
        });
        payment_data.set_vault_session_details(vault_details);
    }
    Ok(())
}

/// Call the internal payment-methods service to create a PM session and extract both
/// `internal_vault` (sdk_authorization) and `external_vault_details` from the response.
#[cfg(feature = "v1")]
async fn call_internal_pm_session_create_for_vault(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    customer_id: Option<&id_type::CustomerId>,
) -> RouterResult<Option<api::VaultDetails>> {
    use common_utils::request::Headers;
    use payment_methods::client::{
        CreatePaymentMethodSession, CreatePaymentMethodSessionV1Request, PaymentMethodClient,
    };

    let processor_merchant_id = platform.get_processor().get_account().get_id();
    let profile_id = profile.get_id();
    let internal_api_key = &state
        .conf
        .internal_merchant_id_profile_id_auth
        .internal_api_key;

    let mut headers = Headers::new();
    headers.insert((
        crate::headers::X_PROFILE_ID.to_string(),
        profile_id.get_string_repr().to_string().into_masked(),
    ));
    headers.insert((
        crate::headers::X_MERCHANT_ID.to_string(),
        processor_merchant_id
            .get_string_repr()
            .to_string()
            .into_masked(),
    ));
    headers.insert((
        crate::headers::X_INTERNAL_API_KEY.to_string(),
        internal_api_key.clone().expose().to_string().into_masked(),
    ));

    let client = PaymentMethodClient::new(
        &state.conf.micro_services.payment_methods_base_url,
        &headers,
        &state.conf.trace_header.header_name,
    );

    // Guest flow (no customer) uses volatile storage; a known customer uses persistent. This is
    // forwarded to the modular PM service, which in turn drives the external vault session create.
    let storage_type = if customer_id.is_some() {
        common_enums::StorageType::Persistent
    } else {
        common_enums::StorageType::Volatile
    };

    let request = CreatePaymentMethodSessionV1Request {
        customer_id: customer_id.cloned(),
        modular_service_prefix: state.conf.micro_services.payment_methods_prefix.0.clone(),
        storage_type,
    };

    let response = CreatePaymentMethodSession::call(state, &client, request)
        .await
        .map_err(|err| {
            router_env::logger::error!(?err, "Internal PM session create for vault failed");
            errors::ApiErrorResponse::InternalServerError
        })
        .attach_printable("Failed to create PM session via internal service for vault details")?;

    // Build the internal vault details from the sdk_authorization returned by the PM service.
    let internal_vault =
        response
            .sdk_authorization
            .map(|sdk_auth| api::InternalVaultSessionDetails {
                sdk_authorization: sdk_auth,
            });

    let external_vault_details = response.external_vault_details;

    // Only return Some if at least one of the two parts is present.
    if internal_vault.is_none() && external_vault_details.is_none() {
        router_env::logger::warn!(
            "Internal PM session create returned neither sdk_authorization nor external_vault_details"
        );
        return Ok(None);
    }

    Ok(Some(api::VaultDetails {
        internal_vault,
        external_vault_details,
    }))
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
    is_modular_service_enabled: bool,
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
    // Always route vault session creation through the modular PM service when the org is eligible
    // for it (not just when an external vault is configured). When no external vault is set up, the
    // PM service returns the internal Hyperswitch vault SDK authorization, which is the SaaS default.
    if is_modular_service_enabled {
        let customer_id = customer.as_ref().map(|c| c.get_id());

        let vault_details =
            call_internal_pm_session_create_for_vault(state, platform, profile, customer_id)
                .await
                .unwrap_or_else(|err| {
                    router_env::logger::warn!(
                        ?err,
                        "Failed to fetch vault details via internal PM session service"
                    );
                    None
                });

        payment_data.set_vault_session_details(vault_details);
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
    storage_type: common_enums::StorageType,
) -> RouterResult<Option<api::VaultSessionDetails>> {
    let connector = VaultConnectors::try_from(merchant_connector_account_type.get_connector_name())
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
            VaultConnectors::Vgs,
            router_types::ConnectorAuthType::SignatureKey { api_secret, .. },
        ) => {
            let sdk_env = match state.conf.env {
                router_env::Env::Sandbox
                | router_env::Env::Development
                | router_env::Env::Integ => "sandbox",
                router_env::Env::Production => "live",
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
            VaultConnectors::HyperswitchVault,
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
                storage_type,
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

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
async fn generate_hyperswitch_vault_session_details(
    state: &SessionState,
    platform: &domain::Platform,
    merchant_connector_account_type: &domain::MerchantConnectorAccountTypeDetails,
    connector_customer_id: Option<String>,
    connector_name: String,
    vault_publishable_key: hyperswitch_masking::Secret<String>,
    vault_profile_id: hyperswitch_masking::Secret<String>,
    storage_type: common_enums::StorageType,
) -> RouterResult<Option<api::VaultSessionDetails>> {
    let connector_response = call_external_vault_create(
        state,
        platform,
        connector_name,
        merchant_connector_account_type,
        connector_customer_id.clone(),
        storage_type,
    )
    .await?;

    match connector_response.response {
        Ok(router_types::VaultResponseData::ExternalVaultCreateResponse {
            session_id,
            client_secret,
        }) => {
            // Build the base64-encoded SDK authorization for the Hyperswitch Vault session,
            // mirroring the payment-method-session-create response, instead of exposing the
            // individual vault keys.
            let sdk_authorization =
                Option::<hyperswitch_domain_models::sdk_auth::SdkAuthorization>::from(
                    hyperswitch_domain_models::sdk_auth::SdkAuthorizationContext {
                        platform: platform.to_owned(),
                        publishable_key: vault_publishable_key.expose(),
                        profile_id: id_type::ProfileId::try_from(std::borrow::Cow::from(
                            vault_profile_id.expose(),
                        ))
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Invalid profile_id in Hyperswitch Vault connector auth",
                        )?,
                        client_secret: client_secret.expose(),
                        customer_id: connector_customer_id
                            .map(id_type::GlobalCustomerId::new_unchecked),
                        payment_method_session_id: Some(
                            id_type::GlobalPaymentMethodSessionId::new_unchecked(
                                session_id.clone().expose(),
                            ),
                        ),
                    },
                )
                .map(|auth| auth.encode())
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to encode Hyperswitch Vault SDK authorization")?;

            match sdk_authorization {
                Some(sdk_authorization) => Ok(Some(api::VaultSessionDetails::HyperswitchVault(
                    api::HyperswitchVaultSessionDetails {
                        sdk_authorization: sdk_authorization.into(),
                    },
                ))),
                None => {
                    router_env::logger::warn!(
                        "No SDK authorization generated for Hyperswitch Vault session (non-API initiator)"
                    );
                    Ok(None)
                }
            }
        }

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
    storage_type: common_enums::StorageType,
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
        Some(storage_type),
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

#[cfg(feature = "v2")]
pub async fn fetch_external_vault_details(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    customer: &Option<domain::Customer>,
    storage_type: common_enums::StorageType,
) -> RouterResult<Option<api::VaultSessionDetails>> {
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

    // Connector-customer creation is optional. For the guest flow (no customer) we skip it
    // entirely and call the external vault with a `null` customer id. When a customer is present,
    // reuse the existing connector customer or create one at the vault connector.
    let connector_customer_id = match customer {
        Some(_) => get_or_create_vault_connector_customer(
            state,
            platform,
            customer,
            &merchant_connector_account,
            profile.get_id(),
        )
        .await
        .map_err(|err| {
            router_env::logger::error!(?err, "Failed to get or create vault connector customer");
            err
        })?,
        None => {
            router_env::logger::info!(
                "No customer present for external vault session; skipping connector customer creation (guest flow)"
            );
            None
        }
    };

    generate_vault_session_details(
        state,
        platform,
        &merchant_connector_account,
        connector_customer_id,
        storage_type,
    )
    .await
}

/// Returns the existing connector customer ID for the given vault MCA, or creates one if absent.
/// Returns `None` only if the connector does not require a customer.
#[cfg(feature = "v2")]
async fn get_or_create_vault_connector_customer(
    state: &SessionState,
    platform: &domain::Platform,
    customer: &Option<domain::Customer>,
    merchant_connector_account_type: &domain::MerchantConnectorAccountTypeDetails,
    profile_id: &id_type::ProfileId,
) -> RouterResult<Option<String>> {
    use hyperswitch_domain_models::router_request_types::ConnectorCustomerData;

    let db_mca = merchant_connector_account_type
        .get_inner_db_merchant_connector_account()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Vault MCA is missing, cannot create connector customer")?;

    let connector_name = db_mca.get_connector_name_as_string();
    let merchant_connector_id = db_mca.get_id();

    let connector = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_id.clone()),
    )?;

    let (should_create, existing_id) = customers::should_call_connector_create_customer(
        &connector,
        customer,
        merchant_connector_account_type,
    );

    match should_create {
        false => {
            router_env::logger::info!(
                vault_mca_id = %merchant_connector_id.get_string_repr(),
                has_existing_connector_customer_id = existing_id.is_some(),
                "Vault connector customer already exists for MCA, skipping creation"
            );
            Ok(existing_id.map(ToOwned::to_owned))
        }
        true => {
            router_env::logger::info!(
                vault_mca_id = %merchant_connector_id.get_string_repr(),
                has_customer = customer.is_some(),
                "No existing vault connector customer for MCA, creating one now"
            );

            // No existing connector customer – create one now.
            let gateway_context = gateway_context::RouterGatewayContext::direct(
                platform.get_processor().clone(),
                merchant_connector_account_type.clone(),
                platform.get_processor().get_account().get_id().clone(),
                profile_id.clone(),
                None,
            );

            let vault_mca = match merchant_connector_account_type {
                domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(mca) => {
                    Ok(mca.as_ref())
                }
                _ => Err(
                    report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                        "MerchantConnectorDetails not supported for vault operations",
                    ),
                ),
            }?;

            // Construct vault router data, then convert to CreateConnectorCustomer flow
            let vault_router_data_v2 =
                core_utils::construct_vault_router_data::<ExternalVaultCreateFlow>(
                    state,
                    platform.get_processor().get_account().get_id(),
                    vault_mca,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await?;

            let old_vault_router_data =
                VaultConnectorFlowData::to_old_router_data(vault_router_data_v2)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Cannot convert vault router data for connector customer creation",
                    )?;

            // Extract name and email from the customer record for the connector customer creation request.
            let (customer_name, customer_email) = if let Some(cust) = customer.as_ref() {
                let name = cust.name.clone().map(|n| n.into_inner());
                let email = cust.email.clone().map(common_utils::pii::Email::from);
                router_env::logger::info!(
                    vault_customer_name_present = name.is_some(),
                    vault_customer_email_present = email.is_some(),
                    "Building vault connector customer request"
                );
                (name, email)
            } else {
                router_env::logger::warn!("No customer record available when creating vault connector customer; name will be None");
                (None, None)
            };

            let customer_request_data = ConnectorCustomerData {
                description: None,
                email: customer_email,
                phone: None,
                name: customer_name,
                preprocessing_id: None,
                payment_method_data: None,
                split_payments: None,
                setup_future_usage: None,
                customer_acceptance: None,
                customer_id: None,
                billing_address: None,
                metadata: None,
                currency: None,
            };

            let customer_response_data: Result<
                router_types::PaymentsResponseData,
                router_types::ErrorResponse,
            > = Err(router_types::ErrorResponse::default());

            let customer_router_data = helpers::router_data_type_conversion::<
                _,
                api::CreateConnectorCustomer,
                _,
                _,
                _,
                _,
            >(
                old_vault_router_data,
                customer_request_data,
                customer_response_data,
            );

            let new_connector_customer_id = customers::create_connector_customer(
                state,
                &connector,
                &customer_router_data,
                customer_router_data.request.clone(),
                &gateway_context,
            )
            .await?;

            // Persist the newly created connector customer ID back to the customer record.
            let customer_update = customers::update_connector_customer_in_customers(
                merchant_connector_account_type,
                customer.as_ref(),
                new_connector_customer_id.clone(),
                platform.get_initiator(),
            )
            .await;

            if let Some((cust, update)) = customer.clone().zip(customer_update) {
                let db = &*state.store;
                db.update_customer_by_global_id(
                    &cust.get_id().clone(),
                    cust,
                    update,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to persist connector customer ID for vault session")?;
            }

            Ok(new_connector_customer_id)
        }
    }
}
