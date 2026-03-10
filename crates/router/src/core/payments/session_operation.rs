use std::{fmt::Debug, str::FromStr};

pub use common_enums::enums::CallConnectorAction;
use common_utils::id_type;
use error_stack::ResultExt;
pub use hyperswitch_domain_models::{
    mandates::MandateData,
    payment_address::PaymentAddress,
    payments::{HeaderPayload, PaymentIntentData},
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
            self as payments_core, call_multiple_connectors_service,
            flows::{ConstructFlowSpecificData, Feature},
            helpers, helpers as payment_helpers, operations,
            operations::{BoxedOperation, Operation},
            transformers, vault_session, OperationSessionGetters, OperationSessionSetters,
        },
        utils as core_utils,
    },
    db::errors::ConnectorErrorExt,
    errors::RouterResponse,
    routes::{app::ReqState, SessionState},
    services::{self, connector_integration_interface::RouterDataConversion},
    types::{
        self as router_types,
        api::{self, enums as api_enums, ConnectorCommon},
        domain, storage,
    },
    utils::{OptionExt, ValueExt},
};

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_session_core<F, Res, Req, Op, FData, D>(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
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
            platform.clone(),
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
        &platform,
    )
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_session_operation_core<F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: domain::Platform,
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
        .validate_request(&req, &platform)?;

    let operations::GetTrackerResponse { mut payment_data } = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &payment_id,
            &req,
            &platform,
            &profile,
            &header_payload,
        )
        .await?;

    let (_operation, customer) = operation
        .to_domain()?
        .get_customer_details(
            state,
            &mut payment_data,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Failed while fetching/creating customer")?;

    vault_session::populate_vault_session_details(
        state,
        req_state.clone(),
        &customer,
        &platform,
        &operation,
        &profile,
        &mut payment_data,
        header_payload.clone(),
    )
    .await?;

    let connector = operation
        .to_domain()?
        .perform_routing(&platform, &profile, &state.clone(), &mut payment_data)
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
                    platform.get_processor().get_account().storage_scheme,
                    None,
                    platform.get_processor().get_key_store(),
                    None,
                    header_payload.clone(),
                )
                .await?;
            // todo: call surcharge manager for session token call.
            Box::pin(call_multiple_connectors_service(
                state,
                &platform,
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
