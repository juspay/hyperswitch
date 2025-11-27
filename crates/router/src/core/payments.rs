pub mod access_token;
pub mod conditional_configs;
pub mod customers;
pub mod flows;
pub mod gateway;
pub mod helpers;
pub mod operations;

#[cfg(feature = "retry")]
pub mod retry;
pub mod routing;
#[cfg(feature = "v2")]
pub mod session_operation;
pub mod tokenization;
pub mod transformers;
pub mod types;
#[cfg(feature = "v2")]
pub mod vault_session;
#[cfg(feature = "olap")]
use std::collections::HashMap;
use std::{
    collections::HashSet, fmt::Debug, marker::PhantomData, ops::Deref, str::FromStr, time::Instant,
    vec::IntoIter,
};

use external_services::grpc_client;
#[cfg(feature = "v2")]
pub mod payment_methods;

use std::future;

#[cfg(feature = "olap")]
use api_models::admin::MerchantConnectorInfo;
#[cfg(feature = "v2")]
use api_models::payments::RevenueRecoveryGetIntentResponse;
use api_models::{
    self, enums,
    mandates::RecurringDetails,
    payments::{self as payments_api},
};
pub use common_enums::enums::{CallConnectorAction, ExecutionMode, ExecutionPath, GatewaySystem};
use common_types::payments as common_payments_types;
use common_utils::{
    ext_traits::{AsyncExt, StringExt},
    id_type, pii,
    types::{AmountConvertor, MinorUnit, Surcharge},
};
use diesel_models::{ephemeral_key, fraud_check::FraudCheck, refund as diesel_refund};
use error_stack::{report, ResultExt};
use events::EventInfo;
use futures::future::join_all;
use helpers::{decrypt_paze_token, ApplePayData};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::{
    PaymentAttemptListData, PaymentCancelData, PaymentCaptureData, PaymentConfirmData,
    PaymentIntentData, PaymentStatusData,
};
pub use hyperswitch_domain_models::{
    mandates::MandateData,
    payment_address::PaymentAddress,
    payments::{self as domain_payments, HeaderPayload},
    router_data::{PaymentMethodToken, RouterData},
    router_request_types::CustomerDetails,
};
use hyperswitch_domain_models::{
    payments::{self, payment_intent::CustomerData, ClickToPayMetaData},
    router_data::AccessToken,
};
use masking::{ExposeInterface, PeekInterface, Secret};
#[cfg(feature = "v2")]
use operations::ValidateStatusForOperation;
use redis_interface::errors::RedisError;
use router_env::{instrument, tracing};
#[cfg(feature = "olap")]
use router_types::transformers::ForeignFrom;
use rustc_hash::FxHashMap;
use scheduler::utils as pt_utils;
#[cfg(feature = "v2")]
pub use session_operation::payments_session_core;
#[cfg(feature = "olap")]
use strum::IntoEnumIterator;

#[cfg(feature = "v1")]
pub use self::operations::{
    PaymentApprove, PaymentCancel, PaymentCancelPostCapture, PaymentCapture, PaymentConfirm,
    PaymentCreate, PaymentExtendAuthorization, PaymentIncrementalAuthorization,
    PaymentPostSessionTokens, PaymentReject, PaymentSession, PaymentSessionUpdate, PaymentStatus,
    PaymentUpdate, PaymentUpdateMetadata,
};
use self::{
    conditional_configs::perform_decision_management,
    flows::{ConstructFlowSpecificData, Feature},
    gateway::context as gateway_context,
    operations::{BoxedOperation, Operation, PaymentResponse},
    routing::{self as self_routing, SessionFlowRoutingInput},
};
use super::{
    errors::StorageErrorExt, payment_methods::surcharge_decision_configs, routing::TransactionData,
    unified_connector_service::should_call_unified_connector_service,
};
#[cfg(feature = "v1")]
use crate::core::blocklist::utils as blocklist_utils;
#[cfg(feature = "v1")]
use crate::core::card_testing_guard::utils as card_testing_guard_utils;
#[cfg(feature = "v1")]
use crate::core::debit_routing;
#[cfg(feature = "frm")]
use crate::core::fraud_check as frm_core;
#[cfg(feature = "v2")]
use crate::core::payment_methods::vault;
#[cfg(feature = "v1")]
use crate::core::payments::helpers::{
    process_through_direct, process_through_direct_with_shadow_unified_connector_service,
    process_through_ucs,
};
#[cfg(feature = "v2")]
use crate::core::revenue_recovery::get_workflow_entries;
#[cfg(feature = "v2")]
use crate::core::revenue_recovery::map_to_recovery_payment_item;
#[cfg(feature = "v1")]
use crate::core::routing::helpers as routing_helpers;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use crate::types::api::convert_connector_data_to_routable_connectors;
use crate::{
    configs::settings::{
        ApplePayPreDecryptFlow, GooglePayPreDecryptFlow, PaymentFlow, PaymentMethodTypeTokenFilter,
    },
    consts,
    core::{
        errors::{self, CustomResult, RouterResponse, RouterResult},
        payment_methods::{cards, network_tokenization},
        payouts,
        routing::{self as core_routing},
        unified_authentication_service::types::{ClickToPay, UnifiedAuthenticationService},
        utils as core_utils,
    },
    db::StorageInterface,
    logger,
    routes::{app::ReqState, metrics, payment_methods::ParentPaymentMethodToken, SessionState},
    services::{self, api::Authenticate, ConnectorRedirectResponse},
    types::{
        self as router_types,
        api::{self, ConnectorCallType, ConnectorCommon},
        domain,
        storage::{self, enums as storage_enums, payment_attempt::PaymentAttemptExt},
        transformers::ForeignTryInto,
    },
    utils::{
        self, add_apple_pay_flow_metrics, add_connector_http_status_code_metrics, Encode,
        OptionExt, ValueExt,
    },
    workflows::payment_sync,
};
#[cfg(feature = "v1")]
use crate::{
    core::{
        authentication as authentication_core,
        unified_connector_service::update_gateway_system_in_feature_metadata,
    },
    types::{api::authentication, BrowserInformation},
};

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_operation_core<F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: &domain::Profile,
    operation: Op,
    req: Req,
    get_tracker_response: operations::GetTrackerResponse<D>,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
) -> RouterResult<(
    D,
    Req,
    Option<domain::Customer>,
    Option<u16>,
    Option<u128>,
    common_types::domain::ConnectorResponseData,
)>
where
    F: Send + Clone + Sync,
    Req: Send + Sync + Authenticate,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,

    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    RouterData<F, FData, router_types::PaymentsResponseData>:
        hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<F, FData, D>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
    FData: Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    // Get the trackers related to track the state of the payment
    let operations::GetTrackerResponse { mut payment_data } = get_tracker_response;

    operation
        .to_domain()?
        .create_or_fetch_payment_method(state, &platform, profile, &mut payment_data)
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

    operation
        .to_domain()?
        .run_decision_manager(state, &mut payment_data, profile)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to run decision manager")?;

    let connector = operation
        .to_domain()?
        .perform_routing(&platform, profile, state, &mut payment_data)
        .await?;

    let mut connector_http_status_code = None;
    let (payment_data, connector_response_data) = match connector {
        ConnectorCallType::PreDetermined(connector_data) => {
            let (mca_type_details, updated_customer, router_data, tokenization_action) =
                call_connector_service_prerequisites(
                    state,
                    req_state.clone(),
                    &platform,
                    connector_data.connector_data.clone(),
                    &operation,
                    &mut payment_data,
                    &customer,
                    call_connector_action.clone(),
                    None,
                    header_payload.clone(),
                    None,
                    profile,
                    false,
                    false, //should_retry_with_pan is set to false in case of PreDetermined ConnectorCallType
                    req.should_return_raw_response(),
                )
                .await?;

            let router_data = decide_unified_connector_service_call(
                state,
                req_state.clone(),
                &platform,
                connector_data.connector_data.clone(),
                &operation,
                &mut payment_data,
                &customer,
                call_connector_action.clone(),
                None, // schedule_time is not used in PreDetermined ConnectorCallType
                header_payload.clone(),
                #[cfg(feature = "frm")]
                None,
                profile,
                false,
                false, //should_retry_with_pan is set to false in case of PreDetermined ConnectorCallType
                req.should_return_raw_response(),
                mca_type_details,
                router_data,
                updated_customer,
                tokenization_action,
            )
            .await?;

            let connector_response_data = common_types::domain::ConnectorResponseData {
                raw_connector_response: router_data.raw_connector_response.clone(),
            };

            let payments_response_operation = Box::new(PaymentResponse);

            connector_http_status_code = router_data.connector_http_status_code;
            add_connector_http_status_code_metrics(connector_http_status_code);

            payments_response_operation
                .to_post_update_tracker()?
                .save_pm_and_mandate(state, &router_data, &platform, &mut payment_data, profile)
                .await?;

            let payment_data = payments_response_operation
                .to_post_update_tracker()?
                .update_tracker(
                    state,
                    payment_data,
                    router_data,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await?;

            (payment_data, connector_response_data)
        }
        ConnectorCallType::Retryable(connectors) => {
            let mut connectors = connectors.clone().into_iter();
            let connector_data = get_connector_data(&mut connectors)?;

            let (mca_type_details, updated_customer, router_data, tokenization_action) =
                call_connector_service_prerequisites(
                    state,
                    req_state.clone(),
                    &platform,
                    connector_data.connector_data.clone(),
                    &operation,
                    &mut payment_data,
                    &customer,
                    call_connector_action.clone(),
                    None,
                    header_payload.clone(),
                    None,
                    profile,
                    false,
                    false, //should_retry_with_pan is set to false in case of Retryable ConnectorCallType
                    req.should_return_raw_response(),
                )
                .await?;

            let router_data = decide_unified_connector_service_call(
                state,
                req_state.clone(),
                &platform,
                connector_data.connector_data.clone(),
                &operation,
                &mut payment_data,
                &customer,
                call_connector_action.clone(),
                None, // schedule_time is not used in Retryable ConnectorCallType
                header_payload.clone(),
                #[cfg(feature = "frm")]
                None,
                profile,
                true,
                false, //should_retry_with_pan is set to false in case of PreDetermined ConnectorCallType
                req.should_return_raw_response(),
                mca_type_details,
                router_data,
                updated_customer,
                tokenization_action,
            )
            .await?;

            let connector_response_data = common_types::domain::ConnectorResponseData {
                raw_connector_response: router_data.raw_connector_response.clone(),
            };

            let payments_response_operation = Box::new(PaymentResponse);

            connector_http_status_code = router_data.connector_http_status_code;
            add_connector_http_status_code_metrics(connector_http_status_code);

            payments_response_operation
                .to_post_update_tracker()?
                .save_pm_and_mandate(state, &router_data, &platform, &mut payment_data, profile)
                .await?;

            let payment_data = payments_response_operation
                .to_post_update_tracker()?
                .update_tracker(
                    state,
                    payment_data,
                    router_data,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await?;

            (payment_data, connector_response_data)
        }
        ConnectorCallType::SessionMultiple(vec) => todo!(),
        ConnectorCallType::Skip => (
            payment_data,
            common_types::domain::ConnectorResponseData {
                raw_connector_response: None,
            },
        ),
    };

    let payment_intent_status = payment_data.get_payment_intent().status;

    // Delete the tokens after payment
    payment_data
        .get_payment_attempt()
        .payment_token
        .as_ref()
        .zip(Some(payment_data.get_payment_attempt().payment_method_type))
        .map(ParentPaymentMethodToken::return_key_for_token)
        .async_map(|key_for_token| async move {
            let _ = vault::delete_payment_token(state, &key_for_token, payment_intent_status)
                .await
                .inspect_err(|err| logger::error!("Failed to delete payment_token: {:?}", err));
        })
        .await;

    Ok((
        payment_data,
        req,
        customer,
        connector_http_status_code,
        None,
        connector_response_data,
    ))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn internal_payments_operation_core<F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: &domain::Profile,
    operation: Op,
    req: Req,
    get_tracker_response: operations::GetTrackerResponse<D>,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
) -> RouterResult<(
    D,
    Req,
    Option<u16>,
    Option<u128>,
    common_types::domain::ConnectorResponseData,
)>
where
    F: Send + Clone + Sync,
    Req: Send + Sync + Authenticate,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,

    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    RouterData<F, FData, router_types::PaymentsResponseData>:
        hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<F, FData, D>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
    FData: Send + Sync + Clone + serde::Serialize,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    // Get the trackers related to track the state of the payment
    let operations::GetTrackerResponse { mut payment_data } = get_tracker_response;

    let connector_data = operation
        .to_domain()?
        .get_connector_from_request(state, &req, &mut payment_data)
        .await?;

    let merchant_connector_account = payment_data
        .get_merchant_connector_details()
        .map(domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails)
        .ok_or_else(|| {
            error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Merchant connector details not found in payment data")
        })?;

    operation
        .to_domain()?
        .populate_payment_data(
            state,
            &mut payment_data,
            &platform,
            profile,
            &connector_data,
        )
        .await?;

    let router_data = connector_service_decider(
        state,
        req_state.clone(),
        &platform,
        connector_data.clone(),
        &operation,
        &mut payment_data,
        call_connector_action.clone(),
        header_payload.clone(),
        profile,
        req.should_return_raw_response(),
        merchant_connector_account,
    )
    .await?;

    let connector_response_data = common_types::domain::ConnectorResponseData {
        raw_connector_response: router_data.raw_connector_response.clone(),
    };

    let payments_response_operation = Box::new(PaymentResponse);

    let connector_http_status_code = router_data.connector_http_status_code;
    add_connector_http_status_code_metrics(connector_http_status_code);

    let payment_data = payments_response_operation
        .to_post_update_tracker()?
        .update_tracker(
            state,
            payment_data,
            router_data,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await?;

    Ok((
        payment_data,
        req,
        connector_http_status_code,
        None,
        connector_response_data,
    ))
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_operation_core<'a, F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    profile_id_from_auth_layer: Option<id_type::ProfileId>,
    operation: Op,
    req: Req,
    call_connector_action: CallConnectorAction,
    shadow_ucs_call_connector_action: Option<CallConnectorAction>,
    auth_flow: services::AuthFlow,
    eligible_connectors: Option<Vec<common_enums::RoutableConnectors>>,
    header_payload: HeaderPayload,
) -> RouterResult<(D, Req, Option<domain::Customer>, Option<u16>, Option<u128>)>
where
    F: Send + Clone + Sync + Debug + 'static,
    Req: Authenticate + Clone,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,

    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
    FData: Send + Sync + Clone + router_types::Capturable + 'static + serde::Serialize,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    tracing::Span::current().record(
        "merchant_id",
        platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
    );
    let (operation, validate_result) = operation
        .to_validate_request()?
        .validate_request(&req, platform)?;

    tracing::Span::current().record("payment_id", format!("{}", validate_result.payment_id));
    // get profile from headers
    let operations::GetTrackerResponse {
        operation,
        customer_details,
        mut payment_data,
        business_profile,
        mandate_type,
    } = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &validate_result.payment_id,
            &req,
            platform,
            auth_flow,
            &header_payload,
        )
        .await?;

    operation
        .to_get_tracker()?
        .validate_request_with_state(state, &req, &mut payment_data, &business_profile)
        .await?;

    core_utils::validate_profile_id_from_auth_layer(
        profile_id_from_auth_layer,
        &payment_data.get_payment_intent().clone(),
    )?;

    let (operation, customer) = operation
        .to_domain()?
        // get_customer_details
        .get_or_create_customer_details(
            state,
            &mut payment_data,
            customer_details,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Failed while fetching/creating customer")?;

    let authentication_type =
        call_decision_manager(state, platform, &business_profile, &payment_data).await?;

    payment_data.set_authentication_type_in_attempt(authentication_type);

    let connector = get_connector_choice(
        &operation,
        state,
        &req,
        platform,
        &business_profile,
        &mut payment_data,
        eligible_connectors,
        mandate_type,
    )
    .await?;

    let payment_method_token = get_decrypted_wallet_payment_method_token(
        &operation,
        state,
        platform,
        &mut payment_data,
        connector.as_ref(),
    )
    .await?;

    payment_method_token.map(|token| payment_data.set_payment_method_token(Some(token)));

    let (connector, debit_routing_output) = debit_routing::perform_debit_routing(
        &operation,
        state,
        &business_profile,
        &mut payment_data,
        connector,
    )
    .await;

    operation
        .to_domain()?
        .apply_three_ds_authentication_strategy(state, &mut payment_data, &business_profile)
        .await?;

    let should_add_task_to_process_tracker = should_add_task_to_process_tracker(&payment_data);

    let locale = header_payload.locale.clone();

    payment_data = tokenize_in_router_when_confirm_false_or_external_authentication(
        state,
        &operation,
        &mut payment_data,
        &validate_result,
        platform.get_processor().get_key_store(),
        &customer,
        &business_profile,
    )
    .await?;

    let mut connector_http_status_code = None;
    let mut external_latency = None;
    if let Some(connector_details) = connector {
        // Fetch and check FRM configs
        #[cfg(feature = "frm")]
        let mut frm_info = None;
        #[allow(unused_variables, unused_mut)]
        let mut should_continue_transaction: bool = true;
        #[cfg(feature = "frm")]
        let mut should_continue_capture: bool = true;
        #[cfg(feature = "frm")]
        let frm_configs = if state.conf.frm.enabled {
            Box::pin(frm_core::call_frm_before_connector_call(
                &operation,
                platform,
                &mut payment_data,
                state,
                &mut frm_info,
                &customer,
                &mut should_continue_transaction,
                &mut should_continue_capture,
            ))
            .await?
        } else {
            None
        };
        #[cfg(feature = "frm")]
        logger::debug!(
            "frm_configs: {:?}\nshould_continue_transaction: {:?}\nshould_continue_capture: {:?}",
            frm_configs,
            should_continue_transaction,
            should_continue_capture,
        );

        let is_eligible_for_uas = helpers::is_merchant_eligible_authentication_service(
            platform.get_processor().get_account().get_id(),
            state,
        )
        .await?;

        if <Req as Authenticate>::is_external_three_ds_data_passed_by_merchant(&req) {
            let maybe_connector_enum = match &connector_details {
                ConnectorCallType::PreDetermined(connector_data) => {
                    Some(connector_data.connector_data.connector_name)
                }
                ConnectorCallType::Retryable(connector_list) => connector_list
                    .first()
                    .map(|c| c.connector_data.connector_name),
                ConnectorCallType::SessionMultiple(_) => None,
            };

            if let Some(connector_enum) = maybe_connector_enum {
                if connector_enum.is_separate_authentication_supported() {
                    logger::info!(
                        "Proceeding with external authentication data provided by the merchant for connector: {:?}",
                        connector_enum
                    );
                }
            }
        } else if is_eligible_for_uas {
            operation
                .to_domain()?
                .call_unified_authentication_service_if_eligible(
                    state,
                    &mut payment_data,
                    &mut should_continue_transaction,
                    &connector_details,
                    &business_profile,
                    platform.get_processor().get_key_store(),
                    mandate_type,
                )
                .await?;
        } else {
            logger::info!(
                "skipping authentication service call since the merchant is not eligible."
            );

            operation
                .to_domain()?
                .call_external_three_ds_authentication_if_eligible(
                    state,
                    &mut payment_data,
                    &mut should_continue_transaction,
                    &connector_details,
                    &business_profile,
                    platform.get_processor().get_key_store(),
                    mandate_type,
                )
                .await?;
        };

        operation
            .to_domain()?
            .payments_dynamic_tax_calculation(
                state,
                &mut payment_data,
                &connector_details,
                &business_profile,
                platform,
            )
            .await?;

        if should_continue_transaction {
            #[cfg(feature = "frm")]
            match (
                should_continue_capture,
                payment_data.get_payment_attempt().capture_method,
            ) {
                (
                    false,
                    Some(storage_enums::CaptureMethod::Automatic)
                    | Some(storage_enums::CaptureMethod::SequentialAutomatic),
                )
                | (false, Some(storage_enums::CaptureMethod::Scheduled)) => {
                    if let Some(info) = &mut frm_info {
                        if let Some(frm_data) = &mut info.frm_data {
                            frm_data.fraud_check.payment_capture_method =
                                payment_data.get_payment_attempt().capture_method;
                        }
                    }
                    payment_data
                        .set_capture_method_in_attempt(storage_enums::CaptureMethod::Manual);
                    logger::debug!("payment_id : {:?} capture method has been changed to manual, since it has configured Post FRM flow",payment_data.get_payment_attempt().payment_id);
                }
                _ => (),
            };

            payment_data = match connector_details {
                ConnectorCallType::PreDetermined(ref connector) => {
                    #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                    let routable_connectors = convert_connector_data_to_routable_connectors(
                        std::slice::from_ref(connector),
                    )
                    .map_err(|e| logger::error!(routable_connector_error=?e))
                    .unwrap_or_default();
                    let schedule_time = if should_add_task_to_process_tracker {
                        payment_sync::get_sync_process_schedule_time(
                            &*state.store,
                            connector.connector_data.connector.id(),
                            platform.get_processor().get_account().get_id(),
                            0,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while getting process schedule time")?
                    } else {
                        None
                    };

                    let (merchant_connector_account, router_data, tokenization_action) =
                        call_connector_service_prerequisites(
                            state,
                            platform,
                            connector.connector_data.clone(),
                            &operation,
                            &mut payment_data,
                            &customer,
                            &validate_result,
                            &business_profile,
                            false,
                            None,
                        )
                        .await?;

                    let (router_data, mca) = decide_unified_connector_service_call(
                        state,
                        req_state.clone(),
                        platform,
                        connector.connector_data.clone(),
                        &operation,
                        &mut payment_data,
                        &customer,
                        call_connector_action.clone(),
                        shadow_ucs_call_connector_action.clone(),
                        &validate_result,
                        schedule_time,
                        header_payload.clone(),
                        #[cfg(feature = "frm")]
                        frm_info.as_ref().and_then(|fi| fi.suggested_action),
                        #[cfg(not(feature = "frm"))]
                        None,
                        &business_profile,
                        false,
                        <Req as Authenticate>::should_return_raw_response(&req),
                        merchant_connector_account,
                        router_data,
                        tokenization_action,
                    )
                    .await?;

                    let op_ref = &operation;
                    let should_trigger_post_processing_flows = is_operation_confirm(&operation);

                    let operation = Box::new(PaymentResponse);

                    connector_http_status_code = router_data.connector_http_status_code;
                    external_latency = router_data.external_latency;
                    //add connector http status code metrics
                    add_connector_http_status_code_metrics(connector_http_status_code);

                    operation
                        .to_post_update_tracker()?
                        .save_pm_and_mandate(
                            state,
                            &router_data,
                            platform,
                            &mut payment_data,
                            &business_profile,
                        )
                        .await?;

                    let mut payment_data = operation
                        .to_post_update_tracker()?
                        .update_tracker(
                            state,
                            payment_data,
                            router_data,
                            platform.get_processor().get_key_store(),
                            platform.get_processor().get_account().storage_scheme,
                            &locale,
                            #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                            routable_connectors,
                            #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                            &business_profile,
                        )
                        .await?;

                    if should_trigger_post_processing_flows {
                        complete_postprocessing_steps_if_required(
                            state,
                            platform,
                            &customer,
                            &mca,
                            &connector.connector_data,
                            &mut payment_data,
                            op_ref,
                            Some(header_payload.clone()),
                        )
                        .await?;
                    }

                    if is_eligible_for_uas {
                        complete_confirmation_for_click_to_pay_if_required(
                            state,
                            platform,
                            &payment_data,
                        )
                        .await?;
                    }

                    payment_data
                }

                ConnectorCallType::Retryable(ref connectors) => {
                    #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                    let routable_connectors =
                        convert_connector_data_to_routable_connectors(connectors)
                            .map_err(|e| logger::error!(routable_connector_error=?e))
                            .unwrap_or_default();

                    let mut connectors = connectors.clone().into_iter();

                    let (connector_data, routing_decision) =
                        get_connector_data_with_routing_decision(
                            &mut connectors,
                            &business_profile,
                            debit_routing_output,
                        )?;

                    let schedule_time = if should_add_task_to_process_tracker {
                        payment_sync::get_sync_process_schedule_time(
                            &*state.store,
                            connector_data.connector.id(),
                            platform.get_processor().get_account().get_id(),
                            0,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while getting process schedule time")?
                    } else {
                        None
                    };

                    let (merchant_connector_account, router_data, tokenization_action) =
                        call_connector_service_prerequisites(
                            state,
                            platform,
                            connector_data.clone(),
                            &operation,
                            &mut payment_data,
                            &customer,
                            &validate_result,
                            &business_profile,
                            false,
                            routing_decision,
                        )
                        .await?;

                    let (router_data, mca) = decide_unified_connector_service_call(
                        state,
                        req_state.clone(),
                        platform,
                        connector_data.clone(),
                        &operation,
                        &mut payment_data,
                        &customer,
                        call_connector_action.clone(),
                        shadow_ucs_call_connector_action,
                        &validate_result,
                        schedule_time,
                        header_payload.clone(),
                        #[cfg(feature = "frm")]
                        frm_info.as_ref().and_then(|fi| fi.suggested_action),
                        #[cfg(not(feature = "frm"))]
                        None,
                        &business_profile,
                        false,
                        <Req as Authenticate>::should_return_raw_response(&req),
                        merchant_connector_account,
                        router_data,
                        tokenization_action,
                    )
                    .await?;

                    #[cfg(all(feature = "retry", feature = "v1"))]
                    let mut router_data = router_data;
                    #[cfg(all(feature = "retry", feature = "v1"))]
                    {
                        use crate::core::payments::retry::{self, GsmValidation};
                        let config_bool = retry::config_should_call_gsm(
                            &*state.store,
                            platform.get_processor().get_account().get_id(),
                            &business_profile,
                        )
                        .await;

                        if config_bool && router_data.should_call_gsm() {
                            router_data = retry::do_gsm_actions(
                                state,
                                req_state.clone(),
                                &mut payment_data,
                                connectors,
                                &connector_data,
                                router_data,
                                platform,
                                &operation,
                                &customer,
                                &validate_result,
                                schedule_time,
                                #[cfg(feature = "frm")]
                                frm_info.as_ref().and_then(|fi| fi.suggested_action),
                                #[cfg(not(feature = "frm"))]
                                None,
                                &business_profile,
                            )
                            .await?;
                        };
                    }

                    let op_ref = &operation;
                    let should_trigger_post_processing_flows = is_operation_confirm(&operation);

                    let operation = Box::new(PaymentResponse);
                    connector_http_status_code = router_data.connector_http_status_code;
                    external_latency = router_data.external_latency;
                    //add connector http status code metrics
                    add_connector_http_status_code_metrics(connector_http_status_code);

                    operation
                        .to_post_update_tracker()?
                        .save_pm_and_mandate(
                            state,
                            &router_data,
                            platform,
                            &mut payment_data,
                            &business_profile,
                        )
                        .await?;

                    let mut payment_data = operation
                        .to_post_update_tracker()?
                        .update_tracker(
                            state,
                            payment_data,
                            router_data,
                            platform.get_processor().get_key_store(),
                            platform.get_processor().get_account().storage_scheme,
                            &locale,
                            #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                            routable_connectors,
                            #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                            &business_profile,
                        )
                        .await?;

                    if should_trigger_post_processing_flows {
                        complete_postprocessing_steps_if_required(
                            state,
                            platform,
                            &customer,
                            &mca,
                            &connector_data,
                            &mut payment_data,
                            op_ref,
                            Some(header_payload.clone()),
                        )
                        .await?;
                    }

                    if is_eligible_for_uas {
                        complete_confirmation_for_click_to_pay_if_required(
                            state,
                            platform,
                            &payment_data,
                        )
                        .await?;
                    }

                    payment_data
                }

                ConnectorCallType::SessionMultiple(connectors) => {
                    let session_surcharge_details =
                        call_surcharge_decision_management_for_session_flow(
                            state,
                            platform,
                            &business_profile,
                            payment_data.get_payment_attempt(),
                            payment_data.get_payment_intent(),
                            payment_data.get_billing_address(),
                            &connectors,
                        )
                        .await?;
                    Box::pin(call_multiple_connectors_service(
                        state,
                        platform,
                        connectors,
                        &operation,
                        payment_data,
                        &customer,
                        session_surcharge_details,
                        &business_profile,
                        header_payload.clone(),
                        <Req as Authenticate>::should_return_raw_response(&req),
                    ))
                    .await?
                }
            };

            #[cfg(feature = "frm")]
            if let Some(fraud_info) = &mut frm_info {
                #[cfg(feature = "v1")]
                Box::pin(frm_core::post_payment_frm_core(
                    state,
                    req_state,
                    platform,
                    &mut payment_data,
                    fraud_info,
                    frm_configs
                        .clone()
                        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "frm_configs",
                        })
                        .attach_printable("Frm configs label not found")?,
                    &customer,
                    &mut should_continue_capture,
                ))
                .await?;
            }
        } else {
            (_, payment_data) = operation
                .to_update_tracker()?
                .update_trackers(
                    state,
                    req_state,
                    payment_data.clone(),
                    customer.clone(),
                    validate_result.storage_scheme,
                    None,
                    platform.get_processor().get_key_store(),
                    #[cfg(feature = "frm")]
                    frm_info.and_then(|info| info.suggested_action),
                    #[cfg(not(feature = "frm"))]
                    None,
                    header_payload.clone(),
                )
                .await?;
        }

        let payment_intent_status = payment_data.get_payment_intent().status;

        payment_data
            .get_payment_attempt()
            .payment_token
            .as_ref()
            .zip(payment_data.get_payment_attempt().payment_method)
            .map(ParentPaymentMethodToken::create_key_for_token)
            .async_map(|key_for_hyperswitch_token| async move {
                if key_for_hyperswitch_token
                    .should_delete_payment_method_token(payment_intent_status)
                {
                    let _ = key_for_hyperswitch_token.delete(state).await;
                }
            })
            .await;
    } else {
        (_, payment_data) = operation
            .to_update_tracker()?
            .update_trackers(
                state,
                req_state,
                payment_data.clone(),
                customer.clone(),
                validate_result.storage_scheme,
                None,
                platform.get_processor().get_key_store(),
                None,
                header_payload.clone(),
            )
            .await?;
    }

    let cloned_payment_data = payment_data.clone();
    let cloned_customer = customer.clone();

    #[cfg(feature = "v1")]
    operation
        .to_domain()?
        .store_extended_card_info_temporarily(
            state,
            payment_data.get_payment_intent().get_id(),
            &business_profile,
            payment_data.get_payment_method_data(),
        )
        .await?;

    utils::trigger_payments_webhook(
        platform.clone(),
        business_profile,
        cloned_payment_data,
        cloned_customer,
        state,
        operation,
    )
    .await
    .map_err(|error| logger::warn!(payments_outgoing_webhook_error=?error))
    .ok();

    Ok((
        payment_data,
        req,
        customer,
        connector_http_status_code,
        external_latency,
    ))
}

#[cfg(feature = "v1")]
// This function is intended for use when the feature being implemented is not aligned with the
// core payment operations.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn proxy_for_payments_operation_core<F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile_id_from_auth_layer: Option<id_type::ProfileId>,
    operation: Op,
    req: Req,
    call_connector_action: CallConnectorAction,
    auth_flow: services::AuthFlow,
    header_payload: HeaderPayload,
    return_raw_connector_response: Option<bool>,
) -> RouterResult<(D, Req, Option<domain::Customer>, Option<u16>, Option<u128>)>
where
    F: Send + Clone + Sync,
    Req: Authenticate + Clone,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,

    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector: services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>
        + Send
        + Sync,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
    FData: Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    tracing::Span::current().record(
        "merchant_id",
        platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
    );
    let (operation, validate_result) = operation
        .to_validate_request()?
        .validate_request(&req, &platform)?;

    tracing::Span::current().record("payment_id", format!("{}", validate_result.payment_id));

    let operations::GetTrackerResponse {
        operation,
        customer_details,
        mut payment_data,
        business_profile,
        mandate_type: _,
    } = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &validate_result.payment_id,
            &req,
            &platform,
            auth_flow,
            &header_payload,
        )
        .await?;

    core_utils::validate_profile_id_from_auth_layer(
        profile_id_from_auth_layer,
        &payment_data.get_payment_intent().clone(),
    )?;

    common_utils::fp_utils::when(!should_call_connector(&operation, &payment_data), || {
        Err(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration).attach_printable(format!(
            "Nti and card details based mit flow is not support for this {operation:?} payment operation"
        ))
    })?;

    let connector_choice = operation
        .to_domain()?
        .get_connector(
            &platform,
            &state.clone(),
            &req,
            payment_data.get_payment_intent(),
        )
        .await?;

    let connector = set_eligible_connector_for_nti_in_payment_data(
        state,
        &business_profile,
        platform.get_processor().get_key_store(),
        &mut payment_data,
        connector_choice,
    )
    .await?;

    let should_add_task_to_process_tracker = should_add_task_to_process_tracker(&payment_data);

    let locale = header_payload.locale.clone();

    let schedule_time = if should_add_task_to_process_tracker {
        payment_sync::get_sync_process_schedule_time(
            &*state.store,
            connector.connector.id(),
            platform.get_processor().get_account().get_id(),
            0,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while getting process schedule time")?
    } else {
        None
    };

    let (operation, customer) = operation
        .to_domain()?
        .get_or_create_customer_details(
            state,
            &mut payment_data,
            customer_details,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Failed while fetching/creating customer")?;
    let (router_data, mca) = proxy_for_call_connector_service(
        state,
        req_state.clone(),
        &platform,
        connector.clone(),
        &operation,
        &mut payment_data,
        &customer,
        call_connector_action.clone(),
        &validate_result,
        schedule_time,
        header_payload.clone(),
        &business_profile,
        return_raw_connector_response,
    )
    .await?;

    let op_ref = &operation;
    let should_trigger_post_processing_flows = is_operation_confirm(&operation);

    let operation = Box::new(PaymentResponse);

    let connector_http_status_code = router_data.connector_http_status_code;
    let external_latency = router_data.external_latency;

    add_connector_http_status_code_metrics(connector_http_status_code);

    #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
    let routable_connectors =
        convert_connector_data_to_routable_connectors(&[connector.clone().into()])
            .map_err(|e| logger::error!(routable_connector_error=?e))
            .unwrap_or_default();

    let mut payment_data = operation
        .to_post_update_tracker()?
        .update_tracker(
            state,
            payment_data,
            router_data,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
            &locale,
            #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
            routable_connectors,
            #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
            &business_profile,
        )
        .await?;

    if should_trigger_post_processing_flows {
        complete_postprocessing_steps_if_required(
            state,
            &platform,
            &None,
            &mca,
            &connector,
            &mut payment_data,
            op_ref,
            Some(header_payload.clone()),
        )
        .await?;
    }

    let cloned_payment_data = payment_data.clone();

    utils::trigger_payments_webhook(
        platform.clone(),
        business_profile,
        cloned_payment_data,
        None,
        state,
        operation,
    )
    .await
    .map_err(|error| logger::warn!(payments_outgoing_webhook_error=?error))
    .ok();

    Ok((
        payment_data,
        req,
        None,
        connector_http_status_code,
        external_latency,
    ))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn proxy_for_payments_operation_core<F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    get_tracker_response: operations::GetTrackerResponse<D>,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
    return_raw_connector_response: Option<bool>,
) -> RouterResult<(D, Req, Option<u16>, Option<u128>)>
where
    F: Send + Clone + Sync,
    Req: Send + Sync + Authenticate,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,

    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    RouterData<F, FData, router_types::PaymentsResponseData>:
        hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<F, FData, D>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
    FData: Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    // Get the trackers related to track the state of the payment
    let operations::GetTrackerResponse { mut payment_data } = get_tracker_response;
    // consume the req merchant_connector_id and set it in the payment_data
    let connector = operation
        .to_domain()?
        .perform_routing(&platform, &profile, state, &mut payment_data)
        .await?;

    let payment_data = match connector {
        ConnectorCallType::PreDetermined(connector_data) => {
            let router_data = proxy_for_call_connector_service(
                state,
                req_state.clone(),
                &platform,
                connector_data.connector_data.clone(),
                &operation,
                &mut payment_data,
                call_connector_action.clone(),
                header_payload.clone(),
                &profile,
                return_raw_connector_response,
            )
            .await?;

            let payments_response_operation = Box::new(PaymentResponse);

            payments_response_operation
                .to_post_update_tracker()?
                .update_tracker(
                    state,
                    payment_data,
                    router_data,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await?
        }
        ConnectorCallType::Retryable(vec) => todo!(),
        ConnectorCallType::SessionMultiple(vec) => todo!(),
        ConnectorCallType::Skip => payment_data,
    };

    Ok((payment_data, req, None, None))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn external_vault_proxy_for_payments_operation_core<F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    get_tracker_response: operations::GetTrackerResponse<D>,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
    return_raw_connector_response: Option<bool>,
) -> RouterResult<(D, Req, Option<u16>, Option<u128>)>
where
    F: Send + Clone + Sync,
    Req: Send + Sync + Authenticate,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,

    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    RouterData<F, FData, router_types::PaymentsResponseData>:
        hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<F, FData, D>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
    FData: Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    // Get the trackers related to track the state of the payment
    let operations::GetTrackerResponse { mut payment_data } = get_tracker_response;

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

    operation
        .to_domain()?
        .create_or_fetch_payment_method(state, &platform, &profile, &mut payment_data)
        .await?;

    // consume the req merchant_connector_id and set it in the payment_data
    let connector = operation
        .to_domain()?
        .perform_routing(&platform, &profile, state, &mut payment_data)
        .await?;

    let payment_data = match connector {
        ConnectorCallType::PreDetermined(connector_data) => {
            let (mca_type_details, external_vault_mca_type_details, updated_customer, router_data) =
                call_connector_service_prerequisites_for_external_vault_proxy(
                    state,
                    req_state.clone(),
                    &platform,
                    connector_data.connector_data.clone(),
                    &operation,
                    &mut payment_data,
                    &customer,
                    call_connector_action.clone(),
                    None,
                    header_payload.clone(),
                    None,
                    &profile,
                    false,
                    false, //should_retry_with_pan is set to false in case of PreDetermined ConnectorCallType
                    req.should_return_raw_response(),
                )
                .await?;

            let router_data = call_unified_connector_service_for_external_proxy(
                state,
                req_state.clone(),
                &platform,
                connector_data.connector_data.clone(),
                &operation,
                &mut payment_data,
                &customer,
                call_connector_action.clone(),
                None, // schedule_time is not used in PreDetermined ConnectorCallType
                header_payload.clone(),
                #[cfg(feature = "frm")]
                None,
                &profile,
                false,
                false, //should_retry_with_pan is set to false in case of PreDetermined ConnectorCallType
                req.should_return_raw_response(),
                mca_type_details,
                external_vault_mca_type_details,
                router_data,
                updated_customer,
            )
            .await?;

            // update payment method if its a successful transaction
            if router_data.status.is_success() {
                operation
                    .to_domain()?
                    .update_payment_method(state, &platform, &mut payment_data)
                    .await;
            }

            let payments_response_operation = Box::new(PaymentResponse);

            payments_response_operation
                .to_post_update_tracker()?
                .update_tracker(
                    state,
                    payment_data,
                    router_data,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await?
        }
        ConnectorCallType::Retryable(_) => todo!(),
        ConnectorCallType::SessionMultiple(_) => todo!(),
        ConnectorCallType::Skip => payment_data,
    };

    Ok((payment_data, req, None, None))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_intent_operation_core<F, Req, Op, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
) -> RouterResult<(D, Req, Option<domain::Customer>)>
where
    F: Send + Clone + Sync,
    Req: Clone,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    tracing::Span::current().record(
        "merchant_id",
        platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
    );

    let _validate_result = operation
        .to_validate_request()?
        .validate_request(&req, &platform)?;

    tracing::Span::current().record("global_payment_id", payment_id.get_string_repr());

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

    let (_operation, payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            state,
            req_state,
            payment_data,
            customer.clone(),
            platform.get_processor().get_account().storage_scheme,
            None,
            platform.get_processor().get_key_store(),
            None,
            header_payload,
        )
        .await?;

    Ok((payment_data, req, customer))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_attempt_operation_core<F, Req, Op, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
) -> RouterResult<(D, Req, Option<domain::Customer>)>
where
    F: Send + Clone + Sync,
    Req: Clone,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    tracing::Span::current().record(
        "merchant_id",
        platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
    );

    let _validate_result = operation
        .to_validate_request()?
        .validate_request(&req, &platform)?;

    tracing::Span::current().record("global_payment_id", payment_id.get_string_repr());

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

    let (_operation, payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            state,
            req_state,
            payment_data,
            customer.clone(),
            platform.get_processor().get_account().storage_scheme,
            None,
            platform.get_processor().get_key_store(),
            None,
            header_payload,
        )
        .await?;

    Ok((payment_data, req, customer))
}

#[instrument(skip_all)]
#[cfg(feature = "v1")]
pub async fn call_decision_manager<F, D>(
    state: &SessionState,
    platform: &domain::Platform,
    _business_profile: &domain::Profile,
    payment_data: &D,
) -> RouterResult<Option<enums::AuthenticationType>>
where
    F: Clone,
    D: OperationSessionGetters<F>,
{
    let setup_mandate = payment_data.get_setup_mandate();
    let payment_method_data = payment_data.get_payment_method_data();
    let payment_dsl_data = core_routing::PaymentsDslInput::new(
        setup_mandate,
        payment_data.get_payment_attempt(),
        payment_data.get_payment_intent(),
        payment_method_data,
        payment_data.get_address(),
        payment_data.get_recurring_details(),
        payment_data.get_currency(),
    );
    let algorithm_ref: api::routing::RoutingAlgorithmRef = platform
        .get_processor()
        .get_account()
        .routing_algorithm
        .clone()
        .map(|val| val.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the routing algorithm")?
        .unwrap_or_default();

    let output = perform_decision_management(
        state,
        algorithm_ref,
        platform.get_processor().get_account().get_id(),
        &payment_dsl_data,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Could not decode the conditional config")?;
    Ok(payment_dsl_data
        .payment_attempt
        .authentication_type
        .or(output.override_3ds))
}

// TODO: Move to business profile surcharge column
#[instrument(skip_all)]
#[cfg(feature = "v2")]
pub fn call_decision_manager<F>(
    state: &SessionState,
    record: common_types::payments::DecisionManagerRecord,
    payment_data: &PaymentConfirmData<F>,
) -> RouterResult<Option<enums::AuthenticationType>>
where
    F: Clone,
{
    let payment_method_data = payment_data.get_payment_method_data();
    let payment_dsl_data = core_routing::PaymentsDslInput::new(
        None,
        payment_data.get_payment_attempt(),
        payment_data.get_payment_intent(),
        payment_method_data,
        payment_data.get_address(),
        None,
        payment_data.get_currency(),
    );

    let output = perform_decision_management(record, &payment_dsl_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the conditional config")?;

    Ok(output.override_3ds)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
async fn populate_surcharge_details<F>(
    state: &SessionState,
    payment_data: &mut PaymentData<F>,
) -> RouterResult<()>
where
    F: Send + Clone,
{
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
async fn populate_surcharge_details<F>(
    state: &SessionState,
    payment_data: &mut PaymentData<F>,
) -> RouterResult<()>
where
    F: Send + Clone,
{
    if payment_data
        .payment_intent
        .surcharge_applicable
        .unwrap_or(false)
    {
        logger::debug!("payment_intent.surcharge_applicable = true");
        if let Some(surcharge_details) = payment_data.payment_attempt.get_surcharge_details() {
            // if retry payment, surcharge would have been populated from the previous attempt. Use the same surcharge
            let surcharge_details =
                types::SurchargeDetails::from((&surcharge_details, &payment_data.payment_attempt));
            payment_data.surcharge_details = Some(surcharge_details);
            return Ok(());
        }
        let raw_card_key = payment_data
            .payment_method_data
            .as_ref()
            .and_then(helpers::get_key_params_for_surcharge_details)
            .map(|(payment_method, payment_method_type, card_network)| {
                types::SurchargeKey::PaymentMethodData(
                    payment_method,
                    payment_method_type,
                    card_network,
                )
            });
        let saved_card_key = payment_data.token.clone().map(types::SurchargeKey::Token);

        let surcharge_key = raw_card_key
            .or(saved_card_key)
            .get_required_value("payment_method_data or payment_token")?;
        logger::debug!(surcharge_key_confirm =? surcharge_key);

        let calculated_surcharge_details =
            match types::SurchargeMetadata::get_individual_surcharge_detail_from_redis(
                state,
                surcharge_key,
                &payment_data.payment_attempt.attempt_id,
            )
            .await
            {
                Ok(surcharge_details) => Some(surcharge_details),
                Err(err) if err.current_context() == &RedisError::NotFound => None,
                Err(err) => {
                    Err(err).change_context(errors::ApiErrorResponse::InternalServerError)?
                }
            };

        payment_data.surcharge_details = calculated_surcharge_details.clone();

        //Update payment_attempt net_amount with surcharge details
        payment_data
            .payment_attempt
            .net_amount
            .set_surcharge_details(calculated_surcharge_details);
    } else {
        let surcharge_details =
            payment_data
                .payment_attempt
                .get_surcharge_details()
                .map(|surcharge_details| {
                    logger::debug!("surcharge sent in payments create request");
                    types::SurchargeDetails::from((
                        &surcharge_details,
                        &payment_data.payment_attempt,
                    ))
                });
        payment_data.surcharge_details = surcharge_details;
    }
    Ok(())
}

#[inline]
pub fn get_connector_data(
    connectors: &mut IntoIter<api::ConnectorRoutingData>,
) -> RouterResult<api::ConnectorRoutingData> {
    connectors
        .next()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Connector not found in connectors iterator")
}

#[cfg(feature = "v1")]
pub fn get_connector_with_networks(
    connectors: &mut IntoIter<api::ConnectorRoutingData>,
) -> Option<(api::ConnectorData, enums::CardNetwork)> {
    connectors.find_map(|connector| {
        connector
            .network
            .map(|network| (connector.connector_data, network))
    })
}

#[cfg(feature = "v1")]
fn get_connector_data_with_routing_decision(
    connectors: &mut IntoIter<api::ConnectorRoutingData>,
    business_profile: &domain::Profile,
    debit_routing_output_optional: Option<api_models::open_router::DebitRoutingOutput>,
) -> RouterResult<(
    api::ConnectorData,
    Option<routing_helpers::RoutingDecisionData>,
)> {
    if business_profile.is_debit_routing_enabled && debit_routing_output_optional.is_some() {
        if let Some((data, card_network)) = get_connector_with_networks(connectors) {
            let debit_routing_output =
                debit_routing_output_optional.get_required_value("debit routing output")?;
            let routing_decision =
                routing_helpers::RoutingDecisionData::get_debit_routing_decision_data(
                    card_network,
                    Some(debit_routing_output),
                );
            return Ok((data, Some(routing_decision)));
        }
    }

    Ok((get_connector_data(connectors)?.connector_data, None))
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn call_surcharge_decision_management_for_session_flow(
    _state: &SessionState,
    _platform: &domain::Platform,
    _payment_attempt: &storage::PaymentAttempt,
    _payment_intent: &storage::PaymentIntent,
    _billing_address: Option<hyperswitch_domain_models::address::Address>,
    _session_connector_data: &[api::SessionConnectorData],
) -> RouterResult<Option<api::SessionSurchargeDetails>> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn call_surcharge_decision_management_for_session_flow(
    state: &SessionState,
    platform: &domain::Platform,
    _business_profile: &domain::Profile,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    billing_address: Option<hyperswitch_domain_models::address::Address>,
    session_connector_data: &api::SessionConnectorDatas,
) -> RouterResult<Option<api::SessionSurchargeDetails>> {
    if let Some(surcharge_amount) = payment_attempt.net_amount.get_surcharge_amount() {
        Ok(Some(api::SessionSurchargeDetails::PreDetermined(
            types::SurchargeDetails {
                original_amount: payment_attempt.net_amount.get_order_amount(),
                surcharge: Surcharge::Fixed(surcharge_amount),
                tax_on_surcharge: None,
                surcharge_amount,
                tax_on_surcharge_amount: payment_attempt
                    .net_amount
                    .get_tax_on_surcharge()
                    .unwrap_or_default(),
            },
        )))
    } else {
        let payment_method_type_list = session_connector_data
            .iter()
            .map(|session_connector_data| session_connector_data.payment_method_sub_type)
            .collect();

        #[cfg(feature = "v1")]
        let algorithm_ref: api::routing::RoutingAlgorithmRef = platform
            .get_processor()
            .get_account()
            .routing_algorithm
            .clone()
            .map(|val| val.parse_value("routing algorithm"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not decode the routing algorithm")?
            .unwrap_or_default();

        // TODO: Move to business profile surcharge column
        #[cfg(feature = "v2")]
        let algorithm_ref: api::routing::RoutingAlgorithmRef = todo!();

        let surcharge_results =
            surcharge_decision_configs::perform_surcharge_decision_management_for_session_flow(
                state,
                algorithm_ref,
                payment_attempt,
                payment_intent,
                billing_address,
                &payment_method_type_list,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("error performing surcharge decision operation")?;

        Ok(if surcharge_results.is_empty_result() {
            None
        } else {
            Some(api::SessionSurchargeDetails::Calculated(surcharge_results))
        })
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_core<F, Res, Req, Op, FData, D>(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    operation: Op,
    req: Req,
    auth_flow: services::AuthFlow,
    call_connector_action: CallConnectorAction,
    shadow_ucs_call_connector_action: Option<CallConnectorAction>,
    eligible_connectors: Option<Vec<enums::Connector>>,
    header_payload: HeaderPayload,
) -> RouterResponse<Res>
where
    F: Send + Clone + Sync + Debug + 'static,
    FData: Send + Sync + Clone + router_types::Capturable + 'static + serde::Serialize,
    Op: Operation<F, Req, Data = D> + Send + Sync + Clone,
    Req: Debug + Authenticate + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    Res: transformers::ToResponse<F, D, Op>,
    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,
    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
{
    let eligible_routable_connectors = eligible_connectors.map(|connectors| {
        connectors
            .into_iter()
            .flat_map(|c| c.foreign_try_into())
            .collect()
    });
    let (payment_data, _req, customer, connector_http_status_code, external_latency) =
        payments_operation_core::<_, _, _, _, _>(
            &state,
            req_state,
            &platform,
            profile_id,
            operation.clone(),
            req,
            call_connector_action,
            shadow_ucs_call_connector_action,
            auth_flow,
            eligible_routable_connectors,
            header_payload.clone(),
        )
        .await?;

    Res::generate_response(
        payment_data,
        customer,
        auth_flow,
        &state.base_url,
        operation,
        &state.conf.connector_request_reference_id_config,
        connector_http_status_code,
        external_latency,
        header_payload.x_hs_latency,
    )
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn proxy_for_payments_core<F, Res, Req, Op, FData, D>(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    operation: Op,
    req: Req,
    auth_flow: services::AuthFlow,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
    return_raw_connector_response: Option<bool>,
) -> RouterResponse<Res>
where
    F: Send + Clone + Sync,
    FData: Send + Sync + Clone,
    Op: Operation<F, Req, Data = D> + Send + Sync + Clone,
    Req: Debug + Authenticate + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    Res: transformers::ToResponse<F, D, Op>,
    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
{
    let (payment_data, _req, customer, connector_http_status_code, external_latency) =
        proxy_for_payments_operation_core::<_, _, _, _, _>(
            &state,
            req_state,
            platform,
            profile_id,
            operation.clone(),
            req,
            call_connector_action,
            auth_flow,
            header_payload.clone(),
            return_raw_connector_response,
        )
        .await?;

    Res::generate_response(
        payment_data,
        customer,
        auth_flow,
        &state.base_url,
        operation,
        &state.conf.connector_request_reference_id_config,
        connector_http_status_code,
        external_latency,
        header_payload.x_hs_latency,
    )
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn proxy_for_payments_core<F, Res, Req, Op, FData, D>(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
    return_raw_connector_response: Option<bool>,
) -> RouterResponse<Res>
where
    F: Send + Clone + Sync,
    Req: Send + Sync + Authenticate,
    FData: Send + Sync + Clone,
    Op: Operation<F, Req, Data = D> + ValidateStatusForOperation + Send + Sync + Clone,
    Req: Debug,
    D: OperationSessionGetters<F>
        + OperationSessionSetters<F>
        + transformers::GenerateResponse<Res>
        + Send
        + Sync
        + Clone,
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    PaymentResponse: Operation<F, FData, Data = D>,

    RouterData<F, FData, router_types::PaymentsResponseData>:
        hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<F, FData, D>,
{
    operation
        .to_validate_request()?
        .validate_request(&req, &platform)?;

    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            &state,
            &payment_id,
            &req,
            &platform,
            &profile,
            &header_payload,
        )
        .await?;

    let (payment_data, _req, connector_http_status_code, external_latency) =
        proxy_for_payments_operation_core::<_, _, _, _, _>(
            &state,
            req_state,
            platform.clone(),
            profile.clone(),
            operation.clone(),
            req,
            get_tracker_response,
            call_connector_action,
            header_payload.clone(),
            return_raw_connector_response,
        )
        .await?;

    payment_data.generate_response(
        &state,
        connector_http_status_code,
        external_latency,
        header_payload.x_hs_latency,
        &platform,
        &profile,
        None,
    )
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn external_vault_proxy_for_payments_core<F, Res, Req, Op, FData, D>(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
    return_raw_connector_response: Option<bool>,
) -> RouterResponse<Res>
where
    F: Send + Clone + Sync,
    Req: Send + Sync + Authenticate,
    FData: Send + Sync + Clone,
    Op: Operation<F, Req, Data = D> + ValidateStatusForOperation + Send + Sync + Clone,
    Req: Debug,
    D: OperationSessionGetters<F>
        + OperationSessionSetters<F>
        + transformers::GenerateResponse<Res>
        + Send
        + Sync
        + Clone,
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    PaymentResponse: Operation<F, FData, Data = D>,

    RouterData<F, FData, router_types::PaymentsResponseData>:
        hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<F, FData, D>,
{
    operation
        .to_validate_request()?
        .validate_request(&req, &platform)?;

    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            &state,
            &payment_id,
            &req,
            &platform,
            &profile,
            &header_payload,
        )
        .await?;

    let (payment_data, _req, connector_http_status_code, external_latency) =
        external_vault_proxy_for_payments_operation_core::<_, _, _, _, _>(
            &state,
            req_state,
            platform.clone(),
            profile.clone(),
            operation.clone(),
            req,
            get_tracker_response,
            call_connector_action,
            header_payload.clone(),
            return_raw_connector_response,
        )
        .await?;

    payment_data.generate_response(
        &state,
        connector_http_status_code,
        external_latency,
        header_payload.x_hs_latency,
        &platform,
        &profile,
        None,
    )
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn record_attempt_core(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    req: api_models::payments::PaymentsAttemptRecordRequest,
    payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
) -> RouterResponse<api_models::payments::PaymentAttemptRecordResponse> {
    tracing::Span::current().record(
        "merchant_id",
        platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
    );

    let operation: &operations::payment_attempt_record::PaymentAttemptRecord =
        &operations::payment_attempt_record::PaymentAttemptRecord;
    let boxed_operation: BoxedOperation<
        '_,
        api::RecordAttempt,
        api_models::payments::PaymentsAttemptRecordRequest,
        domain_payments::PaymentAttemptRecordData<api::RecordAttempt>,
    > = Box::new(operation);

    let _validate_result = boxed_operation
        .to_validate_request()?
        .validate_request(&req, &platform)?;

    tracing::Span::current().record("global_payment_id", payment_id.get_string_repr());

    let operations::GetTrackerResponse { payment_data } = boxed_operation
        .to_get_tracker()?
        .get_trackers(
            &state,
            &payment_id,
            &req,
            &platform,
            &profile,
            &header_payload,
        )
        .await?;
    let default_payment_status_data = PaymentStatusData {
        flow: PhantomData,
        payment_intent: payment_data.payment_intent.clone(),
        payment_attempt: payment_data.payment_attempt.clone(),
        attempts: None,
        should_sync_with_connector: false,
        payment_address: payment_data.payment_address.clone(),
        merchant_connector_details: None,
    };

    let payment_status_data = (req.triggered_by == common_enums::TriggeredBy::Internal)
    .then(|| default_payment_status_data.clone())
    .async_unwrap_or_else(|| async {
        match Box::pin(proxy_for_payments_operation_core::<
            api::PSync,
            _,
            _,
            _,
            PaymentStatusData<api::PSync>,
        >(
            &state,
            req_state.clone(),
            platform.clone(),
            profile.clone(),
            operations::PaymentGet,
            api::PaymentsRetrieveRequest {
                force_sync: true,
                expand_attempts: false,
                param: None,
                return_raw_connector_response: None,
                merchant_connector_details: None,
            },
            operations::GetTrackerResponse {
                payment_data: PaymentStatusData {
                    flow: PhantomData,
                    payment_intent: payment_data.payment_intent.clone(),
                    payment_attempt: payment_data.payment_attempt.clone(),
                    attempts: None,
                    should_sync_with_connector: true,
                    payment_address: payment_data.payment_address.clone(),
                    merchant_connector_details: None,
                },
            },
            CallConnectorAction::Trigger,
            HeaderPayload::default(),
            None,
        ))
        .await
        {
            Ok((data, _, _, _)) => data,
            Err(err) => {
                router_env::logger::error!(error=?err, "proxy_for_payments_operation_core failed for external payment attempt");
                default_payment_status_data
            }
        }
    })
    .await;

    let record_payment_data = domain_payments::PaymentAttemptRecordData {
        flow: PhantomData,
        payment_intent: payment_status_data.payment_intent,
        payment_attempt: payment_status_data.payment_attempt,
        revenue_recovery_data: payment_data.revenue_recovery_data.clone(),
        payment_address: payment_data.payment_address.clone(),
    };

    let (_operation, final_payment_data) = boxed_operation
        .to_update_tracker()?
        .update_trackers(
            &state,
            req_state,
            record_payment_data,
            None,
            platform.get_processor().get_account().storage_scheme,
            None,
            platform.get_processor().get_key_store(),
            None,
            header_payload.clone(),
        )
        .await?;

    transformers::GenerateResponse::generate_response(
        final_payment_data,
        &state,
        None,
        None,
        header_payload.x_hs_latency,
        &platform,
        &profile,
        None,
    )
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_intent_core<F, Res, Req, Op, D>(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
) -> RouterResponse<Res>
where
    F: Send + Clone + Sync,
    Op: Operation<F, Req, Data = D> + Send + Sync + Clone,
    Req: Debug + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    Res: transformers::ToResponse<F, D, Op>,
{
    let (payment_data, _req, customer) = payments_intent_operation_core::<_, _, _, _>(
        &state,
        req_state,
        platform.clone(),
        profile,
        operation.clone(),
        req,
        payment_id,
        header_payload.clone(),
    )
    .await?;

    Res::generate_response(
        payment_data,
        customer,
        &state.base_url,
        operation,
        &state.conf.connector_request_reference_id_config,
        None,
        None,
        header_payload.x_hs_latency,
        &platform,
    )
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_list_attempts_using_payment_intent_id<F, Res, Req, Op, D>(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
) -> RouterResponse<Res>
where
    F: Send + Clone + Sync,
    Op: Operation<F, Req, Data = D> + Send + Sync + Clone,
    Req: Debug + Clone,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
    Res: transformers::ToResponse<F, D, Op>,
{
    let (payment_data, _req, customer) = payments_attempt_operation_core::<_, _, _, _>(
        &state,
        req_state,
        platform.clone(),
        profile,
        operation.clone(),
        req,
        payment_id,
        header_payload.clone(),
    )
    .await?;

    Res::generate_response(
        payment_data,
        customer,
        &state.base_url,
        operation,
        &state.conf.connector_request_reference_id_config,
        None,
        None,
        header_payload.x_hs_latency,
        &platform,
    )
}

#[cfg(all(feature = "v2", feature = "olap"))]
#[allow(clippy::too_many_arguments)]
pub async fn revenue_recovery_get_intent_core(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    request: api_models::payments::PaymentsGetIntentRequest,
    global_payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
) -> RouterResponse<RevenueRecoveryGetIntentResponse> {
    use hyperswitch_domain_models::payments::PaymentIntentData;

    use crate::{
        core::revenue_recovery::{get_workflow_entries, map_recovery_status},
        types::storage::revenue_recovery_redis_operation::RedisTokenManager,
    };

    // Get payment intent using the existing operation
    let (payment_data, _req, customer) = Box::pin(payments_intent_operation_core::<
        api::PaymentGetIntent,
        _,
        _,
        PaymentIntentData<api::PaymentGetIntent>,
    >(
        &state,
        req_state,
        platform.clone(),
        profile.clone(),
        operations::PaymentGetIntent,
        request.clone(),
        global_payment_id.clone(),
        header_payload.clone(),
    ))
    .await?;

    let payment_intent = payment_data.get_payment_intent();

    // Get workflow entries to determine recovery status
    let (calculate_workflow, execute_workflow) = get_workflow_entries(&state, &payment_intent.id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch workflow entries")?;

    // Get billing connector account to get retry threshold
    let billing_mca_id = payment_intent.get_billing_merchant_connector_account_id();
    let max_retry_threshold = if let Some(billing_mca_id) = billing_mca_id {
        state
            .store
            .find_merchant_connector_account_by_id(
                &billing_mca_id,
                platform.get_processor().get_key_store(),
            )
            .await
            .ok()
            .and_then(|mca| mca.get_retry_threshold())
            .unwrap_or(0)
    } else {
        0
    };

    // Map recovery status
    let recovery_status = map_recovery_status(
        payment_intent.status,
        calculate_workflow.as_ref(),
        execute_workflow.as_ref(),
        payment_intent.attempt_count,
        max_retry_threshold.try_into().unwrap_or(0),
    );

    // Get card_attached count from Redis
    let card_attached = if let Some(connector_customer_id) =
        payment_intent.get_connector_customer_id_from_feature_metadata()
    {
        RedisTokenManager::get_connector_customer_payment_processor_tokens(
            &state,
            &connector_customer_id,
        )
        .await
        .ok()
        .and_then(|tokens| tokens.len().try_into().ok())
        .unwrap_or(0)
    } else {
        0
    };

    let response = transformers::generate_revenue_recovery_get_intent_response(
        payment_data,
        recovery_status,
        card_attached,
    );

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_get_intent_using_merchant_reference(
    state: SessionState,
    platform: domain::Platform,
    profile: domain::Profile,
    req_state: ReqState,
    merchant_reference_id: &id_type::PaymentReferenceId,
    header_payload: HeaderPayload,
) -> RouterResponse<api::PaymentsIntentResponse> {
    let db = state.store.as_ref();
    let storage_scheme = platform.get_processor().get_account().storage_scheme;
    let payment_intent = db
        .find_payment_intent_by_merchant_reference_id_profile_id(
            merchant_reference_id,
            profile.get_id(),
            platform.get_processor().get_key_store(),
            &storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let (payment_data, _req, customer) = Box::pin(payments_intent_operation_core::<
        api::PaymentGetIntent,
        _,
        _,
        PaymentIntentData<api::PaymentGetIntent>,
    >(
        &state,
        req_state,
        platform.clone(),
        profile.clone(),
        operations::PaymentGetIntent,
        api_models::payments::PaymentsGetIntentRequest {
            id: payment_intent.get_id().clone(),
        },
        payment_intent.get_id().clone(),
        header_payload.clone(),
    ))
    .await?;

    transformers::ToResponse::<
        api::PaymentGetIntent,
        PaymentIntentData<api::PaymentGetIntent>,
        operations::PaymentGetIntent,
    >::generate_response(
        payment_data,
        customer,
        &state.base_url,
        operations::PaymentGetIntent,
        &state.conf.connector_request_reference_id_config,
        None,
        None,
        header_payload.x_hs_latency,
        &platform,
    )
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_core<F, Res, Req, Op, FData, D>(
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
    Req: Send + Sync + Authenticate,
    FData: Send + Sync + Clone + serde::Serialize,
    Op: Operation<F, Req, Data = D> + ValidateStatusForOperation + Send + Sync + Clone,
    Req: Debug,
    D: OperationSessionGetters<F>
        + OperationSessionSetters<F>
        + transformers::GenerateResponse<Res>
        + Send
        + Sync
        + Clone,
    // To create connector flow specific interface data
    D: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,

    // To create updatable objects in post update tracker
    RouterData<F, FData, router_types::PaymentsResponseData>:
        hyperswitch_domain_models::router_data::TrackerPostUpdateObjects<F, FData, D>,
{
    // Validate the request fields
    operation
        .to_validate_request()?
        .validate_request(&req, &platform)?;

    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            &state,
            &payment_id,
            &req,
            &platform,
            &profile,
            &header_payload,
        )
        .await?;

    let (payment_data, connector_http_status_code, external_latency, connector_response_data) =
        if state.conf.merchant_id_auth.merchant_id_auth_enabled {
            let (
                payment_data,
                _req,
                connector_http_status_code,
                external_latency,
                connector_response_data,
            ) = internal_payments_operation_core::<_, _, _, _, _>(
                &state,
                req_state,
                platform.clone(),
                &profile,
                operation.clone(),
                req,
                get_tracker_response,
                call_connector_action,
                header_payload.clone(),
            )
            .await?;
            (
                payment_data,
                connector_http_status_code,
                external_latency,
                connector_response_data,
            )
        } else {
            let (
                payment_data,
                _req,
                _customer,
                connector_http_status_code,
                external_latency,
                connector_response_data,
            ) = payments_operation_core::<_, _, _, _, _>(
                &state,
                req_state,
                platform.clone(),
                &profile,
                operation.clone(),
                req,
                get_tracker_response,
                call_connector_action,
                header_payload.clone(),
            )
            .await?;
            (
                payment_data,
                connector_http_status_code,
                external_latency,
                connector_response_data,
            )
        };

    payment_data.generate_response(
        &state,
        connector_http_status_code,
        external_latency,
        header_payload.x_hs_latency,
        &platform,
        &profile,
        Some(connector_response_data),
    )
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v2")]
pub(crate) async fn payments_execute_wrapper(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    request: payments_api::PaymentsConfirmIntentRequest,
    header_payload: HeaderPayload,
    payment_id: id_type::GlobalPaymentId,
) -> RouterResponse<payments_api::PaymentsResponse> {
    if request.split_payment_method_data.is_none() {
        Box::pin(payments_core::<
            api::Authorize,
            api_models::payments::PaymentsResponse,
            _,
            _,
            _,
            PaymentConfirmData<api::Authorize>,
        >(
            state,
            req_state,
            platform,
            profile,
            operations::PaymentIntentConfirm,
            request,
            payment_id,
            CallConnectorAction::Trigger,
            header_payload,
        ))
        .await
    } else {
        Box::pin(super::split_payments::split_payments_execute_core(
            state,
            req_state,
            platform,
            profile,
            request,
            header_payload,
            payment_id,
        ))
        .await
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v2")]
pub(crate) async fn payments_create_and_confirm_intent(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    request: payments_api::PaymentsRequest,
    header_payload: HeaderPayload,
) -> RouterResponse<payments_api::PaymentsResponse> {
    use hyperswitch_domain_models::{
        payments::PaymentIntentData, router_flow_types::PaymentCreateIntent,
    };

    let payment_id = id_type::GlobalPaymentId::generate(&state.conf.cell_information.id);

    let payload = payments_api::PaymentsCreateIntentRequest::from(&request);

    let create_intent_response = Box::pin(payments_intent_core::<
        PaymentCreateIntent,
        payments_api::PaymentsIntentResponse,
        _,
        _,
        PaymentIntentData<PaymentCreateIntent>,
    >(
        state.clone(),
        req_state.clone(),
        platform.clone(),
        profile.clone(),
        operations::PaymentIntentCreate,
        payload,
        payment_id.clone(),
        header_payload.clone(),
    ))
    .await?;

    logger::info!(?create_intent_response);

    let create_intent_response = create_intent_response
        .get_json_body()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unexpected response from payments core")?;

    let payload = payments_api::PaymentsConfirmIntentRequest::from(&request);

    let confirm_intent_response = decide_authorize_or_setup_intent_flow(
        state,
        req_state,
        platform,
        profile,
        &create_intent_response,
        payload,
        payment_id,
        header_payload,
    )
    .await?;

    logger::info!(?confirm_intent_response);

    Ok(confirm_intent_response)
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
async fn decide_authorize_or_setup_intent_flow(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    create_intent_response: &payments_api::PaymentsIntentResponse,
    confirm_intent_request: payments_api::PaymentsConfirmIntentRequest,
    payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
) -> RouterResponse<payments_api::PaymentsResponse> {
    use hyperswitch_domain_models::{
        payments::PaymentConfirmData,
        router_flow_types::{Authorize, SetupMandate},
    };

    if create_intent_response.amount_details.order_amount == MinorUnit::zero() {
        Box::pin(payments_core::<
            SetupMandate,
            api_models::payments::PaymentsResponse,
            _,
            _,
            _,
            PaymentConfirmData<SetupMandate>,
        >(
            state,
            req_state,
            platform,
            profile,
            operations::PaymentIntentConfirm,
            confirm_intent_request,
            payment_id,
            CallConnectorAction::Trigger,
            header_payload,
        ))
        .await
    } else {
        Box::pin(payments_core::<
            Authorize,
            api_models::payments::PaymentsResponse,
            _,
            _,
            _,
            PaymentConfirmData<Authorize>,
        >(
            state,
            req_state,
            platform,
            profile,
            operations::PaymentIntentConfirm,
            confirm_intent_request,
            payment_id,
            CallConnectorAction::Trigger,
            header_payload,
        ))
        .await
    }
}

fn is_start_pay<Op: Debug>(operation: &Op) -> bool {
    format!("{operation:?}").eq("PaymentStart")
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentsRedirectResponseData {
    pub connector: Option<String>,
    pub param: Option<String>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub json_payload: Option<serde_json::Value>,
    pub resource_id: api::PaymentIdType,
    pub force_sync: bool,
    pub creds_identifier: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentsRedirectResponseData {
    pub payment_id: id_type::GlobalPaymentId,
    pub query_params: String,
    pub json_payload: Option<serde_json::Value>,
}

#[async_trait::async_trait]
pub trait PaymentRedirectFlow: Sync {
    // Associated type for call_payment_flow response
    type PaymentFlowResponse;

    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &SessionState,
        req_state: ReqState,
        platform: domain::Platform,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        connector: String,
        payment_id: id_type::PaymentId,
    ) -> RouterResult<Self::PaymentFlowResponse>;

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &SessionState,
        req_state: ReqState,
        platform: domain::Platform,
        profile: domain::Profile,
        req: PaymentsRedirectResponseData,
    ) -> RouterResult<Self::PaymentFlowResponse>;

    fn get_payment_action(&self) -> services::PaymentAction;

    #[cfg(feature = "v1")]
    fn generate_response(
        &self,
        payment_flow_response: &Self::PaymentFlowResponse,
        payment_id: id_type::PaymentId,
        connector: String,
    ) -> RouterResult<services::ApplicationResponse<api::RedirectionResponse>>;

    #[cfg(feature = "v2")]
    fn generate_response(
        &self,
        payment_flow_response: &Self::PaymentFlowResponse,
    ) -> RouterResult<services::ApplicationResponse<api::RedirectionResponse>>;

    #[cfg(feature = "v1")]
    async fn handle_payments_redirect_response(
        &self,
        state: SessionState,
        req_state: ReqState,
        platform: domain::Platform,
        req: PaymentsRedirectResponseData,
    ) -> RouterResponse<api::RedirectionResponse> {
        metrics::REDIRECTION_TRIGGERED.add(
            1,
            router_env::metric_attributes!(
                (
                    "connector",
                    req.connector.to_owned().unwrap_or("null".to_string()),
                ),
                (
                    "merchant_id",
                    platform.get_processor().get_account().get_id().clone()
                ),
            ),
        );
        let connector = req.connector.clone().get_required_value("connector")?;

        let query_params = req.param.clone().get_required_value("param")?;

        #[cfg(feature = "v1")]
        let resource_id = api::PaymentIdTypeExt::get_payment_intent_id(&req.resource_id)
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_id",
            })?;

        #[cfg(feature = "v2")]
        //TODO: Will get the global payment id from the resource id, we need to handle this in the further flow
        let resource_id: id_type::PaymentId = todo!();

        // This connector data is ephemeral, the call payment flow will get new connector data
        // with merchant account details, so the connector_id can be safely set to None here
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector,
            api::GetToken::Connector,
            None,
        )?;

        let flow_type = connector_data
            .connector
            .get_flow_type(
                &query_params,
                req.json_payload.clone(),
                self.get_payment_action(),
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to decide the response flow")?;

        let payment_flow_response = self
            .call_payment_flow(
                &state,
                req_state,
                platform,
                req.clone(),
                flow_type,
                connector.clone(),
                resource_id.clone(),
            )
            .await?;

        self.generate_response(&payment_flow_response, resource_id, connector)
    }

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn handle_payments_redirect_response(
        &self,
        state: SessionState,
        req_state: ReqState,
        platform: domain::Platform,
        profile: domain::Profile,
        request: PaymentsRedirectResponseData,
    ) -> RouterResponse<api::RedirectionResponse> {
        metrics::REDIRECTION_TRIGGERED.add(
            1,
            router_env::metric_attributes!((
                "merchant_id",
                platform.get_processor().get_account().get_id().clone()
            )),
        );

        let payment_flow_response = self
            .call_payment_flow(&state, req_state, platform, profile, request)
            .await?;

        self.generate_response(&payment_flow_response)
    }
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectCompleteAuthorize;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl PaymentRedirectFlow for PaymentRedirectCompleteAuthorize {
    type PaymentFlowResponse = router_types::RedirectPaymentFlowResponse;

    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &SessionState,
        req_state: ReqState,
        platform: domain::Platform,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        _connector: String,
        _payment_id: id_type::PaymentId,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let payment_confirm_req = api::PaymentsRequest {
            payment_id: Some(req.resource_id.clone()),
            merchant_id: req.merchant_id.clone(),
            merchant_connector_details: req.creds_identifier.map(|creds_id| {
                api::MerchantConnectorDetailsWrap {
                    creds_identifier: creds_id,
                    encoded_data: None,
                }
            }),
            feature_metadata: Some(api_models::payments::FeatureMetadata {
                redirect_response: Some(api_models::payments::RedirectResponse {
                    param: req.param.map(Secret::new),
                    json_payload: Some(req.json_payload.unwrap_or(serde_json::json!({})).into()),
                }),
                search_tags: None,
                apple_pay_recurring_details: None,
            }),
            ..Default::default()
        };
        let response = Box::pin(payments_core::<
            api::CompleteAuthorize,
            api::PaymentsResponse,
            _,
            _,
            _,
            _,
        >(
            state.clone(),
            req_state,
            platform.clone(),
            None,
            operations::payment_complete_authorize::CompleteAuthorize,
            payment_confirm_req,
            services::api::AuthFlow::Merchant,
            connector_action,
            None,
            None,
            HeaderPayload::default(),
        ))
        .await?;
        let payments_response = match response {
            services::ApplicationResponse::Json(response) => Ok(response),
            services::ApplicationResponse::JsonWithHeaders((response, _)) => Ok(response),
            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get the response in json"),
        }?;
        let profile_id = payments_response
            .profile_id
            .as_ref()
            .get_required_value("profile_id")?;
        let business_profile = state
            .store
            .find_business_profile_by_profile_id(
                platform.get_processor().get_key_store(),
                profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;
        Ok(router_types::RedirectPaymentFlowResponse {
            payments_response,
            business_profile,
        })
    }

    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::CompleteAuthorize
    }

    fn generate_response(
        &self,
        payment_flow_response: &Self::PaymentFlowResponse,
        payment_id: id_type::PaymentId,
        connector: String,
    ) -> RouterResult<services::ApplicationResponse<api::RedirectionResponse>> {
        let payments_response = &payment_flow_response.payments_response;
        // There might be multiple redirections needed for some flows
        // If the status is requires customer action, then send the startpay url again
        // The redirection data must have been provided and updated by the connector
        let redirection_response = match payments_response.status {
            enums::IntentStatus::RequiresCustomerAction => {
                let startpay_url = payments_response
                    .next_action
                    .clone()
                    .and_then(|next_action_data| match next_action_data {
                        api_models::payments::NextActionData::RedirectToUrl { redirect_to_url } => Some(redirect_to_url),
                        api_models::payments::NextActionData::RedirectInsidePopup{popup_url, ..} => Some(popup_url),
                        api_models::payments::NextActionData::DisplayBankTransferInformation { .. } => None,
                        api_models::payments::NextActionData::ThirdPartySdkSessionToken { .. } => None,
                        api_models::payments::NextActionData::QrCodeInformation{..} => None,
                        api_models::payments::NextActionData::FetchQrCodeInformation{..} => None,
                        api_models::payments::NextActionData::DisplayVoucherInformation{ .. } => None,
                        api_models::payments::NextActionData::WaitScreenInformation{..} => None,
                        api_models::payments::NextActionData::ThreeDsInvoke{..} => None,
                        api_models::payments::NextActionData::InvokeSdkClient{..} => None,
                        api_models::payments::NextActionData::CollectOtp{ .. } => None,
                        api_models::payments::NextActionData::InvokeHiddenIframe{ .. } => None,
                        api_models::payments::NextActionData::InvokeUpiIntentSdk{ .. } => None,
                        api_models::payments::NextActionData::InvokeUpiQrFlow{ .. } => None,
                    })
                    .ok_or(errors::ApiErrorResponse::InternalServerError)

                    .attach_printable(
                        "did not receive redirect to url when status is requires customer action",
                    )?;
                Ok(api::RedirectionResponse {
                    return_url: String::new(),
                    params: vec![],
                    return_url_with_query_params: startpay_url,
                    http_method: "GET".to_string(),
                    headers: vec![],
                })
            }
            // If the status is terminal status, then redirect to merchant return url to provide status
            enums::IntentStatus::Succeeded
            | enums::IntentStatus::Failed
            | enums::IntentStatus::Cancelled | enums::IntentStatus::RequiresCapture| enums::IntentStatus::Processing=> helpers::get_handle_response_url(
                payment_id,
                &payment_flow_response.business_profile,
                payments_response,
                connector,
            ),
            _ => Err(errors::ApiErrorResponse::InternalServerError).attach_printable_lazy(|| format!("Could not proceed with payment as payment status {} cannot be handled during redirection",payments_response.status))?
        }?;
        if payments_response
            .is_iframe_redirection_enabled
            .unwrap_or(false)
        {
            // html script to check if inside iframe, then send post message to parent for redirection else redirect self to return_url
            let html = core_utils::get_html_redirect_response_popup(
                redirection_response.return_url_with_query_params,
            )?;
            Ok(services::ApplicationResponse::Form(Box::new(
                services::RedirectionFormData {
                    redirect_form: services::RedirectForm::Html { html_data: html },
                    payment_method_data: None,
                    amount: payments_response.amount.to_string(),
                    currency: payments_response.currency.clone(),
                },
            )))
        } else {
            Ok(services::ApplicationResponse::JsonForRedirection(
                redirection_response,
            ))
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectSync;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl PaymentRedirectFlow for PaymentRedirectSync {
    type PaymentFlowResponse = router_types::RedirectPaymentFlowResponse;

    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &SessionState,
        req_state: ReqState,
        platform: domain::Platform,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        _connector: String,
        _payment_id: id_type::PaymentId,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let payment_sync_req = api::PaymentsRetrieveRequest {
            resource_id: req.resource_id,
            merchant_id: req.merchant_id,
            param: req.param,
            force_sync: req.force_sync,
            connector: req.connector,
            merchant_connector_details: req.creds_identifier.map(|creds_id| {
                api::MerchantConnectorDetailsWrap {
                    creds_identifier: creds_id,
                    encoded_data: None,
                }
            }),
            client_secret: None,
            expand_attempts: None,
            expand_captures: None,
            all_keys_required: None,
        };
        let response = Box::pin(
            payments_core::<api::PSync, api::PaymentsResponse, _, _, _, _>(
                state.clone(),
                req_state,
                platform.clone(),
                None,
                PaymentStatus,
                payment_sync_req,
                services::api::AuthFlow::Merchant,
                connector_action,
                None,
                None,
                HeaderPayload::default(),
            ),
        )
        .await?;
        let payments_response = match response {
            services::ApplicationResponse::Json(response) => Ok(response),
            services::ApplicationResponse::JsonWithHeaders((response, _)) => Ok(response),
            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get the response in json"),
        }?;
        let profile_id = payments_response
            .profile_id
            .as_ref()
            .get_required_value("profile_id")?;
        let business_profile = state
            .store
            .find_business_profile_by_profile_id(
                platform.get_processor().get_key_store(),
                profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;
        Ok(router_types::RedirectPaymentFlowResponse {
            payments_response,
            business_profile,
        })
    }
    fn generate_response(
        &self,
        payment_flow_response: &Self::PaymentFlowResponse,
        payment_id: id_type::PaymentId,
        connector: String,
    ) -> RouterResult<services::ApplicationResponse<api::RedirectionResponse>> {
        let payments_response = &payment_flow_response.payments_response;
        let redirect_response = helpers::get_handle_response_url(
            payment_id.clone(),
            &payment_flow_response.business_profile,
            payments_response,
            connector.clone(),
        )?;
        if payments_response
            .is_iframe_redirection_enabled
            .unwrap_or(false)
        {
            // html script to check if inside iframe, then send post message to parent for redirection else redirect self to return_url
            let html = core_utils::get_html_redirect_response_popup(
                redirect_response.return_url_with_query_params,
            )?;
            Ok(services::ApplicationResponse::Form(Box::new(
                services::RedirectionFormData {
                    redirect_form: services::RedirectForm::Html { html_data: html },
                    payment_method_data: None,
                    amount: payments_response.amount.to_string(),
                    currency: payments_response.currency.clone(),
                },
            )))
        } else {
            Ok(services::ApplicationResponse::JsonForRedirection(
                redirect_response,
            ))
        }
    }

    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::PSync
    }
}

#[cfg(feature = "v2")]
impl ValidateStatusForOperation for &PaymentRedirectSync {
    fn validate_status_for_operation(
        &self,
        intent_status: common_enums::IntentStatus,
    ) -> Result<(), errors::ApiErrorResponse> {
        match intent_status {
            common_enums::IntentStatus::RequiresCustomerAction => Ok(()),
            common_enums::IntentStatus::Succeeded
            | common_enums::IntentStatus::Conflicted
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::CancelledPostCapture
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable
            | common_enums::IntentStatus::Expired => {
                Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                    current_flow: format!("{self:?}"),
                    field_name: "status".to_string(),
                    current_value: intent_status.to_string(),
                    states: ["requires_customer_action".to_string()].join(", "),
                })
            }
        }
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl PaymentRedirectFlow for PaymentRedirectSync {
    type PaymentFlowResponse =
        router_types::RedirectPaymentFlowResponse<PaymentStatusData<api::PSync>>;

    async fn call_payment_flow(
        &self,
        state: &SessionState,
        req_state: ReqState,
        platform: domain::Platform,
        profile: domain::Profile,
        req: PaymentsRedirectResponseData,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let payment_id = req.payment_id.clone();

        let payment_sync_request = api::PaymentsRetrieveRequest {
            param: Some(req.query_params.clone()),
            force_sync: true,
            expand_attempts: false,
            return_raw_connector_response: None,
            merchant_connector_details: None, // TODO: Implement for connectors requiring 3DS or redirection-based authentication flows.
        };

        let operation = operations::PaymentGet;
        let boxed_operation: BoxedOperation<
            '_,
            api::PSync,
            api::PaymentsRetrieveRequest,
            PaymentStatusData<api::PSync>,
        > = Box::new(operation);

        let get_tracker_response = boxed_operation
            .to_get_tracker()?
            .get_trackers(
                state,
                &payment_id,
                &payment_sync_request,
                &platform,
                &profile,
                &HeaderPayload::default(),
            )
            .await?;

        let payment_data = &get_tracker_response.payment_data;
        self.validate_status_for_operation(payment_data.payment_intent.status)?;

        let payment_attempt = payment_data.payment_attempt.clone();

        let connector = payment_attempt
            .connector
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "connector is not set in payment attempt in finish redirection flow",
            )?;

        // This connector data is ephemeral, the call payment flow will get new connector data
        // with merchant account details, so the connector_id can be safely set to None here
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector,
            api::GetToken::Connector,
            None,
        )?;

        let call_connector_action = connector_data
            .connector
            .get_flow_type(
                &req.query_params,
                req.json_payload.clone(),
                self.get_payment_action(),
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to decide the response flow")?;

        let (payment_data, _, _, _, _, _) =
            Box::pin(payments_operation_core::<api::PSync, _, _, _, _>(
                state,
                req_state,
                platform,
                &profile,
                operation,
                payment_sync_request,
                get_tracker_response,
                call_connector_action,
                HeaderPayload::default(),
            ))
            .await?;

        Ok(router_types::RedirectPaymentFlowResponse {
            payment_data,
            profile,
        })
    }
    fn generate_response(
        &self,
        payment_flow_response: &Self::PaymentFlowResponse,
    ) -> RouterResult<services::ApplicationResponse<api::RedirectionResponse>> {
        let payment_intent = &payment_flow_response.payment_data.payment_intent;
        let profile = &payment_flow_response.profile;

        let return_url = payment_intent
            .return_url
            .as_ref()
            .or(profile.return_url.as_ref())
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("return url not found in payment intent and profile")?
            .to_owned();

        let return_url = return_url
            .add_query_params(("id", payment_intent.id.get_string_repr()))
            .add_query_params(("status", &payment_intent.status.to_string()));

        let return_url_str = return_url.into_inner().to_string();

        Ok(services::ApplicationResponse::JsonForRedirection(
            api::RedirectionResponse {
                return_url_with_query_params: return_url_str,
            },
        ))
    }

    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::PSync
    }
}

#[derive(Clone, Debug)]
pub struct PaymentAuthenticateCompleteAuthorize;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl PaymentRedirectFlow for PaymentAuthenticateCompleteAuthorize {
    type PaymentFlowResponse = router_types::AuthenticatePaymentFlowResponse;

    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &SessionState,
        req_state: ReqState,
        platform: domain::Platform,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        connector: String,
        payment_id: id_type::PaymentId,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let merchant_id = platform.get_processor().get_account().get_id().clone();

        let payment_intent = state
            .store
            .find_payment_intent_by_payment_id_merchant_id(
                &payment_id,
                &merchant_id,
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        let payment_attempt = state
            .store
            .find_payment_attempt_by_attempt_id_merchant_id(
                &payment_intent.active_attempt.get_id(),
                &merchant_id,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        let authentication_id = payment_attempt
            .authentication_id
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("missing authentication_id in payment_attempt")?;
        let authentication = state
            .store
            .find_authentication_by_merchant_id_authentication_id(&merchant_id, &authentication_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::AuthenticationNotFound {
                id: authentication_id.get_string_repr().to_string(),
            })?;
        // Fetching merchant_connector_account to check if pull_mechanism is enabled for 3ds connector
        let authentication_merchant_connector_account = helpers::get_merchant_connector_account(
            state,
            &merchant_id,
            None,
            platform.get_processor().get_key_store(),
            &payment_intent
                .profile_id
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("missing profile_id in payment_intent")?,
            &payment_attempt
                .authentication_connector
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("missing authentication connector in payment_intent")?,
            None,
        )
        .await?;
        let is_pull_mechanism_enabled =
            utils::check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata(
                authentication_merchant_connector_account
                    .get_metadata()
                    .map(|metadata| metadata.expose()),
            );
        let response = if is_pull_mechanism_enabled
            || authentication.authentication_type
                != Some(common_enums::DecoupledAuthenticationType::Challenge)
        {
            let payment_confirm_req = api::PaymentsRequest {
                payment_id: Some(req.resource_id.clone()),
                merchant_id: req.merchant_id.clone(),
                feature_metadata: Some(api_models::payments::FeatureMetadata {
                    redirect_response: Some(api_models::payments::RedirectResponse {
                        param: req.param.map(Secret::new),
                        json_payload: Some(
                            req.json_payload.unwrap_or(serde_json::json!({})).into(),
                        ),
                    }),
                    search_tags: None,
                    apple_pay_recurring_details: None,
                }),
                ..Default::default()
            };
            Box::pin(payments_core::<
                api::Authorize,
                api::PaymentsResponse,
                _,
                _,
                _,
                _,
            >(
                state.clone(),
                req_state,
                platform.clone(),
                None,
                PaymentConfirm,
                payment_confirm_req,
                services::api::AuthFlow::Merchant,
                connector_action,
                None,
                None,
                HeaderPayload::with_source(enums::PaymentSource::ExternalAuthenticator),
            ))
            .await?
        } else {
            let payment_sync_req = api::PaymentsRetrieveRequest {
                resource_id: req.resource_id,
                merchant_id: req.merchant_id,
                param: req.param,
                force_sync: req.force_sync,
                connector: req.connector,
                merchant_connector_details: req.creds_identifier.map(|creds_id| {
                    api::MerchantConnectorDetailsWrap {
                        creds_identifier: creds_id,
                        encoded_data: None,
                    }
                }),
                client_secret: None,
                expand_attempts: None,
                expand_captures: None,
                all_keys_required: None,
            };
            Box::pin(
                payments_core::<api::PSync, api::PaymentsResponse, _, _, _, _>(
                    state.clone(),
                    req_state,
                    platform.clone(),
                    None,
                    PaymentStatus,
                    payment_sync_req,
                    services::api::AuthFlow::Merchant,
                    connector_action,
                    None,
                    None,
                    HeaderPayload::default(),
                ),
            )
            .await?
        };
        let payments_response = match response {
            services::ApplicationResponse::Json(response) => Ok(response),
            services::ApplicationResponse::JsonWithHeaders((response, _)) => Ok(response),
            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get the response in json"),
        }?;
        // When intent status is RequiresCustomerAction, Set poll_id in redis to allow the fetch status of poll through retrieve_poll_status api from client
        if payments_response.status == common_enums::IntentStatus::RequiresCustomerAction {
            let req_poll_id = core_utils::get_external_authentication_request_poll_id(&payment_id);
            let poll_id = core_utils::get_poll_id(&merchant_id, req_poll_id.clone());
            let redis_conn = state
                .store
                .get_redis_conn()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get redis connection")?;
            redis_conn
                .set_key_with_expiry(
                    &poll_id.into(),
                    api_models::poll::PollStatus::Pending.to_string(),
                    consts::POLL_ID_TTL,
                )
                .await
                .change_context(errors::StorageError::KVError)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to add poll_id in redis")?;
        };
        let default_poll_config = router_types::PollConfig::default();
        let default_config_str = default_poll_config
            .encode_to_string_of_json()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while stringifying default poll config")?;
        let poll_config = state
            .store
            .find_config_by_key_unwrap_or(
                &router_types::PollConfig::get_poll_config_key(connector),
                Some(default_config_str),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("The poll config was not found in the DB")?;
        let poll_config: router_types::PollConfig = poll_config
            .config
            .parse_struct("PollConfig")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while parsing PollConfig")?;
        let profile_id = payments_response
            .profile_id
            .as_ref()
            .get_required_value("profile_id")?;
        let business_profile = state
            .store
            .find_business_profile_by_profile_id(
                platform.get_processor().get_key_store(),
                profile_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;
        Ok(router_types::AuthenticatePaymentFlowResponse {
            payments_response,
            poll_config,
            business_profile,
        })
    }
    fn generate_response(
        &self,
        payment_flow_response: &Self::PaymentFlowResponse,
        payment_id: id_type::PaymentId,
        connector: String,
    ) -> RouterResult<services::ApplicationResponse<api::RedirectionResponse>> {
        let payments_response = &payment_flow_response.payments_response;
        let redirect_response = helpers::get_handle_response_url(
            payment_id.clone(),
            &payment_flow_response.business_profile,
            payments_response,
            connector.clone(),
        )?;
        // html script to check if inside iframe, then send post message to parent for redirection else redirect self to return_url
        let html = core_utils::get_html_redirect_response_for_external_authentication(
            redirect_response.return_url_with_query_params,
            payments_response,
            payment_id,
            &payment_flow_response.poll_config,
        )?;
        Ok(services::ApplicationResponse::Form(Box::new(
            services::RedirectionFormData {
                redirect_form: services::RedirectForm::Html { html_data: html },
                payment_method_data: None,
                amount: payments_response.amount.to_string(),
                currency: payments_response.currency.clone(),
            },
        )))
    }

    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::PaymentAuthenticateCompleteAuthorize
    }
}

#[cfg(feature = "v1")]
pub async fn get_decrypted_wallet_payment_method_token<F, Req, D>(
    operation: &BoxedOperation<'_, F, Req, D>,
    state: &SessionState,
    platform: &domain::Platform,
    payment_data: &mut D,
    connector_call_type_optional: Option<&ConnectorCallType>,
) -> CustomResult<Option<PaymentMethodToken>, errors::ApiErrorResponse>
where
    F: Send + Clone + Sync,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    if is_operation_confirm(operation)
        && payment_data.get_payment_attempt().payment_method
            == Some(storage_enums::PaymentMethod::Wallet)
        && payment_data.get_payment_method_data().is_some()
    {
        let wallet_type = payment_data
            .get_payment_attempt()
            .payment_method_type
            .get_required_value("payment_method_type")?;

        let wallet: Box<dyn WalletFlow<F, D>> = match wallet_type {
            storage_enums::PaymentMethodType::ApplePay => Box::new(ApplePayWallet),
            storage_enums::PaymentMethodType::Paze => Box::new(PazeWallet),
            storage_enums::PaymentMethodType::GooglePay => Box::new(GooglePayWallet),
            _ => return Ok(None),
        };

        // Check if the wallet has already decrypted the token from the payment data.
        // If a pre-decrypted token is available, use it directly to avoid redundant decryption.
        if let Some(predecrypted_token) = wallet.check_predecrypted_token(payment_data)? {
            logger::debug!("Using predecrypted token for wallet");
            return Ok(Some(predecrypted_token));
        }

        let merchant_connector_account =
            get_merchant_connector_account_for_wallet_decryption_flow::<F, D>(
                state,
                platform,
                payment_data,
                connector_call_type_optional,
            )
            .await?;

        let decide_wallet_flow = &wallet
            .decide_wallet_flow(state, payment_data, &merchant_connector_account)
            .attach_printable("Failed to decide wallet flow")?
            .async_map(|payment_price_data| async move {
                wallet
                    .decrypt_wallet_token(&payment_price_data, payment_data)
                    .await
            })
            .await
            .transpose()
            .attach_printable("Failed to decrypt Wallet token")?;
        Ok(decide_wallet_flow.clone())
    } else {
        Ok(None)
    }
}

#[cfg(feature = "v1")]
pub async fn get_merchant_connector_account_for_wallet_decryption_flow<F, D>(
    state: &SessionState,
    platform: &domain::Platform,
    payment_data: &mut D,
    connector_call_type_optional: Option<&ConnectorCallType>,
) -> RouterResult<helpers::MerchantConnectorAccountType>
where
    F: Send + Clone + Sync,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    let connector_call_type = connector_call_type_optional
        .get_required_value("connector_call_type")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let connector_routing_data = match connector_call_type {
        ConnectorCallType::PreDetermined(connector_routing_data) => connector_routing_data,
        ConnectorCallType::Retryable(connector_routing_data) => connector_routing_data
            .first()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Found no connector routing data in retryable call")?,
        ConnectorCallType::SessionMultiple(_session_connector_data) => {
            return Err(errors::ApiErrorResponse::InternalServerError).attach_printable(
                "SessionMultiple connector call type is invalid in confirm calls",
            );
        }
    };

    construct_profile_id_and_get_mca(
        state,
        platform,
        payment_data,
        &connector_routing_data
            .connector_data
            .connector_name
            .to_string(),
        connector_routing_data
            .connector_data
            .merchant_connector_id
            .as_ref(),
        false,
    )
    .await
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn call_connector_service<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    validate_result: &operations::ValidateResult,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
    business_profile: &domain::Profile,
    is_retry_payment: bool,
    return_raw_connector_response: Option<bool>,
    merchant_connector_account: helpers::MerchantConnectorAccountType,
    mut router_data: RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    tokenization_action: TokenizationAction,
    context: gateway_context::RouterGatewayContext,
) -> RouterResult<(
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    helpers::MerchantConnectorAccountType,
)>
where
    F: Send + Clone + Sync,
    RouterDReq: Send + Sync,

    // To create connector flow specific interface data
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, RouterDReq, router_types::PaymentsResponseData>,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>: Feature<F, RouterDReq> + Send,
    // To construct connector flow specific api
    dyn api::Connector: services::api::ConnectorIntegration<F, RouterDReq, router_types::PaymentsResponseData>
        + Send
        + Sync,
{
    let add_access_token_result = router_data
        .add_access_token(
            state,
            &connector,
            platform,
            payment_data.get_creds_identifier(),
        )
        .await?;

    router_data = router_data.add_session_token(state, &connector).await?;

    let should_continue_further = access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &call_connector_action,
    );

    let should_continue_further = match router_data
        .create_order_at_connector(state, &connector, should_continue_further)
        .await?
    {
        Some(create_order_response) => {
            if let Ok(order_id) = create_order_response.clone().create_order_result {
                payment_data.set_connector_response_reference_id(Some(order_id.clone()))
            }

            // Set the response in routerdata response to carry forward
            router_data
                .update_router_data_with_create_order_response(create_order_response.clone());
            create_order_response.create_order_result.ok().is_some()
        }
        // If create order is not required, then we can proceed with further processing
        None => true,
    };

    let updated_customer = call_create_connector_customer_if_required(
        state,
        customer,
        platform,
        &merchant_connector_account,
        payment_data,
        router_data.access_token.as_ref(),
    )
    .await?;

    #[cfg(feature = "v1")]
    if let Some(connector_customer_id) = {
        core_utils::get_connector_customer_id(
            &state.conf,
            &connector.connector_name.to_string(),
            payment_data.get_connector_customer_id(),
            &payment_data.get_payment_intent().customer_id,
            &payment_data.get_payment_method_info().cloned(),
            payment_data.get_payment_attempt(),
        )?
    } {
        router_data.connector_customer = Some(connector_customer_id);
    }

    router_data.payment_method_token = payment_data.get_payment_method_token().cloned();

    let payment_method_token_response = router_data
        .add_payment_method_token(
            state,
            &connector,
            &tokenization_action,
            should_continue_further,
        )
        .await?;

    let mut should_continue_further =
        tokenization::update_router_data_with_payment_method_token_result(
            payment_method_token_response,
            &mut router_data,
            is_retry_payment,
            should_continue_further,
        );

    (router_data, should_continue_further) = complete_preprocessing_steps_if_required(
        state,
        &connector,
        payment_data,
        router_data,
        operation,
        should_continue_further,
    )
    .await?;

    if let Ok(router_types::PaymentsResponseData::PreProcessingResponse {
        session_token: Some(session_token),
        ..
    }) = router_data.response.to_owned()
    {
        payment_data.push_sessions_token(session_token);
    };

    // In case of authorize flow, pre-task and post-tasks are being called in build request
    // if we do not want to proceed further, then the function will return Ok(None, false)
    let (connector_request, should_continue_further) = if should_continue_further {
        // Check if the actual flow specific request can be built with available data
        router_data
            .build_flow_specific_connector_request(state, &connector, call_connector_action.clone())
            .await?
    } else {
        (None, false)
    };

    if should_add_task_to_process_tracker(payment_data) {
        operation
            .to_domain()?
            .add_task_to_process_tracker(
                state,
                payment_data.get_payment_attempt(),
                validate_result.requeue,
                schedule_time,
            )
            .await
            .map_err(|error| logger::error!(process_tracker_error=?error))
            .ok();
    }

    // Update the payment trackers just before calling the connector
    // Since the request is already built in the previous step,
    // there should be no error in request construction from hyperswitch end
    (_, *payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            state,
            req_state,
            payment_data.clone(),
            customer.clone(),
            platform.get_processor().get_account().storage_scheme,
            updated_customer,
            platform.get_processor().get_key_store(),
            frm_suggestion,
            header_payload.clone(),
        )
        .await?;

    let router_data = if should_continue_further {
        // The status of payment_attempt and intent will be updated in the previous step
        // update this in router_data.
        // This is added because few connector integrations do not update the status,
        // and rely on previous status set in router_data
        router_data.status = payment_data.get_payment_attempt().status;
        router_data
            .decide_flows(
                state,
                &connector,
                call_connector_action,
                connector_request,
                business_profile,
                header_payload.clone(),
                return_raw_connector_response,
                context,
            )
            .await
    } else {
        Ok(router_data)
    }?;

    Ok((router_data, merchant_connector_account))
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn call_connector_service_prerequisites<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    customer: &Option<domain::Customer>,
    validate_result: &operations::ValidateResult,
    business_profile: &domain::Profile,
    should_retry_with_pan: bool,
    routing_decision: Option<routing_helpers::RoutingDecisionData>,
) -> RouterResult<(
    helpers::MerchantConnectorAccountType,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    TokenizationAction,
)>
where
    F: Send + Clone + Sync,
    RouterDReq: Send + Clone + Sync,

    // To create connector flow specific interface data
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, RouterDReq, router_types::PaymentsResponseData>,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>:
        Feature<F, RouterDReq> + Send + Clone,
    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, RouterDReq, router_types::PaymentsResponseData>,
{
    let merchant_connector_account = construct_profile_id_and_get_mca(
        state,
        platform,
        payment_data,
        &connector.connector_name.to_string(),
        connector.merchant_connector_id.as_ref(),
        false,
    )
    .await?;

    let customer_acceptance = payment_data
        .get_payment_attempt()
        .customer_acceptance
        .clone();

    if is_pre_network_tokenization_enabled(
        state,
        business_profile,
        customer_acceptance,
        connector.connector_name,
    ) {
        let payment_method_data = payment_data.get_payment_method_data();
        let customer_id = payment_data.get_payment_intent().customer_id.clone();
        if let (Some(domain::PaymentMethodData::Card(card_data)), Some(customer_id)) =
            (payment_method_data, customer_id)
        {
            let vault_operation =
                get_vault_operation_for_pre_network_tokenization(state, customer_id, card_data)
                    .await;
            match vault_operation {
                payments::VaultOperation::SaveCardAndNetworkTokenData(
                    card_and_network_token_data,
                ) => {
                    payment_data.set_vault_operation(
                        payments::VaultOperation::SaveCardAndNetworkTokenData(Box::new(
                            *card_and_network_token_data.clone(),
                        )),
                    );

                    payment_data.set_payment_method_data(Some(
                        domain::PaymentMethodData::NetworkToken(
                            card_and_network_token_data
                                .network_token
                                .network_token_data
                                .clone(),
                        ),
                    ));
                }
                payments::VaultOperation::SaveCardData(card_data_for_vault) => payment_data
                    .set_vault_operation(payments::VaultOperation::SaveCardData(
                        card_data_for_vault.clone(),
                    )),
                payments::VaultOperation::ExistingVaultData(_) => (),
            }
        }
    }

    if payment_data
        .get_payment_attempt()
        .merchant_connector_id
        .is_none()
    {
        payment_data.set_merchant_connector_id_in_attempt(merchant_connector_account.get_mca_id());
    }

    operation
        .to_domain()?
        .populate_payment_data(state, payment_data, platform, business_profile, &connector)
        .await?;

    let (pd, tokenization_action) = get_connector_tokenization_action_when_confirm_true(
        state,
        operation,
        payment_data,
        validate_result,
        platform.get_processor().get_key_store(),
        customer,
        business_profile,
        should_retry_with_pan,
    )
    .await?;
    *payment_data = pd;

    // This is used to apply any kind of routing decision to the required data,
    // before the call to `connector` is made.
    routing_decision.map(|decision| decision.apply_routing_decision(payment_data));

    // Validating the blocklist guard and generate the fingerprint
    blocklist_guard(state, platform, operation, payment_data).await?;

    let merchant_recipient_data = payment_data
        .get_merchant_recipient_data(state, platform, &merchant_connector_account, &connector)
        .await?;

    let router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            platform,
            customer,
            &merchant_connector_account,
            merchant_recipient_data,
            None,
            payment_data.get_payment_attempt().payment_method,
            payment_data.get_payment_attempt().payment_method_type,
        )
        .await?;

    let connector_request_reference_id = router_data.connector_request_reference_id.clone();
    payment_data
        .set_connector_request_reference_id_in_payment_attempt(connector_request_reference_id);

    Ok((merchant_connector_account, router_data, tokenization_action))
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn decide_unified_connector_service_call<'a, F, RouterDReq, ApiRequest, D>(
    state: &'a SessionState,
    req_state: ReqState,
    platform: &'a domain::Platform,
    connector: api::ConnectorData,
    operation: &'a BoxedOperation<'a, F, ApiRequest, D>,
    payment_data: &'a mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    shadow_ucs_call_connector_action: Option<CallConnectorAction>,
    validate_result: &'a operations::ValidateResult,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
    business_profile: &'a domain::Profile,
    is_retry_payment: bool,
    all_keys_required: Option<bool>,
    merchant_connector_account: helpers::MerchantConnectorAccountType,
    mut router_data: RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    tokenization_action: TokenizationAction,
) -> RouterResult<(
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    helpers::MerchantConnectorAccountType,
)>
where
    F: Send + Clone + Sync + Debug + 'static,
    RouterDReq: Send + Sync + Clone + 'static + serde::Serialize,

    // To create connector flow specific interface data
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, RouterDReq, router_types::PaymentsResponseData>,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>:
        Feature<F, RouterDReq> + Send + Clone + serde::Serialize,
    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, RouterDReq, router_types::PaymentsResponseData>,
{
    let (execution_path, updated_state) = should_call_unified_connector_service(
        state,
        platform,
        &router_data,
        Some(payment_data),
        call_connector_action.clone(),
        shadow_ucs_call_connector_action.clone(),
    )
    .await?;

    let ucs_flow = if matches!(
        execution_path,
        ExecutionPath::UnifiedConnectorService | ExecutionPath::ShadowUnifiedConnectorService
    ) {
        let cached_access_token = access_token::get_cached_access_token_for_ucs(
            state,
            &connector,
            platform,
            router_data.payment_method,
            payment_data.get_creds_identifier(),
        )
        .await?;

        // Set cached access token in router_data if available
        if let Some(access_token) = cached_access_token {
            router_data.access_token = Some(access_token);
        }
        true
    } else {
        false
    };

    let is_ucs_granular_flow =
        gateway::GRANULAR_GATEWAY_SUPPORTED_FLOWS.contains(&std::any::type_name::<F>());

    if is_ucs_granular_flow && ucs_flow {
        logger::info!("Current flow is UCS Granular flow");
        let lineage_ids = grpc_client::LineageIds::new(
            business_profile.merchant_id.clone(),
            business_profile.get_id().clone(),
        );

        let execution_mode = match execution_path {
            ExecutionPath::UnifiedConnectorService => ExecutionMode::Primary,
            ExecutionPath::ShadowUnifiedConnectorService => ExecutionMode::Shadow,
            // ExecutionMode is irrelevant for Direct path in this context
            ExecutionPath::Direct => ExecutionMode::NotApplicable,
        };

        let gateway_context = gateway_context::RouterGatewayContext {
            creds_identifier: payment_data.get_creds_identifier().map(|id| id.to_string()),
            platform: platform.clone(),
            header_payload: header_payload.clone(),
            lineage_ids,
            merchant_connector_account: merchant_connector_account.clone(),
            execution_path,
            execution_mode,
        };
        // Update feature metadata to track Direct routing usage for stickiness
        update_gateway_system_in_feature_metadata(
            payment_data,
            gateway_context.get_gateway_system(),
        )?;
        call_connector_service(
            &updated_state,
            req_state,
            platform,
            connector,
            operation,
            payment_data,
            customer,
            call_connector_action,
            validate_result,
            schedule_time,
            header_payload,
            frm_suggestion,
            business_profile,
            is_retry_payment,
            all_keys_required,
            merchant_connector_account,
            router_data,
            tokenization_action,
            gateway_context,
        )
        .await
    } else {
        record_time_taken_with(|| async {
            match execution_path {
                // Process through UCS when system is UCS and not handling response or if it is a UCS webhook action
                ExecutionPath::UnifiedConnectorService => {
                    process_through_ucs(
                        &updated_state,
                        req_state,
                        platform,
                        operation,
                        payment_data,
                        customer,
                        call_connector_action,
                        validate_result,
                        schedule_time,
                        header_payload,
                        frm_suggestion,
                        business_profile,
                        merchant_connector_account,
                        &connector,
                        router_data,
                    )
                    .await
                }

                // Process through Direct with Shadow UCS
                ExecutionPath::ShadowUnifiedConnectorService => {
                    process_through_direct_with_shadow_unified_connector_service(
                        &updated_state,
                        req_state,
                        platform,
                        connector,
                        operation,
                        payment_data,
                        customer,
                        call_connector_action,
                        shadow_ucs_call_connector_action,
                        validate_result,
                        schedule_time,
                        header_payload,
                        frm_suggestion,
                        business_profile,
                        is_retry_payment,
                        all_keys_required,
                        merchant_connector_account,
                        router_data,
                        tokenization_action,
                    )
                    .await
                }

                // Process through Direct gateway
                ExecutionPath::Direct => {
                    process_through_direct(
                        state,
                        req_state,
                        platform,
                        connector,
                        operation,
                        payment_data,
                        customer,
                        call_connector_action,
                        validate_result,
                        schedule_time,
                        header_payload,
                        frm_suggestion,
                        business_profile,
                        is_retry_payment,
                        all_keys_required,
                        merchant_connector_account,
                        router_data,
                        tokenization_action,
                    )
                    .await
                }
            }
        })
        .await
    }
}

async fn record_time_taken_with<F, Fut, R>(f: F) -> RouterResult<R>
where
    F: FnOnce() -> Fut,
    Fut: future::Future<Output = RouterResult<R>>,
{
    let stime = Instant::now();
    let result = f().await;
    let etime = Instant::now();
    let duration = etime.saturating_duration_since(stime);
    tracing::info!(duration = format!("Duration taken: {}", duration.as_millis()));
    result
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn call_connector_service<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
    business_profile: &domain::Profile,
    is_retry_payment: bool,
    should_retry_with_pan: bool,
    return_raw_connector_response: Option<bool>,
    merchant_connector_account_type_details: domain::MerchantConnectorAccountTypeDetails,
    mut router_data: RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    updated_customer: Option<storage::CustomerUpdate>,
    tokenization_action: TokenizationAction,
) -> RouterResult<RouterData<F, RouterDReq, router_types::PaymentsResponseData>>
where
    F: Send + Clone + Sync,
    RouterDReq: Send + Sync,

    // To create connector flow specific interface data
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, RouterDReq, router_types::PaymentsResponseData>,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>: Feature<F, RouterDReq> + Send,
    // To construct connector flow specific api
    dyn api::Connector: services::api::ConnectorIntegration<F, RouterDReq, router_types::PaymentsResponseData>
        + Send
        + Sync,
{
    let add_access_token_result = router_data
        .add_access_token(
            state,
            &connector,
            platform,
            payment_data.get_creds_identifier(),
        )
        .await?;

    router_data = router_data.add_session_token(state, &connector).await?;

    let should_continue_further = access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &call_connector_action,
    );
    let payment_method_token_response = router_data
        .add_payment_method_token(
            state,
            &connector,
            &tokenization_action,
            should_continue_further,
        )
        .await?;
    let should_continue_further = tokenization::update_router_data_with_payment_method_token_result(
        payment_method_token_response,
        &mut router_data,
        is_retry_payment,
        should_continue_further,
    );
    let should_continue = match router_data
        .create_order_at_connector(state, &connector, should_continue_further)
        .await?
    {
        Some(create_order_response) => {
            if let Ok(order_id) = create_order_response.clone().create_order_result {
                payment_data.set_connector_response_reference_id(Some(order_id))
            }

            // Set the response in routerdata response to carry forward
            router_data
                .update_router_data_with_create_order_response(create_order_response.clone());
            create_order_response.create_order_result.ok().map(|_| ())
        }
        // If create order is not required, then we can proceed with further processing
        None => Some(()),
    };

    // In case of authorize flow, pre-task and post-tasks are being called in build request
    // if we do not want to proceed further, then the function will return Ok(None, false)
    let (connector_request, should_continue_further) = match should_continue {
        Some(_) => {
            router_data
                .build_flow_specific_connector_request(
                    state,
                    &connector,
                    call_connector_action.clone(),
                )
                .await?
        }
        None => (None, false),
    };

    // Update the payment trackers just before calling the connector
    // Since the request is already built in the previous step,
    // there should be no error in request construction from hyperswitch end
    (_, *payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            state,
            req_state,
            payment_data.clone(),
            customer.clone(),
            platform.get_processor().get_account().storage_scheme,
            updated_customer,
            platform.get_processor().get_key_store(),
            frm_suggestion,
            header_payload.clone(),
        )
        .await?;

    let router_data = if should_continue_further {
        // The status of payment_attempt and intent will be updated in the previous step
        // update this in router_data.
        // This is added because few connector integrations do not update the status,
        // and rely on previous status set in router_data
        // TODO: status is already set when constructing payment data, why should this be done again?
        // router_data.status = payment_data.get_payment_attempt().status;
        let lineage_ids = grpc_client::LineageIds::new(
            business_profile.merchant_id.clone(),
            business_profile.get_id().clone(),
        );
        let gateway_context = gateway_context::RouterGatewayContext {
            creds_identifier: payment_data.get_creds_identifier().map(|id| id.to_string()),
            platform: platform.clone(),
            header_payload: header_payload.clone(),
            lineage_ids,
            merchant_connector_account: merchant_connector_account_type_details,
            execution_path: ExecutionPath::Direct,
            execution_mode: ExecutionMode::NotApplicable,
        };
        router_data
            .decide_flows(
                state,
                &connector,
                call_connector_action,
                connector_request,
                business_profile,
                header_payload.clone(),
                return_raw_connector_response,
                gateway_context,
            )
            .await
    } else {
        Ok(router_data)
    }?;

    Ok(router_data)
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
#[instrument(skip_all)]
pub async fn call_connector_service_prerequisites<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
    business_profile: &domain::Profile,
    is_retry_payment: bool,
    should_retry_with_pan: bool,
    all_keys_required: Option<bool>,
) -> RouterResult<(
    domain::MerchantConnectorAccountTypeDetails,
    Option<storage::CustomerUpdate>,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    TokenizationAction,
)>
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
    let merchant_connector_account_type_details =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
            helpers::get_merchant_connector_account_v2(
                state,
                platform.get_processor().get_key_store(),
                connector.merchant_connector_id.as_ref(),
            )
            .await?,
        ));

    operation
        .to_domain()?
        .populate_payment_data(state, payment_data, platform, business_profile, &connector)
        .await?;

    let updated_customer = call_create_connector_customer_if_required(
        state,
        customer,
        platform,
        &merchant_connector_account_type_details,
        payment_data,
    )
    .await?;

    let router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            platform,
            customer,
            &merchant_connector_account_type_details,
            None,
            Some(header_payload),
        )
        .await?;

    let tokenization_action = operation
        .to_domain()?
        .get_connector_tokenization_action(state, payment_data)
        .await?;

    Ok((
        merchant_connector_account_type_details,
        updated_customer,
        router_data,
        tokenization_action,
    ))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
#[instrument(skip_all)]
pub async fn call_connector_service_prerequisites_for_external_vault_proxy<
    F,
    RouterDReq,
    ApiRequest,
    D,
>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
    business_profile: &domain::Profile,
    is_retry_payment: bool,
    should_retry_with_pan: bool,
    all_keys_required: Option<bool>,
) -> RouterResult<(
    domain::MerchantConnectorAccountTypeDetails,
    domain::MerchantConnectorAccountTypeDetails,
    Option<storage::CustomerUpdate>,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
)>
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
    // get merchant connector account related to external vault
    let external_vault_source: id_type::MerchantConnectorAccountId = business_profile
        .external_vault_connector_details
        .clone()
        .map(|connector_details| connector_details.vault_connector_id.clone())
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("mca_id not present for external vault")?;

    let external_vault_merchant_connector_account_type_details =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
            helpers::get_merchant_connector_account_v2(
                state,
                platform.get_processor().get_key_store(),
                Some(&external_vault_source),
            )
            .await?,
        ));

    let (
        merchant_connector_account_type_details,
        updated_customer,
        router_data,
        _tokenization_action,
    ) = call_connector_service_prerequisites(
        state,
        req_state,
        platform,
        connector,
        operation,
        payment_data,
        customer,
        call_connector_action,
        schedule_time,
        header_payload,
        frm_suggestion,
        business_profile,
        is_retry_payment,
        should_retry_with_pan,
        all_keys_required,
    )
    .await?;
    Ok((
        merchant_connector_account_type_details,
        external_vault_merchant_connector_account_type_details,
        updated_customer,
        router_data,
    ))
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn internal_call_connector_service_prerequisites<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    business_profile: &domain::Profile,
) -> RouterResult<(
    domain::MerchantConnectorAccountTypeDetails,
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
)>
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
    let merchant_connector_details =
        payment_data
            .get_merchant_connector_details()
            .ok_or_else(|| {
                error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Merchant connector details not found in payment data")
            })?;
    let merchant_connector_account =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(
            merchant_connector_details,
        );

    operation
        .to_domain()?
        .populate_payment_data(state, payment_data, platform, business_profile, &connector)
        .await?;

    let router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            platform,
            &None,
            &merchant_connector_account,
            None,
            None,
        )
        .await?;

    Ok((merchant_connector_account, router_data))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn connector_service_decider<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
    business_profile: &domain::Profile,
    return_raw_connector_response: Option<bool>,
    merchant_connector_account_type_details: domain::MerchantConnectorAccountTypeDetails,
) -> RouterResult<RouterData<F, RouterDReq, router_types::PaymentsResponseData>>
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
    let mut router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            platform,
            &None,
            &merchant_connector_account_type_details,
            None,
            Some(header_payload.clone()),
        )
        .await?;

    // do order creation
    let (execution_path, updated_state) = should_call_unified_connector_service(
        state,
        platform,
        &router_data,
        Some(payment_data),
        call_connector_action.clone(),
        None,
    )
    .await?;

    let (connector_request, should_continue_further) =
        if matches!(execution_path, ExecutionPath::Direct) {
            let mut should_continue_further = true;

            let should_continue = match router_data
                .create_order_at_connector(state, &connector, should_continue_further)
                .await?
            {
                Some(create_order_response) => {
                    if let Ok(order_id) = create_order_response.clone().create_order_result {
                        payment_data.set_connector_response_reference_id(Some(order_id))
                    }

                    // Set the response in routerdata response to carry forward
                    router_data.update_router_data_with_create_order_response(
                        create_order_response.clone(),
                    );
                    create_order_response.create_order_result.ok().map(|_| ())
                }
                // If create order is not required, then we can proceed with further processing
                None => Some(()),
            };

            let should_continue: (Option<common_utils::request::Request>, bool) =
                match should_continue {
                    Some(_) => {
                        router_data
                            .build_flow_specific_connector_request(
                                state,
                                &connector,
                                call_connector_action.clone(),
                            )
                            .await?
                    }
                    None => (None, false),
                };
            should_continue
        } else {
            // If unified connector service is called, these values are not used
            // as the request is built in the unified connector service call
            (None, false)
        };

    (_, *payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            state,
            req_state,
            payment_data.clone(),
            None, // customer is not used in internal flows
            platform.get_processor().get_account().storage_scheme,
            None,
            platform.get_processor().get_key_store(),
            None, // frm_suggestion is not used in internal flows
            header_payload.clone(),
        )
        .await?;

    record_time_taken_with(|| async {
        if matches!(execution_path, ExecutionPath::UnifiedConnectorService) {
            router_env::logger::info!(
                "Processing payment through UCS gateway system- payment_id={}, attempt_id={}",
                payment_data.get_payment_intent().id.get_string_repr(),
                payment_data.get_payment_attempt().id.get_string_repr()
            );
            let lineage_ids = grpc_client::LineageIds::new(business_profile.merchant_id.clone(), business_profile.get_id().clone());

            // Extract merchant_order_reference_id from payment data for UCS audit trail
            let merchant_order_reference_id = payment_data.get_payment_intent().merchant_reference_id
                .clone()
                .map(|id| id.get_string_repr().to_string());
            let creds_identifier = payment_data.get_creds_identifier().map(str::to_owned);

            router_data
                .call_unified_connector_service(
                    state,
                    &header_payload,
                    lineage_ids,
                    merchant_connector_account_type_details.clone(),
                    platform,
                    &connector,
                    ExecutionMode::Primary, // UCS is called in primary mode
                    merchant_order_reference_id,
                    call_connector_action,
                    creds_identifier,
                )
                .await?;

            Ok(router_data)
        } else {
            Err(
                errors::ApiErrorResponse::InternalServerError
            )
            .attach_printable("Unified connector service is down and traditional connector service fallback is not implemented")
        }
    })
    .await
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn decide_unified_connector_service_call<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
    business_profile: &domain::Profile,
    is_retry_payment: bool,
    should_retry_with_pan: bool,
    return_raw_connector_response: Option<bool>,
    merchant_connector_account_type_details: domain::MerchantConnectorAccountTypeDetails,
    mut router_data: RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    updated_customer: Option<storage::CustomerUpdate>,
    tokenization_action: TokenizationAction,
) -> RouterResult<RouterData<F, RouterDReq, router_types::PaymentsResponseData>>
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
    record_time_taken_with(|| async {
        let (execution, updated_state) = should_call_unified_connector_service(
            state,
            platform,
            &router_data,
            Some(payment_data),
            call_connector_action.clone(),
            None,
        )
        .await?;
        if matches!(execution, ExecutionPath::UnifiedConnectorService) {
            router_env::logger::info!(
                "Executing payment through UCS gateway system - payment_id={}, attempt_id={}",
                payment_data.get_payment_intent().id.get_string_repr(),
                payment_data.get_payment_attempt().id.get_string_repr()
            );
            if should_add_task_to_process_tracker(payment_data) {
                operation
                    .to_domain()?
                    .add_task_to_process_tracker(
                        state,
                        payment_data.get_payment_attempt(),
                        false,
                        schedule_time,
                    )
                    .await
                    .map_err(|error| logger::error!(process_tracker_error=?error))
                    .ok();
            }

            (_, *payment_data) = operation
                .to_update_tracker()?
                .update_trackers(
                    state,
                    req_state,
                    payment_data.clone(),
                    customer.clone(),
                    platform.get_processor().get_account().storage_scheme,
                    None,
                    platform.get_processor().get_key_store(),
                    frm_suggestion,
                    header_payload.clone(),
                )
                .await?;
            let lineage_ids = grpc_client::LineageIds::new(
                business_profile.merchant_id.clone(),
                business_profile.get_id().clone(),
            );

            // Extract merchant_order_reference_id from payment data for UCS audit trail
            let merchant_order_reference_id = payment_data.get_payment_intent().merchant_reference_id
                .clone()
                .map(|id| id.get_string_repr().to_string());
            let creds_identifier = payment_data.get_creds_identifier().map(str::to_owned);

            // Check for cached access token in Redis (no generation for UCS flows)
            let cached_access_token = access_token::get_cached_access_token_for_ucs(
                state,
                &connector,
                platform,
                router_data.payment_method,
                payment_data.get_creds_identifier(),
            )
            .await?;

            // Set cached access token in router_data if available
            if let Some(access_token) = cached_access_token {
                router_data.access_token = Some(access_token);
            }

            router_data
                .call_unified_connector_service(
                    state,
                    &header_payload,
                    lineage_ids,
                    merchant_connector_account_type_details.clone(),
                    platform,
                    &connector,
                    ExecutionMode::Primary, //UCS is called in primary mode
                    merchant_order_reference_id,
                    call_connector_action,
                    creds_identifier
                )
                .await?;

            Ok(router_data)
        } else {
            if matches!(execution, ExecutionPath::ShadowUnifiedConnectorService) {
                router_env::logger::info!(
                    "Shadow UCS mode not implemented in v2, processing through direct path - payment_id={}, attempt_id={}",
                    payment_data.get_payment_intent().id.get_string_repr(),
                    payment_data.get_payment_attempt().id.get_string_repr()
                );
            } else {
                router_env::logger::info!(
                    "Processing payment through Direct gateway system - payment_id={}, attempt_id={}",
                    payment_data.get_payment_intent().id.get_string_repr(),
                    payment_data.get_payment_attempt().id.get_string_repr()
                );
            }


            let session_state = if matches!(execution, ExecutionPath::ShadowUnifiedConnectorService) {
                &updated_state
            } else {
                state
            };

            call_connector_service(
                session_state,
                req_state,
                platform,
                connector,
                operation,
                payment_data,
                customer,
                call_connector_action,
                schedule_time,
                header_payload,
                frm_suggestion,
                business_profile,
                is_retry_payment,
                should_retry_with_pan,
                return_raw_connector_response,
                merchant_connector_account_type_details,
                router_data,
                updated_customer,
                tokenization_action,
            )
            .await
        }
    })
    .await
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn call_unified_connector_service_for_external_proxy<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    _connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    customer: &Option<domain::Customer>,
    _call_connector_action: CallConnectorAction,
    _schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
    business_profile: &domain::Profile,
    _is_retry_payment: bool,
    _should_retry_with_pan: bool,
    _return_raw_connector_response: Option<bool>,
    merchant_connector_account_type_details: domain::MerchantConnectorAccountTypeDetails,
    external_vault_merchant_connector_account_type_details: domain::MerchantConnectorAccountTypeDetails,
    mut router_data: RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    _updated_customer: Option<storage::CustomerUpdate>,
) -> RouterResult<RouterData<F, RouterDReq, router_types::PaymentsResponseData>>
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
    record_time_taken_with(|| async {
        (_, *payment_data) = operation
            .to_update_tracker()?
            .update_trackers(
                state,
                req_state,
                payment_data.clone(),
                customer.clone(),
                platform.get_processor().get_account().storage_scheme,
                None,
                platform.get_processor().get_key_store(),
                frm_suggestion,
                header_payload.clone(),
            )
            .await?;
        let lineage_ids = grpc_client::LineageIds::new(
            business_profile.merchant_id.clone(),
            business_profile.get_id().clone(),
        );

        // Extract merchant_order_reference_id from payment data for UCS audit trail
        let merchant_order_reference_id = payment_data
            .get_payment_intent()
            .merchant_reference_id
            .clone()
            .map(|id| id.get_string_repr().to_string());

        router_data
            .call_unified_connector_service_with_external_vault_proxy(
                state,
                &header_payload,
                lineage_ids,
                merchant_connector_account_type_details.clone(),
                external_vault_merchant_connector_account_type_details.clone(),
                platform,
                ExecutionMode::Primary, //UCS is called in primary mode
                merchant_order_reference_id,
            )
            .await?;

        Ok(router_data)
    })
    .await
}

#[cfg(feature = "v1")]
// This function does not perform the tokenization action, as the payment method is not saved in this flow.
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn proxy_for_call_connector_service<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    validate_result: &operations::ValidateResult,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,

    business_profile: &domain::Profile,
    return_raw_connector_response: Option<bool>,
) -> RouterResult<(
    RouterData<F, RouterDReq, router_types::PaymentsResponseData>,
    helpers::MerchantConnectorAccountType,
)>
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
    let stime_connector = Instant::now();

    let merchant_connector_account = construct_profile_id_and_get_mca(
        state,
        platform,
        payment_data,
        &connector.connector_name.to_string(),
        connector.merchant_connector_id.as_ref(),
        false,
    )
    .await?;

    if payment_data
        .get_payment_attempt()
        .merchant_connector_id
        .is_none()
    {
        payment_data.set_merchant_connector_id_in_attempt(merchant_connector_account.get_mca_id());
    }

    let merchant_recipient_data = None;

    let mut router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            platform,
            customer,
            &merchant_connector_account,
            merchant_recipient_data,
            Some(header_payload.clone()),
            payment_data.get_payment_attempt().payment_method,
            payment_data.get_payment_attempt().payment_method_type,
        )
        .await?;

    let add_access_token_result = router_data
        .add_access_token(
            state,
            &connector,
            platform,
            payment_data.get_creds_identifier(),
        )
        .await?;

    router_data = router_data.add_session_token(state, &connector).await?;

    let mut should_continue_further = access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &call_connector_action,
    );

    (router_data, should_continue_further) = complete_preprocessing_steps_if_required(
        state,
        &connector,
        payment_data,
        router_data,
        operation,
        should_continue_further,
    )
    .await?;

    if let Ok(router_types::PaymentsResponseData::PreProcessingResponse {
        session_token: Some(session_token),
        ..
    }) = router_data.response.to_owned()
    {
        payment_data.push_sessions_token(session_token);
    };

    let (connector_request, should_continue_further) = if should_continue_further {
        // Check if the actual flow specific request can be built with available data
        router_data
            .build_flow_specific_connector_request(state, &connector, call_connector_action.clone())
            .await?
    } else {
        (None, false)
    };

    if should_add_task_to_process_tracker(payment_data) {
        operation
            .to_domain()?
            .add_task_to_process_tracker(
                state,
                payment_data.get_payment_attempt(),
                validate_result.requeue,
                schedule_time,
            )
            .await
            .map_err(|error| logger::error!(process_tracker_error=?error))
            .ok();
    }

    let updated_customer = None;
    let frm_suggestion = None;

    (_, *payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            state,
            req_state,
            payment_data.clone(),
            customer.clone(),
            platform.get_processor().get_account().storage_scheme,
            updated_customer,
            platform.get_processor().get_key_store(),
            frm_suggestion,
            header_payload.clone(),
        )
        .await?;

    let lineage_ids = grpc_client::LineageIds::new(
        business_profile.merchant_id.clone(),
        business_profile.get_id().clone(),
    );

    // TODO: determine execution path for proxy call connector service
    let execution_path = ExecutionPath::Direct;
    // Execution mode is irrelevant for Direct execution path
    let execution_mode = ExecutionMode::NotApplicable;

    let gateway_context = gateway_context::RouterGatewayContext {
        creds_identifier: payment_data.get_creds_identifier().map(|id| id.to_string()),
        platform: platform.clone(),
        header_payload: header_payload.clone(),
        lineage_ids,
        merchant_connector_account: merchant_connector_account.clone(),
        execution_path,
        execution_mode,
    };

    let router_data = if should_continue_further {
        // The status of payment_attempt and intent will be updated in the previous step
        // update this in router_data.
        // This is added because few connector integrations do not update the status,
        // and rely on previous status set in router_data
        router_data.status = payment_data.get_payment_attempt().status;
        router_data
            .decide_flows(
                state,
                &connector,
                call_connector_action,
                connector_request,
                business_profile,
                header_payload.clone(),
                return_raw_connector_response,
                gateway_context,
            )
            .await
    } else {
        Ok(router_data)
    }?;

    let etime_connector = Instant::now();
    let duration_connector = etime_connector.saturating_duration_since(stime_connector);
    tracing::info!(duration = format!("Duration taken: {}", duration_connector.as_millis()));

    Ok((router_data, merchant_connector_account))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn proxy_for_call_connector_service<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
    business_profile: &domain::Profile,
    return_raw_connector_response: Option<bool>,
) -> RouterResult<RouterData<F, RouterDReq, router_types::PaymentsResponseData>>
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
    let stime_connector = Instant::now();

    let merchant_connector_account =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
            helpers::get_merchant_connector_account_v2(
                state,
                platform.get_processor().get_key_store(),
                connector.merchant_connector_id.as_ref(),
            )
            .await?,
        ));
    operation
        .to_domain()?
        .populate_payment_data(state, payment_data, platform, business_profile, &connector)
        .await?;

    let mut router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            platform,
            &None,
            &merchant_connector_account,
            None,
            Some(header_payload.clone()),
        )
        .await?;

    let add_access_token_result = router_data
        .add_access_token(
            state,
            &connector,
            platform,
            payment_data.get_creds_identifier(),
        )
        .await?;

    router_data = router_data.add_session_token(state, &connector).await?;

    let mut should_continue_further = access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &call_connector_action,
    );

    let (connector_request, should_continue_further) = if should_continue_further {
        router_data
            .build_flow_specific_connector_request(state, &connector, call_connector_action.clone())
            .await?
    } else {
        (None, false)
    };

    (_, *payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            state,
            req_state,
            payment_data.clone(),
            None,
            platform.get_processor().get_account().storage_scheme,
            None,
            platform.get_processor().get_key_store(),
            None,
            header_payload.clone(),
        )
        .await?;
    let lineage_ids = grpc_client::LineageIds::new(
        business_profile.merchant_id.clone(),
        business_profile.get_id().clone(),
    );

    let gateway_context = gateway_context::RouterGatewayContext {
        creds_identifier: payment_data.get_creds_identifier().map(|id| id.to_string()),
        platform: platform.clone(),
        header_payload: header_payload.clone(),
        lineage_ids,
        merchant_connector_account: merchant_connector_account.clone(),
        execution_path: ExecutionPath::Direct,
        execution_mode: ExecutionMode::NotApplicable,
    };

    let router_data = if should_continue_further {
        router_data
            .decide_flows(
                state,
                &connector,
                call_connector_action,
                connector_request,
                business_profile,
                header_payload.clone(),
                return_raw_connector_response,
                gateway_context,
            )
            .await
    } else {
        Ok(router_data)
    }?;

    let etime_connector = Instant::now();
    let duration_connector = etime_connector.saturating_duration_since(stime_connector);
    tracing::info!(duration = format!("Duration taken: {}", duration_connector.as_millis()));

    Ok(router_data)
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn call_connector_service_for_external_vault_proxy<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
    business_profile: &domain::Profile,
    return_raw_connector_response: Option<bool>,
) -> RouterResult<RouterData<F, RouterDReq, router_types::PaymentsResponseData>>
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
    let stime_connector = Instant::now();

    let merchant_connector_account =
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
            helpers::get_merchant_connector_account_v2(
                state,
                platform.get_processor().get_key_store(),
                connector.merchant_connector_id.as_ref(),
            )
            .await?,
        ));
    operation
        .to_domain()?
        .populate_payment_data(state, payment_data, platform, business_profile, &connector)
        .await?;

    let mut router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            platform,
            &None,
            &merchant_connector_account,
            None,
            Some(header_payload.clone()),
        )
        .await?;

    // let add_access_token_result = router_data
    //     .add_access_token(
    //         state,
    //         &connector,
    //         platform,
    //         payment_data.get_creds_identifier(),
    //     )
    //     .await?;

    // router_data = router_data.add_session_token(state, &connector).await?;

    // let mut should_continue_further = access_token::update_router_data_with_access_token_result(
    //     &add_access_token_result,
    //     &mut router_data,
    //     &call_connector_action,
    // );
    let should_continue_further = true;

    let (connector_request, should_continue_further) = if should_continue_further {
        router_data
            .build_flow_specific_connector_request(state, &connector, call_connector_action.clone())
            .await?
    } else {
        (None, false)
    };

    (_, *payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            state,
            req_state,
            payment_data.clone(),
            None,
            platform.get_processor().get_account().storage_scheme,
            None,
            platform.get_processor().get_key_store(),
            None,
            header_payload.clone(),
        )
        .await?;
    let lineage_ids = grpc_client::LineageIds::new(
        business_profile.merchant_id.clone(),
        business_profile.get_id().clone(),
    );

    let gateway_context = gateway_context::RouterGatewayContext {
        creds_identifier: payment_data.get_creds_identifier().map(|id| id.to_string()),
        platform: platform.clone(),
        header_payload: header_payload.clone(),
        lineage_ids,
        merchant_connector_account: merchant_connector_account.clone(),
        execution_path: ExecutionPath::Direct,
        execution_mode: ExecutionMode::NotApplicable,
    };

    let router_data = if should_continue_further {
        router_data
            .decide_flows(
                state,
                &connector,
                call_connector_action,
                connector_request,
                business_profile,
                header_payload.clone(),
                return_raw_connector_response,
                gateway_context,
            )
            .await
    } else {
        Ok(router_data)
    }?;

    let etime_connector = Instant::now();
    let duration_connector = etime_connector.saturating_duration_since(stime_connector);
    tracing::info!(duration = format!("Duration taken: {}", duration_connector.as_millis()));

    Ok(router_data)
}
struct ApplePayWallet;
struct PazeWallet;
struct GooglePayWallet;

#[async_trait::async_trait]
pub trait WalletFlow<F, D>: Send + Sync
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    /// Check if wallet data is already decrypted and return token if so
    fn check_predecrypted_token(
        &self,
        _payment_data: &D,
    ) -> CustomResult<Option<PaymentMethodToken>, errors::ApiErrorResponse> {
        // Default implementation returns None (no pre-decrypted data)
        Ok(None)
    }

    fn decide_wallet_flow(
        &self,
        state: &SessionState,
        payment_data: &D,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> CustomResult<Option<DecideWalletFlow>, errors::ApiErrorResponse>;

    async fn decrypt_wallet_token(
        &self,
        wallet_flow: &DecideWalletFlow,
        payment_data: &D,
    ) -> CustomResult<PaymentMethodToken, errors::ApiErrorResponse>;
}

#[async_trait::async_trait]
impl<F, D> WalletFlow<F, D> for PazeWallet
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    fn decide_wallet_flow(
        &self,
        state: &SessionState,
        _payment_data: &D,
        _merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> CustomResult<Option<DecideWalletFlow>, errors::ApiErrorResponse> {
        let paze_keys = state
            .conf
            .paze_decrypt_keys
            .as_ref()
            .get_required_value("Paze decrypt keys")
            .attach_printable("Paze decrypt keys not found in the configuration")?;

        let wallet_flow = DecideWalletFlow::PazeDecrypt(PazePaymentProcessingDetails {
            paze_private_key: paze_keys.get_inner().paze_private_key.clone(),
            paze_private_key_passphrase: paze_keys.get_inner().paze_private_key_passphrase.clone(),
        });
        Ok(Some(wallet_flow))
    }

    async fn decrypt_wallet_token(
        &self,
        wallet_flow: &DecideWalletFlow,
        payment_data: &D,
    ) -> CustomResult<PaymentMethodToken, errors::ApiErrorResponse> {
        let paze_payment_processing_details = wallet_flow
            .get_paze_payment_processing_details()
            .get_required_value("Paze payment processing details")
            .attach_printable(
                "Paze payment processing details not found in Paze decryption flow",
            )?;

        let paze_wallet_data = payment_data
                .get_payment_method_data()
                .and_then(|payment_method_data| payment_method_data.get_wallet_data())
                .and_then(|wallet_data| wallet_data.get_paze_wallet_data())
                .get_required_value("Paze wallet token").attach_printable(
                    "Paze wallet data not found in the payment method data during the Paze decryption flow",
                )?;

        let paze_data = decrypt_paze_token(
            paze_wallet_data.clone(),
            paze_payment_processing_details.paze_private_key.clone(),
            paze_payment_processing_details
                .paze_private_key_passphrase
                .clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to decrypt paze token")?;

        let paze_decrypted_data = paze_data
            .parse_value::<hyperswitch_domain_models::router_data::PazeDecryptedData>(
                "PazeDecryptedData",
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to parse PazeDecryptedData")?;
        Ok(PaymentMethodToken::PazeDecrypt(Box::new(
            paze_decrypted_data,
        )))
    }
}

#[async_trait::async_trait]
impl<F, D> WalletFlow<F, D> for ApplePayWallet
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    fn check_predecrypted_token(
        &self,
        payment_data: &D,
    ) -> CustomResult<Option<PaymentMethodToken>, errors::ApiErrorResponse> {
        let apple_pay_wallet_data = payment_data
            .get_payment_method_data()
            .and_then(|payment_method_data| payment_method_data.get_wallet_data())
            .and_then(|wallet_data| wallet_data.get_apple_pay_wallet_data());

        let result = if let Some(data) = apple_pay_wallet_data {
            match &data.payment_data {
                common_payments_types::ApplePayPaymentData::Encrypted(_) => None,
                common_payments_types::ApplePayPaymentData::Decrypted(
                    apple_pay_predecrypt_data,
                ) => {
                    helpers::validate_card_expiry(
                        &apple_pay_predecrypt_data.application_expiration_month,
                        &apple_pay_predecrypt_data.application_expiration_year,
                    )?;
                    Some(PaymentMethodToken::ApplePayDecrypt(Box::new(
                        apple_pay_predecrypt_data.clone(),
                    )))
                }
            }
        } else {
            None
        };
        Ok(result)
    }

    fn decide_wallet_flow(
        &self,
        state: &SessionState,
        payment_data: &D,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> CustomResult<Option<DecideWalletFlow>, errors::ApiErrorResponse> {
        let apple_pay_metadata = check_apple_pay_metadata(state, Some(merchant_connector_account));

        add_apple_pay_flow_metrics(
            &apple_pay_metadata,
            payment_data.get_payment_attempt().connector.clone(),
            payment_data.get_payment_attempt().merchant_id.clone(),
        );

        let wallet_flow = match apple_pay_metadata {
            Some(domain::ApplePayFlow::Simplified(payment_processing_details)) => Some(
                DecideWalletFlow::ApplePayDecrypt(payment_processing_details),
            ),
            Some(domain::ApplePayFlow::Manual) | None => None,
        };
        Ok(wallet_flow)
    }

    async fn decrypt_wallet_token(
        &self,
        wallet_flow: &DecideWalletFlow,
        payment_data: &D,
    ) -> CustomResult<PaymentMethodToken, errors::ApiErrorResponse> {
        let apple_pay_payment_processing_details = wallet_flow
            .get_apple_pay_payment_processing_details()
            .get_required_value("Apple Pay payment processing details")
            .attach_printable(
                "Apple Pay payment processing details not found in Apple Pay decryption flow",
            )?;
        let apple_pay_wallet_data = payment_data
                .get_payment_method_data()
                .and_then(|payment_method_data| payment_method_data.get_wallet_data())
                .and_then(|wallet_data| wallet_data.get_apple_pay_wallet_data())
                .get_required_value("Apple Pay wallet token").attach_printable(
                    "Apple Pay wallet data not found in the payment method data during the Apple Pay decryption flow",
                )?;

        let apple_pay_data =
            ApplePayData::token_json(domain::WalletData::ApplePay(apple_pay_wallet_data.clone()))
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to parse apple pay token to json")?
                .decrypt(
                    &apple_pay_payment_processing_details.payment_processing_certificate,
                    &apple_pay_payment_processing_details.payment_processing_certificate_key,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to decrypt apple pay token")?;

        let apple_pay_predecrypt_internal = apple_pay_data
            .parse_value::<hyperswitch_domain_models::router_data::ApplePayPredecryptDataInternal>(
                "ApplePayPredecryptDataInternal",
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "failed to parse decrypted apple pay response to ApplePayPredecryptData",
            )?;

        let apple_pay_predecrypt =
            common_types::payments::ApplePayPredecryptData::try_from(apple_pay_predecrypt_internal)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "failed to convert ApplePayPredecryptDataInternal to ApplePayPredecryptData",
                )?;

        Ok(PaymentMethodToken::ApplePayDecrypt(Box::new(
            apple_pay_predecrypt,
        )))
    }
}

#[async_trait::async_trait]
impl<F, D> WalletFlow<F, D> for GooglePayWallet
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    fn check_predecrypted_token(
        &self,
        payment_data: &D,
    ) -> CustomResult<Option<PaymentMethodToken>, errors::ApiErrorResponse> {
        let google_pay_wallet_data = payment_data
            .get_payment_method_data()
            .and_then(|payment_method_data| payment_method_data.get_wallet_data())
            .and_then(|wallet_data| wallet_data.get_google_pay_wallet_data());

        let result = if let Some(data) = google_pay_wallet_data {
            match &data.tokenization_data {
                common_payments_types::GpayTokenizationData::Encrypted(_) => None,
                common_payments_types::GpayTokenizationData::Decrypted(
                    google_pay_predecrypt_data,
                ) => {
                    helpers::validate_card_expiry(
                        &google_pay_predecrypt_data.card_exp_month,
                        &google_pay_predecrypt_data.card_exp_year,
                    )?;
                    Some(PaymentMethodToken::GooglePayDecrypt(Box::new(
                        google_pay_predecrypt_data.clone(),
                    )))
                }
            }
        } else {
            None
        };
        Ok(result)
    }
    fn decide_wallet_flow(
        &self,
        state: &SessionState,
        _payment_data: &D,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> CustomResult<Option<DecideWalletFlow>, errors::ApiErrorResponse> {
        Ok(
            get_google_pay_connector_wallet_details(state, merchant_connector_account)
                .map(DecideWalletFlow::GooglePayDecrypt),
        )
    }

    async fn decrypt_wallet_token(
        &self,
        wallet_flow: &DecideWalletFlow,
        payment_data: &D,
    ) -> CustomResult<PaymentMethodToken, errors::ApiErrorResponse> {
        let google_pay_payment_processing_details = wallet_flow
            .get_google_pay_payment_processing_details()
            .get_required_value("Google Pay payment processing details")
            .attach_printable(
                "Google Pay payment processing details not found in Google Pay decryption flow",
            )?;

        let google_pay_wallet_data = payment_data
                .get_payment_method_data()
                .and_then(|payment_method_data| payment_method_data.get_wallet_data())
                .and_then(|wallet_data| wallet_data.get_google_pay_wallet_data())
                .get_required_value("Paze wallet token").attach_printable(
                    "Google Pay wallet data not found in the payment method data during the Google Pay decryption flow",
                )?;

        let decryptor = helpers::GooglePayTokenDecryptor::new(
            google_pay_payment_processing_details
                .google_pay_root_signing_keys
                .clone(),
            google_pay_payment_processing_details
                .google_pay_recipient_id
                .clone(),
            google_pay_payment_processing_details
                .google_pay_private_key
                .clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to create google pay token decryptor")?;

        // should_verify_token is set to false to disable verification of token
        let google_pay_data_internal = decryptor
            .decrypt_token(
                google_pay_wallet_data
                    .tokenization_data
                    .get_encrypted_google_pay_token()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?
                    .clone(),
                false,
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to decrypt google pay token")?;
        let google_pay_data =
            common_types::payments::GPayPredecryptData::from(google_pay_data_internal);
        Ok(PaymentMethodToken::GooglePayDecrypt(Box::new(
            google_pay_data,
        )))
    }
}

#[derive(Debug, Clone)]
pub enum DecideWalletFlow {
    ApplePayDecrypt(payments_api::PaymentProcessingDetails),
    PazeDecrypt(PazePaymentProcessingDetails),
    GooglePayDecrypt(GooglePayPaymentProcessingDetails),
    SkipDecryption,
}

impl DecideWalletFlow {
    fn get_paze_payment_processing_details(&self) -> Option<&PazePaymentProcessingDetails> {
        if let Self::PazeDecrypt(details) = self {
            Some(details)
        } else {
            None
        }
    }

    fn get_apple_pay_payment_processing_details(
        &self,
    ) -> Option<&payments_api::PaymentProcessingDetails> {
        if let Self::ApplePayDecrypt(details) = self {
            Some(details)
        } else {
            None
        }
    }

    fn get_google_pay_payment_processing_details(
        &self,
    ) -> Option<&GooglePayPaymentProcessingDetails> {
        if let Self::GooglePayDecrypt(details) = self {
            Some(details)
        } else {
            None
        }
    }
}

pub async fn get_merchant_bank_data_for_open_banking_connectors(
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    platform: &domain::Platform,
    connector: &api::ConnectorData,
    state: &SessionState,
) -> RouterResult<Option<router_types::MerchantRecipientData>> {
    let merchant_data = merchant_connector_account
        .get_additional_merchant_data()
        .get_required_value("additional_merchant_data")?
        .into_inner()
        .peek()
        .clone();

    let merchant_recipient_data = merchant_data
        .parse_value::<router_types::AdditionalMerchantData>("AdditionalMerchantData")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("failed to decode MerchantRecipientData")?;

    let connector_name = enums::Connector::to_string(&connector.connector_name);
    let locker_based_connector_list = state.conf.locker_based_open_banking_connectors.clone();
    let contains = locker_based_connector_list
        .connector_list
        .contains(connector_name.as_str());

    let recipient_id = helpers::get_recipient_id_for_open_banking(&merchant_recipient_data)?;
    let final_recipient_data = if let Some(id) = recipient_id {
        if contains {
            // Customer Id for OpenBanking connectors will be merchant_id as the account data stored at locker belongs to the merchant
            let merchant_id_str = platform
                .get_processor()
                .get_account()
                .get_id()
                .get_string_repr()
                .to_owned();
            let cust_id = id_type::CustomerId::try_from(std::borrow::Cow::from(merchant_id_str))
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to convert to CustomerId")?;
            let locker_resp = cards::get_payment_method_from_hs_locker(
                state,
                platform.get_processor().get_key_store(),
                &cust_id,
                platform.get_processor().get_account().get_id(),
                id.as_str(),
                Some(enums::LockerChoice::HyperswitchCardVault),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Merchant bank account data could not be fetched from locker")?;

            let parsed: router_types::MerchantAccountData = locker_resp
                .peek()
                .to_string()
                .parse_struct("MerchantAccountData")
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

            Some(router_types::MerchantRecipientData::AccountData(parsed))
        } else {
            Some(router_types::MerchantRecipientData::ConnectorRecipientId(
                Secret::new(id),
            ))
        }
    } else {
        None
    };
    Ok(final_recipient_data)
}

async fn blocklist_guard<F, ApiRequest, D>(
    state: &SessionState,
    platform: &domain::Platform,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
) -> CustomResult<bool, errors::ApiErrorResponse>
where
    F: Send + Clone + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let merchant_id = platform.get_processor().get_account().get_id();
    let blocklist_enabled_key = merchant_id.get_blocklist_guard_key();
    let blocklist_guard_enabled = state
        .store
        .find_config_by_key_unwrap_or(&blocklist_enabled_key, Some("false".to_string()))
        .await;

    let blocklist_guard_enabled: bool = match blocklist_guard_enabled {
        Ok(config) => serde_json::from_str(&config.config).unwrap_or(false),

        // If it is not present in db we are defaulting it to false
        Err(inner) => {
            if !inner.current_context().is_db_not_found() {
                logger::error!("Error fetching guard blocklist enabled config {:?}", inner);
            }
            false
        }
    };

    if blocklist_guard_enabled {
        Ok(operation
            .to_domain()?
            .guard_payment_against_blocklist(state, platform, payment_data)
            .await?)
    } else {
        Ok(false)
    }
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn call_multiple_connectors_service<F, Op, Req, D>(
    state: &SessionState,
    platform: &domain::Platform,
    connectors: api::SessionConnectorDatas,
    _operation: &Op,
    mut payment_data: D,
    customer: &Option<domain::Customer>,
    _session_surcharge_details: Option<api::SessionSurchargeDetails>,
    business_profile: &domain::Profile,
    header_payload: HeaderPayload,
    return_raw_connector_response: Option<bool>,
) -> RouterResult<D>
where
    Op: Debug,
    F: Send + Clone + Sync,

    // To create connector flow specific interface data
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, Req, router_types::PaymentsResponseData>,
    RouterData<F, Req, router_types::PaymentsResponseData>: Feature<F, Req>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, Req, router_types::PaymentsResponseData>,
{
    let call_connectors_start_time = Instant::now();
    let mut join_handlers = Vec::with_capacity(connectors.len());
    for session_connector_data in connectors.iter() {
        let merchant_connector_account =
            domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
                helpers::get_merchant_connector_account_v2(
                    state,
                    platform.get_processor().get_key_store(),
                    session_connector_data
                        .connector
                        .merchant_connector_id
                        .as_ref(),
                )
                .await?,
            ));

        let connector_id = session_connector_data.connector.connector.id();
        let router_data = payment_data
            .construct_router_data(
                state,
                connector_id,
                platform,
                customer,
                &merchant_connector_account,
                None,
                Some(header_payload.clone()),
            )
            .await?;
        let lineage_ids = grpc_client::LineageIds::new(
            business_profile.merchant_id.clone(),
            business_profile.get_id().clone(),
        );
        let gateway_context = gateway_context::RouterGatewayContext {
            creds_identifier: payment_data.get_creds_identifier().map(|id| id.to_string()),
            platform: platform.clone(),
            header_payload: header_payload.clone(),
            lineage_ids,
            merchant_connector_account: merchant_connector_account.clone(),
            execution_path: ExecutionPath::Direct,
            execution_mode: ExecutionMode::NotApplicable,
        };

        let res = router_data.decide_flows(
            state,
            &session_connector_data.connector,
            CallConnectorAction::Trigger,
            None,
            business_profile,
            header_payload.clone(),
            return_raw_connector_response,
            gateway_context,
        );

        join_handlers.push(res);
    }

    let result = join_all(join_handlers).await;

    for (connector_res, session_connector) in result.into_iter().zip(connectors) {
        let connector_name = session_connector.connector.connector_name.to_string();
        match connector_res {
            Ok(connector_response) => {
                if let Ok(router_types::PaymentsResponseData::SessionResponse {
                    session_token,
                    ..
                }) = connector_response.response.clone()
                {
                    // If session token is NoSessionTokenReceived, it is not pushed into the sessions_token as there is no response or there can be some error
                    // In case of error, that error is already logged
                    if !matches!(
                        session_token,
                        api_models::payments::SessionToken::NoSessionTokenReceived,
                    ) {
                        payment_data.push_sessions_token(session_token);
                    }
                }
                if let Err(connector_error_response) = connector_response.response {
                    logger::error!(
                        "sessions_connector_error {} {:?}",
                        connector_name,
                        connector_error_response
                    );
                }
            }
            Err(api_error) => {
                logger::error!("sessions_api_error {} {:?}", connector_name, api_error);
            }
        }
    }

    let call_connectors_end_time = Instant::now();
    let call_connectors_duration =
        call_connectors_end_time.saturating_duration_since(call_connectors_start_time);
    tracing::info!(duration = format!("Duration taken: {}", call_connectors_duration.as_millis()));

    Ok(payment_data)
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn call_multiple_connectors_service<F, Op, Req, D>(
    state: &SessionState,
    platform: &domain::Platform,
    connectors: api::SessionConnectorDatas,
    _operation: &Op,
    mut payment_data: D,
    customer: &Option<domain::Customer>,
    session_surcharge_details: Option<api::SessionSurchargeDetails>,
    business_profile: &domain::Profile,
    header_payload: HeaderPayload,
    return_raw_connector_response: Option<bool>,
) -> RouterResult<D>
where
    Op: Debug,
    F: Send + Clone,

    // To create connector flow specific interface data
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    D: ConstructFlowSpecificData<F, Req, router_types::PaymentsResponseData>,
    RouterData<F, Req, router_types::PaymentsResponseData>: Feature<F, Req>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, Req, router_types::PaymentsResponseData>,
{
    let call_connectors_start_time = Instant::now();
    let mut join_handlers = Vec::with_capacity(connectors.len());
    for session_connector_data in connectors.iter() {
        let connector_id = session_connector_data.connector.connector.id();

        let merchant_connector_account = construct_profile_id_and_get_mca(
            state,
            platform,
            &payment_data,
            &session_connector_data.connector.connector_name.to_string(),
            session_connector_data
                .connector
                .merchant_connector_id
                .as_ref(),
            false,
        )
        .await?;

        payment_data.set_surcharge_details(session_surcharge_details.as_ref().and_then(
            |session_surcharge_details| {
                session_surcharge_details.fetch_surcharge_details(
                    session_connector_data.payment_method_sub_type.into(),
                    session_connector_data.payment_method_sub_type,
                    None,
                )
            },
        ));

        let router_data = payment_data
            .construct_router_data(
                state,
                connector_id,
                platform,
                customer,
                &merchant_connector_account,
                None,
                Some(header_payload.clone()),
                Some(session_connector_data.payment_method_type),
                Some(session_connector_data.payment_method_sub_type),
            )
            .await?;

        let lineage_ids = grpc_client::LineageIds::new(
            business_profile.merchant_id.clone(),
            business_profile.get_id().clone(),
        );

        // TODO: determine execution path for SDK session token call.
        let execution_path = ExecutionPath::Direct;
        let execution_mode = ExecutionMode::NotApplicable;

        let gateway_context = gateway_context::RouterGatewayContext {
            creds_identifier: payment_data.get_creds_identifier().map(|id| id.to_string()),
            platform: platform.clone(),
            header_payload: header_payload.clone(),
            lineage_ids,
            merchant_connector_account: merchant_connector_account.clone(),
            execution_path,
            execution_mode,
        };

        let res = router_data.decide_flows(
            state,
            &session_connector_data.connector,
            CallConnectorAction::Trigger,
            None,
            business_profile,
            header_payload.clone(),
            return_raw_connector_response,
            gateway_context,
        );

        join_handlers.push(res);
    }

    let result = join_all(join_handlers).await;

    for (connector_res, session_connector) in result.into_iter().zip(connectors) {
        let connector_name = session_connector.connector.connector_name.to_string();
        match connector_res {
            Ok(connector_response) => {
                if let Ok(router_types::PaymentsResponseData::SessionResponse {
                    session_token,
                    ..
                }) = connector_response.response.clone()
                {
                    // If session token is NoSessionTokenReceived, it is not pushed into the sessions_token as there is no response or there can be some error
                    // In case of error, that error is already logged
                    if !matches!(
                        session_token,
                        api_models::payments::SessionToken::NoSessionTokenReceived,
                    ) {
                        payment_data.push_sessions_token(session_token);
                    }
                }
                if let Err(connector_error_response) = connector_response.response {
                    logger::error!(
                        "sessions_connector_error {} {:?}",
                        connector_name,
                        connector_error_response
                    );
                }
            }
            Err(api_error) => {
                logger::error!("sessions_api_error {} {:?}", connector_name, api_error);
            }
        }
    }

    // If click_to_pay is enabled and authentication_product_ids is configured in profile, we need to attach click_to_pay block in the session response for invoking click_to_pay SDK
    if business_profile.is_click_to_pay_enabled {
        if let Some(value) = business_profile.authentication_product_ids.clone() {
            let session_token = get_session_token_for_click_to_pay(
                state,
                platform.get_processor().get_account().get_id(),
                platform,
                value,
                payment_data.get_payment_intent(),
                business_profile.get_id(),
            )
            .await?;
            payment_data.push_sessions_token(session_token);
        }
    }

    let call_connectors_end_time = Instant::now();
    let call_connectors_duration =
        call_connectors_end_time.saturating_duration_since(call_connectors_start_time);
    tracing::info!(duration = format!("Duration taken: {}", call_connectors_duration.as_millis()));

    Ok(payment_data)
}

#[cfg(feature = "v1")]
pub async fn get_session_token_for_click_to_pay(
    state: &SessionState,
    merchant_id: &id_type::MerchantId,
    platform: &domain::Platform,
    authentication_product_ids: common_types::payments::AuthenticationConnectorAccountMap,
    payment_intent: &payments::PaymentIntent,
    profile_id: &id_type::ProfileId,
) -> RouterResult<api_models::payments::SessionToken> {
    let click_to_pay_mca_id = authentication_product_ids
        .get_click_to_pay_connector_account_id()
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "authentication_product_ids",
        })?;
    let merchant_connector_account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            &click_to_pay_mca_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: click_to_pay_mca_id.get_string_repr().to_string(),
        })?;
    let click_to_pay_metadata: ClickToPayMetaData = merchant_connector_account
        .metadata
        .parse_value("ClickToPayMetaData")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing ClickToPayMetaData")?;
    let transaction_currency = payment_intent
        .currency
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("currency is not present in payment_data.payment_intent")?;
    let required_amount_type = common_utils::types::StringMajorUnitForConnector;
    let transaction_amount = required_amount_type
        .convert(payment_intent.amount, transaction_currency)
        .change_context(errors::ApiErrorResponse::AmountConversionFailed {
            amount_type: "string major unit",
        })?;

    let customer_details_value = payment_intent
        .customer_details
        .clone()
        .get_required_value("customer_details")?;

    let customer_details: CustomerData = customer_details_value
        .parse_value("CustomerData")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while parsing customer data from payment intent")?;

    validate_customer_details_for_click_to_pay(&customer_details)?;

    let provider = match merchant_connector_account.connector_name.as_str() {
        "ctp_mastercard" => Some(enums::CtpServiceProvider::Mastercard),
        "ctp_visa" => Some(enums::CtpServiceProvider::Visa),
        _ => None,
    };

    let card_brands = get_card_brands_based_on_active_merchant_connector_account(
        state,
        profile_id,
        platform.get_processor().get_key_store(),
    )
    .await?;

    Ok(api_models::payments::SessionToken::ClickToPay(Box::new(
        api_models::payments::ClickToPaySessionResponse {
            dpa_id: click_to_pay_metadata.dpa_id,
            dpa_name: click_to_pay_metadata.dpa_name,
            locale: click_to_pay_metadata.locale,
            card_brands,
            acquirer_bin: click_to_pay_metadata.acquirer_bin,
            acquirer_merchant_id: click_to_pay_metadata.acquirer_merchant_id,
            merchant_category_code: click_to_pay_metadata.merchant_category_code,
            merchant_country_code: click_to_pay_metadata.merchant_country_code,
            transaction_amount,
            transaction_currency_code: transaction_currency,
            phone_number: customer_details.phone.clone(),
            email: customer_details.email.clone(),
            phone_country_code: customer_details.phone_country_code.clone(),
            provider,
            dpa_client_id: click_to_pay_metadata.dpa_client_id.clone(),
        },
    )))
}

#[cfg(feature = "v1")]
async fn get_card_brands_based_on_active_merchant_connector_account(
    state: &SessionState,
    profile_id: &id_type::ProfileId,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<HashSet<enums::CardNetwork>> {
    let merchant_configured_payment_connectors = state
        .store
        .list_enabled_connector_accounts_by_profile_id(
            profile_id,
            key_store,
            common_enums::ConnectorType::PaymentProcessor,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error when fetching merchant connector accounts")?;

    let payment_connectors_eligible_for_click_to_pay =
        state.conf.authentication_providers.click_to_pay.clone();

    let filtered_payment_connector_accounts: Vec<
        hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    > = merchant_configured_payment_connectors
        .into_iter()
        .filter(|account| {
            enums::Connector::from_str(&account.connector_name)
                .ok()
                .map(|connector| payment_connectors_eligible_for_click_to_pay.contains(&connector))
                .unwrap_or(false)
        })
        .collect();

    let mut card_brands = HashSet::new();

    for account in filtered_payment_connector_accounts {
        if let Some(values) = &account.payment_methods_enabled {
            for val in values {
                let payment_methods_enabled: api_models::admin::PaymentMethodsEnabled =
                    serde_json::from_value(val.peek().to_owned()).inspect_err(|err| {
                        logger::error!("Failed to parse Payment methods enabled data set from dashboard because {}", err)
                    })
                    .change_context(errors::ApiErrorResponse::InternalServerError)?;
                if let Some(payment_method_types) = payment_methods_enabled.payment_method_types {
                    for payment_method_type in payment_method_types {
                        if let Some(networks) = payment_method_type.card_networks {
                            card_brands.extend(networks);
                        }
                    }
                }
            }
        }
    }
    Ok(card_brands)
}

pub fn validate_customer_details_for_click_to_pay(
    customer_details: &CustomerData,
) -> RouterResult<()> {
    match (
        customer_details.phone.as_ref(),
        customer_details.phone_country_code.as_ref(),
        customer_details.email.as_ref()
    ) {
        (None, None, Some(_)) => Ok(()),
        (Some(_), Some(_), Some(_)) => Ok(()),
        (Some(_), Some(_), None) => Ok(()),
        (Some(_), None, Some(_)) => Ok(()),
        (None, Some(_), None) => Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "phone",
        })
        .attach_printable("phone number is not present in payment_intent.customer_details"),
        (Some(_), None, None) => Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "phone_country_code",
        })
        .attach_printable("phone_country_code is not present in payment_intent.customer_details"),
        (_, _, _) => Err(errors::ApiErrorResponse::MissingRequiredFields {
            field_names: vec!["phone", "phone_country_code", "email"],
        })
        .attach_printable("either of phone, phone_country_code or email is not present in payment_intent.customer_details"),
    }
}

#[cfg(feature = "v1")]
pub async fn call_create_connector_customer_if_required<F, Req, D>(
    state: &SessionState,
    customer: &Option<domain::Customer>,
    platform: &domain::Platform,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    payment_data: &mut D,
    access_token: Option<&AccessToken>,
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
    let connector_name = payment_data.get_payment_attempt().connector.clone();

    match connector_name {
        Some(connector_name) => {
            let connector = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &connector_name,
                api::GetToken::Connector,
                merchant_connector_account.get_mca_id(),
            )?;

            let label = {
                let connector_label = core_utils::get_connector_label(
                    payment_data.get_payment_intent().business_country,
                    payment_data.get_payment_intent().business_label.as_ref(),
                    payment_data
                        .get_payment_attempt()
                        .business_sub_label
                        .as_ref(),
                    &connector_name,
                );

                if let Some(connector_label) = merchant_connector_account
                    .get_mca_id()
                    .map(|mca_id| mca_id.get_string_repr().to_string())
                    .or(connector_label)
                {
                    connector_label
                } else {
                    let profile_id = payment_data
                        .get_payment_intent()
                        .profile_id
                        .as_ref()
                        .get_required_value("profile_id")
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("profile_id is not set in payment_intent")?;

                    format!("{connector_name}_{}", profile_id.get_string_repr())
                }
            };

            let (should_call_connector, existing_connector_customer_id) =
                customers::should_call_connector_create_customer(
                    &connector,
                    customer,
                    payment_data.get_payment_attempt(),
                    &label,
                );

            if should_call_connector {
                // Create customer at connector and update the customer table to store this data
                let mut customer_router_data = payment_data
                    .construct_router_data(
                        state,
                        connector.connector.id(),
                        platform,
                        customer,
                        merchant_connector_account,
                        None,
                        None,
                        payment_data.get_payment_attempt().payment_method,
                        payment_data.get_payment_attempt().payment_method_type,
                    )
                    .await?;

                customer_router_data.access_token = access_token.cloned();

                let connector_customer_id = customer_router_data
                    .create_connector_customer(state, &connector)
                    .await?;

                let customer_update = customers::update_connector_customer_in_customers(
                    &label,
                    customer.as_ref(),
                    connector_customer_id.clone(),
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
        None => Ok(None),
    }
}

#[cfg(feature = "v2")]
pub async fn call_create_connector_customer_if_required<F, Req, D>(
    state: &SessionState,
    customer: &Option<domain::Customer>,
    platform: &domain::Platform,
    merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
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
    let connector_name = payment_data.get_payment_attempt().connector.clone();

    match connector_name {
        Some(connector_name) => {
            let connector = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &connector_name,
                api::GetToken::Connector,
                merchant_connector_account.get_id(),
            )?;

            let (should_call_connector, existing_connector_customer_id) =
                customers::should_call_connector_create_customer(
                    &connector,
                    customer,
                    payment_data.get_payment_attempt(),
                    merchant_connector_account,
                );

            if should_call_connector {
                // Create customer at connector and update the customer table to store this data
                let router_data = payment_data
                    .construct_router_data(
                        state,
                        connector.connector.id(),
                        platform,
                        customer,
                        merchant_connector_account,
                        None,
                        None,
                    )
                    .await?;

                let connector_customer_id = router_data
                    .create_connector_customer(state, &connector)
                    .await?;

                let customer_update = customers::update_connector_customer_in_customers(
                    merchant_connector_account,
                    customer.as_ref(),
                    connector_customer_id.clone(),
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
        None => Ok(None),
    }
}

async fn complete_preprocessing_steps_if_required<F, Req, Q, D>(
    state: &SessionState,
    connector: &api::ConnectorData,
    payment_data: &D,
    mut router_data: RouterData<F, Req, router_types::PaymentsResponseData>,
    operation: &BoxedOperation<'_, F, Q, D>,
    should_continue_payment: bool,
) -> RouterResult<(RouterData<F, Req, router_types::PaymentsResponseData>, bool)>
where
    F: Send + Clone + Sync,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
    Req: Send + Sync,
    RouterData<F, Req, router_types::PaymentsResponseData>: Feature<F, Req> + Send,
    dyn api::Connector:
        services::api::ConnectorIntegration<F, Req, router_types::PaymentsResponseData>,
{
    //TODO: For ACH transfers, if preprocessing_step is not required for connectors encountered in future, add the check
    let router_data_and_should_continue_payment = match payment_data.get_payment_method_data() {
        Some(domain::PaymentMethodData::BankTransfer(_)) => (router_data, should_continue_payment),
        Some(domain::PaymentMethodData::Wallet(_)) => {
            if is_preprocessing_required_for_wallets(connector.connector_name.to_string()) {
                (
                    router_data.preprocessing_steps(state, connector).await?,
                    false,
                )
            } else if connector.connector_name == router_types::Connector::Paysafe {
                match payment_data.get_payment_method_data() {
                    Some(domain::PaymentMethodData::Wallet(domain::WalletData::ApplePay(_))) => {
                        router_data = router_data.preprocessing_steps(state, connector).await?;
                        let is_error_in_response = router_data.response.is_err();
                        (router_data, !is_error_in_response)
                    }
                    _ => (router_data, should_continue_payment),
                }
            } else {
                (router_data, should_continue_payment)
            }
        }
        Some(domain::PaymentMethodData::Card(_)) => {
            if connector.connector_name == router_types::Connector::Payme
                && !matches!(format!("{operation:?}").as_str(), "CompleteAuthorize")
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                let is_error_in_response = router_data.response.is_err();
                // If is_error_in_response is true, should_continue_payment should be false, we should throw the error
                (router_data, !is_error_in_response)
            } else if connector.connector_name == router_types::Connector::Nmi
                && !matches!(format!("{operation:?}").as_str(), "CompleteAuthorize")
                && router_data.auth_type == storage_enums::AuthenticationType::ThreeDs
                && !matches!(
                    payment_data
                        .get_payment_attempt()
                        .external_three_ds_authentication_attempted,
                    Some(true)
                )
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                (router_data, false)
            } else if connector.connector_name == router_types::Connector::Paysafe
                && router_data.auth_type == storage_enums::AuthenticationType::NoThreeDs
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                let is_error_in_response = router_data.response.is_err();
                // If is_error_in_response is true, should_continue_payment should be false, we should throw the error
                (router_data, !is_error_in_response)
            } else if (connector.connector_name == router_types::Connector::Cybersource
                || connector.connector_name == router_types::Connector::Barclaycard)
                && is_operation_complete_authorize(&operation)
                && router_data.auth_type == storage_enums::AuthenticationType::ThreeDs
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                // Should continue the flow only if no redirection_data is returned else a response with redirection form shall be returned
                let should_continue = matches!(
                    router_data.response,
                    Ok(router_types::PaymentsResponseData::TransactionResponse {
                        ref redirection_data,
                        ..
                    }) if redirection_data.is_none()
                ) && router_data.status
                    != common_enums::AttemptStatus::AuthenticationFailed;
                (router_data, should_continue)
            } else if router_data.auth_type == common_enums::AuthenticationType::ThreeDs
                && (((connector.connector_name == router_types::Connector::Nuvei && {
                    #[cfg(feature = "v1")]
                    {
                        payment_data
                            .get_payment_intent()
                            .request_external_three_ds_authentication
                            != Some(true)
                    }
                    #[cfg(feature = "v2")]
                    {
                        payment_data
                            .get_payment_intent()
                            .request_external_three_ds_authentication
                            != Some(true).into()
                    }
                }) || connector.connector_name == router_types::Connector::Shift4)
                    && !is_operation_complete_authorize(&operation))
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;
                (router_data, should_continue_payment)
            } else if connector.connector_name == router_types::Connector::Xendit
                && is_operation_confirm(&operation)
            {
                match payment_data.get_payment_intent().split_payments {
                    Some(common_types::payments::SplitPaymentsRequest::XenditSplitPayment(
                        common_types::payments::XenditSplitRequest::MultipleSplits(_),
                    )) => {
                        router_data = router_data.preprocessing_steps(state, connector).await?;
                        let is_error_in_response = router_data.response.is_err();
                        (router_data, !is_error_in_response)
                    }
                    _ => (router_data, should_continue_payment),
                }
            } else if connector.connector_name == router_types::Connector::Redsys
                && router_data.auth_type == common_enums::AuthenticationType::ThreeDs
                && is_operation_confirm(&operation)
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;
                let should_continue = match router_data.response {
                    Ok(router_types::PaymentsResponseData::TransactionResponse {
                        ref connector_metadata,
                        ..
                    }) => {
                        let three_ds_invoke_data: Option<
                            api_models::payments::PaymentsConnectorThreeDsInvokeData,
                        > = connector_metadata.clone().and_then(|metadata| {
                            metadata
                                .parse_value("PaymentsConnectorThreeDsInvokeData")
                                .ok() // "ThreeDsInvokeData was not found; proceeding with the payment flow without triggering the ThreeDS invoke action"
                        });
                        three_ds_invoke_data.is_none()
                    }
                    _ => false,
                };
                (router_data, should_continue)
            } else if router_data.auth_type == common_enums::AuthenticationType::ThreeDs
                && connector.connector_name == router_types::Connector::Nexixpay
                && is_operation_complete_authorize(&operation)
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;
                let is_error_in_response = router_data.response.is_err();
                (router_data, !is_error_in_response)
            } else {
                (router_data, should_continue_payment)
            }
        }
        Some(domain::PaymentMethodData::GiftCard(gift_card_data)) => {
            if connector.connector_name == router_types::Connector::Adyen
                && matches!(gift_card_data.deref(), domain::GiftCardData::Givex(..))
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                let is_error_in_response = router_data.response.is_err();
                // If is_error_in_response is true, should_continue_payment should be false, we should throw the error
                (router_data, !is_error_in_response)
            } else {
                (router_data, should_continue_payment)
            }
        }
        Some(domain::PaymentMethodData::BankDebit(_)) => {
            if connector.connector_name == router_types::Connector::Gocardless
                || connector.connector_name == router_types::Connector::Nordea
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;
                let is_error_in_response = router_data.response.is_err();
                // If is_error_in_response is true, should_continue_payment should be false, we should throw the error
                (router_data, !is_error_in_response)
            } else {
                (router_data, should_continue_payment)
            }
        }
        _ => {
            // 3DS validation for paypal cards after verification (authorize call)
            if connector.connector_name == router_types::Connector::Paypal
                && payment_data.get_payment_attempt().get_payment_method()
                    == Some(storage_enums::PaymentMethod::Card)
                && matches!(format!("{operation:?}").as_str(), "CompleteAuthorize")
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;
                let is_error_in_response = router_data.response.is_err();
                // If is_error_in_response is true, should_continue_payment should be false, we should throw the error
                (router_data, !is_error_in_response)
            } else {
                (router_data, should_continue_payment)
            }
        }
    };

    Ok(router_data_and_should_continue_payment)
}

#[cfg(feature = "v1")]
async fn complete_confirmation_for_click_to_pay_if_required<F, D>(
    state: &SessionState,
    platform: &domain::Platform,
    payment_data: &D,
) -> RouterResult<()>
where
    F: Send + Clone + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let payment_attempt = payment_data.get_payment_attempt();
    let payment_intent = payment_data.get_payment_intent();
    let service_details = payment_data.get_click_to_pay_service_details();
    let authentication = payment_data.get_authentication();

    let should_do_uas_confirmation_call = service_details
        .as_ref()
        .map(|details| details.is_network_confirmation_call_required())
        .unwrap_or(false);
    if should_do_uas_confirmation_call
        && (payment_intent.status == storage_enums::IntentStatus::Succeeded
            || payment_intent.status == storage_enums::IntentStatus::Failed)
    {
        let authentication_connector_id = authentication
            .as_ref()
            .and_then(|auth| auth.authentication.merchant_connector_id.clone())
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to get authentication connector id from authentication table",
            )?;
        let key_store = platform.get_processor().get_key_store();
        let merchant_id = platform.get_processor().get_account().get_id();

        let connector_mca = state
            .store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                merchant_id,
                &authentication_connector_id,
                key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: authentication_connector_id.get_string_repr().to_string(),
            })?;

        let payment_method = payment_attempt
            .payment_method
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get payment method from payment attempt")?;

        ClickToPay::confirmation(
            state,
            payment_attempt.authentication_id.as_ref(),
            payment_intent.currency,
            payment_attempt.status,
            service_details.cloned(),
            &helpers::MerchantConnectorAccountType::DbVal(Box::new(connector_mca.clone())),
            &connector_mca.connector_name,
            payment_method,
            payment_attempt.net_amount.get_order_amount(),
            Some(&payment_intent.payment_id),
            merchant_id,
        )
        .await?;
        Ok(())
    } else {
        Ok(())
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
async fn complete_postprocessing_steps_if_required<F, Q, RouterDReq, D>(
    state: &SessionState,
    platform: &domain::Platform,
    customer: &Option<domain::Customer>,
    merchant_conn_account: &helpers::MerchantConnectorAccountType,
    connector: &api::ConnectorData,
    payment_data: &mut D,
    _operation: &BoxedOperation<'_, F, Q, D>,
    header_payload: Option<HeaderPayload>,
) -> RouterResult<RouterData<F, RouterDReq, router_types::PaymentsResponseData>>
where
    F: Send + Clone + Sync,
    RouterDReq: Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,

    RouterData<F, RouterDReq, router_types::PaymentsResponseData>: Feature<F, RouterDReq> + Send,
    dyn api::Connector:
        services::api::ConnectorIntegration<F, RouterDReq, router_types::PaymentsResponseData>,
    D: ConstructFlowSpecificData<F, RouterDReq, router_types::PaymentsResponseData>,
{
    let mut router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            platform,
            customer,
            merchant_conn_account,
            None,
            header_payload,
            payment_data.get_payment_attempt().payment_method,
            payment_data.get_payment_attempt().payment_method_type,
        )
        .await?;

    match payment_data.get_payment_method_data() {
        Some(domain::PaymentMethodData::OpenBanking(domain::OpenBankingData::OpenBankingPIS {
            ..
        })) => {
            if connector.connector_name == router_types::Connector::Plaid {
                router_data = router_data.postprocessing_steps(state, connector).await?;
                let token = if let Ok(ref res) = router_data.response {
                    match res {
                        router_types::PaymentsResponseData::PostProcessingResponse {
                            session_token,
                        } => session_token
                            .as_ref()
                            .map(|token| api::SessionToken::OpenBanking(token.clone())),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(t) = token {
                    payment_data.push_sessions_token(t);
                }

                Ok(router_data)
            } else {
                Ok(router_data)
            }
        }
        _ => Ok(router_data),
    }
}

pub fn is_preprocessing_required_for_wallets(connector_name: String) -> bool {
    connector_name == *"trustpay" || connector_name == *"payme"
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_profile_id_and_get_mca<'a, F, D>(
    state: &'a SessionState,
    platform: &domain::Platform,
    payment_data: &D,
    connector_name: &str,
    merchant_connector_id: Option<&id_type::MerchantConnectorAccountId>,
    _should_validate: bool,
) -> RouterResult<helpers::MerchantConnectorAccountType>
where
    F: Clone,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    let profile_id = payment_data
        .get_payment_intent()
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("profile_id is not set in payment_intent")?
        .clone();

    #[cfg(feature = "v2")]
    let profile_id = payment_data.get_payment_intent().profile_id.clone();

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        platform.get_processor().get_account().get_id(),
        payment_data.get_creds_identifier(),
        platform.get_processor().get_key_store(),
        &profile_id,
        connector_name,
        merchant_connector_id,
    )
    .await?;

    Ok(merchant_connector_account)
}

#[cfg(feature = "v2")]
fn is_payment_method_tokenization_enabled_for_connector(
    state: &SessionState,
    connector_name: &str,
    payment_method: storage::enums::PaymentMethod,
    payment_method_type: Option<storage::enums::PaymentMethodType>,
    mandate_flow_enabled: storage_enums::FutureUsage,
) -> RouterResult<bool> {
    let connector_tokenization_filter = state.conf.tokenization.0.get(connector_name);

    Ok(connector_tokenization_filter
        .map(|connector_filter| {
            connector_filter
                .payment_method
                .clone()
                .contains(&payment_method)
                && is_payment_method_type_allowed_for_connector(
                    payment_method_type,
                    connector_filter.payment_method_type.clone(),
                )
                && is_payment_flow_allowed_for_connector(
                    mandate_flow_enabled,
                    connector_filter.flow.clone(),
                )
        })
        .unwrap_or(false))
}
// Determines connector tokenization eligibility: if no flow restriction, allow for one-off/CIT with raw cards; if flow = “mandates”, only allow MIT off-session with stored tokens.
#[cfg(feature = "v2")]
fn is_payment_flow_allowed_for_connector(
    mandate_flow_enabled: storage_enums::FutureUsage,
    payment_flow: Option<PaymentFlow>,
) -> bool {
    if payment_flow.is_none() {
        true
    } else {
        matches!(payment_flow, Some(PaymentFlow::Mandates))
            && matches!(mandate_flow_enabled, storage_enums::FutureUsage::OffSession)
    }
}

#[cfg(feature = "v1")]
fn is_payment_method_tokenization_enabled_for_connector(
    state: &SessionState,
    connector_name: &str,
    payment_method: storage::enums::PaymentMethod,
    payment_method_type: Option<storage::enums::PaymentMethodType>,
    payment_method_token: Option<&PaymentMethodToken>,
    mandate_flow_enabled: Option<storage_enums::FutureUsage>,
) -> RouterResult<bool> {
    let connector_tokenization_filter = state.conf.tokenization.0.get(connector_name);

    Ok(connector_tokenization_filter
        .map(|connector_filter| {
            connector_filter
                .payment_method
                .clone()
                .contains(&payment_method)
                && is_payment_method_type_allowed_for_connector(
                    payment_method_type,
                    connector_filter.payment_method_type.clone(),
                )
                && is_apple_pay_pre_decrypt_type_connector_tokenization(
                    payment_method_type,
                    payment_method_token,
                    connector_filter.apple_pay_pre_decrypt_flow.clone(),
                )
                && is_google_pay_pre_decrypt_type_connector_tokenization(
                    payment_method_type,
                    payment_method_token,
                    connector_filter.google_pay_pre_decrypt_flow.clone(),
                )
                && is_payment_flow_allowed_for_connector(
                    mandate_flow_enabled,
                    connector_filter.flow.clone(),
                )
        })
        .unwrap_or(false))
}
#[cfg(feature = "v1")]
fn is_payment_flow_allowed_for_connector(
    mandate_flow_enabled: Option<storage_enums::FutureUsage>,
    payment_flow: Option<PaymentFlow>,
) -> bool {
    if payment_flow.is_none() {
        true
    } else {
        matches!(payment_flow, Some(PaymentFlow::Mandates))
            && matches!(
                mandate_flow_enabled,
                Some(storage_enums::FutureUsage::OffSession)
            )
    }
}

fn is_apple_pay_pre_decrypt_type_connector_tokenization(
    payment_method_type: Option<storage::enums::PaymentMethodType>,
    payment_method_token: Option<&PaymentMethodToken>,
    apple_pay_pre_decrypt_flow_filter: Option<ApplePayPreDecryptFlow>,
) -> bool {
    match (payment_method_type, payment_method_token) {
        (
            Some(storage::enums::PaymentMethodType::ApplePay),
            Some(PaymentMethodToken::ApplePayDecrypt(..)),
        ) => !matches!(
            apple_pay_pre_decrypt_flow_filter,
            Some(ApplePayPreDecryptFlow::NetworkTokenization)
        ),
        _ => true,
    }
}

fn is_google_pay_pre_decrypt_type_connector_tokenization(
    payment_method_type: Option<storage::enums::PaymentMethodType>,
    payment_method_token: Option<&PaymentMethodToken>,
    google_pay_pre_decrypt_flow_filter: Option<GooglePayPreDecryptFlow>,
) -> bool {
    if let (
        Some(storage::enums::PaymentMethodType::GooglePay),
        Some(PaymentMethodToken::GooglePayDecrypt(..)),
    ) = (payment_method_type, payment_method_token)
    {
        !matches!(
            google_pay_pre_decrypt_flow_filter,
            Some(GooglePayPreDecryptFlow::NetworkTokenization)
        )
    } else {
        // Always return true for non–Google Pay pre-decrypt cases,
        // because the filter is only relevant for Google Pay pre-decrypt tokenization.
        // Returning true ensures that other payment methods or token types are not blocked.
        true
    }
}

fn decide_apple_pay_flow(
    state: &SessionState,
    payment_method_type: Option<enums::PaymentMethodType>,
    merchant_connector_account: Option<&helpers::MerchantConnectorAccountType>,
) -> Option<domain::ApplePayFlow> {
    payment_method_type.and_then(|pmt| match pmt {
        enums::PaymentMethodType::ApplePay => {
            check_apple_pay_metadata(state, merchant_connector_account)
        }
        _ => None,
    })
}

fn check_apple_pay_metadata(
    state: &SessionState,
    merchant_connector_account: Option<&helpers::MerchantConnectorAccountType>,
) -> Option<domain::ApplePayFlow> {
    merchant_connector_account.and_then(|mca| {
        let metadata = mca.get_metadata();
        metadata.and_then(|apple_pay_metadata| {
            let parsed_metadata = apple_pay_metadata
                .clone()
                .parse_value::<api_models::payments::ApplepayCombinedSessionTokenData>(
                    "ApplepayCombinedSessionTokenData",
                )
                .map(|combined_metadata| {
                    api_models::payments::ApplepaySessionTokenMetadata::ApplePayCombined(
                        combined_metadata.apple_pay_combined,
                    )
                })
                .or_else(|_| {
                    apple_pay_metadata
                        .parse_value::<api_models::payments::ApplepaySessionTokenData>(
                            "ApplepaySessionTokenData",
                        )
                        .map(|old_metadata| {
                            api_models::payments::ApplepaySessionTokenMetadata::ApplePay(
                                old_metadata.apple_pay,
                            )
                        })
                })
                .map_err(|error| {
                    logger::warn!(?error, "Failed to Parse Value to ApplepaySessionTokenData")
                });

            parsed_metadata.ok().map(|metadata| match metadata {
                api_models::payments::ApplepaySessionTokenMetadata::ApplePayCombined(
                    apple_pay_combined,
                ) => match apple_pay_combined {
                    api_models::payments::ApplePayCombinedMetadata::Simplified { .. } => {
                        domain::ApplePayFlow::Simplified(payments_api::PaymentProcessingDetails {
                            payment_processing_certificate: state
                                .conf
                                .applepay_decrypt_keys
                                .get_inner()
                                .apple_pay_ppc
                                .clone(),
                            payment_processing_certificate_key: state
                                .conf
                                .applepay_decrypt_keys
                                .get_inner()
                                .apple_pay_ppc_key
                                .clone(),
                        })
                    }
                    api_models::payments::ApplePayCombinedMetadata::Manual {
                        payment_request_data: _,
                        session_token_data,
                    } => {
                        if let Some(manual_payment_processing_details_at) =
                            session_token_data.payment_processing_details_at
                        {
                            match manual_payment_processing_details_at {
                                payments_api::PaymentProcessingDetailsAt::Hyperswitch(
                                    payment_processing_details,
                                ) => domain::ApplePayFlow::Simplified(payment_processing_details),
                                payments_api::PaymentProcessingDetailsAt::Connector => {
                                    domain::ApplePayFlow::Manual
                                }
                            }
                        } else {
                            domain::ApplePayFlow::Manual
                        }
                    }
                },
                api_models::payments::ApplepaySessionTokenMetadata::ApplePay(_) => {
                    domain::ApplePayFlow::Manual
                }
            })
        })
    })
}

fn get_google_pay_connector_wallet_details(
    state: &SessionState,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
) -> Option<GooglePayPaymentProcessingDetails> {
    let google_pay_root_signing_keys = state
        .conf
        .google_pay_decrypt_keys
        .as_ref()
        .map(|google_pay_keys| google_pay_keys.google_pay_root_signing_keys.clone());
    match merchant_connector_account.get_connector_wallets_details() {
        Some(wallet_details) => {
            let google_pay_wallet_details = wallet_details
                .parse_value::<api_models::payments::GooglePayWalletDetails>(
                    "GooglePayWalletDetails",
                )
                .map_err(|error| {
                    logger::warn!(?error, "Failed to Parse Value to GooglePayWalletDetails")
                });

            google_pay_wallet_details
                .ok()
                .and_then(
                    |google_pay_wallet_details| {
                        match google_pay_wallet_details
                        .google_pay
                        .provider_details {
                            api_models::payments::GooglePayProviderDetails::GooglePayMerchantDetails(merchant_details) => {
                                match (
                                    merchant_details
                                        .merchant_info
                                        .tokenization_specification
                                        .parameters
                                        .private_key,
                                    google_pay_root_signing_keys,
                                    merchant_details
                                        .merchant_info
                                        .tokenization_specification
                                        .parameters
                                        .recipient_id,
                                    ) {
                                        (Some(google_pay_private_key), Some(google_pay_root_signing_keys), Some(google_pay_recipient_id)) => {
                                            Some(GooglePayPaymentProcessingDetails {
                                                google_pay_private_key,
                                                google_pay_root_signing_keys,
                                                google_pay_recipient_id
                                            })
                                        }
                                        _ => {
                                            logger::warn!("One or more of the following fields are missing in GooglePayMerchantDetails: google_pay_private_key, google_pay_root_signing_keys, google_pay_recipient_id");
                                            None
                                        }
                                    }
                            }
                        }
                    }
                )
        }
        None => None,
    }
}

fn is_payment_method_type_allowed_for_connector(
    current_pm_type: Option<storage::enums::PaymentMethodType>,
    pm_type_filter: Option<PaymentMethodTypeTokenFilter>,
) -> bool {
    match (current_pm_type).zip(pm_type_filter) {
        Some((pm_type, type_filter)) => match type_filter {
            PaymentMethodTypeTokenFilter::AllAccepted => true,
            PaymentMethodTypeTokenFilter::EnableOnly(enabled) => enabled.contains(&pm_type),
            PaymentMethodTypeTokenFilter::DisableOnly(disabled) => !disabled.contains(&pm_type),
        },
        None => true, // Allow all types if payment_method_type is not present
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
async fn decide_payment_method_tokenize_action(
    state: &SessionState,
    connector_name: &str,
    payment_method: storage::enums::PaymentMethod,
    payment_intent_data: payments::PaymentIntent,
    pm_parent_token: Option<&str>,
    is_connector_tokenization_enabled: bool,
) -> RouterResult<TokenizationAction> {
    if matches!(
        payment_intent_data.split_payments,
        Some(common_types::payments::SplitPaymentsRequest::StripeSplitPayment(_))
    ) {
        Ok(TokenizationAction::TokenizeInConnector)
    } else {
        match pm_parent_token {
            None => Ok(if is_connector_tokenization_enabled {
                TokenizationAction::TokenizeInConnectorAndRouter
            } else {
                TokenizationAction::TokenizeInRouter
            }),

            Some(token) => {
                let redis_conn = state
                    .store
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to get redis connection")?;

                let key = format!(
                    "pm_token_{}_{}_{}",
                    token.to_owned(),
                    payment_method,
                    connector_name
                );

                let connector_token_option = redis_conn
                    .get_key::<Option<String>>(&key.into())
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to fetch the token from redis")?;

                match connector_token_option {
                    Some(connector_token) => {
                        Ok(TokenizationAction::ConnectorToken(connector_token))
                    }
                    None => Ok(if is_connector_tokenization_enabled {
                        TokenizationAction::TokenizeInConnectorAndRouter
                    } else {
                        TokenizationAction::TokenizeInRouter
                    }),
                }
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PazePaymentProcessingDetails {
    pub paze_private_key: Secret<String>,
    pub paze_private_key_passphrase: Secret<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GooglePayPaymentProcessingDetails {
    pub google_pay_private_key: Secret<String>,
    pub google_pay_root_signing_keys: Secret<String>,
    pub google_pay_recipient_id: Secret<String>,
}
#[cfg(feature = "v1")]
#[derive(Clone, Debug)]
pub enum TokenizationAction {
    TokenizeInRouter,
    TokenizeInConnector,
    TokenizeInConnectorAndRouter,
    ConnectorToken(String),
    SkipConnectorTokenization,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub enum TokenizationAction {
    TokenizeInConnector,
    SkipConnectorTokenization,
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn get_connector_tokenization_action_when_confirm_true<F, Req, D>(
    state: &SessionState,
    operation: &BoxedOperation<'_, F, Req, D>,
    payment_data: &mut D,
    validate_result: &operations::ValidateResult,
    merchant_key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
    business_profile: &domain::Profile,
    should_retry_with_pan: bool,
) -> RouterResult<(D, TokenizationAction)>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let connector = payment_data.get_payment_attempt().connector.to_owned();

    let is_mandate = payment_data
        .get_mandate_id()
        .as_ref()
        .and_then(|inner| inner.mandate_reference_id.as_ref())
        .map(|mandate_reference| match mandate_reference {
            api_models::payments::MandateReferenceId::ConnectorMandateId(_) => true,
            api_models::payments::MandateReferenceId::NetworkMandateId(_)
            | api_models::payments::MandateReferenceId::NetworkTokenWithNTI(_) => false,
        })
        .unwrap_or(false);

    let payment_data_and_tokenization_action = match connector {
        Some(_) if is_mandate => (
            payment_data.to_owned(),
            TokenizationAction::SkipConnectorTokenization,
        ),
        Some(connector) if is_operation_confirm(&operation) => {
            let payment_method = payment_data
                .get_payment_attempt()
                .payment_method
                .get_required_value("payment_method")?;
            let payment_method_type = payment_data.get_payment_attempt().payment_method_type;

            let mandate_flow_enabled = payment_data
                .get_payment_attempt()
                .setup_future_usage_applied;

            let is_connector_tokenization_enabled =
                is_payment_method_tokenization_enabled_for_connector(
                    state,
                    &connector,
                    payment_method,
                    payment_method_type,
                    payment_data.get_payment_method_token(),
                    mandate_flow_enabled,
                )?;

            let payment_method_action = decide_payment_method_tokenize_action(
                state,
                &connector,
                payment_method,
                payment_data.get_payment_intent().clone(),
                payment_data.get_token(),
                is_connector_tokenization_enabled,
            )
            .await?;

            let connector_tokenization_action = match payment_method_action {
                TokenizationAction::TokenizeInRouter => {
                    let (_operation, payment_method_data, pm_id) = operation
                        .to_domain()?
                        .make_pm_data(
                            state,
                            payment_data,
                            validate_result.storage_scheme,
                            merchant_key_store,
                            customer,
                            business_profile,
                            should_retry_with_pan,
                        )
                        .await?;
                    payment_data.set_payment_method_data(payment_method_data);
                    payment_data.set_payment_method_id_in_attempt(pm_id);

                    TokenizationAction::SkipConnectorTokenization
                }
                TokenizationAction::TokenizeInConnector => TokenizationAction::TokenizeInConnector,
                TokenizationAction::TokenizeInConnectorAndRouter => {
                    let (_operation, payment_method_data, pm_id) = operation
                        .to_domain()?
                        .make_pm_data(
                            state,
                            payment_data,
                            validate_result.storage_scheme,
                            merchant_key_store,
                            customer,
                            business_profile,
                            should_retry_with_pan,
                        )
                        .await?;

                    payment_data.set_payment_method_data(payment_method_data);
                    payment_data.set_payment_method_id_in_attempt(pm_id);
                    TokenizationAction::TokenizeInConnector
                }
                TokenizationAction::ConnectorToken(token) => {
                    payment_data.set_pm_token(token);
                    TokenizationAction::SkipConnectorTokenization
                }
                TokenizationAction::SkipConnectorTokenization => {
                    TokenizationAction::SkipConnectorTokenization
                }
            };
            (payment_data.to_owned(), connector_tokenization_action)
        }
        _ => (
            payment_data.to_owned(),
            TokenizationAction::SkipConnectorTokenization,
        ),
    };

    Ok(payment_data_and_tokenization_action)
}

#[cfg(feature = "v2")]
pub async fn tokenize_in_router_when_confirm_false_or_external_authentication<F, Req, D>(
    state: &SessionState,
    operation: &BoxedOperation<'_, F, Req, D>,
    payment_data: &mut D,
    validate_result: &operations::ValidateResult,
    merchant_key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
    business_profile: &domain::Profile,
) -> RouterResult<D>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    todo!()
}

#[cfg(feature = "v1")]
pub async fn tokenize_in_router_when_confirm_false_or_external_authentication<F, Req, D>(
    state: &SessionState,
    operation: &BoxedOperation<'_, F, Req, D>,
    payment_data: &mut D,
    validate_result: &operations::ValidateResult,
    merchant_key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
    business_profile: &domain::Profile,
) -> RouterResult<D>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    // On confirm is false and only router related
    let is_external_authentication_requested = payment_data
        .get_payment_intent()
        .request_external_three_ds_authentication;
    let payment_data =
        if !is_operation_confirm(operation) || is_external_authentication_requested == Some(true) {
            let (_operation, payment_method_data, pm_id) = operation
                .to_domain()?
                .make_pm_data(
                    state,
                    payment_data,
                    validate_result.storage_scheme,
                    merchant_key_store,
                    customer,
                    business_profile,
                    false,
                )
                .await?;
            payment_data.set_payment_method_data(payment_method_data);
            if let Some(payment_method_id) = pm_id {
                payment_data.set_payment_method_id_in_attempt(Some(payment_method_id));
            }
            payment_data
        } else {
            payment_data
        };
    Ok(payment_data.to_owned())
}

#[derive(Clone)]
pub struct MandateConnectorDetails {
    pub connector: String,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
}

#[derive(Clone)]
pub struct PaymentData<F>
where
    F: Clone,
{
    pub flow: PhantomData<F>,
    pub payment_intent: storage::PaymentIntent,
    pub payment_attempt: storage::PaymentAttempt,
    pub multiple_capture_data: Option<types::MultipleCaptureData>,
    pub amount: api::Amount,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub mandate_connector: Option<MandateConnectorDetails>,
    pub currency: storage_enums::Currency,
    pub setup_mandate: Option<MandateData>,
    pub customer_acceptance: Option<common_payments_types::CustomerAcceptance>,
    pub address: PaymentAddress,
    pub token: Option<String>,
    pub token_data: Option<storage::PaymentTokenData>,
    pub confirm: Option<bool>,
    pub force_sync: Option<bool>,
    pub all_keys_required: Option<bool>,
    pub payment_method_data: Option<domain::PaymentMethodData>,
    pub payment_method_token: Option<PaymentMethodToken>,
    pub payment_method_info: Option<domain::PaymentMethod>,
    pub refunds: Vec<diesel_refund::Refund>,
    pub disputes: Vec<storage::Dispute>,
    pub attempts: Option<Vec<storage::PaymentAttempt>>,
    pub sessions_token: Vec<api::SessionToken>,
    pub card_cvc: Option<Secret<String>>,
    pub email: Option<pii::Email>,
    pub creds_identifier: Option<String>,
    pub pm_token: Option<String>,
    pub connector_customer_id: Option<String>,
    pub recurring_mandate_payment_data:
        Option<hyperswitch_domain_models::router_data::RecurringMandatePaymentData>,
    pub ephemeral_key: Option<ephemeral_key::EphemeralKey>,
    pub redirect_response: Option<api_models::payments::RedirectResponse>,
    pub surcharge_details: Option<types::SurchargeDetails>,
    pub frm_message: Option<FraudCheck>,
    pub payment_link_data: Option<api_models::payments::PaymentLinkResponse>,
    pub incremental_authorization_details: Option<IncrementalAuthorizationDetails>,
    pub authorizations: Vec<diesel_models::authorization::Authorization>,
    pub authentication: Option<domain::authentication::AuthenticationStore>,
    pub recurring_details: Option<RecurringDetails>,
    pub poll_config: Option<router_types::PollConfig>,
    pub tax_data: Option<TaxData>,
    pub session_id: Option<String>,
    pub service_details: Option<api_models::payments::CtpServiceDetails>,
    pub card_testing_guard_data:
        Option<hyperswitch_domain_models::card_testing_guard_data::CardTestingGuardData>,
    pub vault_operation: Option<domain_payments::VaultOperation>,
    pub threeds_method_comp_ind: Option<api_models::payments::ThreeDsCompletionIndicator>,
    pub whole_connector_response: Option<Secret<String>>,
    pub is_manual_retry_enabled: Option<bool>,
    pub is_l2_l3_enabled: bool,
    pub external_authentication_data: Option<api_models::payments::ExternalThreeDsData>,
}

#[cfg(feature = "v1")]
#[derive(Clone)]
pub struct PaymentEligibilityData {
    pub payment_method_data: Option<domain::PaymentMethodData>,
    pub payment_intent: storage::PaymentIntent,
    pub browser_info: Option<pii::SecretSerdeValue>,
}

#[cfg(feature = "v1")]
impl PaymentEligibilityData {
    pub async fn from_request(
        state: &SessionState,
        platform: &domain::Platform,
        payments_eligibility_request: &api_models::payments::PaymentsEligibilityRequest,
    ) -> CustomResult<Self, errors::ApiErrorResponse> {
        let payment_method_data = payments_eligibility_request
            .payment_method_data
            .payment_method_data
            .clone()
            .map(domain::PaymentMethodData::from);
        let browser_info = payments_eligibility_request
            .browser_info
            .clone()
            .map(|browser_info| {
                serde_json::to_value(browser_info)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encode payout method data")
            })
            .transpose()?
            .map(pii::SecretSerdeValue::new);
        let payment_intent = state
            .store
            .find_payment_intent_by_payment_id_merchant_id(
                &payments_eligibility_request.payment_id,
                platform.get_processor().get_account().get_id(),
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        Ok(Self {
            payment_method_data,
            browser_info,
            payment_intent,
        })
    }
}

#[derive(Clone, serde::Serialize, Debug)]
pub struct TaxData {
    pub shipping_details: hyperswitch_domain_models::address::Address,
    pub payment_method_type: enums::PaymentMethodType,
}

#[derive(Clone, serde::Serialize, Debug)]
pub struct PaymentEvent {
    payment_intent: storage::PaymentIntent,
    payment_attempt: storage::PaymentAttempt,
}

impl<F: Clone> PaymentData<F> {
    // Get the method by which a card is discovered during a payment
    #[cfg(feature = "v1")]
    fn get_card_discovery_for_card_payment_method(&self) -> Option<common_enums::CardDiscovery> {
        match self.payment_attempt.payment_method {
            Some(storage_enums::PaymentMethod::Card) => {
                if self
                    .token_data
                    .as_ref()
                    .map(storage::PaymentTokenData::is_permanent_card)
                    .unwrap_or(false)
                {
                    Some(common_enums::CardDiscovery::SavedCard)
                } else if self.service_details.is_some() {
                    Some(common_enums::CardDiscovery::ClickToPay)
                } else {
                    Some(common_enums::CardDiscovery::Manual)
                }
            }
            _ => None,
        }
    }

    fn to_event(&self) -> PaymentEvent {
        PaymentEvent {
            payment_intent: self.payment_intent.clone(),
            payment_attempt: self.payment_attempt.clone(),
        }
    }
}

impl EventInfo for PaymentEvent {
    type Data = Self;
    fn data(&self) -> error_stack::Result<Self::Data, events::EventsError> {
        Ok(self.clone())
    }

    fn key(&self) -> String {
        "payment".to_string()
    }
}

#[derive(Debug, Default, Clone)]
pub struct IncrementalAuthorizationDetails {
    pub additional_amount: MinorUnit,
    pub total_amount: MinorUnit,
    pub reason: Option<String>,
    pub authorization_id: Option<String>,
}

pub async fn get_payment_link_response_from_id(
    state: &SessionState,
    payment_link_id: &str,
) -> CustomResult<api_models::payments::PaymentLinkResponse, errors::ApiErrorResponse> {
    let db = &*state.store;

    let payment_link_object = db
        .find_payment_link_by_payment_link_id(payment_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentLinkNotFound)?;

    Ok(api_models::payments::PaymentLinkResponse {
        link: payment_link_object.link_to_pay.clone(),
        secure_link: payment_link_object.secure_link,
        payment_link_id: payment_link_object.payment_link_id,
    })
}

#[cfg(feature = "v1")]
pub fn if_not_create_change_operation<'a, Op, F>(
    status: storage_enums::IntentStatus,
    confirm: Option<bool>,
    current: &'a Op,
) -> BoxedOperation<'a, F, api::PaymentsRequest, PaymentData<F>>
where
    F: Send + Clone + Sync,
    Op: Operation<F, api::PaymentsRequest, Data = PaymentData<F>> + Send + Sync,
    &'a Op: Operation<F, api::PaymentsRequest, Data = PaymentData<F>>,
    PaymentStatus: Operation<F, api::PaymentsRequest, Data = PaymentData<F>>,
    &'a PaymentStatus: Operation<F, api::PaymentsRequest, Data = PaymentData<F>>,
{
    if confirm.unwrap_or(false) {
        Box::new(PaymentConfirm)
    } else {
        match status {
            storage_enums::IntentStatus::RequiresConfirmation
            | storage_enums::IntentStatus::RequiresCustomerAction
            | storage_enums::IntentStatus::RequiresPaymentMethod => Box::new(current),
            _ => Box::new(&PaymentStatus),
        }
    }
}

#[cfg(feature = "v1")]
pub fn is_confirm<'a, F: Clone + Send, R, Op>(
    operation: &'a Op,
    confirm: Option<bool>,
) -> BoxedOperation<'a, F, R, PaymentData<F>>
where
    PaymentConfirm: Operation<F, R, Data = PaymentData<F>>,
    &'a PaymentConfirm: Operation<F, R, Data = PaymentData<F>>,
    Op: Operation<F, R, Data = PaymentData<F>> + Send + Sync,
    &'a Op: Operation<F, R, Data = PaymentData<F>>,
{
    if confirm.unwrap_or(false) {
        Box::new(&PaymentConfirm)
    } else {
        Box::new(operation)
    }
}

#[cfg(feature = "v1")]
pub fn should_call_connector<Op: Debug, F: Clone, D>(operation: &Op, payment_data: &D) -> bool
where
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    match format!("{operation:?}").as_str() {
        "PaymentConfirm" => true,
        "PaymentStart" => {
            !matches!(
                payment_data.get_payment_intent().status,
                storage_enums::IntentStatus::Failed | storage_enums::IntentStatus::Succeeded
            ) && payment_data
                .get_payment_attempt()
                .authentication_data
                .is_none()
        }
        "PaymentStatus" => {
            payment_data.get_all_keys_required().unwrap_or(false)
                || matches!(
                    payment_data.get_payment_intent().status,
                    storage_enums::IntentStatus::Processing
                        | storage_enums::IntentStatus::RequiresCustomerAction
                        | storage_enums::IntentStatus::RequiresMerchantAction
                        | storage_enums::IntentStatus::RequiresCapture
                        | storage_enums::IntentStatus::PartiallyCapturedAndCapturable
                ) && payment_data.get_force_sync().unwrap_or(false)
        }
        "PaymentCancel" => matches!(
            payment_data.get_payment_intent().status,
            storage_enums::IntentStatus::RequiresCapture
                | storage_enums::IntentStatus::PartiallyCapturedAndCapturable
        ),
        "PaymentCancelPostCapture" => matches!(
            payment_data.get_payment_intent().status,
            storage_enums::IntentStatus::Succeeded
                | storage_enums::IntentStatus::PartiallyCaptured
                | storage_enums::IntentStatus::PartiallyCapturedAndCapturable
        ),
        "PaymentCapture" => {
            matches!(
                payment_data.get_payment_intent().status,
                storage_enums::IntentStatus::RequiresCapture
                    | storage_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
                    | storage_enums::IntentStatus::PartiallyCapturedAndCapturable
            ) || (matches!(
                payment_data.get_payment_intent().status,
                storage_enums::IntentStatus::Processing
            ) && matches!(
                payment_data.get_capture_method(),
                Some(storage_enums::CaptureMethod::ManualMultiple)
            ))
        }
        "CompleteAuthorize" => true,
        "PaymentApprove" => true,
        "PaymentReject" => true,
        "PaymentSession" => true,
        "PaymentSessionUpdate" => true,
        "PaymentPostSessionTokens" => true,
        "PaymentUpdateMetadata" => true,
        "PaymentExtendAuthorization" => matches!(
            payment_data.get_payment_intent().status,
            storage_enums::IntentStatus::RequiresCapture
                | storage_enums::IntentStatus::PartiallyAuthorizedAndRequiresCapture
        ),
        "PaymentIncrementalAuthorization" => matches!(
            payment_data.get_payment_intent().status,
            storage_enums::IntentStatus::RequiresCapture
        ),
        _ => false,
    }
}

pub fn is_operation_confirm<Op: Debug>(operation: &Op) -> bool {
    matches!(format!("{operation:?}").as_str(), "PaymentConfirm")
}

pub fn is_operation_complete_authorize<Op: Debug>(operation: &Op) -> bool {
    matches!(format!("{operation:?}").as_str(), "CompleteAuthorize")
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn list_payments(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<api::PaymentListResponse> {
    helpers::validate_payment_list_request(&constraints)?;
    let merchant_id = platform.get_processor().get_account().get_id();
    let db = state.store.as_ref();
    let payment_intents = helpers::filter_by_constraints(
        &state,
        &(constraints, profile_id_list).try_into()?,
        merchant_id,
        platform.get_processor().get_key_store(),
        platform.get_processor().get_account().storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let collected_futures = payment_intents.into_iter().map(|pi| {
        async {
            match db
                .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                    &pi.payment_id,
                    merchant_id,
                    &pi.active_attempt.get_id(),
                    // since OLAP doesn't have KV. Force to get the data from PSQL.
                    storage_enums::MerchantStorageScheme::PostgresOnly,
                )
                .await
            {
                Ok(pa) => Some(Ok((pi, pa))),
                Err(error) => {
                    if matches!(
                        error.current_context(),
                        errors::StorageError::ValueNotFound(_)
                    ) {
                        logger::warn!(
                            ?error,
                            "payment_attempts missing for payment_id : {:?}",
                            pi.payment_id,
                        );
                        return None;
                    }
                    Some(Err(error))
                }
            }
        }
    });

    //If any of the response are Err, we will get Result<Err(_)>
    let pi_pa_tuple_vec: Result<Vec<(storage::PaymentIntent, storage::PaymentAttempt)>, _> =
        join_all(collected_futures)
            .await
            .into_iter()
            .flatten() //Will ignore `None`, will only flatten 1 level
            .collect::<Result<Vec<(storage::PaymentIntent, storage::PaymentAttempt)>, _>>();
    //Will collect responses in same order async, leading to sorted responses

    //Converting Intent-Attempt array to Response if no error
    let data: Vec<api::PaymentsResponse> = pi_pa_tuple_vec
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .into_iter()
        .map(ForeignFrom::foreign_from)
        .collect();

    Ok(services::ApplicationResponse::Json(
        api::PaymentListResponse {
            size: data.len(),
            data,
        },
    ))
}

#[cfg(all(feature = "v2", feature = "olap"))]
pub async fn list_payments(
    state: SessionState,
    platform: domain::Platform,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<payments_api::PaymentListResponse> {
    common_utils::metrics::utils::record_operation_time(
        async {
            let limit = &constraints.limit;
            helpers::validate_payment_list_request_for_joins(*limit)?;
            let db: &dyn StorageInterface = state.store.as_ref();
            let fetch_constraints = constraints.clone().into();
            let list: Vec<(storage::PaymentIntent, Option<storage::PaymentAttempt>)> = db
                .get_filtered_payment_intents_attempt(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let data: Vec<api_models::payments::PaymentsListResponseItem> =
                list.into_iter().map(ForeignFrom::foreign_from).collect();

            let active_attempt_ids = db
                .get_filtered_active_attempt_ids_for_total_count(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while retrieving active_attempt_ids for merchant")?;

            let total_count = if constraints.has_no_attempt_filters() {
                i64::try_from(active_attempt_ids.len())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while converting from usize to i64")
            } else {
                let active_attempt_ids = active_attempt_ids
                    .into_iter()
                    .flatten()
                    .collect::<Vec<String>>();

                db.get_total_count_of_filtered_payment_attempts(
                    platform.get_processor().get_account().get_id(),
                    &active_attempt_ids,
                    constraints.connector,
                    constraints.payment_method_type,
                    constraints.payment_method_subtype,
                    constraints.authentication_type,
                    constraints.merchant_connector_id,
                    constraints.card_network,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while retrieving total count of payment attempts")
            }?;

            Ok(services::ApplicationResponse::Json(
                api_models::payments::PaymentListResponse {
                    count: data.len(),
                    total_count,
                    data,
                },
            ))
        },
        &metrics::PAYMENT_LIST_LATENCY,
        router_env::metric_attributes!((
            "merchant_id",
            platform.get_processor().get_account().get_id().clone()
        )),
    )
    .await
}

#[cfg(all(feature = "v2", feature = "olap"))]
pub async fn revenue_recovery_list_payments(
    state: SessionState,
    platform: domain::Platform,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<payments_api::RecoveryPaymentListResponse> {
    common_utils::metrics::utils::record_operation_time(
        async {
            let limit = &constraints.limit;
            helpers::validate_payment_list_request_for_joins(*limit)?;
            let db: &dyn StorageInterface = state.store.as_ref();
            let fetch_constraints = constraints.clone().into();
            let list: Vec<(storage::PaymentIntent, Option<storage::PaymentAttempt>)> = db
                .get_filtered_payment_intents_attempt(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

            // Get all billing connector account IDs
            let billing_connector_ids: Vec<_> = list
                .iter()
                .map(|(payment_intent, _)| {
                    payment_intent.get_billing_merchant_connector_account_id()
                })
                .collect();

            // Create futures for workflow lookups
            let workflow_futures: Vec<_> = list
                .iter()
                .map(|(payment_intent, _)| get_workflow_entries(&state, &payment_intent.id))
                .collect();

            let billing_connector_futures: Vec<_> = billing_connector_ids
                .into_iter()
                .map(|billing_mca_id| {
                    let platform_clone = platform.clone(); // Clone for each future
                    async move {
                        if let Some(billing_mca_id) = billing_mca_id {
                            db.find_merchant_connector_account_by_id(
                                &billing_mca_id,
                                platform_clone.get_processor().get_key_store(),
                            )
                            .await
                            .ok()
                        } else {
                            None
                        }
                    }
                })
                .collect();

            let workflow_results = join_all(workflow_futures).await;
            let billing_connector_results = join_all(billing_connector_futures).await;

            let data: Vec<api_models::payments::RecoveryPaymentsListResponseItem> = list
                .into_iter()
                .zip(workflow_results.into_iter())
                .zip(billing_connector_results.into_iter())
                .map(
                    |(
                        ((payment_intent, payment_attempt), workflow_result),
                        billing_connector_account,
                    )| {
                        let (calculate_workflow, execute_workflow) =
                            workflow_result.unwrap_or((None, None));

                        // Get retry threshold from billing connector account
                        let max_retry_threshold = billing_connector_account
                            .as_ref()
                            .and_then(|mca| mca.get_retry_threshold())
                            .unwrap_or(0); // Default fallback

                        // Use custom mapping function
                        map_to_recovery_payment_item(
                            payment_intent,
                            payment_attempt,
                            calculate_workflow,
                            execute_workflow,
                            max_retry_threshold.try_into().unwrap_or(0),
                        )
                    },
                )
                .collect();

            let active_attempt_ids = db
                .get_filtered_active_attempt_ids_for_total_count(
                    platform.get_processor().get_account().get_id(),
                    &fetch_constraints,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while retrieving active_attempt_ids for merchant")?;

            let total_count = if constraints.has_no_attempt_filters() {
                i64::try_from(active_attempt_ids.len())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while converting from usize to i64")
            } else {
                let active_attempt_ids = active_attempt_ids
                    .into_iter()
                    .flatten()
                    .collect::<Vec<String>>();

                db.get_total_count_of_filtered_payment_attempts(
                    platform.get_processor().get_account().get_id(),
                    &active_attempt_ids,
                    constraints.connector,
                    constraints.payment_method_type,
                    constraints.payment_method_subtype,
                    constraints.authentication_type,
                    constraints.merchant_connector_id,
                    constraints.card_network,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while retrieving total count of payment attempts")
            }?;

            Ok(services::ApplicationResponse::Json(
                api_models::payments::RecoveryPaymentListResponse {
                    count: data.len(),
                    total_count,
                    data,
                },
            ))
        },
        &metrics::PAYMENT_LIST_LATENCY,
        router_env::metric_attributes!((
            "merchant_id",
            platform.get_processor().get_account().get_id().clone()
        )),
    )
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn apply_filters_on_payments(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    constraints: api::PaymentListFilterConstraints,
) -> RouterResponse<api::PaymentListResponseV2> {
    common_utils::metrics::utils::record_operation_time(
        async {
            let limit = &constraints.limit;
            helpers::validate_payment_list_request_for_joins(*limit)?;
            let db: &dyn StorageInterface = state.store.as_ref();
            let pi_fetch_constraints = (constraints.clone(), profile_id_list.clone()).try_into()?;
            let list: Vec<(storage::PaymentIntent, storage::PaymentAttempt)> = db
                .get_filtered_payment_intents_attempt(
                    platform.get_processor().get_account().get_id(),
                    &pi_fetch_constraints,
                    platform.get_processor().get_key_store(),
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let data: Vec<api::PaymentsResponse> =
                list.into_iter().map(ForeignFrom::foreign_from).collect();

            let active_attempt_ids = db
                .get_filtered_active_attempt_ids_for_total_count(
                    platform.get_processor().get_account().get_id(),
                    &pi_fetch_constraints,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

            let total_count = if constraints.has_no_attempt_filters() {
                i64::try_from(active_attempt_ids.len())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while converting from usize to i64")
            } else {
                db.get_total_count_of_filtered_payment_attempts(
                    platform.get_processor().get_account().get_id(),
                    &active_attempt_ids,
                    constraints.connector,
                    constraints.payment_method,
                    constraints.payment_method_type,
                    constraints.authentication_type,
                    constraints.merchant_connector_id,
                    constraints.card_network,
                    constraints.card_discovery,
                    platform.get_processor().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
            }?;

            Ok(services::ApplicationResponse::Json(
                api::PaymentListResponseV2 {
                    count: data.len(),
                    total_count,
                    data,
                },
            ))
        },
        &metrics::PAYMENT_LIST_LATENCY,
        router_env::metric_attributes!((
            "merchant_id",
            platform.get_processor().get_account().get_id().clone()
        )),
    )
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_filters_for_payments(
    state: SessionState,
    platform: domain::Platform,
    time_range: common_utils::types::TimeRange,
) -> RouterResponse<api::PaymentListFilters> {
    let db = state.store.as_ref();
    let pi = db
        .filter_payment_intents_by_time_range_constraints(
            platform.get_processor().get_account().get_id(),
            &time_range,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let filters = db
        .get_filters_for_payments(
            pi.as_slice(),
            platform.get_processor().get_account().get_id(),
            // since OLAP doesn't have KV. Force to get the data from PSQL.
            storage_enums::MerchantStorageScheme::PostgresOnly,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    Ok(services::ApplicationResponse::Json(
        api::PaymentListFilters {
            connector: filters.connector,
            currency: filters.currency,
            status: filters.status,
            payment_method: filters.payment_method,
            payment_method_type: filters.payment_method_type,
            authentication_type: filters.authentication_type,
        },
    ))
}

#[cfg(feature = "olap")]
pub async fn get_payment_filters(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
) -> RouterResponse<api::PaymentListFiltersV2> {
    let merchant_connector_accounts = if let services::ApplicationResponse::Json(data) =
        super::admin::list_payment_connectors(
            state,
            platform.get_processor().get_account().get_id().to_owned(),
            profile_id_list,
        )
        .await?
    {
        data
    } else {
        return Err(errors::ApiErrorResponse::InternalServerError.into());
    };

    let mut connector_map: HashMap<String, Vec<MerchantConnectorInfo>> = HashMap::new();
    let mut payment_method_types_map: HashMap<
        enums::PaymentMethod,
        HashSet<enums::PaymentMethodType>,
    > = HashMap::new();

    // populate connector map
    merchant_connector_accounts
        .iter()
        .filter_map(|merchant_connector_account| {
            merchant_connector_account
                .connector_label
                .as_ref()
                .map(|label| {
                    let info = merchant_connector_account.to_merchant_connector_info(label);
                    (merchant_connector_account.get_connector_name(), info)
                })
        })
        .for_each(|(connector_name, info)| {
            connector_map
                .entry(connector_name.to_string())
                .or_default()
                .push(info);
        });

    // populate payment method type map
    merchant_connector_accounts
        .iter()
        .flat_map(|merchant_connector_account| {
            merchant_connector_account.payment_methods_enabled.as_ref()
        })
        .map(|payment_methods_enabled| {
            payment_methods_enabled
                .iter()
                .filter_map(|payment_method_enabled| {
                    payment_method_enabled
                        .get_payment_method_type()
                        .map(|types_vec| {
                            (
                                payment_method_enabled.get_payment_method(),
                                types_vec.clone(),
                            )
                        })
                })
        })
        .for_each(|payment_methods_enabled| {
            payment_methods_enabled.for_each(
                |(payment_method_option, payment_method_types_vec)| {
                    if let Some(payment_method) = payment_method_option {
                        payment_method_types_map
                            .entry(payment_method)
                            .or_default()
                            .extend(payment_method_types_vec.iter().filter_map(
                                |req_payment_method_types| {
                                    req_payment_method_types.get_payment_method_type()
                                },
                            ));
                    }
                },
            );
        });

    Ok(services::ApplicationResponse::Json(
        api::PaymentListFiltersV2 {
            connector: connector_map,
            currency: enums::Currency::iter().collect(),
            status: enums::IntentStatus::iter().collect(),
            payment_method: payment_method_types_map,
            authentication_type: enums::AuthenticationType::iter().collect(),
            card_network: enums::CardNetwork::iter().collect(),
            card_discovery: enums::CardDiscovery::iter().collect(),
        },
    ))
}

#[cfg(feature = "olap")]
pub async fn get_aggregates_for_payments(
    state: SessionState,
    platform: domain::Platform,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    time_range: common_utils::types::TimeRange,
) -> RouterResponse<api::PaymentsAggregateResponse> {
    let db = state.store.as_ref();
    let intent_status_with_count = db
        .get_intent_status_with_count(
            platform.get_processor().get_account().get_id(),
            profile_id_list,
            &time_range,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let mut status_map: HashMap<enums::IntentStatus, i64> =
        intent_status_with_count.into_iter().collect();
    for status in enums::IntentStatus::iter() {
        status_map.entry(status).or_default();
    }

    Ok(services::ApplicationResponse::Json(
        api::PaymentsAggregateResponse {
            status_with_count: status_map,
        },
    ))
}

#[cfg(feature = "v1")]
pub async fn add_process_sync_task(
    db: &dyn StorageInterface,
    payment_attempt: &storage::PaymentAttempt,
    schedule_time: time::PrimitiveDateTime,
) -> CustomResult<(), errors::StorageError> {
    let tracking_data = api::PaymentsRetrieveRequest {
        force_sync: true,
        merchant_id: Some(payment_attempt.merchant_id.clone()),
        resource_id: api::PaymentIdType::PaymentAttemptId(payment_attempt.get_id().to_owned()),
        ..Default::default()
    };
    let runner = storage::ProcessTrackerRunner::PaymentsSyncWorkflow;
    let task = "PAYMENTS_SYNC";
    let tag = ["SYNC", "PAYMENT"];
    let process_tracker_id = pt_utils::get_process_tracker_id(
        runner,
        task,
        payment_attempt.get_id(),
        &payment_attempt.merchant_id,
    );
    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        tracking_data,
        None,
        schedule_time,
        common_types::consts::API_VERSION,
    )
    .map_err(errors::StorageError::from)?;

    db.insert_process(process_tracker_entry).await?;
    Ok(())
}

#[cfg(feature = "v2")]
pub async fn reset_process_sync_task(
    db: &dyn StorageInterface,
    payment_attempt: &storage::PaymentAttempt,
    schedule_time: time::PrimitiveDateTime,
) -> Result<(), errors::ProcessTrackerError> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn reset_process_sync_task(
    db: &dyn StorageInterface,
    payment_attempt: &storage::PaymentAttempt,
    schedule_time: time::PrimitiveDateTime,
) -> Result<(), errors::ProcessTrackerError> {
    let runner = storage::ProcessTrackerRunner::PaymentsSyncWorkflow;
    let task = "PAYMENTS_SYNC";
    let process_tracker_id = pt_utils::get_process_tracker_id(
        runner,
        task,
        payment_attempt.get_id(),
        &payment_attempt.merchant_id,
    );
    let psync_process = db
        .find_process_by_id(&process_tracker_id)
        .await?
        .ok_or(errors::ProcessTrackerError::ProcessFetchingFailed)?;
    db.as_scheduler()
        .reset_process(psync_process, schedule_time)
        .await?;
    Ok(())
}

#[cfg(feature = "v1")]
pub fn update_straight_through_routing<F, D>(
    payment_data: &mut D,
    request_straight_through: serde_json::Value,
) -> CustomResult<(), errors::ParsingError>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let _: api_models::routing::StaticRoutingAlgorithm = request_straight_through
        .clone()
        .parse_value("RoutingAlgorithm")
        .attach_printable("Invalid straight through routing rules format")?;

    payment_data.set_straight_through_algorithm_in_payment_attempt(request_straight_through);

    Ok(())
}

#[cfg(feature = "v1")]
pub fn is_pre_network_tokenization_enabled(
    state: &SessionState,
    business_profile: &domain::Profile,
    customer_acceptance: Option<Secret<serde_json::Value>>,
    connector_name: enums::Connector,
) -> bool {
    let ntid_supported_connectors = &state
        .conf
        .network_transaction_id_supported_connectors
        .connector_list;

    let is_nt_supported_connector = ntid_supported_connectors.contains(&connector_name);

    business_profile.is_network_tokenization_enabled
        && business_profile.is_pre_network_tokenization_enabled
        && customer_acceptance.is_some()
        && is_nt_supported_connector
}

#[cfg(feature = "v1")]
pub async fn get_vault_operation_for_pre_network_tokenization(
    state: &SessionState,
    customer_id: id_type::CustomerId,
    card_data: &hyperswitch_domain_models::payment_method_data::Card,
) -> payments::VaultOperation {
    let pre_tokenization_response =
        tokenization::pre_payment_tokenization(state, customer_id, card_data)
            .await
            .ok();
    match pre_tokenization_response {
        Some((Some(token_response), Some(token_ref))) => {
            let token_data = domain::NetworkTokenData::from(token_response);
            let network_token_data_for_vault = payments::NetworkTokenDataForVault {
                network_token_data: token_data.clone(),
                network_token_req_ref_id: token_ref,
            };

            payments::VaultOperation::SaveCardAndNetworkTokenData(Box::new(
                payments::CardAndNetworkTokenDataForVault {
                    card_data: card_data.clone(),
                    network_token: network_token_data_for_vault.clone(),
                },
            ))
        }
        Some((None, Some(token_ref))) => {
            payments::VaultOperation::SaveCardData(payments::CardDataForVault {
                card_data: card_data.clone(),
                network_token_req_ref_id: Some(token_ref),
            })
        }
        _ => payments::VaultOperation::SaveCardData(payments::CardDataForVault {
            card_data: card_data.clone(),
            network_token_req_ref_id: None,
        }),
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn get_connector_choice<F, Req, D>(
    operation: &BoxedOperation<'_, F, Req, D>,
    state: &SessionState,
    req: &Req,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    payment_data: &mut D,
    eligible_connectors: Option<Vec<enums::RoutableConnectors>>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<Option<ConnectorCallType>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let connector_choice = operation
        .to_domain()?
        .get_connector(
            platform,
            &state.clone(),
            req,
            payment_data.get_payment_intent(),
        )
        .await?;

    let connector = if should_call_connector(operation, payment_data) {
        Some(match connector_choice {
            api::ConnectorChoice::SessionMultiple(connectors) => {
                let routing_output = perform_session_token_routing(
                    state.clone(),
                    platform,
                    business_profile,
                    payment_data,
                    connectors,
                )
                .await?;
                ConnectorCallType::SessionMultiple(routing_output)
            }

            api::ConnectorChoice::StraightThrough(straight_through) => {
                connector_selection(
                    state,
                    platform,
                    business_profile,
                    payment_data,
                    Some(straight_through),
                    eligible_connectors,
                    mandate_type,
                )
                .await?
            }

            api::ConnectorChoice::Decide => {
                connector_selection(
                    state,
                    platform,
                    business_profile,
                    payment_data,
                    None,
                    eligible_connectors,
                    mandate_type,
                )
                .await?
            }
        })
    } else if let api::ConnectorChoice::StraightThrough(algorithm) = connector_choice {
        update_straight_through_routing(payment_data, algorithm)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update straight through routing algorithm")?;

        None
    } else {
        None
    };
    Ok(connector)
}

async fn get_eligible_connector_for_nti<T: core_routing::GetRoutableConnectorsForChoice, F, D>(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_data: &D,
    connector_choice: T,
    business_profile: &domain::Profile,
) -> RouterResult<(
    api_models::payments::MandateReferenceId,
    hyperswitch_domain_models::payment_method_data::CardDetailsForNetworkTransactionId,
    api::ConnectorData,
)>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    // Since this flow will only be used in the MIT flow, recurring details are mandatory.
    let recurring_payment_details = payment_data
        .get_recurring_details()
        .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
        .attach_printable("Failed to fetch recurring details for mit")?;

    let (mandate_reference_id, card_details_for_network_transaction_id)= hyperswitch_domain_models::payment_method_data::CardDetailsForNetworkTransactionId::get_nti_and_card_details_for_mit_flow(recurring_payment_details.clone()).get_required_value("network transaction id and card details").attach_printable("Failed to fetch network transaction id and card details for mit")?;

    helpers::validate_card_expiry(
        &card_details_for_network_transaction_id.card_exp_month,
        &card_details_for_network_transaction_id.card_exp_year,
    )?;

    let network_transaction_id_supported_connectors = &state
        .conf
        .network_transaction_id_supported_connectors
        .connector_list
        .iter()
        .map(|value| value.to_string())
        .collect::<HashSet<_>>();

    let eligible_connector_data_list = connector_choice
        .get_routable_connectors(&*state.store, business_profile)
        .await?
        .filter_network_transaction_id_flow_supported_connectors(
            network_transaction_id_supported_connectors.to_owned(),
        )
        .construct_dsl_and_perform_eligibility_analysis(
            state,
            key_store,
            payment_data,
            business_profile.get_id(),
        )
        .await
        .attach_printable("Failed to fetch eligible connector data")?;

    let eligible_connector_data = eligible_connector_data_list
        .first()
        .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
        .attach_printable(
            "No eligible connector found for the network transaction id based mit flow",
        )?;
    Ok((
        mandate_reference_id,
        card_details_for_network_transaction_id,
        eligible_connector_data.clone(),
    ))
}

pub async fn set_eligible_connector_for_nti_in_payment_data<F, D>(
    state: &SessionState,
    business_profile: &domain::Profile,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut D,
    connector_choice: api::ConnectorChoice,
) -> RouterResult<api::ConnectorData>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let (mandate_reference_id, card_details_for_network_transaction_id, eligible_connector_data) =
        match connector_choice {
            api::ConnectorChoice::StraightThrough(straight_through) => {
                get_eligible_connector_for_nti(
                    state,
                    key_store,
                    payment_data,
                    core_routing::StraightThroughAlgorithmTypeSingle(straight_through),
                    business_profile,
                )
                .await?
            }
            api::ConnectorChoice::Decide => {
                get_eligible_connector_for_nti(
                    state,
                    key_store,
                    payment_data,
                    core_routing::DecideConnector,
                    business_profile,
                )
                .await?
            }
            api::ConnectorChoice::SessionMultiple(_) => {
                Err(errors::ApiErrorResponse::InternalServerError).attach_printable(
                    "Invalid routing rule configured for nti and card details based mit flow",
                )?
            }
        };

    // Set the eligible connector in the attempt
    payment_data
        .set_connector_in_payment_attempt(Some(eligible_connector_data.connector_name.to_string()));

    // Set `NetworkMandateId` as the MandateId
    payment_data.set_mandate_id(payments_api::MandateIds {
        mandate_id: None,
        mandate_reference_id: Some(mandate_reference_id),
    });

    // Set the card details received in the recurring details within the payment method data.
    payment_data.set_payment_method_data(Some(
        hyperswitch_domain_models::payment_method_data::PaymentMethodData::CardDetailsForNetworkTransactionId(card_details_for_network_transaction_id),
    ));

    Ok(eligible_connector_data)
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn connector_selection<F, D>(
    state: &SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    payment_data: &mut D,
    request_straight_through: Option<serde_json::Value>,
    eligible_connectors: Option<Vec<enums::RoutableConnectors>>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let request_straight_through: Option<api::routing::StraightThroughAlgorithm> =
        request_straight_through
            .map(|val| val.parse_value("RoutingAlgorithm"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid straight through routing rules format")?;

    let mut routing_data = storage::RoutingData {
        routed_through: payment_data.get_payment_attempt().connector.clone(),

        merchant_connector_id: payment_data
            .get_payment_attempt()
            .merchant_connector_id
            .clone(),

        algorithm: request_straight_through.clone(),
        routing_info: payment_data
            .get_payment_attempt()
            .straight_through_algorithm
            .clone()
            .map(|val| val.parse_value("PaymentRoutingInfo"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid straight through algorithm format found in payment attempt")?
            .unwrap_or(storage::PaymentRoutingInfo {
                algorithm: None,
                pre_routing_results: None,
            }),
    };

    let decided_connector = decide_connector(
        state.clone(),
        platform,
        business_profile,
        payment_data,
        request_straight_through,
        &mut routing_data,
        eligible_connectors,
        mandate_type,
    )
    .await?;

    let encoded_info = routing_data
        .routing_info
        .encode_to_value()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error serializing payment routing info to serde value")?;

    payment_data.set_connector_in_payment_attempt(routing_data.routed_through);

    payment_data.set_merchant_connector_id_in_attempt(routing_data.merchant_connector_id);
    payment_data.set_straight_through_algorithm_in_payment_attempt(encoded_info);

    Ok(decided_connector)
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn connector_selection<F, D>(
    state: &SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    payment_data: &mut D,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let mut routing_data = storage::RoutingData {
        routed_through: payment_data.get_payment_attempt().connector.clone(),

        merchant_connector_id: payment_data
            .get_payment_attempt()
            .merchant_connector_id
            .clone(),
        pre_routing_connector_choice: payment_data.get_pre_routing_result().and_then(
            |pre_routing_results| {
                pre_routing_results
                    .get(&payment_data.get_payment_attempt().payment_method_subtype)
                    .cloned()
            },
        ),

        algorithm_requested: payment_data
            .get_payment_intent()
            .routing_algorithm_id
            .clone(),
    };

    let payment_dsl_input = core_routing::PaymentsDslInput::new(
        None,
        payment_data.get_payment_attempt(),
        payment_data.get_payment_intent(),
        payment_data.get_payment_method_data(),
        payment_data.get_address(),
        None,
        payment_data.get_currency(),
    );

    let decided_connector = decide_connector(
        state.clone(),
        platform,
        business_profile,
        &mut routing_data,
        payment_dsl_input,
        mandate_type,
    )
    .await?;

    payment_data.set_connector_in_payment_attempt(routing_data.routed_through);

    payment_data.set_merchant_connector_id_in_attempt(routing_data.merchant_connector_id);

    Ok(decided_connector)
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v2")]
pub async fn decide_connector(
    state: SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    routing_data: &mut storage::RoutingData,
    payment_dsl_input: core_routing::PaymentsDslInput<'_>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType> {
    // If the connector was already decided previously, use the same connector
    // This is in case of flows like payments_sync, payments_cancel where the successive operations
    // with the connector have to be made using the same connector account.

    let predetermined_info_cloned = routing_data
        .routed_through
        .as_ref()
        .zip(routing_data.merchant_connector_id.as_ref())
        .map(|(cn_ref, mci_ref)| (cn_ref.clone(), mci_ref.clone()));

    match (
        predetermined_info_cloned,
        routing_data.pre_routing_connector_choice.as_ref(),
    ) {
        // Condition 1: Connector was already decided previously
        (Some((owned_connector_name, owned_merchant_connector_id)), _) => {
            api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &owned_connector_name,
                api::GetToken::Connector,
                Some(owned_merchant_connector_id.clone()),
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid connector name received in 'routed_through'")
            .map(|connector_data| {
                routing_data.routed_through = Some(owned_connector_name);
                ConnectorCallType::PreDetermined(connector_data.into())
            })
        }
        // Condition 2: Pre-routing connector choice
        (None, Some(routable_connector_choice)) => {
            let routable_connector_list = match routable_connector_choice {
                storage::PreRoutingConnectorChoice::Single(routable_connector) => {
                    vec![routable_connector.clone()]
                }
                storage::PreRoutingConnectorChoice::Multiple(routable_connector_list) => {
                    routable_connector_list.clone()
                }
            };

            routable_connector_list
                .first()
                .ok_or_else(|| {
                    report!(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                        .attach_printable("No first routable connector in pre_routing_connector_choice")
                })
                .and_then(|first_routable_connector| {
                    routing_data.routed_through = Some(first_routable_connector.connector.to_string());
                    routing_data
                        .merchant_connector_id
                        .clone_from(&first_routable_connector.merchant_connector_id);

                    let pre_routing_connector_data_list_result: RouterResult<Vec<api::ConnectorData>> = routable_connector_list
                        .iter()
                        .map(|connector_choice| {
                            api::ConnectorData::get_connector_by_name(
                                &state.conf.connectors,
                                &connector_choice.connector.to_string(),
                                api::GetToken::Connector,
                                connector_choice.merchant_connector_id.clone(),
                            )
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Invalid connector name received while processing pre_routing_connector_choice")
                        })
                        .collect::<Result<Vec<_>, _>>(); // Collects into RouterResult<Vec<ConnectorData>>

                    pre_routing_connector_data_list_result
                        .and_then(|list| {
                            list.first()
                                .cloned()
                                .ok_or_else(|| {
                                    report!(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                                        .attach_printable("Empty pre_routing_connector_data_list after mapping")
                                })
                                .map(|first_data| ConnectorCallType::PreDetermined(first_data.into()))
                        })
                })
        }
        (None, None) => {
            route_connector_v2_for_payments(
                &state,
                platform,
                business_profile,
                payment_dsl_input,
                routing_data,
                mandate_type,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v1")]
pub async fn decide_connector<F, D>(
    state: SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    payment_data: &mut D,
    request_straight_through: Option<api::routing::StraightThroughAlgorithm>,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<enums::RoutableConnectors>>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    // If the connector was already decided previously, use the same connector
    // This is in case of flows like payments_sync, payments_cancel where the successive operations
    // with the connector have to be made using the same connector account.
    if let Some(ref connector_name) = payment_data.get_payment_attempt().connector {
        // Connector was already decided previously, use the same connector
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            api::GetToken::Connector,
            payment_data
                .get_payment_attempt()
                .merchant_connector_id
                .clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in 'routed_through'")?;

        routing_data.routed_through = Some(connector_name.clone());
        logger::debug!("euclid_routing: predetermined connector present in attempt");
        return Ok(ConnectorCallType::PreDetermined(connector_data.into()));
    }

    if let Some(mandate_connector_details) = payment_data.get_mandate_connector().as_ref() {
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &mandate_connector_details.connector,
            api::GetToken::Connector,
            mandate_connector_details.merchant_connector_id.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in 'routed_through'")?;

        routing_data.routed_through = Some(mandate_connector_details.connector.clone());

        routing_data
            .merchant_connector_id
            .clone_from(&mandate_connector_details.merchant_connector_id);

        logger::debug!("euclid_routing: predetermined mandate connector");
        return Ok(ConnectorCallType::PreDetermined(connector_data.into()));
    }

    if let Some((pre_routing_results, storage_pm_type)) =
        routing_data.routing_info.pre_routing_results.as_ref().zip(
            payment_data
                .get_payment_attempt()
                .payment_method_type
                .as_ref(),
        )
    {
        if let (Some(routable_connector_choice), None) = (
            pre_routing_results.get(storage_pm_type),
            &payment_data.get_token_data(),
        ) {
            let routable_connector_list = match routable_connector_choice {
                storage::PreRoutingConnectorChoice::Single(routable_connector) => {
                    vec![routable_connector.clone()]
                }
                storage::PreRoutingConnectorChoice::Multiple(routable_connector_list) => {
                    routable_connector_list.clone()
                }
            };

            let mut pre_routing_connector_data_list = vec![];

            let first_routable_connector = routable_connector_list
                .first()
                .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)?;

            routing_data.routed_through = Some(first_routable_connector.connector.to_string());

            routing_data
                .merchant_connector_id
                .clone_from(&first_routable_connector.merchant_connector_id);

            for connector_choice in routable_connector_list.clone() {
                let connector_data = api::ConnectorData::get_connector_by_name(
                    &state.conf.connectors,
                    &connector_choice.connector.to_string(),
                    api::GetToken::Connector,
                    connector_choice.merchant_connector_id.clone(),
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Invalid connector name received")?
                .into();

                pre_routing_connector_data_list.push(connector_data);
            }

            #[cfg(feature = "retry")]
            let should_do_retry = retry::config_should_call_gsm(
                &*state.store,
                platform.get_processor().get_account().get_id(),
                business_profile,
            )
            .await;

            #[cfg(feature = "retry")]
            if payment_data.get_payment_attempt().payment_method_type
                == Some(storage_enums::PaymentMethodType::ApplePay)
                && should_do_retry
            {
                let retryable_connector_data = helpers::get_apple_pay_retryable_connectors(
                    &state,
                    platform,
                    payment_data,
                    &pre_routing_connector_data_list,
                    first_routable_connector
                        .merchant_connector_id
                        .clone()
                        .as_ref(),
                    business_profile.clone(),
                )
                .await?;

                if let Some(connector_data_list) = retryable_connector_data {
                    if connector_data_list.len() > 1 {
                        logger::info!("Constructed apple pay retryable connector list");
                        return Ok(ConnectorCallType::Retryable(connector_data_list));
                    }
                }
            }

            logger::debug!("euclid_routing: pre-routing connector present");

            let first_pre_routing_connector_data_list = pre_routing_connector_data_list
                .first()
                .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)?;

            helpers::override_setup_future_usage_to_on_session(&*state.store, payment_data).await?;

            return Ok(ConnectorCallType::PreDetermined(
                first_pre_routing_connector_data_list.clone(),
            ));
        }
    }

    if let Some(routing_algorithm) = request_straight_through {
        let (mut connectors, check_eligibility) = routing::perform_straight_through_routing(
            &routing_algorithm,
            payment_data.get_creds_identifier(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed execution of straight through routing")?;

        payment_data.set_routing_approach_in_attempt(Some(
            common_enums::RoutingApproach::StraightThroughRouting,
        ));

        if check_eligibility {
            let transaction_data = core_routing::PaymentsDslInput::new(
                payment_data.get_setup_mandate(),
                payment_data.get_payment_attempt(),
                payment_data.get_payment_intent(),
                payment_data.get_payment_method_data(),
                payment_data.get_address(),
                payment_data.get_recurring_details(),
                payment_data.get_currency(),
            );

            connectors = routing::perform_eligibility_analysis_with_fallback(
                &state.clone(),
                platform.get_processor().get_key_store(),
                connectors,
                &TransactionData::Payment(transaction_data),
                eligible_connectors,
                business_profile,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed eligibility analysis and fallback")?;
        }

        let connector_data = connectors
            .into_iter()
            .map(|conn| {
                api::ConnectorData::get_connector_by_name(
                    &state.conf.connectors,
                    &conn.connector.to_string(),
                    api::GetToken::Connector,
                    conn.merchant_connector_id.clone(),
                )
                .map(|connector_data| connector_data.into())
            })
            .collect::<CustomResult<Vec<_>, _>>()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid connector name received")?;

        logger::debug!("euclid_routing: straight through connector present");
        return decide_multiplex_connector_for_normal_or_recurring_payment(
            &state,
            payment_data,
            routing_data,
            connector_data,
            mandate_type,
            business_profile.is_connector_agnostic_mit_enabled,
            business_profile.is_network_tokenization_enabled,
        )
        .await;
    }

    if let Some(ref routing_algorithm) = routing_data.routing_info.algorithm {
        let (mut connectors, check_eligibility) = routing::perform_straight_through_routing(
            routing_algorithm,
            payment_data.get_creds_identifier(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed execution of straight through routing")?;

        if check_eligibility {
            let transaction_data = core_routing::PaymentsDslInput::new(
                payment_data.get_setup_mandate(),
                payment_data.get_payment_attempt(),
                payment_data.get_payment_intent(),
                payment_data.get_payment_method_data(),
                payment_data.get_address(),
                payment_data.get_recurring_details(),
                payment_data.get_currency(),
            );

            connectors = routing::perform_eligibility_analysis_with_fallback(
                &state,
                platform.get_processor().get_key_store(),
                connectors,
                &TransactionData::Payment(transaction_data),
                eligible_connectors,
                business_profile,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed eligibility analysis and fallback")?;
        }

        logger::debug!("euclid_routing: single connector present in algorithm data");
        let connector_data = connectors
            .into_iter()
            .map(|conn| {
                api::ConnectorData::get_connector_by_name(
                    &state.conf.connectors,
                    &conn.connector.to_string(),
                    api::GetToken::Connector,
                    conn.merchant_connector_id,
                )
                .map(|connector_data| connector_data.into())
            })
            .collect::<CustomResult<Vec<_>, _>>()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid connector name received")?;

        return decide_multiplex_connector_for_normal_or_recurring_payment(
            &state,
            payment_data,
            routing_data,
            connector_data,
            mandate_type,
            business_profile.is_connector_agnostic_mit_enabled,
            business_profile.is_network_tokenization_enabled,
        )
        .await;
    }

    let new_pd = payment_data.clone();
    let transaction_data = core_routing::PaymentsDslInput::new(
        new_pd.get_setup_mandate(),
        new_pd.get_payment_attempt(),
        new_pd.get_payment_intent(),
        new_pd.get_payment_method_data(),
        new_pd.get_address(),
        new_pd.get_recurring_details(),
        new_pd.get_currency(),
    );

    route_connector_v1_for_payments(
        &state,
        platform,
        business_profile,
        payment_data,
        transaction_data,
        routing_data,
        eligible_connectors,
        mandate_type,
    )
    .await
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn decide_multiplex_connector_for_normal_or_recurring_payment<F: Clone, D>(
    state: &SessionState,
    payment_data: &mut D,
    routing_data: &mut storage::RoutingData,
    connectors: Vec<api::ConnectorRoutingData>,
    mandate_type: Option<api::MandateTransactionType>,
    is_connector_agnostic_mit_enabled: Option<bool>,
    is_network_tokenization_enabled: bool,
) -> RouterResult<ConnectorCallType>
where
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    match (
        payment_data.get_payment_intent().setup_future_usage,
        payment_data.get_token_data().as_ref(),
        payment_data.get_recurring_details().as_ref(),
        payment_data.get_payment_intent().off_session,
        mandate_type,
    ) {
        (
            Some(storage_enums::FutureUsage::OffSession),
            Some(_),
            None,
            None,
            Some(api::MandateTransactionType::RecurringMandateTransaction),
        )
        | (
            None,
            None,
            Some(RecurringDetails::PaymentMethodId(_)),
            Some(true),
            Some(api::MandateTransactionType::RecurringMandateTransaction),
        )
        | (None, Some(_), None, Some(true), _) => {
            logger::debug!("euclid_routing: performing routing for token-based MIT flow");

            let payment_method_info = payment_data
                .get_payment_method_info()
                .get_required_value("payment_method_info")?
                .clone();

            let retryable_connectors =
                join_all(connectors.into_iter().map(|connector_routing_data| {
                    let payment_method = payment_method_info.clone();
                    async move {
                        let action_types = get_all_action_types(
                            state,
                            is_connector_agnostic_mit_enabled,
                            is_network_tokenization_enabled,
                            &payment_method.clone(),
                            connector_routing_data.connector_data.clone(),
                        )
                        .await;

                        action_types
                            .into_iter()
                            .map(|action_type| api::ConnectorRoutingData {
                                connector_data: connector_routing_data.connector_data.clone(),
                                action_type: Some(action_type),
                                network: connector_routing_data.network.clone(),
                            })
                            .collect::<Vec<_>>()
                    }
                }))
                .await
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

            let chosen_connector_routing_data = retryable_connectors
                .first()
                .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                .attach_printable("no eligible connector found for token-based MIT payment")?;

            let mandate_reference_id = get_mandate_reference_id(
                chosen_connector_routing_data.action_type.clone(),
                chosen_connector_routing_data.clone(),
                payment_data,
                &payment_method_info,
            )?;

            routing_data.routed_through = Some(
                chosen_connector_routing_data
                    .connector_data
                    .connector_name
                    .to_string(),
            );

            routing_data.merchant_connector_id.clone_from(
                &chosen_connector_routing_data
                    .connector_data
                    .merchant_connector_id,
            );

            payment_data.set_mandate_id(payments_api::MandateIds {
                mandate_id: None,
                mandate_reference_id,
            });
            Ok(ConnectorCallType::Retryable(retryable_connectors))
        }
        (
            None,
            None,
            Some(RecurringDetails::ProcessorPaymentToken(_token)),
            Some(true),
            Some(api::MandateTransactionType::RecurringMandateTransaction),
        ) => {
            if let Some(connector) = connectors.first() {
                let connector = &connector.connector_data;
                routing_data.routed_through = Some(connector.connector_name.clone().to_string());
                routing_data
                    .merchant_connector_id
                    .clone_from(&connector.merchant_connector_id);
                Ok(ConnectorCallType::PreDetermined(
                    api::ConnectorData {
                        connector: connector.connector.clone(),
                        connector_name: connector.connector_name,
                        get_token: connector.get_token.clone(),
                        merchant_connector_id: connector.merchant_connector_id.clone(),
                    }
                    .into(),
                ))
            } else {
                logger::error!(
                    "euclid_routing: no eligible connector found for the ppt_mandate payment"
                );
                Err(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration.into())
            }
        }
        _ => {
            helpers::override_setup_future_usage_to_on_session(&*state.store, payment_data).await?;

            let first_choice = connectors
                .first()
                .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                .attach_printable("no eligible connector found for payment")?
                .clone();

            routing_data.routed_through =
                Some(first_choice.connector_data.connector_name.to_string());

            routing_data.merchant_connector_id = first_choice.connector_data.merchant_connector_id;

            Ok(ConnectorCallType::Retryable(connectors))
        }
    }
}

#[cfg(feature = "v1")]
pub fn get_mandate_reference_id<F: Clone, D>(
    action_type: Option<ActionType>,
    connector_routing_data: api::ConnectorRoutingData,
    payment_data: &mut D,
    payment_method_info: &domain::PaymentMethod,
) -> RouterResult<Option<api_models::payments::MandateReferenceId>>
where
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let mandate_reference_id = match action_type {
        Some(ActionType::NetworkTokenWithNetworkTransactionId(network_token_data)) => {
            logger::info!("using network token with network_transaction_id for MIT flow");

            Some(payments_api::MandateReferenceId::NetworkTokenWithNTI(
                network_token_data.into(),
            ))
        }
        Some(ActionType::CardWithNetworkTransactionId(network_transaction_id)) => {
            logger::info!("using card with network_transaction_id for MIT flow");

            Some(payments_api::MandateReferenceId::NetworkMandateId(
                network_transaction_id,
            ))
        }
        Some(ActionType::ConnectorMandate(connector_mandate_details)) => {
            logger::info!("using connector_mandate_id for MIT flow");
            let merchant_connector_id = connector_routing_data
                .connector_data
                .merchant_connector_id
                .as_ref()
                .ok_or_else(|| {
                    report!(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                        .attach_printable("No eligible connector found for token-based MIT flow: no connector mandate details")
                })?;

            let mandate_reference_record = connector_mandate_details
                .get(merchant_connector_id)
                .ok_or_else(|| {
                    report!(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                        .attach_printable("No mandate record found for merchant connector ID")
                })?;

            if let Some(mandate_currency) =
                mandate_reference_record.original_payment_authorized_currency
            {
                if mandate_currency != payment_data.get_currency() {
                    return Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                        reason: "Cross currency mandates not supported".into(),
                    }));
                }
            }

            payment_data.set_recurring_mandate_payment_data(mandate_reference_record.into());

            Some(payments_api::MandateReferenceId::ConnectorMandateId(
                api_models::payments::ConnectorMandateReferenceId::new(
                    Some(mandate_reference_record.connector_mandate_id.clone()),
                    Some(payment_method_info.get_id().clone()),
                    None,
                    mandate_reference_record.mandate_metadata.clone(),
                    mandate_reference_record
                        .connector_mandate_request_reference_id
                        .clone(),
                    None,
                ),
            ))
        }
        None => None,
    };
    Ok(mandate_reference_id)
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn decide_connector_for_normal_or_recurring_payment<F: Clone, D>(
    state: &SessionState,
    payment_data: &mut D,
    routing_data: &mut storage::RoutingData,
    connectors: Vec<api::ConnectorRoutingData>,
    is_connector_agnostic_mit_enabled: Option<bool>,
    payment_method_info: &domain::PaymentMethod,
) -> RouterResult<ConnectorCallType>
where
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let connector_common_mandate_details = payment_method_info
        .get_common_mandate_reference()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get the common mandate reference")?;

    let connector_mandate_details = connector_common_mandate_details.payments.clone();

    let mut connector_choice = None;

    for connector_info in connectors {
        let connector_data = connector_info.connector_data;
        let merchant_connector_id = connector_data
            .merchant_connector_id
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to find the merchant connector id")?;
        if connector_mandate_details
            .clone()
            .map(|connector_mandate_details| {
                connector_mandate_details.contains_key(merchant_connector_id)
            })
            .unwrap_or(false)
        {
            logger::info!("euclid_routing: using connector_mandate_id for MIT flow");
            if let Some(merchant_connector_id) = connector_data.merchant_connector_id.as_ref() {
                if let Some(mandate_reference_record) = connector_mandate_details.clone()
                        .get_required_value("connector_mandate_details")
                            .change_context(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                            .attach_printable("no eligible connector found for token-based MIT flow since there were no connector mandate details")?
                            .get(merchant_connector_id)
                        {
                            common_utils::fp_utils::when(
                                mandate_reference_record
                                    .original_payment_authorized_currency
                                    .map(|mandate_currency| mandate_currency != payment_data.get_currency())
                                    .unwrap_or(false),
                                || {
                                    Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                                        reason: "cross currency mandates not supported".into()
                                    }))
                                },
                            )?;
                            let mandate_reference_id = Some(payments_api::MandateReferenceId::ConnectorMandateId(
                                api_models::payments::ConnectorMandateReferenceId::new(
                                    Some(mandate_reference_record.connector_mandate_id.clone()),
                                    Some(payment_method_info.get_id().clone()),
                                    // update_history
                                    None,
                                    mandate_reference_record.mandate_metadata.clone(),
                                    mandate_reference_record.connector_mandate_request_reference_id.clone(),
                                    None
                                )
                            ));
                            payment_data.set_recurring_mandate_payment_data(
                                mandate_reference_record.into(),
                            );
                            connector_choice = Some((connector_data, mandate_reference_id.clone()));
                            break;
                        }
            }
        } else if is_network_transaction_id_flow(
            state,
            is_connector_agnostic_mit_enabled,
            connector_data.connector_name,
            payment_method_info,
        ) {
            logger::info!("using network_transaction_id for MIT flow");
            let network_transaction_id = payment_method_info
                .network_transaction_id
                .as_ref()
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to fetch the network transaction id")?;

            let mandate_reference_id = Some(payments_api::MandateReferenceId::NetworkMandateId(
                network_transaction_id.to_string(),
            ));

            connector_choice = Some((connector_data, mandate_reference_id.clone()));
            break;
        } else {
            continue;
        }
    }

    let (chosen_connector_data, mandate_reference_id) = connector_choice
        .get_required_value("connector_choice")
        .change_context(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
        .attach_printable("no eligible connector found for token-based MIT payment")?;

    routing_data.routed_through = Some(chosen_connector_data.connector_name.to_string());

    routing_data
        .merchant_connector_id
        .clone_from(&chosen_connector_data.merchant_connector_id);

    payment_data.set_mandate_id(payments_api::MandateIds {
        mandate_id: None,
        mandate_reference_id,
    });

    Ok(ConnectorCallType::PreDetermined(
        chosen_connector_data.into(),
    ))
}

pub fn filter_ntid_supported_connectors(
    connectors: Vec<api::ConnectorRoutingData>,
    ntid_supported_connectors: &HashSet<enums::Connector>,
) -> Vec<api::ConnectorRoutingData> {
    connectors
        .into_iter()
        .filter(|data| ntid_supported_connectors.contains(&data.connector_data.connector_name))
        .collect()
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Eq, PartialEq)]
pub struct NetworkTokenExpiry {
    pub token_exp_month: Option<Secret<String>>,
    pub token_exp_year: Option<Secret<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Eq, PartialEq)]
pub struct NTWithNTIRef {
    pub network_transaction_id: String,
    pub token_exp_month: Option<Secret<String>>,
    pub token_exp_year: Option<Secret<String>>,
}

impl From<NTWithNTIRef> for payments_api::NetworkTokenWithNTIRef {
    fn from(network_token_data: NTWithNTIRef) -> Self {
        Self {
            network_transaction_id: network_token_data.network_transaction_id,
            token_exp_month: network_token_data.token_exp_month,
            token_exp_year: network_token_data.token_exp_year,
        }
    }
}

// This represents the recurring details of a connector which will be used for retries
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub enum ActionType {
    NetworkTokenWithNetworkTransactionId(NTWithNTIRef),
    CardWithNetworkTransactionId(String), // Network Transaction Id
    #[cfg(feature = "v1")]
    ConnectorMandate(hyperswitch_domain_models::mandates::PaymentsMandateReference),
}

pub fn filter_network_tokenization_supported_connectors(
    connectors: Vec<api::ConnectorRoutingData>,
    network_tokenization_supported_connectors: &HashSet<enums::Connector>,
) -> Vec<api::ConnectorRoutingData> {
    connectors
        .into_iter()
        .filter(|data| {
            network_tokenization_supported_connectors.contains(&data.connector_data.connector_name)
        })
        .collect()
}

#[cfg(feature = "v1")]
#[derive(Default)]
pub struct ActionTypesBuilder {
    action_types: Vec<ActionType>,
}

#[cfg(feature = "v1")]
impl ActionTypesBuilder {
    pub fn new() -> Self {
        Self {
            action_types: Vec::new(),
        }
    }

    pub fn with_mandate_flow(
        mut self,
        is_mandate_flow: bool,
        connector_mandate_details: Option<
            hyperswitch_domain_models::mandates::PaymentsMandateReference,
        >,
    ) -> Self {
        if is_mandate_flow {
            self.action_types.extend(
                connector_mandate_details
                    .map(|details| ActionType::ConnectorMandate(details.to_owned())),
            );
        }
        self
    }

    pub async fn with_network_tokenization(
        mut self,
        state: &SessionState,
        is_network_token_with_ntid_flow: IsNtWithNtiFlow,
        is_nt_with_ntid_supported_connector: bool,
        payment_method_info: &domain::PaymentMethod,
    ) -> Self {
        match is_network_token_with_ntid_flow {
            IsNtWithNtiFlow::NtWithNtiSupported(network_transaction_id)
                if is_nt_with_ntid_supported_connector =>
            {
                self.action_types.extend(
                    network_tokenization::do_status_check_for_network_token(
                        state,
                        payment_method_info,
                    )
                    .await
                    .inspect_err(|e| {
                        logger::error!("Status check for network token failed: {:?}", e)
                    })
                    .ok()
                    .map(|(token_exp_month, token_exp_year)| {
                        ActionType::NetworkTokenWithNetworkTransactionId(NTWithNTIRef {
                            token_exp_month,
                            token_exp_year,
                            network_transaction_id,
                        })
                    }),
                );
            }
            _ => (),
        }
        self
    }

    pub fn with_card_network_transaction_id(
        mut self,
        is_card_with_ntid_flow: bool,
        payment_method_info: &domain::PaymentMethod,
    ) -> Self {
        if is_card_with_ntid_flow {
            self.action_types.extend(
                payment_method_info
                    .network_transaction_id
                    .as_ref()
                    .map(|ntid| ActionType::CardWithNetworkTransactionId(ntid.clone())),
            );
        }
        self
    }

    pub fn build(self) -> Vec<ActionType> {
        self.action_types
    }
}

#[cfg(feature = "v1")]
pub async fn get_all_action_types(
    state: &SessionState,
    is_connector_agnostic_mit_enabled: Option<bool>,
    is_network_tokenization_enabled: bool,
    payment_method_info: &domain::PaymentMethod,
    connector: api::ConnectorData,
) -> Vec<ActionType> {
    let merchant_connector_id = connector.merchant_connector_id.as_ref();

    //fetch connectors that support ntid flow
    let ntid_supported_connectors = &state
        .conf
        .network_transaction_id_supported_connectors
        .connector_list;

    //fetch connectors that support network tokenization flow
    let network_tokenization_supported_connectors = &state
        .conf
        .network_tokenization_supported_connectors
        .connector_list;

    let is_network_token_with_ntid_flow = is_network_token_with_network_transaction_id_flow(
        is_connector_agnostic_mit_enabled,
        is_network_tokenization_enabled,
        payment_method_info,
    );
    let is_card_with_ntid_flow = is_network_transaction_id_flow(
        state,
        is_connector_agnostic_mit_enabled,
        connector.connector_name,
        payment_method_info,
    );
    let payments_mandate_reference = payment_method_info
        .get_common_mandate_reference()
        .map_err(|err| {
            logger::warn!("Error getting connector mandate details: {:?}", err);
            err
        })
        .ok()
        .and_then(|details| details.payments);

    let is_mandate_flow = payments_mandate_reference
        .clone()
        .zip(merchant_connector_id)
        .map(|(details, merchant_connector_id)| details.contains_key(merchant_connector_id))
        .unwrap_or(false);

    let is_nt_with_ntid_supported_connector = ntid_supported_connectors
        .contains(&connector.connector_name)
        && network_tokenization_supported_connectors.contains(&connector.connector_name);

    ActionTypesBuilder::new()
        .with_mandate_flow(is_mandate_flow, payments_mandate_reference)
        .with_network_tokenization(
            state,
            is_network_token_with_ntid_flow,
            is_nt_with_ntid_supported_connector,
            payment_method_info,
        )
        .await
        .with_card_network_transaction_id(is_card_with_ntid_flow, payment_method_info)
        .build()
}

pub fn is_network_transaction_id_flow(
    state: &SessionState,
    is_connector_agnostic_mit_enabled: Option<bool>,
    connector: enums::Connector,
    payment_method_info: &domain::PaymentMethod,
) -> bool {
    let ntid_supported_connectors = &state
        .conf
        .network_transaction_id_supported_connectors
        .connector_list;

    is_connector_agnostic_mit_enabled == Some(true)
        && payment_method_info.get_payment_method_type() == Some(storage_enums::PaymentMethod::Card)
        && ntid_supported_connectors.contains(&connector)
        && payment_method_info.network_transaction_id.is_some()
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Eq, PartialEq)]
pub enum IsNtWithNtiFlow {
    NtWithNtiSupported(String), //Network token with Network transaction id supported flow
    NTWithNTINotSupported,      //Network token with Network transaction id not supported
}

pub fn is_network_token_with_network_transaction_id_flow(
    is_connector_agnostic_mit_enabled: Option<bool>,
    is_network_tokenization_enabled: bool,
    payment_method_info: &domain::PaymentMethod,
) -> IsNtWithNtiFlow {
    match (
        is_connector_agnostic_mit_enabled,
        is_network_tokenization_enabled,
        payment_method_info.get_payment_method_type(),
        payment_method_info.network_transaction_id.clone(),
        payment_method_info.network_token_locker_id.is_some(),
        payment_method_info
            .network_token_requestor_reference_id
            .is_some(),
    ) {
        (
            Some(true),
            true,
            Some(storage_enums::PaymentMethod::Card),
            Some(network_transaction_id),
            true,
            true,
        ) => IsNtWithNtiFlow::NtWithNtiSupported(network_transaction_id),
        _ => IsNtWithNtiFlow::NTWithNTINotSupported,
    }
}

pub fn should_add_task_to_process_tracker<F: Clone, D: OperationSessionGetters<F>>(
    payment_data: &D,
) -> bool {
    let connector = payment_data.get_payment_attempt().connector.as_deref();

    !matches!(
        (
            payment_data.get_payment_attempt().get_payment_method(),
            connector
        ),
        (
            Some(storage_enums::PaymentMethod::BankTransfer),
            Some("stripe")
        )
    )
}

#[cfg(feature = "v1")]
pub async fn perform_session_token_routing<F, D>(
    state: SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    payment_data: &mut D,
    connectors: api::SessionConnectorDatas,
) -> RouterResult<api::SessionConnectorDatas>
where
    F: Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F>,
{
    let chosen = connectors.apply_filter_for_session_routing();
    let sfr = SessionFlowRoutingInput {
        state: &state,
        country: payment_data
            .get_address()
            .get_payment_method_billing()
            .and_then(|address| address.address.as_ref())
            .and_then(|details| details.country),
        key_store: platform.get_processor().get_key_store(),
        merchant_account: platform.get_processor().get_account(),
        payment_attempt: payment_data.get_payment_attempt(),
        payment_intent: payment_data.get_payment_intent(),
        chosen,
    };
    let (result, routing_approach) = self_routing::perform_session_flow_routing(
        sfr,
        business_profile,
        &enums::TransactionType::Payment,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("error performing session flow routing")?;

    payment_data.set_routing_approach_in_attempt(routing_approach);

    let final_list = connectors.filter_and_validate_for_session_flow(&result)?;

    Ok(final_list)
}

pub struct SessionTokenRoutingResult {
    pub final_result: api::SessionConnectorDatas,
    pub routing_result:
        FxHashMap<common_enums::PaymentMethodType, Vec<api::routing::SessionRoutingChoice>>,
}
#[cfg(feature = "v2")]
pub async fn perform_session_token_routing<F, D>(
    state: SessionState,
    business_profile: &domain::Profile,
    platform: domain::Platform,
    payment_data: &D,
    connectors: api::SessionConnectorDatas,
) -> RouterResult<SessionTokenRoutingResult>
where
    F: Clone,
    D: OperationSessionGetters<F>,
{
    let chosen = connectors.apply_filter_for_session_routing();
    let sfr = SessionFlowRoutingInput {
        country: payment_data
            .get_payment_intent()
            .billing_address
            .as_ref()
            .and_then(|address| address.get_inner().address.as_ref())
            .and_then(|details| details.country),
        payment_intent: payment_data.get_payment_intent(),

        chosen,
    };
    let result = self_routing::perform_session_flow_routing(
        &state,
        platform.get_processor().get_key_store(),
        sfr,
        business_profile,
        &enums::TransactionType::Payment,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("error performing session flow routing")?;

    let final_list = connectors.filter_and_validate_for_session_flow(&result)?;
    Ok(SessionTokenRoutingResult {
        final_result: final_list,
        routing_result: result,
    })
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn route_connector_v2_for_payments(
    state: &SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    transaction_data: core_routing::PaymentsDslInput<'_>,
    routing_data: &mut storage::RoutingData,
    _mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType> {
    let routing_algorithm_id = routing_data
        .algorithm_requested
        .as_ref()
        .or(business_profile.routing_algorithm_id.as_ref());

    let (connectors, _) = routing::perform_static_routing_v1(
        state,
        platform.get_processor().get_account().get_id(),
        routing_algorithm_id,
        business_profile,
        &TransactionData::Payment(transaction_data.clone()),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let connectors = routing::perform_eligibility_analysis_with_fallback(
        &state.clone(),
        platform.get_processor().get_key_store(),
        connectors,
        &TransactionData::Payment(transaction_data),
        None,
        business_profile,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("failed eligibility analysis and fallback")?;

    connectors
        .first()
        .map(|conn| {
            routing_data.routed_through = Some(conn.connector.to_string());
            routing_data.merchant_connector_id = conn.merchant_connector_id.clone();
            api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &conn.connector.to_string(),
                api::GetToken::Connector,
                conn.merchant_connector_id.clone(),
            )
        })
        .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)?
        .map(|connector_data| ConnectorCallType::PreDetermined(connector_data.into()))
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn route_connector_v1_for_payments<F, D>(
    state: &SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    payment_data: &mut D,
    transaction_data: core_routing::PaymentsDslInput<'_>,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<enums::RoutableConnectors>>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let routing_algorithm_id = {
        let routing_algorithm = business_profile.routing_algorithm.clone();

        let algorithm_ref = routing_algorithm
            .map(|ra| ra.parse_value::<api::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not decode merchant routing algorithm ref")?
            .unwrap_or_default();
        algorithm_ref.algorithm_id
    };

    let (connectors, routing_approach) = routing::perform_static_routing_v1(
        state,
        platform.get_processor().get_account().get_id(),
        routing_algorithm_id.as_ref(),
        business_profile,
        &TransactionData::Payment(transaction_data.clone()),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    payment_data.set_routing_approach_in_attempt(routing_approach);

    #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
    let payment_attempt = transaction_data.payment_attempt.clone();

    let connectors = routing::perform_eligibility_analysis_with_fallback(
        &state.clone(),
        platform.get_processor().get_key_store(),
        connectors,
        &TransactionData::Payment(transaction_data),
        eligible_connectors,
        business_profile,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("failed eligibility analysis and fallback")?;

    // dynamic success based connector selection
    #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
    let connectors = if let Some(algo) = business_profile.dynamic_routing_algorithm.clone() {
        let dynamic_routing_config: api_models::routing::DynamicRoutingAlgorithmRef = algo
            .parse_value("DynamicRoutingAlgorithmRef")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to deserialize DynamicRoutingAlgorithmRef from JSON")?;
        let dynamic_split = api_models::routing::RoutingVolumeSplit {
            routing_type: api_models::routing::RoutingType::Dynamic,
            split: dynamic_routing_config
                .dynamic_routing_volume_split
                .unwrap_or_default(),
        };
        let static_split: api_models::routing::RoutingVolumeSplit =
            api_models::routing::RoutingVolumeSplit {
                routing_type: api_models::routing::RoutingType::Static,
                split: consts::DYNAMIC_ROUTING_MAX_VOLUME
                    - dynamic_routing_config
                        .dynamic_routing_volume_split
                        .unwrap_or_default(),
            };
        let volume_split_vec = vec![dynamic_split, static_split];
        let routing_choice = routing::perform_dynamic_routing_volume_split(volume_split_vec, None)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to perform volume split on routing type")?;

        if routing_choice.routing_type.is_dynamic_routing() {
            if state.conf.open_router.dynamic_routing_enabled {
                routing::perform_dynamic_routing_with_open_router(
                    state,
                    connectors.clone(),
                    business_profile,
                    payment_attempt,
                    payment_data,
                )
                .await
                .map_err(|e| logger::error!(open_routing_error=?e))
                .unwrap_or(connectors)
            } else {
                let dynamic_routing_config_params_interpolator =
                    routing_helpers::DynamicRoutingConfigParamsInterpolator::new(
                        payment_data.get_payment_attempt().payment_method,
                        payment_data.get_payment_attempt().payment_method_type,
                        payment_data.get_payment_attempt().authentication_type,
                        payment_data.get_payment_attempt().currency,
                        payment_data
                            .get_billing_address()
                            .and_then(|address| address.address)
                            .and_then(|address| address.country),
                        payment_data
                            .get_payment_attempt()
                            .payment_method_data
                            .as_ref()
                            .and_then(|data| data.as_object())
                            .and_then(|card| card.get("card"))
                            .and_then(|data| data.as_object())
                            .and_then(|card| card.get("card_network"))
                            .and_then(|network| network.as_str())
                            .map(|network| network.to_string()),
                        payment_data
                            .get_payment_attempt()
                            .payment_method_data
                            .as_ref()
                            .and_then(|data| data.as_object())
                            .and_then(|card| card.get("card"))
                            .and_then(|data| data.as_object())
                            .and_then(|card| card.get("card_isin"))
                            .and_then(|card_isin| card_isin.as_str())
                            .map(|card_isin| card_isin.to_string()),
                    );

                routing::perform_dynamic_routing_with_intelligent_router(
                    state,
                    connectors.clone(),
                    business_profile,
                    dynamic_routing_config_params_interpolator,
                    payment_data,
                )
                .await
                .map_err(|e| logger::error!(dynamic_routing_error=?e))
                .unwrap_or(connectors)
            }
        } else {
            connectors
        }
    } else {
        connectors
    };

    let connector_data = connectors
        .into_iter()
        .map(|conn| {
            api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &conn.connector.to_string(),
                api::GetToken::Connector,
                conn.merchant_connector_id,
            )
            .map(|connector_data| connector_data.into())
        })
        .collect::<CustomResult<Vec<_>, _>>()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received")?;

    decide_multiplex_connector_for_normal_or_recurring_payment(
        state,
        payment_data,
        routing_data,
        connector_data,
        mandate_type,
        business_profile.is_connector_agnostic_mit_enabled,
        business_profile.is_network_tokenization_enabled,
    )
    .await
}

#[cfg(feature = "payouts")]
#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn route_connector_v1_for_payouts(
    state: &SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    transaction_data: &payouts::PayoutData,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<enums::RoutableConnectors>>,
) -> RouterResult<ConnectorCallType> {
    todo!()
}

#[cfg(feature = "payouts")]
#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn route_connector_v1_for_payouts(
    state: &SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    transaction_data: &payouts::PayoutData,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<enums::RoutableConnectors>>,
) -> RouterResult<ConnectorCallType> {
    let routing_algorithm_id = {
        let routing_algorithm = business_profile.payout_routing_algorithm.clone();

        let algorithm_ref = routing_algorithm
            .map(|ra| ra.parse_value::<api::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not decode merchant routing algorithm ref")?
            .unwrap_or_default();
        algorithm_ref.algorithm_id
    };

    let (connectors, _) = routing::perform_static_routing_v1(
        state,
        platform.get_processor().get_account().get_id(),
        routing_algorithm_id.as_ref(),
        business_profile,
        &TransactionData::Payout(transaction_data),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let connectors = routing::perform_eligibility_analysis_with_fallback(
        &state.clone(),
        platform.get_processor().get_key_store(),
        connectors,
        &TransactionData::Payout(transaction_data),
        eligible_connectors,
        business_profile,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("failed eligibility analysis and fallback")?;
    let first_connector_choice = connectors
        .first()
        .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
        .attach_printable("Empty connector list returned")?
        .clone();

    let connector_data = connectors
        .into_iter()
        .map(|conn| {
            api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &conn.connector.to_string(),
                api::GetToken::Connector,
                conn.merchant_connector_id,
            )
            .map(|connector_data| connector_data.into())
        })
        .collect::<CustomResult<Vec<_>, _>>()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received")?;

    routing_data.routed_through = Some(first_connector_choice.connector.to_string());

    routing_data.merchant_connector_id = first_connector_choice.merchant_connector_id;

    Ok(ConnectorCallType::Retryable(connector_data))
}

#[cfg(feature = "v2")]
pub async fn payment_external_authentication(
    _state: SessionState,
    _platform: domain::Platform,
    _req: api_models::payments::PaymentsExternalAuthenticationRequest,
) -> RouterResponse<api_models::payments::PaymentsExternalAuthenticationResponse> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn payment_external_authentication<F: Clone + Sync>(
    state: SessionState,
    platform: domain::Platform,
    req: api_models::payments::PaymentsExternalAuthenticationRequest,
) -> RouterResponse<api_models::payments::PaymentsExternalAuthenticationResponse> {
    use super::unified_authentication_service::types::ExternalAuthentication;
    use crate::core::unified_authentication_service::{
        types::UnifiedAuthenticationService, utils::external_authentication_update_trackers,
    };

    let db = &*state.store;

    let merchant_id = platform.get_processor().get_account().get_id();
    let storage_scheme = platform.get_processor().get_account().storage_scheme;
    let payment_id = req.payment_id;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &payment_id,
            merchant_id,
            platform.get_processor().get_key_store(),
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
    let attempt_id = payment_intent.active_attempt.get_id().clone();
    let payment_attempt = db
        .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
            &payment_intent.payment_id,
            merchant_id,
            &attempt_id.clone(),
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
    if payment_attempt.external_three_ds_authentication_attempted != Some(true) {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message:
                "You cannot authenticate this payment because payment_attempt.external_three_ds_authentication_attempted is false".to_owned(),
        })?
    }
    helpers::validate_payment_status_against_allowed_statuses(
        payment_intent.status,
        &[storage_enums::IntentStatus::RequiresCustomerAction],
        "authenticate",
    )?;

    let optional_customer = match &payment_intent.customer_id {
        Some(customer_id) => Some(
            state
                .store
                .find_customer_by_customer_id_merchant_id(
                    customer_id,
                    platform.get_processor().get_account().get_id(),
                    platform.get_processor().get_key_store(),
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable_lazy(|| {
                    format!("error while finding customer with customer_id {customer_id:?}")
                })?,
        ),
        None => None,
    };

    let profile_id = payment_intent
        .profile_id
        .as_ref()
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("'profile_id' not set in payment intent")?;
    let currency = payment_attempt.currency.get_required_value("currency")?;
    let amount = payment_attempt.get_total_amount();
    let shipping_address = helpers::create_or_find_address_for_payment_by_request(
        &state,
        None,
        payment_intent.shipping_address_id.as_deref(),
        merchant_id,
        payment_intent.customer_id.as_ref(),
        platform.get_processor().get_key_store(),
        &payment_intent.payment_id,
        storage_scheme,
    )
    .await?;
    let billing_address = helpers::create_or_find_address_for_payment_by_request(
        &state,
        None,
        payment_attempt
            .payment_method_billing_address_id
            .as_deref()
            .or(payment_intent.billing_address_id.as_deref()),
        merchant_id,
        payment_intent.customer_id.as_ref(),
        platform.get_processor().get_key_store(),
        &payment_intent.payment_id,
        storage_scheme,
    )
    .await?;
    let authentication_connector = payment_attempt
        .authentication_connector
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("authentication_connector not found in payment_attempt")?;
    let merchant_connector_account = helpers::get_merchant_connector_account(
        &state,
        merchant_id,
        None,
        platform.get_processor().get_key_store(),
        profile_id,
        authentication_connector.as_str(),
        None,
    )
    .await?;
    let authentication = db
        .find_authentication_by_merchant_id_authentication_id(
            merchant_id,
            &payment_attempt
                .authentication_id
                .clone()
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("missing authentication_id in payment_attempt")?,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while fetching authentication record")?;

    let business_profile = state
        .store
        .find_business_profile_by_profile_id(platform.get_processor().get_key_store(), profile_id)
        .await
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let payment_method_details = helpers::get_payment_method_details_from_payment_token(
        &state,
        &payment_attempt,
        &payment_intent,
        platform.get_processor().get_key_store(),
        storage_scheme,
    )
    .await?
    .ok_or(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("missing payment_method_details")?;
    let browser_info: Option<BrowserInformation> = payment_attempt
        .browser_info
        .clone()
        .map(|browser_information| browser_information.parse_value("BrowserInformation"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "browser_info",
        })?;
    let payment_connector_name = payment_attempt
        .connector
        .as_ref()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("missing connector in payment_attempt")?;
    let return_url = Some(helpers::create_authorize_url(
        &state.base_url,
        &payment_attempt.clone(),
        payment_connector_name,
    ));
    let mca_id_option = merchant_connector_account.get_mca_id(); // Bind temporary value
    let merchant_connector_account_id_or_connector_name = mca_id_option
        .as_ref()
        .map(|mca_id| mca_id.get_string_repr())
        .unwrap_or(&authentication_connector);

    let webhook_url = helpers::create_webhook_url(
        &state.base_url,
        merchant_id,
        merchant_connector_account_id_or_connector_name,
    );

    let authentication_details = business_profile
        .authentication_connector_details
        .clone()
        .get_required_value("authentication_connector_details")
        .attach_printable("authentication_connector_details not configured by the merchant")?;

    let authentication_response = if helpers::is_merchant_eligible_authentication_service(
        platform.get_processor().get_account().get_id(),
        &state,
    )
    .await?
    {
        let auth_response =
            <ExternalAuthentication as UnifiedAuthenticationService>::authentication(
                &state,
                &business_profile,
                &payment_method_details.1,
                browser_info,
                Some(amount),
                Some(currency),
                authentication::MessageCategory::Payment,
                req.device_channel,
                authentication.clone(),
                return_url,
                req.sdk_information,
                req.threeds_method_comp_ind,
                optional_customer.and_then(|customer| customer.email.map(pii::Email::from)),
                webhook_url,
                &merchant_connector_account,
                &authentication_connector,
                Some(payment_intent.payment_id),
                authentication.force_3ds_challenge,
                authentication.psd2_sca_exemption_type,
            )
            .await?;
        let authentication = external_authentication_update_trackers(
            &state,
            auth_response,
            authentication.clone(),
            None,
            platform.get_processor().get_key_store(),
            None,
            None,
            None,
            None,
        )
        .await?;
        authentication::AuthenticationResponse::try_from(authentication)?
    } else {
        Box::pin(authentication_core::perform_authentication(
            &state,
            business_profile.merchant_id,
            authentication_connector,
            payment_method_details.0,
            payment_method_details.1,
            billing_address
                .as_ref()
                .map(|address| address.into())
                .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "billing_address",
                })?,
            shipping_address.as_ref().map(|address| address.into()),
            browser_info,
            merchant_connector_account,
            Some(amount),
            Some(currency),
            authentication::MessageCategory::Payment,
            req.device_channel,
            authentication,
            return_url,
            req.sdk_information,
            req.threeds_method_comp_ind,
            optional_customer.and_then(|customer| customer.email.map(pii::Email::from)),
            webhook_url,
            authentication_details.three_ds_requestor_url.clone(),
            payment_intent.psd2_sca_exemption_type,
            payment_intent.payment_id,
            payment_intent.force_3ds_challenge_trigger.unwrap_or(false),
            platform.get_processor().get_key_store(),
        ))
        .await?
    };
    Ok(services::ApplicationResponse::Json(
        api_models::payments::PaymentsExternalAuthenticationResponse {
            transaction_status: authentication_response.trans_status,
            acs_url: authentication_response
                .acs_url
                .as_ref()
                .map(ToString::to_string),
            challenge_request: authentication_response.challenge_request,
            // If challenge_request_key is None, we send "creq" as a static value which is standard 3DS challenge form field name
            challenge_request_key: authentication_response
                .challenge_request_key
                .or(Some(consts::CREQ_CHALLENGE_REQUEST_KEY.to_string())),
            acs_reference_number: authentication_response.acs_reference_number,
            acs_trans_id: authentication_response.acs_trans_id,
            three_dsserver_trans_id: authentication_response.three_dsserver_trans_id,
            acs_signed_content: authentication_response.acs_signed_content,
            three_ds_requestor_url: authentication_details.three_ds_requestor_url,
            three_ds_requestor_app_url: authentication_details.three_ds_requestor_app_url,
        },
    ))
}

#[instrument(skip_all)]
#[cfg(feature = "v2")]
pub async fn payment_start_redirection(
    state: SessionState,
    platform: domain::Platform,
    req: api_models::payments::PaymentStartRedirectionRequest,
) -> RouterResponse<serde_json::Value> {
    let db = &*state.store;

    let storage_scheme = platform.get_processor().get_account().storage_scheme;

    let payment_intent = db
        .find_payment_intent_by_id(
            &req.id,
            platform.get_processor().get_key_store(),
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    //TODO: send valid html error pages in this case, or atleast redirect to valid html error pages
    utils::when(
        payment_intent.status != storage_enums::IntentStatus::RequiresCustomerAction,
        || {
            Err(errors::ApiErrorResponse::PaymentUnexpectedState {
                current_flow: "PaymentStartRedirection".to_string(),
                field_name: "status".to_string(),
                current_value: payment_intent.status.to_string(),
                states: ["requires_customer_action".to_string()].join(", "),
            })
        },
    )?;

    let payment_attempt = db
        .find_payment_attempt_by_id(
            platform.get_processor().get_key_store(),
            payment_intent
                .active_attempt_id
                .as_ref()
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("missing active attempt in payment_intent")?,
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while fetching payment_attempt")?;
    let redirection_data = payment_attempt
        .redirection_data
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("missing authentication_data in payment_attempt")?;

    Ok(services::ApplicationResponse::Form(Box::new(
        services::RedirectionFormData {
            redirect_form: redirection_data,
            payment_method_data: None,
            amount: payment_attempt.amount_details.get_net_amount().to_string(),
            currency: payment_intent.amount_details.currency.to_string(),
        },
    )))
}

#[instrument(skip_all)]
pub async fn get_extended_card_info(
    state: SessionState,
    merchant_id: id_type::MerchantId,
    payment_id: id_type::PaymentId,
) -> RouterResponse<payments_api::ExtendedCardInfoResponse> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let key = helpers::get_redis_key_for_extended_card_info(&merchant_id, &payment_id);
    let payload = redis_conn
        .get_key::<String>(&key.into())
        .await
        .change_context(errors::ApiErrorResponse::ExtendedCardInfoNotFound)?;

    Ok(services::ApplicationResponse::Json(
        payments_api::ExtendedCardInfoResponse { payload },
    ))
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn payments_manual_update(
    state: SessionState,
    req: api_models::payments::PaymentsManualUpdateRequest,
) -> RouterResponse<api_models::payments::PaymentsManualUpdateResponse> {
    let api_models::payments::PaymentsManualUpdateRequest {
        payment_id,
        attempt_id,
        merchant_id,
        attempt_status,
        error_code,
        error_message,
        error_reason,
        connector_transaction_id,
        amount_capturable,
    } = req;
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        .attach_printable("Error while fetching the key store by merchant_id")?;
    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        .attach_printable("Error while fetching the merchant_account by merchant_id")?;
    let payment_attempt = state
        .store
        .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
            &payment_id,
            &merchant_id,
            &attempt_id.clone(),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable(
            "Error while fetching the payment_attempt by payment_id, merchant_id and attempt_id",
        )?;

    if let Some(amount_capturable) = amount_capturable {
        utils::when(
            amount_capturable > payment_attempt.net_amount.get_total_amount(),
            || {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "amount_capturable should be less than or equal to amount".to_string(),
                })
            },
        )?;
    }

    let payment_intent = state
        .store
        .find_payment_intent_by_payment_id_merchant_id(
            &payment_id,
            merchant_account.get_id(),
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable("Error while fetching the payment_intent by payment_id, merchant_id")?;

    let option_gsm = if let Some(((code, message), connector_name)) = error_code
        .as_ref()
        .zip(error_message.as_ref())
        .zip(payment_attempt.connector.as_ref())
    {
        helpers::get_gsm_record(
            &state,
            Some(code.to_string()),
            Some(message.to_string()),
            connector_name.to_string(),
            // We need to get the unified_code and unified_message of the Authorize flow
            "Authorize".to_string(),
        )
        .await
    } else {
        None
    };
    // Update the payment_attempt
    let attempt_update = storage::PaymentAttemptUpdate::ManualUpdate {
        status: attempt_status,
        error_code,
        error_message,
        error_reason,
        updated_by: merchant_account.storage_scheme.to_string(),
        unified_code: option_gsm.as_ref().and_then(|gsm| gsm.unified_code.clone()),
        unified_message: option_gsm.and_then(|gsm| gsm.unified_message),
        connector_transaction_id,
        amount_capturable,
    };
    let updated_payment_attempt = state
        .store
        .update_payment_attempt_with_attempt_id(
            payment_attempt.clone(),
            attempt_update,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        .attach_printable("Error while updating the payment_attempt")?;
    // If the payment_attempt is active attempt for an intent, update the intent status
    if payment_intent.active_attempt.get_id() == payment_attempt.attempt_id {
        let intent_status = enums::IntentStatus::foreign_from(updated_payment_attempt.status);
        let payment_intent_update = storage::PaymentIntentUpdate::ManualUpdate {
            status: Some(intent_status),
            updated_by: merchant_account.storage_scheme.to_string(),
        };
        state
            .store
            .update_payment_intent(
                payment_intent,
                payment_intent_update,
                &key_store,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Error while updating payment_intent")?;
    }
    Ok(services::ApplicationResponse::Json(
        api_models::payments::PaymentsManualUpdateResponse {
            payment_id: updated_payment_attempt.payment_id,
            attempt_id: updated_payment_attempt.attempt_id,
            merchant_id: updated_payment_attempt.merchant_id,
            attempt_status: updated_payment_attempt.status,
            error_code: updated_payment_attempt.error_code,
            error_message: updated_payment_attempt.error_message,
            error_reason: updated_payment_attempt.error_reason,
            connector_transaction_id: updated_payment_attempt.connector_transaction_id,
            amount_capturable: Some(updated_payment_attempt.amount_capturable),
        },
    ))
}

// Trait for Eligibility Checks
#[cfg(feature = "v1")]
#[async_trait::async_trait]
trait EligibilityCheck {
    type Output;

    // Determine if the check should be run based on the runtime checks
    async fn should_run(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
    ) -> CustomResult<bool, errors::ApiErrorResponse>;

    // Run the actual check and return the SDK Next Action if applicable
    async fn execute_check(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        payment_elgibility_data: &PaymentEligibilityData,
        business_profile: &domain::Profile,
    ) -> CustomResult<Self::Output, errors::ApiErrorResponse>;

    fn transform(output: Self::Output) -> Option<api_models::payments::SdkNextAction>;
}

// Result of an Eligibility Check
#[cfg(feature = "v1")]
#[derive(Debug, Clone)]
pub enum CheckResult {
    Allow,
    Deny { message: String },
}

#[cfg(feature = "v1")]
impl From<CheckResult> for Option<api_models::payments::SdkNextAction> {
    fn from(result: CheckResult) -> Self {
        match result {
            CheckResult::Allow => None,
            CheckResult::Deny { message } => Some(api_models::payments::SdkNextAction {
                next_action: api_models::payments::NextActionCall::Deny { message },
            }),
        }
    }
}

// Perform Blocklist Check for the Card Number provided in Payment Method Data
#[cfg(feature = "v1")]
struct BlockListCheck;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl EligibilityCheck for BlockListCheck {
    type Output = CheckResult;

    async fn should_run(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        let merchant_id = platform.get_processor().get_account().get_id();
        let blocklist_enabled_key = merchant_id.get_blocklist_guard_key();
        let blocklist_guard_enabled = state
            .store
            .find_config_by_key_unwrap_or(&blocklist_enabled_key, Some("false".to_string()))
            .await;

        Ok(match blocklist_guard_enabled {
            Ok(config) => serde_json::from_str(&config.config).unwrap_or(false),

            // If it is not present in db we are defaulting it to false
            Err(inner) => {
                if !inner.current_context().is_db_not_found() {
                    logger::error!("Error fetching guard blocklist enabled config {:?}", inner);
                }
                false
            }
        })
    }

    async fn execute_check(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        payment_elgibility_data: &PaymentEligibilityData,
        _business_profile: &domain::Profile,
    ) -> CustomResult<CheckResult, errors::ApiErrorResponse> {
        let should_payment_be_blocked = blocklist_utils::should_payment_be_blocked(
            state,
            platform,
            &payment_elgibility_data.payment_method_data,
        )
        .await?;
        if should_payment_be_blocked {
            Ok(CheckResult::Deny {
                message: "Card number is blocklisted".to_string(),
            })
        } else {
            Ok(CheckResult::Allow)
        }
    }

    fn transform(output: CheckResult) -> Option<api_models::payments::SdkNextAction> {
        output.into()
    }
}

// Perform Card Testing Gaurd Check
#[cfg(feature = "v1")]
struct CardTestingCheck;

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl EligibilityCheck for CardTestingCheck {
    type Output = CheckResult;

    async fn should_run(
        &self,
        _state: &SessionState,
        _platform: &domain::Platform,
    ) -> CustomResult<bool, errors::ApiErrorResponse> {
        // This check is always run as there is no runtime config enablement
        Ok(true)
    }

    async fn execute_check(
        &self,
        state: &SessionState,
        _platform: &domain::Platform,
        payment_elgibility_data: &PaymentEligibilityData,
        business_profile: &domain::Profile,
    ) -> CustomResult<CheckResult, errors::ApiErrorResponse> {
        match &payment_elgibility_data.payment_method_data {
            Some(domain::PaymentMethodData::Card(card)) => {
                match card_testing_guard_utils::validate_card_testing_guard_checks(
                    state,
                    payment_elgibility_data
                        .browser_info
                        .as_ref()
                        .map(|browser_info| browser_info.peek()),
                    card.card_number.clone(),
                    &payment_elgibility_data.payment_intent.customer_id,
                    business_profile,
                )
                .await
                {
                    // If validation succeeds, allow the payment
                    Ok(_) => Ok(CheckResult::Allow),
                    // If validation fails, check the error type
                    Err(e) => match e.current_context() {
                        // If it's a PreconditionFailed error, deny with message
                        errors::ApiErrorResponse::PreconditionFailed { message } => {
                            Ok(CheckResult::Deny {
                                message: message.to_string(),
                            })
                        }
                        // For any other error, propagate it
                        _ => Err(e),
                    },
                }
            }
            // If payment method is not card, allow
            _ => Ok(CheckResult::Allow),
        }
    }

    fn transform(output: CheckResult) -> Option<api_models::payments::SdkNextAction> {
        output.into()
    }
}

// Eligibility Pipeline to run all the eligibility checks in sequence
#[cfg(feature = "v1")]
pub struct EligibilityHandler {
    state: SessionState,
    platform: domain::Platform,
    payment_eligibility_data: PaymentEligibilityData,
    business_profile: domain::Profile,
}

#[cfg(feature = "v1")]
impl EligibilityHandler {
    fn new(
        state: SessionState,
        platform: domain::Platform,
        payment_eligibility_data: PaymentEligibilityData,
        business_profile: domain::Profile,
    ) -> Self {
        Self {
            state,
            platform,
            payment_eligibility_data,
            business_profile,
        }
    }

    async fn run_check<C: EligibilityCheck>(
        &self,
        check: C,
    ) -> CustomResult<Option<api_models::payments::SdkNextAction>, errors::ApiErrorResponse> {
        let should_run = check.should_run(&self.state, &self.platform).await?;
        Ok(match should_run {
            true => check
                .execute_check(
                    &self.state,
                    &self.platform,
                    &self.payment_eligibility_data,
                    &self.business_profile,
                )
                .await
                .map(C::transform)?,
            false => None,
        })
    }
}

#[cfg(all(feature = "oltp", feature = "v1"))]
pub async fn payments_submit_eligibility(
    state: SessionState,
    platform: domain::Platform,
    req: api_models::payments::PaymentsEligibilityRequest,
    payment_id: id_type::PaymentId,
) -> RouterResponse<api_models::payments::PaymentsEligibilityResponse> {
    let payment_eligibility_data =
        PaymentEligibilityData::from_request(&state, &platform, &req).await?;
    let profile_id = payment_eligibility_data
        .payment_intent
        .profile_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("'profile_id' not set in payment intent")?;
    let business_profile = state
        .store
        .find_business_profile_by_profile_id(platform.get_processor().get_key_store(), &profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;
    let eligibility_handler =
        EligibilityHandler::new(state, platform, payment_eligibility_data, business_profile);
    // Run the checks in sequence, short-circuiting on the first that returns a next action
    let sdk_next_action = eligibility_handler
        .run_check(BlockListCheck)
        .await
        .transpose()
        .async_or_else(|| async {
            eligibility_handler
                .run_check(CardTestingCheck)
                .await
                .transpose()
        })
        .await
        .transpose()?
        .unwrap_or(api_models::payments::SdkNextAction {
            next_action: api_models::payments::NextActionCall::Confirm,
        });
    Ok(services::ApplicationResponse::Json(
        api_models::payments::PaymentsEligibilityResponse {
            payment_id,
            sdk_next_action,
        },
    ))
}

pub trait PaymentMethodChecker<F> {
    fn should_update_in_post_update_tracker(&self) -> bool;
    fn should_update_in_update_tracker(&self) -> bool;
}
#[cfg(feature = "v1")]
impl<F: Clone> PaymentMethodChecker<F> for PaymentData<F> {
    fn should_update_in_post_update_tracker(&self) -> bool {
        let payment_method_type = self
            .payment_intent
            .tax_details
            .as_ref()
            .and_then(|tax_details| tax_details.payment_method_type.as_ref().map(|pmt| pmt.pmt));

        matches!(
            payment_method_type,
            Some(storage_enums::PaymentMethodType::Paypal)
        )
    }

    fn should_update_in_update_tracker(&self) -> bool {
        let payment_method_type = self
            .payment_intent
            .tax_details
            .as_ref()
            .and_then(|tax_details| tax_details.payment_method_type.as_ref().map(|pmt| pmt.pmt));

        matches!(
            payment_method_type,
            Some(storage_enums::PaymentMethodType::ApplePay)
                | Some(storage_enums::PaymentMethodType::GooglePay)
        )
    }
}

pub trait OperationSessionGetters<F> {
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt;
    #[cfg(feature = "v2")]
    fn list_payments_attempts(&self) -> &Vec<storage::PaymentAttempt>;
    fn get_payment_intent(&self) -> &storage::PaymentIntent;
    #[cfg(feature = "v2")]
    fn get_client_secret(&self) -> &Option<Secret<String>>;
    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod>;
    fn get_payment_method_token(&self) -> Option<&PaymentMethodToken>;
    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds>;
    fn get_address(&self) -> &PaymentAddress;
    fn get_creds_identifier(&self) -> Option<&str>;
    fn get_token(&self) -> Option<&str>;
    fn get_multiple_capture_data(&self) -> Option<&types::MultipleCaptureData>;
    fn get_payment_link_data(&self) -> Option<api_models::payments::PaymentLinkResponse>;
    fn get_ephemeral_key(&self) -> Option<ephemeral_key::EphemeralKey>;
    fn get_setup_mandate(&self) -> Option<&MandateData>;
    fn get_poll_config(&self) -> Option<router_types::PollConfig>;
    fn get_authentication(
        &self,
    ) -> Option<&hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore>;
    fn get_frm_message(&self) -> Option<FraudCheck>;
    fn get_refunds(&self) -> Vec<diesel_refund::Refund>;
    fn get_disputes(&self) -> Vec<storage::Dispute>;
    fn get_authorizations(&self) -> Vec<diesel_models::authorization::Authorization>;
    fn get_attempts(&self) -> Option<Vec<storage::PaymentAttempt>>;
    fn get_recurring_details(&self) -> Option<&RecurringDetails>;
    // TODO: this should be a mandatory field, should we throw an error instead of returning an Option?
    fn get_payment_intent_profile_id(&self) -> Option<&id_type::ProfileId>;
    fn get_currency(&self) -> storage_enums::Currency;
    fn get_amount(&self) -> api::Amount;
    fn get_payment_attempt_connector(&self) -> Option<&str>;
    fn get_billing_address(&self) -> Option<hyperswitch_domain_models::address::Address>;
    fn get_payment_method_data(&self) -> Option<&domain::PaymentMethodData>;
    fn get_sessions_token(&self) -> Vec<api::SessionToken>;
    fn get_token_data(&self) -> Option<&storage::PaymentTokenData>;
    fn get_mandate_connector(&self) -> Option<&MandateConnectorDetails>;
    fn get_force_sync(&self) -> Option<bool>;
    #[cfg(feature = "v1")]
    fn get_all_keys_required(&self) -> Option<bool>;
    fn get_capture_method(&self) -> Option<enums::CaptureMethod>;
    fn get_merchant_connector_id_in_attempt(&self) -> Option<id_type::MerchantConnectorAccountId>;
    #[cfg(feature = "v2")]
    fn get_merchant_connector_details(
        &self,
    ) -> Option<common_types::domain::MerchantConnectorAuthDetails>;

    fn get_connector_customer_id(&self) -> Option<String>;

    #[cfg(feature = "v1")]
    fn get_whole_connector_response(&self) -> Option<Secret<String>>;

    #[cfg(feature = "v1")]
    fn get_vault_operation(&self) -> Option<&domain_payments::VaultOperation>;

    #[cfg(feature = "v2")]
    fn get_optional_payment_attempt(&self) -> Option<&storage::PaymentAttempt>;

    #[cfg(feature = "v2")]
    fn get_pre_routing_result(
        &self,
    ) -> Option<HashMap<enums::PaymentMethodType, domain::PreRoutingConnectorChoice>>;

    #[cfg(feature = "v2")]
    fn get_optional_external_vault_session_details(&self) -> Option<api::VaultSessionDetails>;
    #[cfg(feature = "v1")]
    fn get_click_to_pay_service_details(&self) -> Option<&api_models::payments::CtpServiceDetails>;

    #[cfg(feature = "v1")]
    fn get_is_manual_retry_enabled(&self) -> Option<bool>;
}

pub trait OperationSessionSetters<F> {
    // Setter functions for PaymentData
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent);
    #[cfg(feature = "v2")]
    fn set_client_secret(&mut self, client_secret: Option<Secret<String>>);
    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt);
    fn set_payment_method_data(&mut self, payment_method_data: Option<domain::PaymentMethodData>);
    fn set_payment_method_token(&mut self, payment_method_token: Option<PaymentMethodToken>);
    fn set_email_if_not_present(&mut self, email: pii::Email);
    fn set_payment_method_id_in_attempt(&mut self, payment_method_id: Option<String>);
    fn set_pm_token(&mut self, token: String);
    fn set_connector_customer_id(&mut self, customer_id: Option<String>);
    fn push_sessions_token(&mut self, token: api::SessionToken);
    fn set_surcharge_details(&mut self, surcharge_details: Option<types::SurchargeDetails>);
    fn set_merchant_connector_id_in_attempt(
        &mut self,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    );
    fn set_card_network(&mut self, card_network: enums::CardNetwork);
    fn set_co_badged_card_data(
        &mut self,
        debit_routing_output: &api_models::open_router::DebitRoutingOutput,
    );
    #[cfg(feature = "v1")]
    fn set_capture_method_in_attempt(&mut self, capture_method: enums::CaptureMethod);
    fn set_frm_message(&mut self, frm_message: FraudCheck);
    fn set_payment_intent_status(&mut self, status: storage_enums::IntentStatus);
    fn set_authentication_type_in_attempt(
        &mut self,
        authentication_type: Option<enums::AuthenticationType>,
    );
    fn set_recurring_mandate_payment_data(
        &mut self,
        recurring_mandate_payment_data:
            hyperswitch_domain_models::router_data::RecurringMandatePaymentData,
    );
    fn set_mandate_id(&mut self, mandate_id: api_models::payments::MandateIds);
    fn set_setup_future_usage_in_payment_intent(
        &mut self,
        setup_future_usage: storage_enums::FutureUsage,
    );

    #[cfg(feature = "v1")]
    fn set_straight_through_algorithm_in_payment_attempt(
        &mut self,
        straight_through_algorithm: serde_json::Value,
    );

    #[cfg(feature = "v2")]
    fn set_prerouting_algorithm_in_payment_intent(
        &mut self,
        straight_through_algorithm: storage::PaymentRoutingInfo,
    );

    fn set_connector_in_payment_attempt(&mut self, connector: Option<String>);

    #[cfg(feature = "v1")]
    fn set_vault_operation(&mut self, vault_operation: domain_payments::VaultOperation);

    #[cfg(feature = "v2")]
    fn set_connector_request_reference_id(&mut self, reference_id: Option<String>);

    fn set_connector_response_reference_id(&mut self, reference_id: Option<String>);

    #[cfg(feature = "v2")]
    fn set_vault_session_details(
        &mut self,
        external_vault_session_details: Option<api::VaultSessionDetails>,
    );
    fn set_routing_approach_in_attempt(&mut self, routing_approach: Option<enums::RoutingApproach>);

    fn set_connector_request_reference_id_in_payment_attempt(
        &mut self,
        connector_request_reference_id: String,
    );
    #[cfg(feature = "v2")]
    fn set_cancellation_reason(&mut self, cancellation_reason: Option<String>);
}

#[cfg(feature = "v1")]
impl<F: Clone> OperationSessionGetters<F> for PaymentData<F> {
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        &self.payment_attempt
    }

    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
        self.payment_method_info.as_ref()
    }

    fn get_payment_method_token(&self) -> Option<&PaymentMethodToken> {
        self.payment_method_token.as_ref()
    }

    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds> {
        self.mandate_id.as_ref()
    }

    // what is this address find out and not required remove this
    fn get_address(&self) -> &PaymentAddress {
        &self.address
    }
    fn get_merchant_connector_id_in_attempt(&self) -> Option<id_type::MerchantConnectorAccountId> {
        self.payment_attempt.merchant_connector_id.clone()
    }

    fn get_creds_identifier(&self) -> Option<&str> {
        self.creds_identifier.as_deref()
    }

    fn get_token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    fn get_multiple_capture_data(&self) -> Option<&types::MultipleCaptureData> {
        self.multiple_capture_data.as_ref()
    }

    fn get_payment_link_data(&self) -> Option<api_models::payments::PaymentLinkResponse> {
        self.payment_link_data.clone()
    }

    fn get_ephemeral_key(&self) -> Option<ephemeral_key::EphemeralKey> {
        self.ephemeral_key.clone()
    }

    fn get_setup_mandate(&self) -> Option<&MandateData> {
        self.setup_mandate.as_ref()
    }

    fn get_poll_config(&self) -> Option<router_types::PollConfig> {
        self.poll_config.clone()
    }

    fn get_authentication(
        &self,
    ) -> Option<&hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore>
    {
        self.authentication.as_ref()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        self.frm_message.clone()
    }

    fn get_refunds(&self) -> Vec<diesel_refund::Refund> {
        self.refunds.clone()
    }

    fn get_disputes(&self) -> Vec<storage::Dispute> {
        self.disputes.clone()
    }

    fn get_authorizations(&self) -> Vec<diesel_models::authorization::Authorization> {
        self.authorizations.clone()
    }

    fn get_attempts(&self) -> Option<Vec<storage::PaymentAttempt>> {
        self.attempts.clone()
    }

    fn get_recurring_details(&self) -> Option<&RecurringDetails> {
        self.recurring_details.as_ref()
    }

    #[cfg(feature = "v1")]
    fn get_payment_intent_profile_id(&self) -> Option<&id_type::ProfileId> {
        self.payment_intent.profile_id.as_ref()
    }

    #[cfg(feature = "v2")]
    fn get_payment_intent_profile_id(&self) -> Option<&id_type::ProfileId> {
        Some(&self.payment_intent.profile_id)
    }

    fn get_currency(&self) -> storage_enums::Currency {
        self.currency
    }

    fn get_amount(&self) -> api::Amount {
        self.amount
    }

    fn get_payment_attempt_connector(&self) -> Option<&str> {
        self.payment_attempt.connector.as_deref()
    }

    fn get_billing_address(&self) -> Option<hyperswitch_domain_models::address::Address> {
        self.address.get_payment_method_billing().cloned()
    }

    fn get_payment_method_data(&self) -> Option<&domain::PaymentMethodData> {
        self.payment_method_data.as_ref()
    }

    fn get_sessions_token(&self) -> Vec<api::SessionToken> {
        self.sessions_token.clone()
    }

    fn get_token_data(&self) -> Option<&storage::PaymentTokenData> {
        self.token_data.as_ref()
    }

    fn get_mandate_connector(&self) -> Option<&MandateConnectorDetails> {
        self.mandate_connector.as_ref()
    }

    fn get_force_sync(&self) -> Option<bool> {
        self.force_sync
    }

    fn get_all_keys_required(&self) -> Option<bool> {
        self.all_keys_required
    }

    fn get_whole_connector_response(&self) -> Option<Secret<String>> {
        self.whole_connector_response.clone()
    }

    #[cfg(feature = "v1")]
    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        self.payment_attempt.capture_method
    }

    #[cfg(feature = "v1")]
    fn get_vault_operation(&self) -> Option<&domain_payments::VaultOperation> {
        self.vault_operation.as_ref()
    }

    fn get_connector_customer_id(&self) -> Option<String> {
        self.connector_customer_id.clone()
    }

    fn get_click_to_pay_service_details(&self) -> Option<&api_models::payments::CtpServiceDetails> {
        self.service_details.as_ref()
    }

    fn get_is_manual_retry_enabled(&self) -> Option<bool> {
        self.is_manual_retry_enabled
    }

    // #[cfg(feature = "v2")]
    // fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
    //     Some(self.payment_intent.capture_method)
    // }

    // #[cfg(feature = "v2")]
    // fn get_optional_payment_attempt(&self) -> Option<&storage::PaymentAttempt> {
    //     todo!();
    // }
}

#[cfg(feature = "v1")]
impl<F: Clone> OperationSessionSetters<F> for PaymentData<F> {
    // Setters Implementation
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }

    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt) {
        self.payment_attempt = payment_attempt;
    }

    fn set_payment_method_data(&mut self, payment_method_data: Option<domain::PaymentMethodData>) {
        self.payment_method_data = payment_method_data;
    }

    fn set_payment_method_token(&mut self, payment_method_token: Option<PaymentMethodToken>) {
        self.payment_method_token = payment_method_token;
    }

    fn set_payment_method_id_in_attempt(&mut self, payment_method_id: Option<String>) {
        self.payment_attempt.payment_method_id = payment_method_id;
    }

    fn set_email_if_not_present(&mut self, email: pii::Email) {
        self.email = self.email.clone().or(Some(email));
    }

    fn set_pm_token(&mut self, token: String) {
        self.pm_token = Some(token);
    }

    fn set_connector_customer_id(&mut self, customer_id: Option<String>) {
        self.connector_customer_id = customer_id;
    }

    fn push_sessions_token(&mut self, token: api::SessionToken) {
        self.sessions_token.push(token);
    }

    fn set_surcharge_details(&mut self, surcharge_details: Option<types::SurchargeDetails>) {
        self.surcharge_details = surcharge_details;
    }

    fn set_merchant_connector_id_in_attempt(
        &mut self,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    ) {
        self.payment_attempt.merchant_connector_id = merchant_connector_id;
    }

    fn set_card_network(&mut self, card_network: enums::CardNetwork) {
        match &mut self.payment_method_data {
            Some(domain::PaymentMethodData::Card(card)) => {
                logger::debug!("Setting card network: {:?}", card_network);
                card.card_network = Some(card_network);
            }
            Some(domain::PaymentMethodData::Wallet(wallet_data)) => match wallet_data {
                hyperswitch_domain_models::payment_method_data::WalletData::ApplePay(wallet) => {
                    logger::debug!("Setting Apple Pay card network: {:?}", card_network);
                    wallet.payment_method.network = card_network.to_string();
                }
                _ => {
                    logger::debug!("Wallet type does not support setting card network.");
                }
            },
            _ => {
                logger::warn!("Payment method data does not support setting card network.");
            }
        }
    }

    fn set_co_badged_card_data(
        &mut self,
        debit_routing_output: &api_models::open_router::DebitRoutingOutput,
    ) {
        let co_badged_card_data =
            api_models::payment_methods::CoBadgedCardData::from(debit_routing_output);
        let card_type = debit_routing_output
            .card_type
            .clone()
            .to_string()
            .to_uppercase();
        if let Some(domain::PaymentMethodData::Card(card)) = &mut self.payment_method_data {
            card.co_badged_card_data = Some(co_badged_card_data);
            card.card_type = Some(card_type);
            logger::debug!("set co-badged card data in payment method data");
        };
    }

    #[cfg(feature = "v1")]
    fn set_capture_method_in_attempt(&mut self, capture_method: enums::CaptureMethod) {
        self.payment_attempt.capture_method = Some(capture_method);
    }

    fn set_frm_message(&mut self, frm_message: FraudCheck) {
        self.frm_message = Some(frm_message);
    }

    fn set_payment_intent_status(&mut self, status: storage_enums::IntentStatus) {
        self.payment_intent.status = status;
    }

    fn set_authentication_type_in_attempt(
        &mut self,
        authentication_type: Option<enums::AuthenticationType>,
    ) {
        self.payment_attempt.authentication_type = authentication_type;
    }

    fn set_recurring_mandate_payment_data(
        &mut self,
        recurring_mandate_payment_data:
            hyperswitch_domain_models::router_data::RecurringMandatePaymentData,
    ) {
        self.recurring_mandate_payment_data = Some(recurring_mandate_payment_data);
    }

    fn set_mandate_id(&mut self, mandate_id: api_models::payments::MandateIds) {
        self.mandate_id = Some(mandate_id);
    }

    #[cfg(feature = "v1")]
    fn set_setup_future_usage_in_payment_intent(
        &mut self,
        setup_future_usage: storage_enums::FutureUsage,
    ) {
        self.payment_intent.setup_future_usage = Some(setup_future_usage);
    }

    #[cfg(feature = "v2")]
    fn set_setup_future_usage_in_payment_intent(
        &mut self,
        setup_future_usage: storage_enums::FutureUsage,
    ) {
        self.payment_intent.setup_future_usage = setup_future_usage;
    }

    #[cfg(feature = "v1")]
    fn set_straight_through_algorithm_in_payment_attempt(
        &mut self,
        straight_through_algorithm: serde_json::Value,
    ) {
        self.payment_attempt.straight_through_algorithm = Some(straight_through_algorithm);
    }

    fn set_connector_in_payment_attempt(&mut self, connector: Option<String>) {
        self.payment_attempt.connector = connector;
    }

    fn set_vault_operation(&mut self, vault_operation: domain_payments::VaultOperation) {
        self.vault_operation = Some(vault_operation);
    }

    fn set_routing_approach_in_attempt(
        &mut self,
        routing_approach: Option<enums::RoutingApproach>,
    ) {
        self.payment_attempt.routing_approach = routing_approach;
    }

    fn set_connector_response_reference_id(&mut self, reference_id: Option<String>) {
        self.payment_attempt.connector_response_reference_id = reference_id;
    }

    fn set_connector_request_reference_id_in_payment_attempt(
        &mut self,
        connector_request_reference_id: String,
    ) {
        self.payment_attempt.connector_request_reference_id = Some(connector_request_reference_id);
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentIntentData<F> {
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        todo!()
    }
    #[cfg(feature = "v2")]
    fn list_payments_attempts(&self) -> &Vec<storage::PaymentAttempt> {
        todo!()
    }

    fn get_client_secret(&self) -> &Option<Secret<String>> {
        &self.client_secret
    }

    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
        todo!()
    }

    fn get_payment_method_token(&self) -> Option<&PaymentMethodToken> {
        todo!()
    }

    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds> {
        todo!()
    }

    // what is this address find out and not required remove this
    fn get_address(&self) -> &PaymentAddress {
        todo!()
    }

    fn get_creds_identifier(&self) -> Option<&str> {
        todo!()
    }

    fn get_token(&self) -> Option<&str> {
        todo!()
    }

    fn get_multiple_capture_data(&self) -> Option<&types::MultipleCaptureData> {
        todo!()
    }

    fn get_payment_link_data(&self) -> Option<api_models::payments::PaymentLinkResponse> {
        todo!()
    }

    fn get_ephemeral_key(&self) -> Option<ephemeral_key::EphemeralKey> {
        todo!()
    }

    fn get_setup_mandate(&self) -> Option<&MandateData> {
        todo!()
    }

    fn get_poll_config(&self) -> Option<router_types::PollConfig> {
        todo!()
    }

    fn get_authentication(
        &self,
    ) -> Option<&hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore>
    {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<diesel_refund::Refund> {
        todo!()
    }

    fn get_disputes(&self) -> Vec<storage::Dispute> {
        todo!()
    }

    fn get_authorizations(&self) -> Vec<diesel_models::authorization::Authorization> {
        todo!()
    }

    fn get_attempts(&self) -> Option<Vec<storage::PaymentAttempt>> {
        todo!()
    }

    fn get_recurring_details(&self) -> Option<&RecurringDetails> {
        todo!()
    }

    fn get_payment_intent_profile_id(&self) -> Option<&id_type::ProfileId> {
        Some(&self.payment_intent.profile_id)
    }

    fn get_currency(&self) -> storage_enums::Currency {
        self.payment_intent.amount_details.currency
    }

    fn get_amount(&self) -> api::Amount {
        todo!()
    }

    fn get_payment_attempt_connector(&self) -> Option<&str> {
        todo!()
    }

    fn get_merchant_connector_id_in_attempt(&self) -> Option<id_type::MerchantConnectorAccountId> {
        todo!()
    }

    fn get_connector_customer_id(&self) -> Option<String> {
        self.connector_customer_id.clone()
    }

    fn get_merchant_connector_details(
        &self,
    ) -> Option<common_types::domain::MerchantConnectorAuthDetails> {
        todo!()
    }

    fn get_billing_address(&self) -> Option<hyperswitch_domain_models::address::Address> {
        todo!()
    }

    fn get_payment_method_data(&self) -> Option<&domain::PaymentMethodData> {
        todo!()
    }

    fn get_sessions_token(&self) -> Vec<api::SessionToken> {
        self.sessions_token.clone()
    }

    fn get_token_data(&self) -> Option<&storage::PaymentTokenData> {
        todo!()
    }

    fn get_mandate_connector(&self) -> Option<&MandateConnectorDetails> {
        todo!()
    }

    fn get_force_sync(&self) -> Option<bool> {
        todo!()
    }

    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        todo!()
    }

    fn get_optional_payment_attempt(&self) -> Option<&storage::PaymentAttempt> {
        todo!();
    }

    fn get_pre_routing_result(
        &self,
    ) -> Option<HashMap<enums::PaymentMethodType, domain::PreRoutingConnectorChoice>> {
        None
    }

    fn get_optional_external_vault_session_details(&self) -> Option<api::VaultSessionDetails> {
        self.vault_session_details.clone()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionSetters<F> for PaymentIntentData<F> {
    fn set_prerouting_algorithm_in_payment_intent(
        &mut self,
        straight_through_algorithm: storage::PaymentRoutingInfo,
    ) {
        self.payment_intent.prerouting_algorithm = Some(straight_through_algorithm);
    }
    // Setters Implementation
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }
    fn set_client_secret(&mut self, client_secret: Option<Secret<String>>) {
        self.client_secret = client_secret;
    }
    fn set_payment_attempt(&mut self, _payment_attempt: storage::PaymentAttempt) {
        todo!()
    }

    fn set_payment_method_data(&mut self, _payment_method_data: Option<domain::PaymentMethodData>) {
        todo!()
    }

    fn set_payment_method_token(&mut self, _payment_method_token: Option<PaymentMethodToken>) {
        todo!()
    }

    fn set_payment_method_id_in_attempt(&mut self, _payment_method_id: Option<String>) {
        todo!()
    }

    fn set_card_network(&mut self, card_network: enums::CardNetwork) {
        todo!()
    }

    fn set_co_badged_card_data(
        &mut self,
        debit_routing_output: &api_models::open_router::DebitRoutingOutput,
    ) {
        todo!()
    }

    fn set_email_if_not_present(&mut self, _email: pii::Email) {
        todo!()
    }

    fn set_pm_token(&mut self, _token: String) {
        todo!()
    }

    fn set_connector_customer_id(&mut self, customer_id: Option<String>) {
        self.connector_customer_id = customer_id;
    }

    fn push_sessions_token(&mut self, token: api::SessionToken) {
        self.sessions_token.push(token);
    }

    fn set_surcharge_details(&mut self, _surcharge_details: Option<types::SurchargeDetails>) {
        todo!()
    }

    fn set_merchant_connector_id_in_attempt(
        &mut self,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    ) {
        todo!()
    }

    fn set_frm_message(&mut self, _frm_message: FraudCheck) {
        todo!()
    }

    fn set_payment_intent_status(&mut self, status: storage_enums::IntentStatus) {
        self.payment_intent.status = status;
    }

    fn set_authentication_type_in_attempt(
        &mut self,
        _authentication_type: Option<enums::AuthenticationType>,
    ) {
        todo!()
    }

    fn set_recurring_mandate_payment_data(
        &mut self,
        _recurring_mandate_payment_data:
            hyperswitch_domain_models::router_data::RecurringMandatePaymentData,
    ) {
        todo!()
    }

    fn set_mandate_id(&mut self, _mandate_id: api_models::payments::MandateIds) {
        todo!()
    }

    fn set_setup_future_usage_in_payment_intent(
        &mut self,
        setup_future_usage: storage_enums::FutureUsage,
    ) {
        self.payment_intent.setup_future_usage = setup_future_usage;
    }

    fn set_connector_in_payment_attempt(&mut self, _connector: Option<String>) {
        todo!()
    }

    fn set_connector_request_reference_id(&mut self, reference_id: Option<String>) {
        todo!()
    }

    fn set_connector_response_reference_id(&mut self, reference_id: Option<String>) {
        todo!()
    }

    fn set_vault_session_details(
        &mut self,
        vault_session_details: Option<api::VaultSessionDetails>,
    ) {
        self.vault_session_details = vault_session_details;
    }

    fn set_routing_approach_in_attempt(
        &mut self,
        routing_approach: Option<enums::RoutingApproach>,
    ) {
        todo!()
    }

    fn set_connector_request_reference_id_in_payment_attempt(
        &mut self,
        connector_request_reference_id: String,
    ) {
        todo!()
    }

    fn set_cancellation_reason(&mut self, cancellation_reason: Option<String>) {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentConfirmData<F> {
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        &self.payment_attempt
    }
    #[cfg(feature = "v2")]
    fn list_payments_attempts(&self) -> &Vec<storage::PaymentAttempt> {
        todo!()
    }
    fn get_client_secret(&self) -> &Option<Secret<String>> {
        todo!()
    }

    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_merchant_connector_details(
        &self,
    ) -> Option<common_types::domain::MerchantConnectorAuthDetails> {
        self.merchant_connector_details.clone()
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
        todo!()
    }

    fn get_payment_method_token(&self) -> Option<&PaymentMethodToken> {
        todo!()
    }

    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds> {
        todo!()
    }

    fn get_address(&self) -> &PaymentAddress {
        &self.payment_address
    }

    fn get_creds_identifier(&self) -> Option<&str> {
        None
    }

    fn get_token(&self) -> Option<&str> {
        todo!()
    }

    fn get_multiple_capture_data(&self) -> Option<&types::MultipleCaptureData> {
        todo!()
    }

    fn get_payment_link_data(&self) -> Option<api_models::payments::PaymentLinkResponse> {
        todo!()
    }

    fn get_ephemeral_key(&self) -> Option<ephemeral_key::EphemeralKey> {
        todo!()
    }

    fn get_setup_mandate(&self) -> Option<&MandateData> {
        todo!()
    }

    fn get_poll_config(&self) -> Option<router_types::PollConfig> {
        todo!()
    }

    fn get_authentication(
        &self,
    ) -> Option<&hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore>
    {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<diesel_refund::Refund> {
        todo!()
    }

    fn get_disputes(&self) -> Vec<storage::Dispute> {
        todo!()
    }

    fn get_authorizations(&self) -> Vec<diesel_models::authorization::Authorization> {
        todo!()
    }

    fn get_attempts(&self) -> Option<Vec<storage::PaymentAttempt>> {
        todo!()
    }

    fn get_recurring_details(&self) -> Option<&RecurringDetails> {
        todo!()
    }

    fn get_payment_intent_profile_id(&self) -> Option<&id_type::ProfileId> {
        Some(&self.payment_intent.profile_id)
    }

    fn get_currency(&self) -> storage_enums::Currency {
        self.payment_intent.amount_details.currency
    }

    fn get_amount(&self) -> api::Amount {
        todo!()
    }

    fn get_payment_attempt_connector(&self) -> Option<&str> {
        self.payment_attempt.connector.as_deref()
    }

    fn get_merchant_connector_id_in_attempt(&self) -> Option<id_type::MerchantConnectorAccountId> {
        self.payment_attempt.merchant_connector_id.clone()
    }

    fn get_connector_customer_id(&self) -> Option<String> {
        todo!()
    }

    fn get_billing_address(&self) -> Option<hyperswitch_domain_models::address::Address> {
        todo!()
    }

    fn get_payment_method_data(&self) -> Option<&domain::PaymentMethodData> {
        self.payment_method_data.as_ref()
    }

    fn get_sessions_token(&self) -> Vec<api::SessionToken> {
        todo!()
    }

    fn get_token_data(&self) -> Option<&storage::PaymentTokenData> {
        todo!()
    }

    fn get_mandate_connector(&self) -> Option<&MandateConnectorDetails> {
        todo!()
    }

    fn get_force_sync(&self) -> Option<bool> {
        todo!()
    }

    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        todo!()
    }

    fn get_optional_payment_attempt(&self) -> Option<&storage::PaymentAttempt> {
        Some(&self.payment_attempt)
    }

    fn get_pre_routing_result(
        &self,
    ) -> Option<HashMap<enums::PaymentMethodType, domain::PreRoutingConnectorChoice>> {
        self.get_payment_intent()
            .prerouting_algorithm
            .clone()
            .and_then(|pre_routing_algorithm| pre_routing_algorithm.pre_routing_results)
    }

    fn get_optional_external_vault_session_details(&self) -> Option<api::VaultSessionDetails> {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionSetters<F> for PaymentConfirmData<F> {
    #[cfg(feature = "v2")]
    fn set_prerouting_algorithm_in_payment_intent(
        &mut self,
        straight_through_algorithm: storage::PaymentRoutingInfo,
    ) {
        self.payment_intent.prerouting_algorithm = Some(straight_through_algorithm);
    }
    // Setters Implementation
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }
    fn set_client_secret(&mut self, client_secret: Option<Secret<String>>) {
        todo!()
    }
    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt) {
        self.payment_attempt = payment_attempt;
    }

    fn set_payment_method_data(&mut self, _payment_method_data: Option<domain::PaymentMethodData>) {
        todo!()
    }

    fn set_payment_method_token(&mut self, _payment_method_token: Option<PaymentMethodToken>) {
        todo!()
    }

    fn set_payment_method_id_in_attempt(&mut self, _payment_method_id: Option<String>) {
        todo!()
    }

    fn set_email_if_not_present(&mut self, _email: pii::Email) {
        todo!()
    }

    fn set_pm_token(&mut self, _token: String) {
        todo!()
    }

    fn set_connector_customer_id(&mut self, _customer_id: Option<String>) {
        // TODO: handle this case. Should we add connector_customer_id in paymentConfirmData?
    }

    fn push_sessions_token(&mut self, _token: api::SessionToken) {
        todo!()
    }

    fn set_card_network(&mut self, card_network: enums::CardNetwork) {
        todo!()
    }

    fn set_co_badged_card_data(
        &mut self,
        debit_routing_output: &api_models::open_router::DebitRoutingOutput,
    ) {
        todo!()
    }

    fn set_surcharge_details(&mut self, _surcharge_details: Option<types::SurchargeDetails>) {
        todo!()
    }

    #[track_caller]
    fn set_merchant_connector_id_in_attempt(
        &mut self,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    ) {
        self.payment_attempt.merchant_connector_id = merchant_connector_id;
    }

    fn set_frm_message(&mut self, _frm_message: FraudCheck) {
        todo!()
    }

    fn set_payment_intent_status(&mut self, status: storage_enums::IntentStatus) {
        self.payment_intent.status = status;
    }

    fn set_authentication_type_in_attempt(
        &mut self,
        _authentication_type: Option<enums::AuthenticationType>,
    ) {
        todo!()
    }

    fn set_recurring_mandate_payment_data(
        &mut self,
        _recurring_mandate_payment_data:
            hyperswitch_domain_models::router_data::RecurringMandatePaymentData,
    ) {
        todo!()
    }

    fn set_mandate_id(&mut self, _mandate_id: api_models::payments::MandateIds) {
        todo!()
    }

    fn set_setup_future_usage_in_payment_intent(
        &mut self,
        setup_future_usage: storage_enums::FutureUsage,
    ) {
        self.payment_intent.setup_future_usage = setup_future_usage;
    }

    fn set_connector_in_payment_attempt(&mut self, connector: Option<String>) {
        self.payment_attempt.connector = connector;
    }

    fn set_connector_request_reference_id(&mut self, reference_id: Option<String>) {
        self.payment_attempt.connector_request_reference_id = reference_id;
    }

    fn set_connector_response_reference_id(&mut self, reference_id: Option<String>) {
        self.payment_attempt.connector_response_reference_id = reference_id;
    }

    fn set_vault_session_details(
        &mut self,
        external_vault_session_details: Option<api::VaultSessionDetails>,
    ) {
        todo!()
    }

    fn set_routing_approach_in_attempt(
        &mut self,
        routing_approach: Option<enums::RoutingApproach>,
    ) {
        todo!()
    }

    fn set_connector_request_reference_id_in_payment_attempt(
        &mut self,
        connector_request_reference_id: String,
    ) {
        todo!()
    }

    fn set_cancellation_reason(&mut self, cancellation_reason: Option<String>) {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentStatusData<F> {
    #[track_caller]
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        &self.payment_attempt
    }
    #[cfg(feature = "v2")]
    fn list_payments_attempts(&self) -> &Vec<storage::PaymentAttempt> {
        todo!()
    }
    fn get_client_secret(&self) -> &Option<Secret<String>> {
        todo!()
    }
    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_merchant_connector_details(
        &self,
    ) -> Option<common_types::domain::MerchantConnectorAuthDetails> {
        self.merchant_connector_details.clone()
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
        todo!()
    }

    fn get_payment_method_token(&self) -> Option<&PaymentMethodToken> {
        todo!()
    }

    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds> {
        todo!()
    }

    fn get_address(&self) -> &PaymentAddress {
        &self.payment_address
    }

    fn get_creds_identifier(&self) -> Option<&str> {
        None
    }

    fn get_token(&self) -> Option<&str> {
        todo!()
    }

    fn get_multiple_capture_data(&self) -> Option<&types::MultipleCaptureData> {
        todo!()
    }

    fn get_payment_link_data(&self) -> Option<api_models::payments::PaymentLinkResponse> {
        todo!()
    }

    fn get_ephemeral_key(&self) -> Option<ephemeral_key::EphemeralKey> {
        todo!()
    }

    fn get_setup_mandate(&self) -> Option<&MandateData> {
        todo!()
    }

    fn get_poll_config(&self) -> Option<router_types::PollConfig> {
        todo!()
    }

    fn get_authentication(
        &self,
    ) -> Option<&hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore>
    {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<diesel_refund::Refund> {
        todo!()
    }

    fn get_disputes(&self) -> Vec<storage::Dispute> {
        todo!()
    }

    fn get_authorizations(&self) -> Vec<diesel_models::authorization::Authorization> {
        todo!()
    }

    fn get_attempts(&self) -> Option<Vec<storage::PaymentAttempt>> {
        todo!()
    }

    fn get_recurring_details(&self) -> Option<&RecurringDetails> {
        todo!()
    }

    fn get_payment_intent_profile_id(&self) -> Option<&id_type::ProfileId> {
        Some(&self.payment_intent.profile_id)
    }

    fn get_currency(&self) -> storage_enums::Currency {
        self.payment_intent.amount_details.currency
    }

    fn get_amount(&self) -> api::Amount {
        todo!()
    }

    fn get_payment_attempt_connector(&self) -> Option<&str> {
        todo!()
    }

    fn get_merchant_connector_id_in_attempt(&self) -> Option<id_type::MerchantConnectorAccountId> {
        todo!()
    }

    fn get_connector_customer_id(&self) -> Option<String> {
        todo!()
    }

    fn get_billing_address(&self) -> Option<hyperswitch_domain_models::address::Address> {
        todo!()
    }

    fn get_payment_method_data(&self) -> Option<&domain::PaymentMethodData> {
        todo!()
    }

    fn get_sessions_token(&self) -> Vec<api::SessionToken> {
        todo!()
    }

    fn get_token_data(&self) -> Option<&storage::PaymentTokenData> {
        todo!()
    }

    fn get_mandate_connector(&self) -> Option<&MandateConnectorDetails> {
        todo!()
    }

    fn get_force_sync(&self) -> Option<bool> {
        todo!()
    }

    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        todo!()
    }

    fn get_optional_payment_attempt(&self) -> Option<&storage::PaymentAttempt> {
        Some(&self.payment_attempt)
    }

    fn get_pre_routing_result(
        &self,
    ) -> Option<HashMap<enums::PaymentMethodType, domain::PreRoutingConnectorChoice>> {
        None
    }

    fn get_optional_external_vault_session_details(&self) -> Option<api::VaultSessionDetails> {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionSetters<F> for PaymentStatusData<F> {
    #[cfg(feature = "v2")]
    fn set_prerouting_algorithm_in_payment_intent(
        &mut self,
        straight_through_algorithm: storage::PaymentRoutingInfo,
    ) {
        self.payment_intent.prerouting_algorithm = Some(straight_through_algorithm);
    }
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }
    fn set_client_secret(&mut self, client_secret: Option<Secret<String>>) {
        todo!()
    }
    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt) {
        self.payment_attempt = payment_attempt;
    }

    fn set_payment_method_data(&mut self, _payment_method_data: Option<domain::PaymentMethodData>) {
        todo!()
    }

    fn set_payment_method_token(&mut self, _payment_method_token: Option<PaymentMethodToken>) {
        todo!()
    }

    fn set_payment_method_id_in_attempt(&mut self, _payment_method_id: Option<String>) {
        todo!()
    }

    fn set_email_if_not_present(&mut self, _email: pii::Email) {
        todo!()
    }

    fn set_card_network(&mut self, card_network: enums::CardNetwork) {
        todo!()
    }

    fn set_co_badged_card_data(
        &mut self,
        debit_routing_output: &api_models::open_router::DebitRoutingOutput,
    ) {
        todo!()
    }

    fn set_pm_token(&mut self, _token: String) {
        todo!()
    }

    fn set_connector_customer_id(&mut self, _customer_id: Option<String>) {
        // TODO: handle this case. Should we add connector_customer_id in paymentConfirmData?
    }

    fn push_sessions_token(&mut self, _token: api::SessionToken) {
        todo!()
    }

    fn set_surcharge_details(&mut self, _surcharge_details: Option<types::SurchargeDetails>) {
        todo!()
    }

    #[track_caller]
    fn set_merchant_connector_id_in_attempt(
        &mut self,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    ) {
        todo!()
    }

    fn set_frm_message(&mut self, _frm_message: FraudCheck) {
        todo!()
    }

    fn set_payment_intent_status(&mut self, status: storage_enums::IntentStatus) {
        self.payment_intent.status = status;
    }

    fn set_authentication_type_in_attempt(
        &mut self,
        _authentication_type: Option<enums::AuthenticationType>,
    ) {
        todo!()
    }

    fn set_recurring_mandate_payment_data(
        &mut self,
        _recurring_mandate_payment_data:
            hyperswitch_domain_models::router_data::RecurringMandatePaymentData,
    ) {
        todo!()
    }

    fn set_mandate_id(&mut self, _mandate_id: api_models::payments::MandateIds) {
        todo!()
    }

    fn set_setup_future_usage_in_payment_intent(
        &mut self,
        setup_future_usage: storage_enums::FutureUsage,
    ) {
        self.payment_intent.setup_future_usage = setup_future_usage;
    }

    fn set_connector_in_payment_attempt(&mut self, connector: Option<String>) {
        self.payment_attempt.connector = connector;
    }

    fn set_connector_request_reference_id(&mut self, reference_id: Option<String>) {
        todo!()
    }

    fn set_connector_response_reference_id(&mut self, reference_id: Option<String>) {
        todo!()
    }

    fn set_vault_session_details(
        &mut self,
        external_vault_session_details: Option<api::VaultSessionDetails>,
    ) {
        todo!()
    }
    fn set_routing_approach_in_attempt(
        &mut self,
        routing_approach: Option<enums::RoutingApproach>,
    ) {
        todo!()
    }

    fn set_connector_request_reference_id_in_payment_attempt(
        &mut self,
        connector_request_reference_id: String,
    ) {
        todo!()
    }

    fn set_cancellation_reason(&mut self, cancellation_reason: Option<String>) {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentCaptureData<F> {
    #[track_caller]
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        &self.payment_attempt
    }
    #[cfg(feature = "v2")]
    fn list_payments_attempts(&self) -> &Vec<storage::PaymentAttempt> {
        todo!()
    }
    fn get_client_secret(&self) -> &Option<Secret<String>> {
        todo!()
    }
    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_merchant_connector_details(
        &self,
    ) -> Option<common_types::domain::MerchantConnectorAuthDetails> {
        todo!()
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
        todo!()
    }

    fn get_payment_method_token(&self) -> Option<&PaymentMethodToken> {
        todo!()
    }

    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds> {
        todo!()
    }

    // what is this address find out and not required remove this
    fn get_address(&self) -> &PaymentAddress {
        todo!()
    }

    fn get_creds_identifier(&self) -> Option<&str> {
        None
    }

    fn get_token(&self) -> Option<&str> {
        todo!()
    }

    fn get_multiple_capture_data(&self) -> Option<&types::MultipleCaptureData> {
        todo!()
    }

    fn get_payment_link_data(&self) -> Option<api_models::payments::PaymentLinkResponse> {
        todo!()
    }

    fn get_ephemeral_key(&self) -> Option<ephemeral_key::EphemeralKey> {
        todo!()
    }

    fn get_setup_mandate(&self) -> Option<&MandateData> {
        todo!()
    }

    fn get_poll_config(&self) -> Option<router_types::PollConfig> {
        todo!()
    }

    fn get_authentication(
        &self,
    ) -> Option<&hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore>
    {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<diesel_refund::Refund> {
        todo!()
    }

    fn get_disputes(&self) -> Vec<storage::Dispute> {
        todo!()
    }

    fn get_authorizations(&self) -> Vec<diesel_models::authorization::Authorization> {
        todo!()
    }

    fn get_attempts(&self) -> Option<Vec<storage::PaymentAttempt>> {
        todo!()
    }

    fn get_recurring_details(&self) -> Option<&RecurringDetails> {
        todo!()
    }

    fn get_payment_intent_profile_id(&self) -> Option<&id_type::ProfileId> {
        Some(&self.payment_intent.profile_id)
    }

    fn get_currency(&self) -> storage_enums::Currency {
        self.payment_intent.amount_details.currency
    }

    fn get_amount(&self) -> api::Amount {
        todo!()
    }

    fn get_payment_attempt_connector(&self) -> Option<&str> {
        todo!()
    }

    fn get_merchant_connector_id_in_attempt(&self) -> Option<id_type::MerchantConnectorAccountId> {
        todo!()
    }

    fn get_connector_customer_id(&self) -> Option<String> {
        todo!()
    }

    fn get_billing_address(&self) -> Option<hyperswitch_domain_models::address::Address> {
        todo!()
    }

    fn get_payment_method_data(&self) -> Option<&domain::PaymentMethodData> {
        todo!()
    }

    fn get_sessions_token(&self) -> Vec<api::SessionToken> {
        todo!()
    }

    fn get_token_data(&self) -> Option<&storage::PaymentTokenData> {
        todo!()
    }

    fn get_mandate_connector(&self) -> Option<&MandateConnectorDetails> {
        todo!()
    }

    fn get_force_sync(&self) -> Option<bool> {
        todo!()
    }

    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        todo!()
    }

    #[cfg(feature = "v2")]
    fn get_optional_payment_attempt(&self) -> Option<&storage::PaymentAttempt> {
        Some(&self.payment_attempt)
    }

    fn get_pre_routing_result(
        &self,
    ) -> Option<HashMap<enums::PaymentMethodType, domain::PreRoutingConnectorChoice>> {
        None
    }

    fn get_optional_external_vault_session_details(&self) -> Option<api::VaultSessionDetails> {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionSetters<F> for PaymentCaptureData<F> {
    #[cfg(feature = "v2")]
    fn set_prerouting_algorithm_in_payment_intent(
        &mut self,
        straight_through_algorithm: storage::PaymentRoutingInfo,
    ) {
        self.payment_intent.prerouting_algorithm = Some(straight_through_algorithm);
    }
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }
    fn set_client_secret(&mut self, client_secret: Option<Secret<String>>) {
        todo!()
    }
    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt) {
        self.payment_attempt = payment_attempt;
    }

    fn set_payment_method_data(&mut self, _payment_method_data: Option<domain::PaymentMethodData>) {
        todo!()
    }

    fn set_payment_method_token(&mut self, _payment_method_token: Option<PaymentMethodToken>) {
        todo!()
    }

    fn set_payment_method_id_in_attempt(&mut self, _payment_method_id: Option<String>) {
        todo!()
    }

    fn set_card_network(&mut self, card_network: enums::CardNetwork) {
        todo!()
    }

    fn set_co_badged_card_data(
        &mut self,
        debit_routing_output: &api_models::open_router::DebitRoutingOutput,
    ) {
        todo!()
    }

    fn set_email_if_not_present(&mut self, _email: pii::Email) {
        todo!()
    }

    fn set_pm_token(&mut self, _token: String) {
        todo!()
    }

    fn set_connector_customer_id(&mut self, _customer_id: Option<String>) {
        // TODO: handle this case. Should we add connector_customer_id in paymentConfirmData?
    }

    fn push_sessions_token(&mut self, _token: api::SessionToken) {
        todo!()
    }

    fn set_surcharge_details(&mut self, _surcharge_details: Option<types::SurchargeDetails>) {
        todo!()
    }

    #[track_caller]
    fn set_merchant_connector_id_in_attempt(
        &mut self,
        merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    ) {
        todo!()
    }

    fn set_frm_message(&mut self, _frm_message: FraudCheck) {
        todo!()
    }

    fn set_payment_intent_status(&mut self, status: storage_enums::IntentStatus) {
        self.payment_intent.status = status;
    }

    fn set_authentication_type_in_attempt(
        &mut self,
        _authentication_type: Option<enums::AuthenticationType>,
    ) {
        todo!()
    }

    fn set_recurring_mandate_payment_data(
        &mut self,
        _recurring_mandate_payment_data:
            hyperswitch_domain_models::router_data::RecurringMandatePaymentData,
    ) {
        todo!()
    }

    fn set_mandate_id(&mut self, _mandate_id: api_models::payments::MandateIds) {
        todo!()
    }

    fn set_setup_future_usage_in_payment_intent(
        &mut self,
        setup_future_usage: storage_enums::FutureUsage,
    ) {
        self.payment_intent.setup_future_usage = setup_future_usage;
    }

    fn set_connector_in_payment_attempt(&mut self, connector: Option<String>) {
        todo!()
    }

    fn set_connector_request_reference_id(&mut self, reference_id: Option<String>) {
        todo!()
    }

    fn set_connector_response_reference_id(&mut self, reference_id: Option<String>) {
        todo!()
    }

    fn set_vault_session_details(
        &mut self,
        external_vault_session_details: Option<api::VaultSessionDetails>,
    ) {
        todo!()
    }

    fn set_routing_approach_in_attempt(
        &mut self,
        routing_approach: Option<enums::RoutingApproach>,
    ) {
        todo!()
    }

    fn set_connector_request_reference_id_in_payment_attempt(
        &mut self,
        connector_request_reference_id: String,
    ) {
        todo!()
    }

    fn set_cancellation_reason(&mut self, cancellation_reason: Option<String>) {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentAttemptListData<F> {
    #[track_caller]
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        todo!()
    }
    #[cfg(feature = "v2")]
    fn list_payments_attempts(&self) -> &Vec<storage::PaymentAttempt> {
        &self.payment_attempt_list
    }
    fn get_client_secret(&self) -> &Option<Secret<String>> {
        todo!()
    }
    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        todo!()
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
        todo!()
    }

    fn get_payment_method_token(&self) -> Option<&PaymentMethodToken> {
        todo!()
    }

    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds> {
        todo!()
    }

    // what is this address find out and not required remove this
    fn get_address(&self) -> &PaymentAddress {
        todo!()
    }

    fn get_creds_identifier(&self) -> Option<&str> {
        None
    }

    fn get_token(&self) -> Option<&str> {
        todo!()
    }

    fn get_multiple_capture_data(&self) -> Option<&types::MultipleCaptureData> {
        todo!()
    }

    fn get_payment_link_data(&self) -> Option<api_models::payments::PaymentLinkResponse> {
        todo!()
    }

    fn get_ephemeral_key(&self) -> Option<ephemeral_key::EphemeralKey> {
        todo!()
    }

    fn get_setup_mandate(&self) -> Option<&MandateData> {
        todo!()
    }

    fn get_poll_config(&self) -> Option<router_types::PollConfig> {
        todo!()
    }

    fn get_authentication(
        &self,
    ) -> Option<&hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore>
    {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<diesel_refund::Refund> {
        todo!()
    }

    fn get_disputes(&self) -> Vec<storage::Dispute> {
        todo!()
    }

    fn get_authorizations(&self) -> Vec<diesel_models::authorization::Authorization> {
        todo!()
    }

    fn get_attempts(&self) -> Option<Vec<storage::PaymentAttempt>> {
        todo!()
    }

    fn get_recurring_details(&self) -> Option<&RecurringDetails> {
        todo!()
    }

    fn get_payment_intent_profile_id(&self) -> Option<&id_type::ProfileId> {
        todo!()
    }

    fn get_currency(&self) -> storage_enums::Currency {
        todo!()
    }

    fn get_amount(&self) -> api::Amount {
        todo!()
    }

    fn get_payment_attempt_connector(&self) -> Option<&str> {
        todo!()
    }

    fn get_merchant_connector_id_in_attempt(&self) -> Option<id_type::MerchantConnectorAccountId> {
        todo!()
    }

    fn get_connector_customer_id(&self) -> Option<String> {
        todo!()
    }

    fn get_billing_address(&self) -> Option<hyperswitch_domain_models::address::Address> {
        todo!()
    }

    fn get_payment_method_data(&self) -> Option<&domain::PaymentMethodData> {
        todo!()
    }

    fn get_sessions_token(&self) -> Vec<api::SessionToken> {
        todo!()
    }

    fn get_token_data(&self) -> Option<&storage::PaymentTokenData> {
        todo!()
    }

    fn get_mandate_connector(&self) -> Option<&MandateConnectorDetails> {
        todo!()
    }

    fn get_force_sync(&self) -> Option<bool> {
        todo!()
    }

    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        todo!()
    }

    #[cfg(feature = "v2")]
    fn get_optional_payment_attempt(&self) -> Option<&storage::PaymentAttempt> {
        todo!()
    }

    fn get_pre_routing_result(
        &self,
    ) -> Option<HashMap<enums::PaymentMethodType, domain::PreRoutingConnectorChoice>> {
        None
    }
    fn get_merchant_connector_details(
        &self,
    ) -> Option<common_types::domain::MerchantConnectorAuthDetails> {
        todo!()
    }

    fn get_optional_external_vault_session_details(&self) -> Option<api::VaultSessionDetails> {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentCancelData<F> {
    #[track_caller]
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        &self.payment_attempt
    }
    fn list_payments_attempts(&self) -> &Vec<storage::PaymentAttempt> {
        todo!()
    }
    fn get_client_secret(&self) -> &Option<Secret<String>> {
        todo!()
    }
    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_merchant_connector_details(
        &self,
    ) -> Option<common_types::domain::MerchantConnectorAuthDetails> {
        todo!()
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
        todo!()
    }

    fn get_payment_method_token(&self) -> Option<&PaymentMethodToken> {
        todo!()
    }

    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds> {
        todo!()
    }

    // what is this address find out and not required remove this
    fn get_address(&self) -> &PaymentAddress {
        todo!()
    }

    fn get_creds_identifier(&self) -> Option<&str> {
        None
    }

    fn get_token(&self) -> Option<&str> {
        todo!()
    }

    fn get_multiple_capture_data(&self) -> Option<&types::MultipleCaptureData> {
        todo!()
    }

    fn get_payment_link_data(&self) -> Option<api_models::payments::PaymentLinkResponse> {
        todo!()
    }

    fn get_ephemeral_key(&self) -> Option<ephemeral_key::EphemeralKey> {
        todo!()
    }

    fn get_setup_mandate(&self) -> Option<&MandateData> {
        todo!()
    }

    fn get_poll_config(&self) -> Option<router_types::PollConfig> {
        todo!()
    }

    fn get_authentication(
        &self,
    ) -> Option<&hyperswitch_domain_models::router_request_types::authentication::AuthenticationStore>
    {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<diesel_refund::Refund> {
        todo!()
    }

    fn get_disputes(&self) -> Vec<storage::Dispute> {
        todo!()
    }

    fn get_authorizations(&self) -> Vec<diesel_models::authorization::Authorization> {
        todo!()
    }

    fn get_attempts(&self) -> Option<Vec<storage::PaymentAttempt>> {
        todo!()
    }

    fn get_recurring_details(&self) -> Option<&RecurringDetails> {
        todo!()
    }

    fn get_payment_intent_profile_id(&self) -> Option<&id_type::ProfileId> {
        todo!()
    }

    fn get_currency(&self) -> storage_enums::Currency {
        todo!()
    }

    fn get_amount(&self) -> api::Amount {
        todo!()
    }

    fn get_payment_attempt_connector(&self) -> Option<&str> {
        todo!()
    }

    fn get_merchant_connector_id_in_attempt(&self) -> Option<id_type::MerchantConnectorAccountId> {
        todo!()
    }

    fn get_connector_customer_id(&self) -> Option<String> {
        todo!()
    }

    fn get_billing_address(&self) -> Option<hyperswitch_domain_models::address::Address> {
        todo!()
    }

    fn get_payment_method_data(&self) -> Option<&domain::PaymentMethodData> {
        todo!()
    }

    fn get_sessions_token(&self) -> Vec<api::SessionToken> {
        todo!()
    }

    fn get_token_data(&self) -> Option<&storage::PaymentTokenData> {
        todo!()
    }

    fn get_mandate_connector(&self) -> Option<&MandateConnectorDetails> {
        todo!()
    }

    fn get_force_sync(&self) -> Option<bool> {
        todo!()
    }

    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        todo!()
    }

    fn get_optional_payment_attempt(&self) -> Option<&storage::PaymentAttempt> {
        None
    }

    fn get_pre_routing_result(
        &self,
    ) -> Option<HashMap<enums::PaymentMethodType, domain::PreRoutingConnectorChoice>> {
        None
    }

    fn get_optional_external_vault_session_details(&self) -> Option<api::VaultSessionDetails> {
        todo!()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionSetters<F> for PaymentCancelData<F> {
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }

    fn set_client_secret(&mut self, _client_secret: Option<Secret<String>>) {
        todo!()
    }

    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt) {
        self.payment_attempt = payment_attempt;
    }

    fn set_payment_method_data(&mut self, _payment_method_data: Option<domain::PaymentMethodData>) {
        todo!()
    }

    fn set_payment_method_token(&mut self, _payment_method_token: Option<PaymentMethodToken>) {
        todo!()
    }

    fn set_email_if_not_present(&mut self, _email: pii::Email) {
        todo!()
    }

    fn set_payment_method_id_in_attempt(&mut self, _payment_method_id: Option<String>) {
        todo!()
    }

    fn set_pm_token(&mut self, _token: String) {
        !todo!()
    }

    fn set_connector_customer_id(&mut self, _customer_id: Option<String>) {
        // TODO: handle this case. Should we add connector_customer_id in PaymentCancelData?
    }

    fn push_sessions_token(&mut self, _token: api::SessionToken) {
        todo!()
    }

    fn set_surcharge_details(&mut self, _surcharge_details: Option<types::SurchargeDetails>) {
        todo!()
    }

    fn set_merchant_connector_id_in_attempt(
        &mut self,
        _merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    ) {
        todo!()
    }

    fn set_card_network(&mut self, _card_network: enums::CardNetwork) {
        todo!()
    }

    fn set_co_badged_card_data(
        &mut self,
        _debit_routing_output: &api_models::open_router::DebitRoutingOutput,
    ) {
        todo!()
    }

    fn set_frm_message(&mut self, _frm_message: FraudCheck) {
        todo!()
    }

    fn set_payment_intent_status(&mut self, status: storage_enums::IntentStatus) {
        self.payment_intent.status = status;
    }

    fn set_authentication_type_in_attempt(
        &mut self,
        _authentication_type: Option<enums::AuthenticationType>,
    ) {
        todo!()
    }

    fn set_recurring_mandate_payment_data(
        &mut self,
        _recurring_mandate_payment_data:
            hyperswitch_domain_models::router_data::RecurringMandatePaymentData,
    ) {
        todo!()
    }

    fn set_mandate_id(&mut self, _mandate_id: api_models::payments::MandateIds) {
        todo!()
    }

    fn set_setup_future_usage_in_payment_intent(
        &mut self,
        setup_future_usage: storage_enums::FutureUsage,
    ) {
        todo!()
    }

    fn set_prerouting_algorithm_in_payment_intent(
        &mut self,
        prerouting_algorithm: storage::PaymentRoutingInfo,
    ) {
        self.payment_intent.prerouting_algorithm = Some(prerouting_algorithm);
    }

    fn set_connector_in_payment_attempt(&mut self, _connector: Option<String>) {
        todo!()
    }

    fn set_connector_request_reference_id(&mut self, _reference_id: Option<String>) {
        todo!()
    }

    fn set_connector_response_reference_id(&mut self, _reference_id: Option<String>) {
        todo!()
    }

    fn set_vault_session_details(
        &mut self,
        _external_vault_session_details: Option<api::VaultSessionDetails>,
    ) {
        todo!()
    }

    fn set_routing_approach_in_attempt(
        &mut self,
        _routing_approach: Option<enums::RoutingApproach>,
    ) {
        todo!()
    }

    fn set_connector_request_reference_id_in_payment_attempt(
        &mut self,
        _connector_request_reference_id: String,
    ) {
        todo!()
    }

    fn set_cancellation_reason(&mut self, cancellation_reason: Option<String>) {
        self.payment_attempt.cancellation_reason = cancellation_reason;
    }
}
