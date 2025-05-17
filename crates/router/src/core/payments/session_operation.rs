use std::fmt::Debug;

pub use common_enums::enums::CallConnectorAction;
use common_utils::id_type;
use error_stack::ResultExt;
pub use hyperswitch_domain_models::{
    mandates::{CustomerAcceptance, MandateData},
    payment_address::PaymentAddress,
    payments::HeaderPayload,
    router_data::{PaymentMethodToken, RouterData},
    router_request_types::CustomerDetails,
};
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, utils::StorageErrorExt, RouterResult},
        payments::{
            call_multiple_connectors_service,
            flows::{ConstructFlowSpecificData, Feature},
            helpers, operations,
            operations::{BoxedOperation, Operation},
            transformers, OperationSessionGetters, OperationSessionSetters,
        },
    },
    errors::RouterResponse,
    routes::{app::ReqState, SessionState},
    services,
    types::{self as router_types, api, domain},
    utils::ValueExt,
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

    populate_external_vault_session_details(state, &merchant_context, &profile, &mut payment_data)
        .await?;

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
            ))
            .await?
        }
    };

    Ok((payment_data, req, customer, None, None))
}

#[cfg(feature = "v2")]
pub async fn populate_external_vault_session_details<F, D>(
    state: &SessionState,
    merchant_context: &domain::MerchantContext,
    profile: &domain::Profile,
    payment_data: &mut D,
) -> RouterResult<()>
where
    F: Send + Clone + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let is_external_vault_enabled = profile.get_is_external_vault_enabled();
    let is_external_vault_sdk_enabled = profile.get_is_external_vault_sdk_enabled();

    if is_external_vault_enabled && is_external_vault_sdk_enabled {
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
            "",
            external_vault_source,
        )
        .await?;

        let connector_name = merchant_connector_account
            .get_connector_name()
            .unwrap_or_default(); // always get the connector name from the merchant_connector_account

        let connector_auth_type: router_types::ConnectorAuthType = merchant_connector_account
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let env = state.conf.env;

        let external_vault_session_details =
            helpers::generate_vault_session_details(&connector_name, env, connector_auth_type);

        payment_data.set_external_vault_session_details(external_vault_session_details);
    }
    Ok(())
}
