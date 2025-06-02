use std::{fmt::Debug, str::FromStr};

pub use common_enums::enums::CallConnectorAction;
use common_utils::id_type;
use error_stack::ResultExt;
pub use hyperswitch_domain_models::{
    mandates::{CustomerAcceptance, MandateData},
    payment_address::PaymentAddress,
    payments::HeaderPayload,
    router_data::{PaymentMethodToken, RouterData},
    router_data_v2::{flow_common_types::VaultConnectorFlowData, RouterDataV2},
    router_flow_types::ExternalVaultCreateFlow,
    router_request_types::CustomerDetails,
    types::{VaultRouterData, VaultRouterDataV2},
};
use hyperswitch_interfaces::{
    api::Connector as ConnectorTrait,
    connector_integration_v2::{ConnectorIntegrationV2, ConnectorV2},
};
use masking::ExposeInterface;
use router_env::{env::Env, instrument, tracing};

use crate::{
    core::{
        errors::{self, utils::StorageErrorExt, RouterResult},
        payments::{
            self as payments_core, call_multiple_connectors_service, customers,
            flows::{ConstructFlowSpecificData, Feature},
            helpers, helpers as payment_helpers, operations,
            operations::{BoxedOperation, Operation},
            transformers, OperationSessionGetters, OperationSessionSetters,
        },
        utils as core_utils,
    },
    db::errors::ConnectorErrorExt,
    errors::RouterResponse,
    routes::{app::ReqState, SessionState},
    services::{self, connector_integration_interface::RouterDataConversion},
    types::{
        self as router_types,
        api::{self, enums as api_enums},
        domain, storage,
    },
    utils::{OptionExt, ValueExt},
};

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_session_core<F, Res, Req, Op, FData, D>(
    state: SessionState,
    req_state: ReqState,
    merchant_context: domain::MerchantContext,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
) -> RouterResponse<Res>
where
    F: Send + Clone + Sync,
    Req: Send + Sync,
    FData: Send + Sync + Clone,
    Op: Operation<F, Req, Data = D> + Send + Sync + Clone,
    Req: Debug,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    Res: transformers::ToResponse<F, D, Op>,
    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,
{
    let (payment_data, _req, customer, connector_http_status_code, external_latency) =
        payments_session_operation_core::<_, _, _, _, _>(
            &state,
            req_state,
            merchant_context.clone(),
            profile,
            operation.clone(),
            req,
            payment_id,
            call_connector_action,
            header_payload.clone(),
        )
        .await?;

    Res::generate_response(
        payment_data,
        customer,
        &state.base_url,
        operation,
        &state.conf.connector_request_reference_id_config,
        connector_http_status_code,
        external_latency,
        header_payload.x_hs_latency,
        &merchant_context,
    )
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_session_operation_core<F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    merchant_context: domain::MerchantContext,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    _call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
) -> RouterResult<(D, Req, Option<domain::Customer>, Option<u16>, Option<u128>)>
where
    F: Send + Clone + Sync,
    Req: Send + Sync,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,

    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,
    FData: Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    let _validate_result = operation
        .to_validate_request()?
        .validate_request(&req, &merchant_context)?;

    let operations::GetTrackerResponse { mut payment_data } = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &payment_id,
            &req,
            &merchant_context,
            &profile,
            &header_payload,
        )
        .await?;

    let (_operation, customer) = operation
        .to_domain()?
        .get_customer_details(
            state,
            &mut payment_data,
            merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Failed while fetching/creating customer")?;

    populate_vault_session_details(state, &merchant_context, &profile, &mut payment_data).await?;

    let connector = operation
        .to_domain()?
        .perform_routing(
            &merchant_context,
            &profile,
            &state.clone(),
            &mut payment_data,
        )
        .await?;

    let payment_data = match connector {
        api::ConnectorCallType::PreDetermined(_connector) => {
            todo!()
        }
        api::ConnectorCallType::Retryable(_connectors) => todo!(),
        api::ConnectorCallType::Skip => todo!(),
        api::ConnectorCallType::SessionMultiple(connectors) => {
            operation
                .to_update_tracker()?
                .update_trackers(
                    state,
                    req_state,
                    payment_data.clone(),
                    customer.clone(),
                    merchant_context.get_merchant_account().storage_scheme,
                    None,
                    merchant_context.get_merchant_key_store(),
                    None,
                    header_payload.clone(),
                )
                .await?;
            // todo: call surcharge manager for session token call.
            Box::pin(call_multiple_connectors_service(
                state,
                &merchant_context,
                connectors,
                &operation,
                payment_data,
                &customer,
                None,
                &profile,
                header_payload.clone(),
                None,
            ))
            .await?
        }
    };

    Ok((payment_data, req, customer, None, None))
}

#[cfg(feature = "v2")]
pub async fn populate_vault_session_details<F, D>(
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    profile: &domain::Profile,
    payment_data: &mut D,
) -> RouterResult<()>
where
    F: Send + Clone + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let is_external_vault_sdk_enabled = profile.is_vault_sdk_enabled();

    if is_external_vault_sdk_enabled {
        let vault_session_details =
            generate_vault_session_details(state, merchant_context, profile).await?;

        payment_data.set_vault_session_details(vault_session_details);
    }
    Ok(())
}

#[cfg(feature = "v2")]
pub async fn generate_vault_session_details(
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    profile: &domain::Profile,
) -> RouterResult<Option<api::VaultSessionDetails>> {
    let external_vault_source = profile
        .external_vault_connector_details
        .as_ref()
        .map(|details| &details.vault_connector_id);

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_context.get_merchant_account().get_id(),
        None,
        merchant_context.get_merchant_key_store(),
        profile.get_id(),
        "", // This is a placeholder for the connector name, which is not used in this context
        external_vault_source,
    )
    .await?;

    let connector_name = merchant_connector_account
        .get_connector_name()
        .unwrap_or_default(); // should not panic since we should always have a connector name
    let connector = api_enums::VaultConnectors::from_str(&connector_name)
        .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;
    let connector_auth_type: router_types::ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    match (connector, connector_auth_type) {
        // create session for vgs vault
        (
            api_enums::VaultConnectors::Vgs,
            router_types::ConnectorAuthType::SignatureKey { api_secret, .. },
        ) => {
            let sdk_env = match state.conf.env {
                Env::Sandbox | Env::Development => "sandbox",
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
            let connector_data: api::ConnectorData = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &connector_name,
                api::GetToken::Connector,
                merchant_connector_account.get_mca_id(),
            )?;

            let connector_response = call_external_vault_create(
                state,
                merchant_context,
                &connector_data,
                &merchant_connector_account,
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
                Ok(res) => {
                    router_env::logger::warn!("Unexpected response from external vault create API");
                    Ok(None)
                }
                Err(_) => {
                    router_env::logger::error!(
                        "Failed to create external vault session for connector: {}",
                        connector_name
                    );
                    Err(errors::ApiErrorResponse::InternalServerError.into())
                }
            }
        }
        _ => {
            router_env::logger::warn!(
                "External vault session creation is not supported for connector: {}",
                connector_name
            );
            Ok(None)
        }
    }
}

#[cfg(feature = "v2")]
async fn call_external_vault_create(
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    connector_data: &api::ConnectorData,
    merchant_connector_account: &payment_helpers::MerchantConnectorAccountType,
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
    let mut router_data = core_utils::construct_vault_router_data(
        state,
        merchant_context.get_merchant_account(),
        merchant_connector_account,
        None,
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
