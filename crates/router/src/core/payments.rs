pub mod access_token;
pub mod conditional_configs;
pub mod connector_integration_v2_impls;
pub mod customers;
pub mod flows;
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
#[cfg(feature = "olap")]
use std::collections::HashMap;
use std::{
    collections::HashSet, fmt::Debug, marker::PhantomData, ops::Deref, time::Instant, vec::IntoIter,
};

#[cfg(feature = "v2")]
pub mod payment_methods;

#[cfg(feature = "olap")]
use api_models::admin::MerchantConnectorInfo;
use api_models::{
    self, enums,
    mandates::RecurringDetails,
    payments::{self as payments_api},
};
pub use common_enums::enums::CallConnectorAction;
use common_utils::{
    ext_traits::{AsyncExt, StringExt},
    id_type, pii,
    types::{AmountConvertor, MinorUnit, Surcharge},
};
use diesel_models::{ephemeral_key, fraud_check::FraudCheck};
use error_stack::{report, ResultExt};
use events::EventInfo;
use futures::future::join_all;
use helpers::{decrypt_paze_token, ApplePayData};
use hyperswitch_domain_models::payments::{payment_intent::CustomerData, ClickToPayMetaData};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::{
    PaymentCaptureData, PaymentConfirmData, PaymentIntentData, PaymentStatusData,
};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::router_response_types::RedirectForm;
pub use hyperswitch_domain_models::{
    mandates::{CustomerAcceptance, MandateData},
    payment_address::PaymentAddress,
    payments::HeaderPayload,
    router_data::{PaymentMethodToken, RouterData},
    router_request_types::CustomerDetails,
};
use masking::{ExposeInterface, PeekInterface, Secret};
#[cfg(feature = "v2")]
use operations::ValidateStatusForOperation;
use redis_interface::errors::RedisError;
use router_env::{instrument, tracing};
#[cfg(feature = "olap")]
use router_types::transformers::ForeignFrom;
use scheduler::utils as pt_utils;
#[cfg(feature = "v2")]
pub use session_operation::payments_session_core;
#[cfg(feature = "olap")]
use strum::IntoEnumIterator;
use time;

#[cfg(feature = "v1")]
pub use self::operations::{
    PaymentApprove, PaymentCancel, PaymentCapture, PaymentConfirm, PaymentCreate,
    PaymentIncrementalAuthorization, PaymentPostSessionTokens, PaymentReject, PaymentSession,
    PaymentSessionUpdate, PaymentStatus, PaymentUpdate,
};
use self::{
    conditional_configs::perform_decision_management,
    flows::{ConstructFlowSpecificData, Feature},
    operations::{BoxedOperation, Operation, PaymentResponse},
    routing::{self as self_routing, SessionFlowRoutingInput},
};
use super::{
    errors::StorageErrorExt, payment_methods::surcharge_decision_configs, routing::TransactionData,
};
#[cfg(feature = "frm")]
use crate::core::fraud_check as frm_core;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use crate::core::routing::helpers as routing_helpers;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use crate::types::api::convert_connector_data_to_routable_connectors;
use crate::{
    configs::settings::{ApplePayPreDecryptFlow, PaymentMethodTypeTokenFilter},
    connector::utils::missing_field_err,
    core::{
        errors::{self, CustomResult, RouterResponse, RouterResult},
        payment_methods::{cards, network_tokenization},
        payouts,
        routing::{self as core_routing},
        utils::{self as core_utils},
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
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use crate::{
    core::authentication as authentication_core,
    types::{api::authentication, BrowserInformation},
};

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_operation_core<F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    profile: domain::Profile,
    operation: Op,
    req: Req,
    get_tracker_response: operations::GetTrackerResponse<D>,
    call_connector_action: CallConnectorAction,
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
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Failed while fetching/creating customer")?;

    operation
        .to_domain()?
        .run_decision_manager(state, &mut payment_data, &profile)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to run decision manager")?;

    let connector = operation
        .to_domain()?
        .perform_routing(
            &merchant_account,
            &profile,
            state,
            &mut payment_data,
            &key_store,
        )
        .await?;

    let payment_data = match connector {
        ConnectorCallType::PreDetermined(connector_data) => {
            let router_data = call_connector_service(
                state,
                req_state.clone(),
                &merchant_account,
                &key_store,
                connector_data.clone(),
                &operation,
                &mut payment_data,
                &customer,
                call_connector_action.clone(),
                None,
                header_payload.clone(),
                #[cfg(feature = "frm")]
                None,
                #[cfg(not(feature = "frm"))]
                None,
                &profile,
                false,
            )
            .await?;

            let payments_response_operation = Box::new(PaymentResponse);

            payments_response_operation
                .to_post_update_tracker()?
                .update_tracker(
                    state,
                    payment_data,
                    router_data,
                    &key_store,
                    merchant_account.storage_scheme,
                )
                .await?
        }
        ConnectorCallType::Retryable(vec) => todo!(),
        ConnectorCallType::SessionMultiple(vec) => todo!(),
        ConnectorCallType::Skip => payment_data,
    };

    Ok((payment_data, req, customer, None, None))
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_operation_core<F, Req, Op, FData, D>(
    state: &SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile_id_from_auth_layer: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    call_connector_action: CallConnectorAction,
    auth_flow: services::AuthFlow,
    eligible_connectors: Option<Vec<common_enums::RoutableConnectors>>,
    header_payload: HeaderPayload,
    platform_merchant_account: Option<domain::MerchantAccount>,
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
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
    FData: Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    tracing::Span::current().record("merchant_id", merchant_account.get_id().get_string_repr());
    let (operation, validate_result) = operation
        .to_validate_request()?
        .validate_request(&req, &merchant_account)?;

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
            &merchant_account,
            &key_store,
            auth_flow,
            &header_payload,
            platform_merchant_account.as_ref(),
        )
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
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Failed while fetching/creating customer")?;

    let authentication_type =
        call_decision_manager(state, &merchant_account, &business_profile, &payment_data).await?;

    payment_data.set_authentication_type_in_attempt(authentication_type);

    let connector = get_connector_choice(
        &operation,
        state,
        &req,
        &merchant_account,
        &business_profile,
        &key_store,
        &mut payment_data,
        eligible_connectors,
        mandate_type,
    )
    .await?;

    let should_add_task_to_process_tracker = should_add_task_to_process_tracker(&payment_data);

    let locale = header_payload.locale.clone();

    payment_data = tokenize_in_router_when_confirm_false_or_external_authentication(
        state,
        &operation,
        &mut payment_data,
        &validate_result,
        &key_store,
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
                &merchant_account,
                &mut payment_data,
                state,
                &mut frm_info,
                &customer,
                &mut should_continue_transaction,
                &mut should_continue_capture,
                key_store.clone(),
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

        if helpers::is_merchant_eligible_authentication_service(merchant_account.get_id(), state)
            .await?
        {
            operation
                .to_domain()?
                .call_unified_authentication_service_if_eligible(
                    state,
                    &mut payment_data,
                    &mut should_continue_transaction,
                    &connector_details,
                    &business_profile,
                    &key_store,
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
                    &key_store,
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
                &key_store,
                &merchant_account,
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
                ConnectorCallType::PreDetermined(connector) => {
                    #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                    let routable_connectors =
                        convert_connector_data_to_routable_connectors(&[connector.clone()])
                            .map_err(|e| logger::error!(routable_connector_error=?e))
                            .unwrap_or_default();
                    let schedule_time = if should_add_task_to_process_tracker {
                        payment_sync::get_sync_process_schedule_time(
                            &*state.store,
                            connector.connector.id(),
                            merchant_account.get_id(),
                            0,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while getting process schedule time")?
                    } else {
                        None
                    };
                    let (router_data, mca) = call_connector_service(
                        state,
                        req_state.clone(),
                        &merchant_account,
                        &key_store,
                        connector.clone(),
                        &operation,
                        &mut payment_data,
                        &customer,
                        call_connector_action.clone(),
                        &validate_result,
                        schedule_time,
                        header_payload.clone(),
                        #[cfg(feature = "frm")]
                        frm_info.as_ref().and_then(|fi| fi.suggested_action),
                        #[cfg(not(feature = "frm"))]
                        None,
                        &business_profile,
                        false,
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
                            &merchant_account,
                            &key_store,
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
                            &key_store,
                            merchant_account.storage_scheme,
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
                            &merchant_account,
                            &key_store,
                            &customer,
                            &mca,
                            &connector,
                            &mut payment_data,
                            op_ref,
                            Some(header_payload.clone()),
                        )
                        .await?;
                    }

                    payment_data
                }

                ConnectorCallType::Retryable(connectors) => {
                    #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                    let routable_connectors =
                        convert_connector_data_to_routable_connectors(&connectors)
                            .map_err(|e| logger::error!(routable_connector_error=?e))
                            .unwrap_or_default();

                    let mut connectors = connectors.into_iter();

                    let connector_data = get_connector_data(&mut connectors)?;

                    let schedule_time = if should_add_task_to_process_tracker {
                        payment_sync::get_sync_process_schedule_time(
                            &*state.store,
                            connector_data.connector.id(),
                            merchant_account.get_id(),
                            0,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while getting process schedule time")?
                    } else {
                        None
                    };
                    let (router_data, mca) = call_connector_service(
                        state,
                        req_state.clone(),
                        &merchant_account,
                        &key_store,
                        connector_data.clone(),
                        &operation,
                        &mut payment_data,
                        &customer,
                        call_connector_action.clone(),
                        &validate_result,
                        schedule_time,
                        header_payload.clone(),
                        #[cfg(feature = "frm")]
                        frm_info.as_ref().and_then(|fi| fi.suggested_action),
                        #[cfg(not(feature = "frm"))]
                        None,
                        &business_profile,
                        false,
                    )
                    .await?;

                    #[cfg(all(feature = "retry", feature = "v1"))]
                    let mut router_data = router_data;
                    #[cfg(all(feature = "retry", feature = "v1"))]
                    {
                        use crate::core::payments::retry::{self, GsmValidation};
                        let config_bool = retry::config_should_call_gsm(
                            &*state.store,
                            merchant_account.get_id(),
                            &business_profile,
                        )
                        .await;

                        if config_bool && router_data.should_call_gsm() {
                            router_data = retry::do_gsm_actions(
                                state,
                                req_state.clone(),
                                &mut payment_data,
                                connectors,
                                connector_data.clone(),
                                router_data,
                                &merchant_account,
                                &key_store,
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
                            &merchant_account,
                            &key_store,
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
                            &key_store,
                            merchant_account.storage_scheme,
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
                            &merchant_account,
                            &key_store,
                            &customer,
                            &mca,
                            &connector_data,
                            &mut payment_data,
                            op_ref,
                            Some(header_payload.clone()),
                        )
                        .await?;
                    }

                    payment_data
                }

                ConnectorCallType::SessionMultiple(connectors) => {
                    let session_surcharge_details =
                        call_surcharge_decision_management_for_session_flow(
                            state,
                            &merchant_account,
                            &business_profile,
                            payment_data.get_payment_attempt(),
                            payment_data.get_payment_intent(),
                            payment_data.get_billing_address(),
                            &connectors,
                        )
                        .await?;
                    Box::pin(call_multiple_connectors_service(
                        state,
                        &merchant_account,
                        &key_store,
                        connectors,
                        &operation,
                        payment_data,
                        &customer,
                        session_surcharge_details,
                        &business_profile,
                        header_payload.clone(),
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
                    &merchant_account,
                    &mut payment_data,
                    fraud_info,
                    frm_configs
                        .clone()
                        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "frm_configs",
                        })
                        .attach_printable("Frm configs label not found")?,
                    &customer,
                    key_store.clone(),
                    &mut should_continue_capture,
                    platform_merchant_account.as_ref(),
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
                    &key_store,
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
                &key_store,
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
        merchant_account,
        business_profile,
        &key_store,
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
    merchant_account: domain::MerchantAccount,
    profile_id_from_auth_layer: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    call_connector_action: CallConnectorAction,
    auth_flow: services::AuthFlow,
    header_payload: HeaderPayload,
    platform_merchant_account: Option<domain::MerchantAccount>,
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
    dyn api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Data = D>,
    FData: Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    tracing::Span::current().record("merchant_id", merchant_account.get_id().get_string_repr());
    let (operation, validate_result) = operation
        .to_validate_request()?
        .validate_request(&req, &merchant_account)?;

    tracing::Span::current().record("payment_id", format!("{}", validate_result.payment_id));

    let operations::GetTrackerResponse {
        operation,
        customer_details: _,
        mut payment_data,
        business_profile,
        mandate_type: _,
    } = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &validate_result.payment_id,
            &req,
            &merchant_account,
            &key_store,
            auth_flow,
            &header_payload,
            platform_merchant_account.as_ref(),
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
            &merchant_account,
            &state.clone(),
            &req,
            payment_data.get_payment_intent(),
            &key_store,
        )
        .await?;

    let connector = set_eligible_connector_for_nti_in_payment_data(
        state,
        &business_profile,
        &key_store,
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
            merchant_account.get_id(),
            0,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while getting process schedule time")?
    } else {
        None
    };

    let (router_data, mca) = proxy_for_call_connector_service(
        state,
        req_state.clone(),
        &merchant_account,
        &key_store,
        connector.clone(),
        &operation,
        &mut payment_data,
        &None,
        call_connector_action.clone(),
        &validate_result,
        schedule_time,
        header_payload.clone(),
        &business_profile,
    )
    .await?;

    let op_ref = &operation;
    let should_trigger_post_processing_flows = is_operation_confirm(&operation);

    let operation = Box::new(PaymentResponse);

    let connector_http_status_code = router_data.connector_http_status_code;
    let external_latency = router_data.external_latency;

    add_connector_http_status_code_metrics(connector_http_status_code);

    #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
    let routable_connectors = convert_connector_data_to_routable_connectors(&[connector.clone()])
        .map_err(|e| logger::error!(routable_connector_error=?e))
        .unwrap_or_default();

    let mut payment_data = operation
        .to_post_update_tracker()?
        .update_tracker(
            state,
            payment_data,
            router_data,
            &key_store,
            merchant_account.storage_scheme,
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
            &merchant_account,
            &key_store,
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
        merchant_account,
        business_profile,
        &key_store,
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
pub async fn payments_intent_operation_core<F, Req, Op, D>(
    state: &SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
    platform_merchant_account: Option<domain::MerchantAccount>,
) -> RouterResult<(D, Req, Option<domain::Customer>)>
where
    F: Send + Clone + Sync,
    Req: Clone,
    Op: Operation<F, Req, Data = D> + Send + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let operation: BoxedOperation<'_, F, Req, D> = Box::new(operation);

    tracing::Span::current().record("merchant_id", merchant_account.get_id().get_string_repr());

    let _validate_result = operation
        .to_validate_request()?
        .validate_request(&req, &merchant_account)?;

    tracing::Span::current().record("global_payment_id", payment_id.get_string_repr());

    let operations::GetTrackerResponse { mut payment_data } = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &payment_id,
            &req,
            &merchant_account,
            &profile,
            &key_store,
            &header_payload,
            platform_merchant_account.as_ref(),
        )
        .await?;

    let (_operation, customer) = operation
        .to_domain()?
        .get_customer_details(
            state,
            &mut payment_data,
            &key_store,
            merchant_account.storage_scheme,
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
            merchant_account.storage_scheme,
            None,
            &key_store,
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
    merchant_account: &domain::MerchantAccount,
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
    let algorithm_ref: api::routing::RoutingAlgorithmRef = merchant_account
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
        merchant_account.get_id(),
        &payment_dsl_data,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Could not decode the conditional config")?;
    Ok(payment_dsl_data
        .payment_attempt
        .authentication_type
        .or(output.override_3ds)
        .or(Some(storage_enums::AuthenticationType::NoThreeDs)))
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
    connectors: &mut IntoIter<api::ConnectorData>,
) -> RouterResult<api::ConnectorData> {
    connectors
        .next()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Connector not found in connectors iterator")
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn call_surcharge_decision_management_for_session_flow(
    _state: &SessionState,
    _merchant_account: &domain::MerchantAccount,
    _business_profile: &domain::Profile,
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
    _merchant_account: &domain::MerchantAccount,
    _business_profile: &domain::Profile,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    billing_address: Option<hyperswitch_domain_models::address::Address>,
    session_connector_data: &[api::SessionConnectorData],
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
            .map(|session_connector_data| session_connector_data.payment_method_type)
            .collect();

        #[cfg(feature = "v1")]
        let algorithm_ref: api::routing::RoutingAlgorithmRef = _merchant_account
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
    merchant_account: domain::MerchantAccount,
    profile_id: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    auth_flow: services::AuthFlow,
    call_connector_action: CallConnectorAction,
    eligible_connectors: Option<Vec<enums::Connector>>,
    header_payload: HeaderPayload,
    platform_merchant_account: Option<domain::MerchantAccount>,
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
            merchant_account,
            profile_id,
            key_store,
            operation.clone(),
            req,
            call_connector_action,
            auth_flow,
            eligible_routable_connectors,
            header_payload.clone(),
            platform_merchant_account,
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
    merchant_account: domain::MerchantAccount,
    profile_id: Option<id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    auth_flow: services::AuthFlow,
    call_connector_action: CallConnectorAction,
    header_payload: HeaderPayload,
    platform_merchant_account: Option<domain::MerchantAccount>,
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
            merchant_account,
            profile_id,
            key_store,
            operation.clone(),
            req,
            call_connector_action,
            auth_flow,
            header_payload.clone(),
            platform_merchant_account,
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
pub async fn payments_intent_core<F, Res, Req, Op, D>(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
    platform_merchant_account: Option<domain::MerchantAccount>,
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
        merchant_account.clone(),
        profile,
        key_store,
        operation.clone(),
        req,
        payment_id,
        header_payload.clone(),
        platform_merchant_account,
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
        &merchant_account,
    )
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_get_intent_using_merchant_reference(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    req_state: ReqState,
    merchant_reference_id: &id_type::PaymentReferenceId,
    header_payload: HeaderPayload,
    platform_merchant_account: Option<domain::MerchantAccount>,
) -> RouterResponse<api::PaymentsIntentResponse> {
    let db = state.store.as_ref();
    let storage_scheme = merchant_account.storage_scheme;
    let key_manager_state = &(&state).into();
    let payment_intent = db
        .find_payment_intent_by_merchant_reference_id_profile_id(
            key_manager_state,
            merchant_reference_id,
            profile.get_id(),
            &key_store,
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
        merchant_account.clone(),
        profile.clone(),
        key_store.clone(),
        operations::PaymentGetIntent,
        api_models::payments::PaymentsGetIntentRequest {
            id: payment_intent.get_id().clone(),
        },
        payment_intent.get_id().clone(),
        header_payload.clone(),
        platform_merchant_account,
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
        &merchant_account,
    )
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn payments_core<F, Res, Req, Op, FData, D>(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
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
        .validate_request(&req, &merchant_account)?;

    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            &state,
            &payment_id,
            &req,
            &merchant_account,
            &profile,
            &key_store,
            &header_payload,
            None,
        )
        .await?;

    let (payment_data, _req, customer, connector_http_status_code, external_latency) =
        payments_operation_core::<_, _, _, _, _>(
            &state,
            req_state,
            merchant_account.clone(),
            key_store,
            profile,
            operation.clone(),
            req,
            get_tracker_response,
            call_connector_action,
            header_payload.clone(),
        )
        .await?;

    payment_data.generate_response(
        &state,
        connector_http_status_code,
        external_latency,
        header_payload.x_hs_latency,
        &merchant_account,
    )
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v2")]
pub(crate) async fn payments_create_and_confirm_intent(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    request: payments_api::PaymentsRequest,
    payment_id: id_type::GlobalPaymentId,
    mut header_payload: HeaderPayload,
    platform_merchant_account: Option<domain::MerchantAccount>,
) -> RouterResponse<payments_api::PaymentsResponse> {
    use actix_http::body::MessageBody;
    use common_utils::ext_traits::BytesExt;
    use hyperswitch_domain_models::{
        payments::{PaymentConfirmData, PaymentIntentData},
        router_flow_types::{Authorize, PaymentCreateIntent, SetupMandate},
    };

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
        merchant_account.clone(),
        profile.clone(),
        key_store.clone(),
        operations::PaymentIntentCreate,
        payload,
        payment_id.clone(),
        header_payload.clone(),
        platform_merchant_account,
    ))
    .await?;

    logger::info!(?create_intent_response);
    let create_intent_response = handle_payments_intent_response(create_intent_response)?;

    // Adding client secret to ensure client secret validation passes during confirm intent step
    header_payload.client_secret = Some(create_intent_response.client_secret.clone());

    let payload = payments_api::PaymentsConfirmIntentRequest::from(&request);

    let confirm_intent_response = decide_authorize_or_setup_intent_flow(
        state,
        req_state,
        merchant_account,
        profile,
        key_store,
        &create_intent_response,
        payload,
        payment_id,
        header_payload,
    )
    .await?;

    logger::info!(?confirm_intent_response);
    let confirm_intent_response = handle_payments_intent_response(confirm_intent_response)?;

    construct_payments_response(create_intent_response, confirm_intent_response)
}

#[cfg(feature = "v2")]
#[inline]
fn handle_payments_intent_response<T>(
    response: hyperswitch_domain_models::api::ApplicationResponse<T>,
) -> CustomResult<T, errors::ApiErrorResponse> {
    match response {
        hyperswitch_domain_models::api::ApplicationResponse::Json(body)
        | hyperswitch_domain_models::api::ApplicationResponse::JsonWithHeaders((body, _)) => {
            Ok(body)
        }
        hyperswitch_domain_models::api::ApplicationResponse::StatusOk
        | hyperswitch_domain_models::api::ApplicationResponse::TextPlain(_)
        | hyperswitch_domain_models::api::ApplicationResponse::JsonForRedirection(_)
        | hyperswitch_domain_models::api::ApplicationResponse::Form(_)
        | hyperswitch_domain_models::api::ApplicationResponse::PaymentLinkForm(_)
        | hyperswitch_domain_models::api::ApplicationResponse::FileData(_)
        | hyperswitch_domain_models::api::ApplicationResponse::GenericLinkForm(_) => {
            Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unexpected response from payment intent core")
        }
    }
}

#[cfg(feature = "v2")]
#[inline]
fn construct_payments_response(
    create_intent_response: payments_api::PaymentsIntentResponse,
    confirm_intent_response: payments_api::PaymentsConfirmIntentResponse,
) -> RouterResponse<payments_api::PaymentsResponse> {
    let response = payments_api::PaymentsResponse {
        id: confirm_intent_response.id,
        status: confirm_intent_response.status,
        amount: confirm_intent_response.amount,
        customer_id: confirm_intent_response.customer_id,
        connector: confirm_intent_response.connector,
        client_secret: confirm_intent_response.client_secret,
        created: confirm_intent_response.created,
        payment_method_data: confirm_intent_response.payment_method_data,
        payment_method_type: confirm_intent_response.payment_method_type,
        payment_method_subtype: confirm_intent_response.payment_method_subtype,
        next_action: confirm_intent_response.next_action,
        connector_transaction_id: confirm_intent_response.connector_transaction_id,
        connector_reference_id: confirm_intent_response.connector_reference_id,
        connector_token_details: confirm_intent_response.connector_token_details,
        merchant_connector_id: confirm_intent_response.merchant_connector_id,
        browser_info: confirm_intent_response.browser_info,
        error: confirm_intent_response.error,
    };

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
async fn decide_authorize_or_setup_intent_flow(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    create_intent_response: &payments_api::PaymentsIntentResponse,
    confirm_intent_request: payments_api::PaymentsConfirmIntentRequest,
    payment_id: id_type::GlobalPaymentId,
    header_payload: HeaderPayload,
) -> RouterResponse<payments_api::PaymentsConfirmIntentResponse> {
    use hyperswitch_domain_models::{
        payments::PaymentConfirmData,
        router_flow_types::{Authorize, SetupMandate},
    };

    if create_intent_response.amount_details.order_amount == MinorUnit::zero() {
        Box::pin(payments_core::<
            SetupMandate,
            api_models::payments::PaymentsConfirmIntentResponse,
            _,
            _,
            _,
            PaymentConfirmData<SetupMandate>,
        >(
            state,
            req_state,
            merchant_account,
            profile,
            key_store,
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
            api_models::payments::PaymentsConfirmIntentResponse,
            _,
            _,
            _,
            PaymentConfirmData<Authorize>,
        >(
            state,
            req_state,
            merchant_account,
            profile,
            key_store,
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
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        connector: String,
        payment_id: id_type::PaymentId,
        platform_merchant_account: Option<domain::MerchantAccount>,
    ) -> RouterResult<Self::PaymentFlowResponse>;

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &SessionState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        profile: domain::Profile,
        req: PaymentsRedirectResponseData,
        platform_merchant_account: Option<domain::MerchantAccount>,
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
        merchant_account: domain::MerchantAccount,
        key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        platform_merchant_account: Option<domain::MerchantAccount>,
    ) -> RouterResponse<api::RedirectionResponse> {
        metrics::REDIRECTION_TRIGGERED.add(
            1,
            router_env::metric_attributes!(
                (
                    "connector",
                    req.connector.to_owned().unwrap_or("null".to_string()),
                ),
                ("merchant_id", merchant_account.get_id().clone()),
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
                merchant_account.clone(),
                key_store,
                req.clone(),
                flow_type,
                connector.clone(),
                resource_id.clone(),
                platform_merchant_account,
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
        merchant_account: domain::MerchantAccount,
        key_store: domain::MerchantKeyStore,
        profile: domain::Profile,
        request: PaymentsRedirectResponseData,
        platform_merchant_account: Option<domain::MerchantAccount>,
    ) -> RouterResponse<api::RedirectionResponse> {
        metrics::REDIRECTION_TRIGGERED.add(
            1,
            router_env::metric_attributes!(("merchant_id", merchant_account.get_id().clone())),
        );

        let payment_flow_response = self
            .call_payment_flow(
                &state,
                req_state,
                merchant_account,
                key_store,
                profile,
                request,
                platform_merchant_account,
            )
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
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        _connector: String,
        _payment_id: id_type::PaymentId,
        platform_merchant_account: Option<domain::MerchantAccount>,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let key_manager_state = &state.into();

        let payment_confirm_req = api::PaymentsRequest {
            payment_id: Some(req.resource_id.clone()),
            merchant_id: req.merchant_id.clone(),
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
            merchant_account,
            None,
            merchant_key_store.clone(),
            operations::payment_complete_authorize::CompleteAuthorize,
            payment_confirm_req,
            services::api::AuthFlow::Merchant,
            connector_action,
            None,
            HeaderPayload::default(),
            platform_merchant_account,
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
            .find_business_profile_by_profile_id(key_manager_state, &merchant_key_store, profile_id)
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
                        api_models::payments::NextActionData::DisplayBankTransferInformation { .. } => None,
                        api_models::payments::NextActionData::ThirdPartySdkSessionToken { .. } => None,
                        api_models::payments::NextActionData::QrCodeInformation{..} => None,
                        api_models::payments::NextActionData::FetchQrCodeInformation{..} => None,
                        api_models::payments::NextActionData::DisplayVoucherInformation{ .. } => None,
                        api_models::payments::NextActionData::WaitScreenInformation{..} => None,
                        api_models::payments::NextActionData::ThreeDsInvoke{..} => None,
                        api_models::payments::NextActionData::InvokeSdkClient{..} => None,
                        api_models::payments::NextActionData::CollectOtp{ .. } => None,
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
        Ok(services::ApplicationResponse::JsonForRedirection(
            redirection_response,
        ))
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
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        _connector: String,
        _payment_id: id_type::PaymentId,
        platform_merchant_account: Option<domain::MerchantAccount>,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let key_manager_state = &state.into();

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
        };
        let response = Box::pin(
            payments_core::<api::PSync, api::PaymentsResponse, _, _, _, _>(
                state.clone(),
                req_state,
                merchant_account,
                None,
                merchant_key_store.clone(),
                PaymentStatus,
                payment_sync_req,
                services::api::AuthFlow::Merchant,
                connector_action,
                None,
                HeaderPayload::default(),
                platform_merchant_account,
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
            .find_business_profile_by_profile_id(key_manager_state, &merchant_key_store, profile_id)
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
        Ok(services::ApplicationResponse::JsonForRedirection(
            helpers::get_handle_response_url(
                payment_id,
                &payment_flow_response.business_profile,
                &payment_flow_response.payments_response,
                connector,
            )?,
        ))
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
            | common_enums::IntentStatus::Failed
            | common_enums::IntentStatus::Cancelled
            | common_enums::IntentStatus::Processing
            | common_enums::IntentStatus::RequiresPaymentMethod
            | common_enums::IntentStatus::RequiresMerchantAction
            | common_enums::IntentStatus::RequiresCapture
            | common_enums::IntentStatus::PartiallyCaptured
            | common_enums::IntentStatus::RequiresConfirmation
            | common_enums::IntentStatus::PartiallyCapturedAndCapturable => {
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
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        profile: domain::Profile,
        req: PaymentsRedirectResponseData,
        platform_merchant_account: Option<domain::MerchantAccount>,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let payment_id = req.payment_id.clone();

        let payment_sync_request = api::PaymentsRetrieveRequest {
            param: Some(req.query_params.clone()),
            force_sync: true,
            expand_attempts: false,
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
                &merchant_account,
                &profile,
                &merchant_key_store,
                &HeaderPayload::default(),
                platform_merchant_account.as_ref(),
            )
            .await?;

        let payment_data = &get_tracker_response.payment_data;
        self.validate_status_for_operation(payment_data.payment_intent.status)?;

        let payment_attempt = payment_data
            .payment_attempt
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("payment_attempt not found in get_tracker_response")?;

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

        let (payment_data, _, _, _, _) =
            Box::pin(payments_operation_core::<api::PSync, _, _, _, _>(
                state,
                req_state,
                merchant_account,
                merchant_key_store.clone(),
                profile.clone(),
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
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        connector: String,
        payment_id: id_type::PaymentId,
        platform_merchant_account: Option<domain::MerchantAccount>,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let merchant_id = merchant_account.get_id().clone();
        let key_manager_state = &state.into();

        let payment_intent = state
            .store
            .find_payment_intent_by_payment_id_merchant_id(
                key_manager_state,
                &payment_id,
                &merchant_id,
                &merchant_key_store,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        let payment_attempt = state
            .store
            .find_payment_attempt_by_attempt_id_merchant_id(
                &payment_intent.active_attempt.get_id(),
                &merchant_id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        let authentication_id = payment_attempt
            .authentication_id
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("missing authentication_id in payment_attempt")?;
        let authentication = state
            .store
            .find_authentication_by_merchant_id_authentication_id(
                &merchant_id,
                authentication_id.clone(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::AuthenticationNotFound {
                id: authentication_id,
            })?;
        // Fetching merchant_connector_account to check if pull_mechanism is enabled for 3ds connector
        let authentication_merchant_connector_account = helpers::get_merchant_connector_account(
            state,
            &merchant_id,
            None,
            &merchant_key_store,
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
                merchant_account,
                None,
                merchant_key_store.clone(),
                PaymentConfirm,
                payment_confirm_req,
                services::api::AuthFlow::Merchant,
                connector_action,
                None,
                HeaderPayload::with_source(enums::PaymentSource::ExternalAuthenticator),
                platform_merchant_account,
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
            };
            Box::pin(
                payments_core::<api::PSync, api::PaymentsResponse, _, _, _, _>(
                    state.clone(),
                    req_state,
                    merchant_account.clone(),
                    None,
                    merchant_key_store.clone(),
                    PaymentStatus,
                    payment_sync_req,
                    services::api::AuthFlow::Merchant,
                    connector_action,
                    None,
                    HeaderPayload::default(),
                    platform_merchant_account,
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
                    crate::consts::POLL_ID_TTL,
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
            .find_business_profile_by_profile_id(key_manager_state, &merchant_key_store, profile_id)
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
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn call_connector_service<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
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
        merchant_account,
        payment_data,
        &connector.connector_name.to_string(),
        connector.merchant_connector_id.as_ref(),
        key_store,
        false,
    )
    .await?;

    #[cfg(feature = "v1")]
    if payment_data
        .get_payment_attempt()
        .merchant_connector_id
        .is_none()
    {
        payment_data.set_merchant_connector_id_in_attempt(merchant_connector_account.get_mca_id());
    }

    operation
        .to_domain()?
        .populate_payment_data(state, payment_data, merchant_account)
        .await?;

    let (pd, tokenization_action) = get_connector_tokenization_action_when_confirm_true(
        state,
        operation,
        payment_data,
        validate_result,
        &merchant_connector_account,
        key_store,
        customer,
        business_profile,
    )
    .await?;
    *payment_data = pd;

    // Validating the blocklist guard and generate the fingerprint
    blocklist_guard(state, merchant_account, key_store, operation, payment_data).await?;

    let updated_customer = call_create_connector_customer_if_required(
        state,
        customer,
        merchant_account,
        key_store,
        &merchant_connector_account,
        payment_data,
    )
    .await?;

    #[cfg(feature = "v1")]
    let merchant_recipient_data = if let Some(true) = payment_data
        .get_payment_intent()
        .is_payment_processor_token_flow
    {
        None
    } else {
        payment_data
            .get_merchant_recipient_data(
                state,
                merchant_account,
                key_store,
                &merchant_connector_account,
                &connector,
            )
            .await?
    };

    // TODO: handle how we read `is_processor_token_flow` in v2 and then call `get_merchant_recipient_data`
    #[cfg(feature = "v2")]
    let merchant_recipient_data = None;

    let mut router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            merchant_account,
            key_store,
            customer,
            &merchant_connector_account,
            merchant_recipient_data,
            None,
        )
        .await?;

    let add_access_token_result = router_data
        .add_access_token(
            state,
            &connector,
            merchant_account,
            payment_data.get_creds_identifier(),
        )
        .await?;

    router_data = router_data.add_session_token(state, &connector).await?;

    let should_continue_further = access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &call_connector_action,
    );

    router_data.payment_method_token = if let Some(decrypted_token) =
        add_decrypted_payment_method_token(tokenization_action.clone(), payment_data).await?
    {
        Some(decrypted_token)
    } else {
        router_data.payment_method_token
    };

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
            merchant_account.storage_scheme,
            updated_customer,
            key_store,
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
pub async fn call_connector_service<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
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

    let merchant_connector_id = connector
        .merchant_connector_id
        .as_ref()
        .get_required_value("merchant_connector_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("connector id is not set")?;

    let merchant_connector_account = state
        .store
        .find_merchant_connector_account_by_id(&state.into(), merchant_connector_id, key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_connector_id.get_string_repr().to_owned(),
        })?;

    let updated_customer = call_create_connector_customer_if_required(
        state,
        customer,
        merchant_account,
        key_store,
        &merchant_connector_account,
        payment_data,
    )
    .await?;

    let mut router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            merchant_account,
            key_store,
            customer,
            &merchant_connector_account,
            None,
            None,
        )
        .await?;

    let add_access_token_result = router_data
        .add_access_token(
            state,
            &connector,
            merchant_account,
            payment_data.get_creds_identifier(),
        )
        .await?;

    router_data = router_data.add_session_token(state, &connector).await?;

    let should_continue_further = access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &call_connector_action,
    );

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
            merchant_account.storage_scheme,
            updated_customer,
            key_store,
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
        router_data
            .decide_flows(
                state,
                &connector,
                call_connector_action,
                connector_request,
                business_profile,
                header_payload.clone(),
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

#[cfg(feature = "v1")]
// This function does not perform the tokenization action, as the payment method is not saved in this flow.
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn proxy_for_call_connector_service<F, RouterDReq, ApiRequest, D>(
    state: &SessionState,
    req_state: ReqState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    validate_result: &operations::ValidateResult,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,

    business_profile: &domain::Profile,
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
        merchant_account,
        payment_data,
        &connector.connector_name.to_string(),
        connector.merchant_connector_id.as_ref(),
        key_store,
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
            merchant_account,
            key_store,
            customer,
            &merchant_connector_account,
            merchant_recipient_data,
            None,
        )
        .await?;

    let add_access_token_result = router_data
        .add_access_token(
            state,
            &connector,
            merchant_account,
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
            merchant_account.storage_scheme,
            updated_customer,
            key_store,
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

pub async fn add_decrypted_payment_method_token<F, D>(
    tokenization_action: TokenizationAction,
    payment_data: &D,
) -> CustomResult<Option<PaymentMethodToken>, errors::ApiErrorResponse>
where
    F: Send + Clone + Sync,
    D: OperationSessionGetters<F> + Send + Sync + Clone,
{
    // Tokenization Action will be DecryptApplePayToken, only when payment method type is Apple Pay
    // and the connector supports Apple Pay predecrypt
    match &tokenization_action {
        TokenizationAction::DecryptApplePayToken(payment_processing_details)
        | TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt(
            payment_processing_details,
        ) => {
            let apple_pay_data = match payment_data.get_payment_method_data() {
                Some(domain::PaymentMethodData::Wallet(domain::WalletData::ApplePay(
                    wallet_data,
                ))) => Some(
                    ApplePayData::token_json(domain::WalletData::ApplePay(wallet_data.clone()))
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("failed to parse apple pay token to json")?
                        .decrypt(
                            &payment_processing_details.payment_processing_certificate,
                            &payment_processing_details.payment_processing_certificate_key,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("failed to decrypt apple pay token")?,
                ),
                _ => None,
            };

            let apple_pay_predecrypt = apple_pay_data
                .parse_value::<hyperswitch_domain_models::router_data::ApplePayPredecryptData>(
                    "ApplePayPredecryptData",
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "failed to parse decrypted apple pay response to ApplePayPredecryptData",
                )?;

            Ok(Some(PaymentMethodToken::ApplePayDecrypt(Box::new(
                apple_pay_predecrypt,
            ))))
        }
        TokenizationAction::DecryptPazeToken(payment_processing_details) => {
            let paze_data = match payment_data.get_payment_method_data() {
                Some(domain::PaymentMethodData::Wallet(domain::WalletData::Paze(wallet_data))) => {
                    Some(
                        decrypt_paze_token(
                            wallet_data.clone(),
                            payment_processing_details.paze_private_key.clone(),
                            payment_processing_details
                                .paze_private_key_passphrase
                                .clone(),
                        )
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("failed to decrypt paze token")?,
                    )
                }
                _ => None,
            };
            let paze_decrypted_data = paze_data
                .parse_value::<hyperswitch_domain_models::router_data::PazeDecryptedData>(
                    "PazeDecryptedData",
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to parse PazeDecryptedData")?;
            Ok(Some(PaymentMethodToken::PazeDecrypt(Box::new(
                paze_decrypted_data,
            ))))
        }
        TokenizationAction::DecryptGooglePayToken(payment_processing_details) => {
            let google_pay_data = match payment_data.get_payment_method_data() {
                Some(domain::PaymentMethodData::Wallet(domain::WalletData::GooglePay(
                    wallet_data,
                ))) => {
                    let decryptor = helpers::GooglePayTokenDecryptor::new(
                        payment_processing_details
                            .google_pay_root_signing_keys
                            .clone(),
                        payment_processing_details.google_pay_recipient_id.clone(),
                        payment_processing_details.google_pay_private_key.clone(),
                    )
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("failed to create google pay token decryptor")?;

                    // should_verify_token is set to false to disable verification of token
                    Some(
                        decryptor
                            .decrypt_token(wallet_data.tokenization_data.token.clone(), false)
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("failed to decrypt google pay token")?,
                    )
                }
                Some(payment_method_data) => {
                    logger::info!(
                        "Invalid payment_method_data found for Google Pay Decrypt Flow: {:?}",
                        payment_method_data.get_payment_method()
                    );
                    None
                }
                None => {
                    logger::info!("No payment_method_data found for Google Pay Decrypt Flow");
                    None
                }
            };

            let google_pay_predecrypt = google_pay_data
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to get GooglePayDecryptedData in response")?;

            Ok(Some(PaymentMethodToken::GooglePayDecrypt(Box::new(
                google_pay_predecrypt,
            ))))
        }
        TokenizationAction::ConnectorToken(_) => {
            logger::info!("Invalid tokenization action found for decryption flow: ConnectorToken",);
            Ok(None)
        }
        token_action => {
            logger::info!(
                "Invalid tokenization action found for decryption flow: {:?}",
                token_action
            );
            Ok(None)
        }
    }
}

pub async fn get_merchant_bank_data_for_open_banking_connectors(
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    key_store: &domain::MerchantKeyStore,
    connector: &api::ConnectorData,
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
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
            let merchant_id_str = merchant_account.get_id().get_string_repr().to_owned();
            let cust_id = id_type::CustomerId::try_from(std::borrow::Cow::from(merchant_id_str))
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to convert to CustomerId")?;
            let locker_resp = cards::get_payment_method_from_hs_locker(
                state,
                key_store,
                &cust_id,
                merchant_account.get_id(),
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
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    operation: &BoxedOperation<'_, F, ApiRequest, D>,
    payment_data: &mut D,
) -> CustomResult<bool, errors::ApiErrorResponse>
where
    F: Send + Clone + Sync,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let merchant_id = merchant_account.get_id();
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
            .guard_payment_against_blocklist(state, merchant_account, key_store, payment_data)
            .await?)
    } else {
        Ok(false)
    }
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn call_multiple_connectors_service<F, Op, Req, D>(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connectors: Vec<api::SessionConnectorData>,
    _operation: &Op,
    mut payment_data: D,
    customer: &Option<domain::Customer>,
    _session_surcharge_details: Option<api::SessionSurchargeDetails>,
    business_profile: &domain::Profile,
    header_payload: HeaderPayload,
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
        let merchant_connector_id = session_connector_data
            .connector
            .merchant_connector_id
            .as_ref()
            .get_required_value("merchant_connector_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("connector id is not set")?;
        // TODO: make this DB call parallel
        let merchant_connector_account = state
            .store
            .find_merchant_connector_account_by_id(&state.into(), merchant_connector_id, key_store)
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: merchant_connector_id.get_string_repr().to_owned(),
            })?;
        let connector_id = session_connector_data.connector.connector.id();
        let router_data = payment_data
            .construct_router_data(
                state,
                connector_id,
                merchant_account,
                key_store,
                customer,
                &merchant_connector_account,
                None,
                None,
            )
            .await?;

        let res = router_data.decide_flows(
            state,
            &session_connector_data.connector,
            CallConnectorAction::Trigger,
            None,
            business_profile,
            header_payload.clone(),
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
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connectors: Vec<api::SessionConnectorData>,
    _operation: &Op,
    mut payment_data: D,
    customer: &Option<domain::Customer>,
    session_surcharge_details: Option<api::SessionSurchargeDetails>,
    business_profile: &domain::Profile,
    header_payload: HeaderPayload,
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
            merchant_account,
            &payment_data,
            &session_connector_data.connector.connector_name.to_string(),
            session_connector_data
                .connector
                .merchant_connector_id
                .as_ref(),
            key_store,
            false,
        )
        .await?;

        payment_data.set_surcharge_details(session_surcharge_details.as_ref().and_then(
            |session_surcharge_details| {
                session_surcharge_details.fetch_surcharge_details(
                    session_connector_data.payment_method_type.into(),
                    session_connector_data.payment_method_type,
                    None,
                )
            },
        ));

        let router_data = payment_data
            .construct_router_data(
                state,
                connector_id,
                merchant_account,
                key_store,
                customer,
                &merchant_connector_account,
                None,
                None,
            )
            .await?;

        let res = router_data.decide_flows(
            state,
            &session_connector_data.connector,
            CallConnectorAction::Trigger,
            None,
            business_profile,
            header_payload.clone(),
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
                merchant_account.get_id(),
                key_store,
                value,
                payment_data.get_payment_intent(),
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
    key_store: &domain::MerchantKeyStore,
    authentication_product_ids: common_types::payments::AuthenticationConnectorAccountMap,
    payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
) -> RouterResult<api_models::payments::SessionToken> {
    let click_to_pay_mca_id = authentication_product_ids
        .get_click_to_pay_connector_account_id()
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "authentication_product_ids",
        })?;
    let key_manager_state = &(state).into();
    let merchant_connector_account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            key_manager_state,
            merchant_id,
            &click_to_pay_mca_id,
            key_store,
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

    Ok(api_models::payments::SessionToken::ClickToPay(Box::new(
        api_models::payments::ClickToPaySessionResponse {
            dpa_id: click_to_pay_metadata.dpa_id,
            dpa_name: click_to_pay_metadata.dpa_name,
            locale: click_to_pay_metadata.locale,
            card_brands: click_to_pay_metadata.card_brands,
            acquirer_bin: click_to_pay_metadata.acquirer_bin,
            acquirer_merchant_id: click_to_pay_metadata.acquirer_merchant_id,
            merchant_category_code: click_to_pay_metadata.merchant_category_code,
            merchant_country_code: click_to_pay_metadata.merchant_country_code,
            transaction_amount,
            transaction_currency_code: transaction_currency,
            phone_number: customer_details.phone.clone(),
            email: customer_details.email.clone(),
            phone_country_code: customer_details.phone_country_code.clone(),
        },
    )))
}

fn validate_customer_details_for_click_to_pay(customer_details: &CustomerData) -> RouterResult<()> {
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
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
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
                    state, &connector, customer, &label,
                );

            if should_call_connector {
                // Create customer at connector and update the customer table to store this data
                let router_data = payment_data
                    .construct_router_data(
                        state,
                        connector.connector.id(),
                        merchant_account,
                        key_store,
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
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    merchant_connector_account: &domain::MerchantConnectorAccount,
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
                Some(merchant_connector_account.get_id()),
            )?;

            let merchant_connector_id = merchant_connector_account.get_id();

            let (should_call_connector, existing_connector_customer_id) =
                customers::should_call_connector_create_customer(
                    state,
                    &connector,
                    customer,
                    &merchant_connector_id,
                );

            if should_call_connector {
                // Create customer at connector and update the customer table to store this data
                let router_data = payment_data
                    .construct_router_data(
                        state,
                        connector.connector.id(),
                        merchant_account,
                        key_store,
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
                    merchant_connector_id,
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
    if !is_operation_complete_authorize(&operation)
        && connector
            .connector_name
            .is_pre_processing_required_before_authorize()
    {
        router_data = router_data.preprocessing_steps(state, connector).await?;
        return Ok((router_data, should_continue_payment));
    }
    //TODO: For ACH transfers, if preprocessing_step is not required for connectors encountered in future, add the check
    let router_data_and_should_continue_payment = match payment_data.get_payment_method_data() {
        Some(domain::PaymentMethodData::BankTransfer(data)) => match data.deref() {
            domain::BankTransferData::AchBankTransfer { .. }
            | domain::BankTransferData::MultibancoBankTransfer { .. }
                if connector.connector_name == router_types::Connector::Stripe =>
            {
                if payment_data
                    .get_payment_attempt()
                    .preprocessing_step_id
                    .is_none()
                {
                    (
                        router_data.preprocessing_steps(state, connector).await?,
                        false,
                    )
                } else {
                    (router_data, should_continue_payment)
                }
            }
            _ => (router_data, should_continue_payment),
        },
        Some(domain::PaymentMethodData::Wallet(_)) => {
            if is_preprocessing_required_for_wallets(connector.connector_name.to_string()) {
                (
                    router_data.preprocessing_steps(state, connector).await?,
                    false,
                )
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
            } else if connector.connector_name == router_types::Connector::Cybersource
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
                && ((connector.connector_name == router_types::Connector::Nexixpay
                    && is_operation_complete_authorize(&operation))
                    || ((connector.connector_name == router_types::Connector::Nuvei
                        || connector.connector_name == router_types::Connector::Shift4)
                        && !is_operation_complete_authorize(&operation)))
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;
                (router_data, should_continue_payment)
            } else {
                (router_data, should_continue_payment)
            }
        }
        Some(domain::PaymentMethodData::GiftCard(_)) => {
            if connector.connector_name == router_types::Connector::Adyen {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                let is_error_in_response = router_data.response.is_err();
                // If is_error_in_response is true, should_continue_payment should be false, we should throw the error
                (router_data, !is_error_in_response)
            } else {
                (router_data, should_continue_payment)
            }
        }
        Some(domain::PaymentMethodData::BankDebit(_)) => {
            if connector.connector_name == router_types::Connector::Gocardless {
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
#[allow(clippy::too_many_arguments)]
async fn complete_postprocessing_steps_if_required<F, Q, RouterDReq, D>(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
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
            merchant_account,
            key_store,
            customer,
            merchant_conn_account,
            None,
            header_payload,
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
    merchant_account: &domain::MerchantAccount,
    payment_data: &D,
    connector_name: &str,
    merchant_connector_id: Option<&id_type::MerchantConnectorAccountId>,
    key_store: &domain::MerchantKeyStore,
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
        merchant_account.get_id(),
        payment_data.get_creds_identifier(),
        key_store,
        &profile_id,
        connector_name,
        merchant_connector_id,
    )
    .await?;

    Ok(merchant_connector_account)
}

fn is_payment_method_tokenization_enabled_for_connector(
    state: &SessionState,
    connector_name: &str,
    payment_method: storage::enums::PaymentMethod,
    payment_method_type: Option<storage::enums::PaymentMethodType>,
    apple_pay_flow: &Option<domain::ApplePayFlow>,
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
                    apple_pay_flow,
                    connector_filter.apple_pay_pre_decrypt_flow.clone(),
                )
        })
        .unwrap_or(false))
}

fn is_apple_pay_pre_decrypt_type_connector_tokenization(
    payment_method_type: Option<storage::enums::PaymentMethodType>,
    apple_pay_flow: &Option<domain::ApplePayFlow>,
    apple_pay_pre_decrypt_flow_filter: Option<ApplePayPreDecryptFlow>,
) -> bool {
    match (payment_method_type, apple_pay_flow) {
        (
            Some(storage::enums::PaymentMethodType::ApplePay),
            Some(domain::ApplePayFlow::Simplified(_)),
        ) => !matches!(
            apple_pay_pre_decrypt_flow_filter,
            Some(ApplePayPreDecryptFlow::NetworkTokenization)
        ),
        _ => true,
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

#[allow(clippy::too_many_arguments)]
async fn decide_payment_method_tokenize_action(
    state: &SessionState,
    connector_name: &str,
    payment_method: storage::enums::PaymentMethod,
    pm_parent_token: Option<&str>,
    is_connector_tokenization_enabled: bool,
    apple_pay_flow: Option<domain::ApplePayFlow>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
) -> RouterResult<TokenizationAction> {
    if let Some(storage_enums::PaymentMethodType::Paze) = payment_method_type {
        // Paze generates a one time use network token which should not be tokenized in the connector or router.
        match &state.conf.paze_decrypt_keys {
            Some(paze_keys) => Ok(TokenizationAction::DecryptPazeToken(
                PazePaymentProcessingDetails {
                    paze_private_key: paze_keys.get_inner().paze_private_key.clone(),
                    paze_private_key_passphrase: paze_keys
                        .get_inner()
                        .paze_private_key_passphrase
                        .clone(),
                },
            )),
            None => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to fetch Paze configs"),
        }
    } else if let Some(storage_enums::PaymentMethodType::GooglePay) = payment_method_type {
        let google_pay_details =
            get_google_pay_connector_wallet_details(state, merchant_connector_account);

        match google_pay_details {
            Some(wallet_details) => Ok(TokenizationAction::DecryptGooglePayToken(wallet_details)),
            None => {
                if is_connector_tokenization_enabled {
                    Ok(TokenizationAction::TokenizeInConnectorAndRouter)
                } else {
                    Ok(TokenizationAction::TokenizeInRouter)
                }
            }
        }
    } else {
        match pm_parent_token {
            None => Ok(match (is_connector_tokenization_enabled, apple_pay_flow) {
                (true, Some(domain::ApplePayFlow::Simplified(payment_processing_details))) => {
                    TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt(
                        payment_processing_details,
                    )
                }
                (true, _) => TokenizationAction::TokenizeInConnectorAndRouter,
                (false, Some(domain::ApplePayFlow::Simplified(payment_processing_details))) => {
                    TokenizationAction::DecryptApplePayToken(payment_processing_details)
                }
                (false, _) => TokenizationAction::TokenizeInRouter,
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
                    None => Ok(match (is_connector_tokenization_enabled, apple_pay_flow) {
                        (
                            true,
                            Some(domain::ApplePayFlow::Simplified(payment_processing_details)),
                        ) => TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt(
                            payment_processing_details,
                        ),
                        (true, _) => TokenizationAction::TokenizeInConnectorAndRouter,
                        (
                            false,
                            Some(domain::ApplePayFlow::Simplified(payment_processing_details)),
                        ) => TokenizationAction::DecryptApplePayToken(payment_processing_details),
                        (false, _) => TokenizationAction::TokenizeInRouter,
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

#[derive(Clone, Debug)]
pub enum TokenizationAction {
    TokenizeInRouter,
    TokenizeInConnector,
    TokenizeInConnectorAndRouter,
    ConnectorToken(String),
    SkipConnectorTokenization,
    DecryptApplePayToken(payments_api::PaymentProcessingDetails),
    TokenizeInConnectorAndApplepayPreDecrypt(payments_api::PaymentProcessingDetails),
    DecryptPazeToken(PazePaymentProcessingDetails),
    DecryptGooglePayToken(GooglePayPaymentProcessingDetails),
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn get_connector_tokenization_action_when_confirm_true<F, Req, D>(
    _state: &SessionState,
    _operation: &BoxedOperation<'_, F, Req, D>,
    payment_data: &mut D,
    _validate_result: &operations::ValidateResult,
    _merchant_connector_account: &helpers::MerchantConnectorAccountType,
    _merchant_key_store: &domain::MerchantKeyStore,
    _customer: &Option<domain::Customer>,
    _business_profile: &domain::Profile,
) -> RouterResult<(D, TokenizationAction)>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    // TODO: Implement this function
    let payment_data = payment_data.to_owned();
    Ok((payment_data, TokenizationAction::SkipConnectorTokenization))
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn get_connector_tokenization_action_when_confirm_true<F, Req, D>(
    state: &SessionState,
    operation: &BoxedOperation<'_, F, Req, D>,
    payment_data: &mut D,
    validate_result: &operations::ValidateResult,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    merchant_key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
    business_profile: &domain::Profile,
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

            let apple_pay_flow =
                decide_apple_pay_flow(state, payment_method_type, Some(merchant_connector_account));

            let is_connector_tokenization_enabled =
                is_payment_method_tokenization_enabled_for_connector(
                    state,
                    &connector,
                    payment_method,
                    payment_method_type,
                    &apple_pay_flow,
                )?;

            add_apple_pay_flow_metrics(
                &apple_pay_flow,
                payment_data.get_payment_attempt().connector.clone(),
                payment_data.get_payment_attempt().merchant_id.clone(),
            );

            let payment_method_action = decide_payment_method_tokenize_action(
                state,
                &connector,
                payment_method,
                payment_data.get_token(),
                is_connector_tokenization_enabled,
                apple_pay_flow,
                payment_method_type,
                merchant_connector_account,
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
                TokenizationAction::DecryptApplePayToken(payment_processing_details) => {
                    TokenizationAction::DecryptApplePayToken(payment_processing_details)
                }
                TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt(
                    payment_processing_details,
                ) => TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt(
                    payment_processing_details,
                ),
                TokenizationAction::DecryptPazeToken(paze_payment_processing_details) => {
                    TokenizationAction::DecryptPazeToken(paze_payment_processing_details)
                }
                TokenizationAction::DecryptGooglePayToken(
                    google_pay_payment_processing_details,
                ) => {
                    TokenizationAction::DecryptGooglePayToken(google_pay_payment_processing_details)
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
    pub customer_acceptance: Option<CustomerAcceptance>,
    pub address: PaymentAddress,
    pub token: Option<String>,
    pub token_data: Option<storage::PaymentTokenData>,
    pub confirm: Option<bool>,
    pub force_sync: Option<bool>,
    pub payment_method_data: Option<domain::PaymentMethodData>,
    pub payment_method_info: Option<domain::PaymentMethod>,
    pub refunds: Vec<storage::Refund>,
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
    pub authentication: Option<storage::Authentication>,
    pub recurring_details: Option<RecurringDetails>,
    pub poll_config: Option<router_types::PollConfig>,
    pub tax_data: Option<TaxData>,
    pub session_id: Option<String>,
    pub service_details: Option<api_models::payments::CtpServiceDetails>,
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

pub trait CustomerDetailsExt {
    type Error;
    fn get_name(&self) -> Result<Secret<String, masking::WithType>, Self::Error>;
    fn get_email(&self) -> Result<pii::Email, Self::Error>;
}

impl CustomerDetailsExt for CustomerDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn get_name(&self) -> Result<Secret<String, masking::WithType>, Self::Error> {
        self.name.clone().ok_or_else(missing_field_err("name"))
    }
    fn get_email(&self) -> Result<pii::Email, Self::Error> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
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
            matches!(
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
        "PaymentCapture" => {
            matches!(
                payment_data.get_payment_intent().status,
                storage_enums::IntentStatus::RequiresCapture
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
    merchant: domain::MerchantAccount,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    key_store: domain::MerchantKeyStore,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<api::PaymentListResponse> {
    use hyperswitch_domain_models::errors::StorageError;
    helpers::validate_payment_list_request(&constraints)?;
    let merchant_id = merchant.get_id();
    let db = state.store.as_ref();
    let payment_intents = helpers::filter_by_constraints(
        &state,
        &(constraints, profile_id_list).try_into()?,
        merchant_id,
        &key_store,
        merchant.storage_scheme,
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
                    if matches!(error.current_context(), StorageError::ValueNotFound(_)) {
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

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn apply_filters_on_payments(
    state: SessionState,
    merchant: domain::MerchantAccount,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    merchant_key_store: domain::MerchantKeyStore,
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
                    &(&state).into(),
                    merchant.get_id(),
                    &pi_fetch_constraints,
                    &merchant_key_store,
                    merchant.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let data: Vec<api::PaymentsResponse> =
                list.into_iter().map(ForeignFrom::foreign_from).collect();

            let active_attempt_ids = db
                .get_filtered_active_attempt_ids_for_total_count(
                    merchant.get_id(),
                    &pi_fetch_constraints,
                    merchant.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

            let total_count = if constraints.has_no_attempt_filters() {
                i64::try_from(active_attempt_ids.len())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while converting from usize to i64")
            } else {
                db.get_total_count_of_filtered_payment_attempts(
                    merchant.get_id(),
                    &active_attempt_ids,
                    constraints.connector,
                    constraints.payment_method,
                    constraints.payment_method_type,
                    constraints.authentication_type,
                    constraints.merchant_connector_id,
                    constraints.card_network,
                    constraints.card_discovery,
                    merchant.storage_scheme,
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
        router_env::metric_attributes!(("merchant_id", merchant.get_id().clone())),
    )
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_filters_for_payments(
    state: SessionState,
    merchant: domain::MerchantAccount,
    merchant_key_store: domain::MerchantKeyStore,
    time_range: common_utils::types::TimeRange,
) -> RouterResponse<api::PaymentListFilters> {
    let db = state.store.as_ref();
    let pi = db
        .filter_payment_intents_by_time_range_constraints(
            &(&state).into(),
            merchant.get_id(),
            &time_range,
            &merchant_key_store,
            merchant.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let filters = db
        .get_filters_for_payments(
            pi.as_slice(),
            merchant.get_id(),
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

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_payment_filters(
    state: SessionState,
    merchant: domain::MerchantAccount,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
) -> RouterResponse<api::PaymentListFiltersV2> {
    let merchant_connector_accounts = if let services::ApplicationResponse::Json(data) =
        super::admin::list_payment_connectors(state, merchant.get_id().to_owned(), profile_id_list)
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
                    (merchant_connector_account.connector_name.clone(), info)
                })
        })
        .for_each(|(connector_name, info)| {
            connector_map
                .entry(connector_name.clone())
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
                        .payment_method_types
                        .as_ref()
                        .map(|types_vec| (payment_method_enabled.payment_method, types_vec.clone()))
                })
        })
        .for_each(|payment_methods_enabled| {
            payment_methods_enabled.for_each(|(payment_method, payment_method_types_vec)| {
                payment_method_types_map
                    .entry(payment_method)
                    .or_default()
                    .extend(
                        payment_method_types_vec
                            .iter()
                            .map(|p| p.payment_method_type),
                    );
            });
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

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_aggregates_for_payments(
    state: SessionState,
    merchant: domain::MerchantAccount,
    profile_id_list: Option<Vec<id_type::ProfileId>>,
    time_range: common_utils::types::TimeRange,
) -> RouterResponse<api::PaymentsAggregateResponse> {
    let db = state.store.as_ref();
    let intent_status_with_count = db
        .get_intent_status_with_count(merchant.get_id(), profile_id_list, &time_range)
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
        schedule_time,
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
    let _: api_models::routing::RoutingAlgorithm = request_straight_through
        .clone()
        .parse_value("RoutingAlgorithm")
        .attach_printable("Invalid straight through routing rules format")?;

    payment_data.set_straight_through_algorithm_in_payment_attempt(request_straight_through);

    Ok(())
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn get_connector_choice<F, Req, D>(
    operation: &BoxedOperation<'_, F, Req, D>,
    state: &SessionState,
    req: &Req,
    merchant_account: &domain::MerchantAccount,
    business_profile: &domain::Profile,
    key_store: &domain::MerchantKeyStore,
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
            merchant_account,
            &state.clone(),
            req,
            payment_data.get_payment_intent(),
            key_store,
        )
        .await?;

    let connector = if should_call_connector(operation, payment_data) {
        Some(match connector_choice {
            api::ConnectorChoice::SessionMultiple(connectors) => {
                let routing_output = perform_session_token_routing(
                    state.clone(),
                    merchant_account,
                    key_store,
                    payment_data,
                    connectors,
                )
                .await?;
                ConnectorCallType::SessionMultiple(routing_output)
            }

            api::ConnectorChoice::StraightThrough(straight_through) => {
                connector_selection(
                    state,
                    merchant_account,
                    business_profile,
                    key_store,
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
                    merchant_account,
                    business_profile,
                    key_store,
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
    merchant_account: &domain::MerchantAccount,
    business_profile: &domain::Profile,
    key_store: &domain::MerchantKeyStore,
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
        merchant_account,
        business_profile,
        key_store,
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

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v2")]
pub async fn decide_connector<F, D>(
    state: SessionState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &domain::Profile,
    key_store: &domain::MerchantKeyStore,
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
    todo!()
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v1")]
pub async fn decide_connector<F, D>(
    state: SessionState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &domain::Profile,
    key_store: &domain::MerchantKeyStore,
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
        return Ok(ConnectorCallType::PreDetermined(connector_data));
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

        return Ok(ConnectorCallType::PreDetermined(connector_data));
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
                .attach_printable("Invalid connector name received")?;

                pre_routing_connector_data_list.push(connector_data);
            }

            #[cfg(feature = "retry")]
            let should_do_retry = retry::config_should_call_gsm(
                &*state.store,
                merchant_account.get_id(),
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
                    merchant_account,
                    payment_data,
                    key_store,
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
                key_store,
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
                key_store,
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
                    conn.merchant_connector_id,
                )
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
        merchant_account,
        business_profile,
        key_store,
        payment_data,
        transaction_data,
        routing_data,
        eligible_connectors,
        mandate_type,
    )
    .await
}

#[cfg(feature = "v2")]
pub async fn decide_multiplex_connector_for_normal_or_recurring_payment<F: Clone, D>(
    state: &SessionState,
    payment_data: &mut D,
    routing_data: &mut storage::RoutingData,
    connectors: Vec<api::ConnectorData>,
    mandate_type: Option<api::MandateTransactionType>,
    is_connector_agnostic_mit_enabled: Option<bool>,
) -> RouterResult<ConnectorCallType>
where
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    todo!()
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn decide_multiplex_connector_for_normal_or_recurring_payment<F: Clone, D>(
    state: &SessionState,
    payment_data: &mut D,
    routing_data: &mut storage::RoutingData,
    connectors: Vec<api::ConnectorData>,
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
            logger::debug!("performing routing for token-based MIT flow");

            let payment_method_info = payment_data
                .get_payment_method_info()
                .get_required_value("payment_method_info")?
                .clone();

            //fetch connectors that support ntid flow
            let ntid_supported_connectors = &state
                .conf
                .network_transaction_id_supported_connectors
                .connector_list;
            //filered connectors list with ntid_supported_connectors
            let filtered_ntid_supported_connectors =
                filter_ntid_supported_connectors(connectors.clone(), ntid_supported_connectors);

            //fetch connectors that support network tokenization flow
            let network_tokenization_supported_connectors = &state
                .conf
                .network_tokenization_supported_connectors
                .connector_list;
            //filered connectors list with ntid_supported_connectors and network_tokenization_supported_connectors
            let filtered_nt_supported_connectors = filter_network_tokenization_supported_connectors(
                filtered_ntid_supported_connectors,
                network_tokenization_supported_connectors,
            );

            let action_type = decide_action_type(
                state,
                is_connector_agnostic_mit_enabled,
                is_network_tokenization_enabled,
                &payment_method_info,
                filtered_nt_supported_connectors.clone(),
            )
            .await;

            match action_type {
                Some(ActionType::NetworkTokenWithNetworkTransactionId(nt_data)) => {
                    logger::info!(
                        "using network_tokenization with network_transaction_id for MIT flow"
                    );

                    let mandate_reference_id =
                        Some(payments_api::MandateReferenceId::NetworkTokenWithNTI(
                            payments_api::NetworkTokenWithNTIRef {
                                network_transaction_id: nt_data.network_transaction_id.to_string(),
                                token_exp_month: nt_data.token_exp_month,
                                token_exp_year: nt_data.token_exp_year,
                            },
                        ));
                    let chosen_connector_data = filtered_nt_supported_connectors
                        .first()
                        .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                        .attach_printable(
                            "no eligible connector found for token-based MIT payment",
                        )?;

                    routing_data.routed_through =
                        Some(chosen_connector_data.connector_name.to_string());

                    routing_data
                        .merchant_connector_id
                        .clone_from(&chosen_connector_data.merchant_connector_id);

                    payment_data.set_mandate_id(payments_api::MandateIds {
                        mandate_id: None,
                        mandate_reference_id,
                    });

                    Ok(ConnectorCallType::PreDetermined(
                        chosen_connector_data.clone(),
                    ))
                }
                None => {
                    decide_connector_for_normal_or_recurring_payment(
                        state,
                        payment_data,
                        routing_data,
                        connectors,
                        is_connector_agnostic_mit_enabled,
                        &payment_method_info,
                    )
                    .await
                }
            }
        }
        (
            None,
            None,
            Some(RecurringDetails::ProcessorPaymentToken(_token)),
            Some(true),
            Some(api::MandateTransactionType::RecurringMandateTransaction),
        ) => {
            if let Some(connector) = connectors.first() {
                routing_data.routed_through = Some(connector.connector_name.clone().to_string());
                routing_data
                    .merchant_connector_id
                    .clone_from(&connector.merchant_connector_id);
                Ok(ConnectorCallType::PreDetermined(api::ConnectorData {
                    connector: connector.connector.clone(),
                    connector_name: connector.connector_name,
                    get_token: connector.get_token.clone(),
                    merchant_connector_id: connector.merchant_connector_id.clone(),
                }))
            } else {
                logger::error!("no eligible connector found for the ppt_mandate payment");
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

            routing_data.routed_through = Some(first_choice.connector_name.to_string());

            routing_data.merchant_connector_id = first_choice.merchant_connector_id;

            Ok(ConnectorCallType::Retryable(connectors))
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[allow(clippy::too_many_arguments)]
pub async fn decide_connector_for_normal_or_recurring_payment<F: Clone, D>(
    state: &SessionState,
    payment_data: &mut D,
    routing_data: &mut storage::RoutingData,
    connectors: Vec<api::ConnectorData>,
    is_connector_agnostic_mit_enabled: Option<bool>,
    payment_method_info: &domain::PaymentMethod,
) -> RouterResult<ConnectorCallType>
where
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    todo!()
}

#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
#[allow(clippy::too_many_arguments)]
pub async fn decide_connector_for_normal_or_recurring_payment<F: Clone, D>(
    state: &SessionState,
    payment_data: &mut D,
    routing_data: &mut storage::RoutingData,
    connectors: Vec<api::ConnectorData>,
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

    let connector_mandate_details = connector_common_mandate_details.payments;

    let mut connector_choice = None;

    for connector_data in connectors {
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
            logger::info!("using connector_mandate_id for MIT flow");
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
                                    Some(mandate_reference_record.connector_mandate_id.clone()),  // connector_mandate_id
                                    Some(payment_method_info.get_id().clone()),                  // payment_method_id
                                    None,                                                        // update_history
                                    mandate_reference_record.mandate_metadata.clone(),           // mandate_metadata
                                    mandate_reference_record.connector_mandate_request_reference_id.clone(), // connector_mandate_request_reference_id
                                )
                            ));
                            payment_data.set_recurring_mandate_payment_data(
                                hyperswitch_domain_models::router_data::RecurringMandatePaymentData {
                                    payment_method_type: mandate_reference_record
                                        .payment_method_type,
                                    original_payment_authorized_amount: mandate_reference_record
                                        .original_payment_authorized_amount,
                                    original_payment_authorized_currency: mandate_reference_record
                                        .original_payment_authorized_currency,
                                    mandate_metadata: mandate_reference_record
                                        .mandate_metadata.clone()
                                });
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

    Ok(ConnectorCallType::PreDetermined(chosen_connector_data))
}

pub fn filter_ntid_supported_connectors(
    connectors: Vec<api::ConnectorData>,
    ntid_supported_connectors: &HashSet<enums::Connector>,
) -> Vec<api::ConnectorData> {
    connectors
        .into_iter()
        .filter(|data| ntid_supported_connectors.contains(&data.connector_name))
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

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Eq, PartialEq)]
pub enum ActionType {
    NetworkTokenWithNetworkTransactionId(NTWithNTIRef),
}

pub fn filter_network_tokenization_supported_connectors(
    connectors: Vec<api::ConnectorData>,
    network_tokenization_supported_connectors: &HashSet<enums::Connector>,
) -> Vec<api::ConnectorData> {
    connectors
        .into_iter()
        .filter(|data| network_tokenization_supported_connectors.contains(&data.connector_name))
        .collect()
}

#[cfg(feature = "v1")]
pub async fn decide_action_type(
    state: &SessionState,
    is_connector_agnostic_mit_enabled: Option<bool>,
    is_network_tokenization_enabled: bool,
    payment_method_info: &domain::PaymentMethod,
    filtered_nt_supported_connectors: Vec<api::ConnectorData>, //network tokenization supported connectors
) -> Option<ActionType> {
    match (
        is_network_token_with_network_transaction_id_flow(
            is_connector_agnostic_mit_enabled,
            is_network_tokenization_enabled,
            payment_method_info,
        ),
        !filtered_nt_supported_connectors.is_empty(),
    ) {
        (IsNtWithNtiFlow::NtWithNtiSupported(network_transaction_id), true) => {
            if let Ok((token_exp_month, token_exp_year)) =
                network_tokenization::do_status_check_for_network_token(state, payment_method_info)
                    .await
            {
                Some(ActionType::NetworkTokenWithNetworkTransactionId(
                    NTWithNTIRef {
                        token_exp_month,
                        token_exp_year,
                        network_transaction_id,
                    },
                ))
            } else {
                None
            }
        }
        (IsNtWithNtiFlow::NtWithNtiSupported(_), false)
        | (IsNtWithNtiFlow::NTWithNTINotSupported, _) => None,
    }
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

pub async fn perform_session_token_routing<F, D>(
    state: SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_data: &D,
    connectors: Vec<api::SessionConnectorData>,
) -> RouterResult<Vec<api::SessionConnectorData>>
where
    F: Clone,
    D: OperationSessionGetters<F>,
{
    // Commenting out this code as `list_payment_method_api` and `perform_session_token_routing`
    // will happen in parallel the behaviour of the session call differ based on filters in
    // list_payment_method_api

    // let routing_info: Option<storage::PaymentRoutingInfo> = payment_data
    //     .get_payment_attempt()
    //     .straight_through_algorithm
    //     .clone()
    //     .map(|val| val.parse_value("PaymentRoutingInfo"))
    //     .transpose()
    //     .change_context(errors::ApiErrorResponse::InternalServerError)
    //     .attach_printable("invalid payment routing info format found in payment attempt")?;

    // if let Some(storage::PaymentRoutingInfo {
    //     pre_routing_results: Some(pre_routing_results),
    //     ..
    // }) = routing_info
    // {
    //     let mut payment_methods: rustc_hash::FxHashMap<
    //         (String, enums::PaymentMethodType),
    //         api::SessionConnectorData,
    //     > = rustc_hash::FxHashMap::from_iter(connectors.iter().map(|c| {
    //         (
    //             (
    //                 c.connector.connector_name.to_string(),
    //                 c.payment_method_type,
    //             ),
    //             c.clone(),
    //         )
    //     }));

    //     let mut final_list: Vec<api::SessionConnectorData> = Vec::new();
    //     for (routed_pm_type, pre_routing_choice) in pre_routing_results.into_iter() {
    //         let routable_connector_list = match pre_routing_choice {
    //             storage::PreRoutingConnectorChoice::Single(routable_connector) => {
    //                 vec![routable_connector.clone()]
    //             }
    //             storage::PreRoutingConnectorChoice::Multiple(routable_connector_list) => {
    //                 routable_connector_list.clone()
    //             }
    //         };
    //         for routable_connector in routable_connector_list {
    //             if let Some(session_connector_data) =
    //                 payment_methods.remove(&(routable_connector.to_string(), routed_pm_type))
    //             {
    //                 final_list.push(session_connector_data);
    //                 break;
    //             }
    //         }
    //     }

    //     if !final_list.is_empty() {
    //         return Ok(final_list);
    //     }
    // }

    let routing_enabled_pms = HashSet::from([
        enums::PaymentMethodType::GooglePay,
        enums::PaymentMethodType::ApplePay,
        enums::PaymentMethodType::Klarna,
        enums::PaymentMethodType::Paypal,
    ]);

    let mut chosen = Vec::<api::SessionConnectorData>::new();
    for connector_data in &connectors {
        if routing_enabled_pms.contains(&connector_data.payment_method_type) {
            chosen.push(connector_data.clone());
        }
    }
    let sfr = SessionFlowRoutingInput {
        state: &state,
        country: payment_data
            .get_address()
            .get_payment_method_billing()
            .and_then(|address| address.address.as_ref())
            .and_then(|details| details.country),
        key_store,
        merchant_account,
        payment_attempt: payment_data.get_payment_attempt(),
        payment_intent: payment_data.get_payment_intent(),

        chosen,
    };
    let result = self_routing::perform_session_flow_routing(sfr, &enums::TransactionType::Payment)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error performing session flow routing")?;

    let mut final_list: Vec<api::SessionConnectorData> = Vec::new();

    for connector_data in connectors {
        if !routing_enabled_pms.contains(&connector_data.payment_method_type) {
            final_list.push(connector_data);
        } else if let Some(choice) = result.get(&connector_data.payment_method_type) {
            let routing_choice = choice
                .first()
                .ok_or(errors::ApiErrorResponse::InternalServerError)?;
            if connector_data.connector.connector_name == routing_choice.connector.connector_name
                && connector_data.connector.merchant_connector_id
                    == routing_choice.connector.merchant_connector_id
            {
                final_list.push(connector_data);
            }
        }
    }

    Ok(final_list)
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn route_connector_v1_for_payments<F, D>(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &domain::Profile,
    key_store: &domain::MerchantKeyStore,
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

    let connectors = routing::perform_static_routing_v1(
        state,
        merchant_account.get_id(),
        routing_algorithm_id.as_ref(),
        business_profile,
        &TransactionData::Payment(transaction_data.clone()),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let connectors = routing::perform_eligibility_analysis_with_fallback(
        &state.clone(),
        key_store,
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
    let connectors = {
        if let Some(algo) = business_profile.dynamic_routing_algorithm.clone() {
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
                    split: crate::consts::DYNAMIC_ROUTING_MAX_VOLUME
                        - dynamic_routing_config
                            .dynamic_routing_volume_split
                            .unwrap_or_default(),
                };
            let volume_split_vec = vec![dynamic_split, static_split];
            let routing_choice =
                routing::perform_dynamic_routing_volume_split(volume_split_vec, None)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("failed to perform volume split on routing type")?;

            if routing_choice.routing_type.is_dynamic_routing() {
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

                routing::perform_dynamic_routing(
                    state,
                    connectors.clone(),
                    business_profile,
                    dynamic_routing_config_params_interpolator,
                )
                .await
                .map_err(|e| logger::error!(dynamic_routing_error=?e))
                .unwrap_or(connectors)
            } else {
                connectors
            }
        } else {
            connectors
        }
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
    merchant_account: &domain::MerchantAccount,
    business_profile: &domain::Profile,
    key_store: &domain::MerchantKeyStore,
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
    merchant_account: &domain::MerchantAccount,
    business_profile: &domain::Profile,
    key_store: &domain::MerchantKeyStore,
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

    let connectors = routing::perform_static_routing_v1(
        state,
        merchant_account.get_id(),
        routing_algorithm_id.as_ref(),
        business_profile,
        &TransactionData::Payout(transaction_data),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let connectors = routing::perform_eligibility_analysis_with_fallback(
        &state.clone(),
        key_store,
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
        })
        .collect::<CustomResult<Vec<_>, _>>()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received")?;

    routing_data.routed_through = Some(first_connector_choice.connector.to_string());

    routing_data.merchant_connector_id = first_connector_choice.merchant_connector_id;

    Ok(ConnectorCallType::Retryable(connector_data))
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub async fn payment_external_authentication(
    _state: SessionState,
    _merchant_account: domain::MerchantAccount,
    _key_store: domain::MerchantKeyStore,
    _req: api_models::payments::PaymentsExternalAuthenticationRequest,
) -> RouterResponse<api_models::payments::PaymentsExternalAuthenticationResponse> {
    todo!()
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument(skip_all)]
pub async fn payment_external_authentication<F: Clone + Sync>(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: api_models::payments::PaymentsExternalAuthenticationRequest,
) -> RouterResponse<api_models::payments::PaymentsExternalAuthenticationResponse> {
    use super::unified_authentication_service::types::ExternalAuthentication;
    use crate::core::unified_authentication_service::{
        types::UnifiedAuthenticationService, utils::external_authentication_update_trackers,
    };

    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let merchant_id = merchant_account.get_id();
    let storage_scheme = merchant_account.storage_scheme;
    let payment_id = req.payment_id;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            key_manager_state,
            &payment_id,
            merchant_id,
            &key_store,
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
                    key_manager_state,
                    customer_id,
                    merchant_account.get_id(),
                    &key_store,
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
        &key_store,
        &payment_intent.payment_id,
        storage_scheme,
    )
    .await?;
    let billing_address = helpers::create_or_find_address_for_payment_by_request(
        &state,
        None,
        payment_intent.billing_address_id.as_deref(),
        merchant_id,
        payment_intent.customer_id.as_ref(),
        &key_store,
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
        &key_store,
        profile_id,
        authentication_connector.as_str(),
        None,
    )
    .await?;
    let authentication = db
        .find_authentication_by_merchant_id_authentication_id(
            merchant_id,
            payment_attempt
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
        .find_business_profile_by_profile_id(key_manager_state, &key_store, profile_id)
        .await
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let payment_method_details = helpers::get_payment_method_details_from_payment_token(
        &state,
        &payment_attempt,
        &payment_intent,
        &key_store,
        storage_scheme,
        &business_profile,
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

    let authentication_response =
        if helpers::is_merchant_eligible_authentication_service(merchant_account.get_id(), &state)
            .await?
        {
            let auth_response =
                <ExternalAuthentication as UnifiedAuthenticationService<F>>::authentication(
                    &state,
                    &business_profile,
                    payment_method_details.1,
                    payment_method_details.0,
                    billing_address
                        .as_ref()
                        .map(|address| address.into())
                        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "billing_address",
                        })?,
                    shipping_address.as_ref().map(|address| address.into()),
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
                    authentication_details.three_ds_requestor_url.clone(),
                    &merchant_connector_account,
                    &authentication_connector,
                )
                .await?;
            let authentication = external_authentication_update_trackers(
                &state,
                auth_response,
                authentication.clone(),
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
            acs_reference_number: authentication_response.acs_reference_number,
            acs_trans_id: authentication_response.acs_trans_id,
            three_dsserver_trans_id: authentication_response.three_dsserver_trans_id,
            acs_signed_content: authentication_response.acs_signed_content,
            three_ds_requestor_url: authentication_details.three_ds_requestor_url,
        },
    ))
}

#[instrument(skip_all)]
#[cfg(feature = "v2")]
pub async fn payment_start_redirection(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: api_models::payments::PaymentStartRedirectionRequest,
) -> RouterResponse<serde_json::Value> {
    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let storage_scheme = merchant_account.storage_scheme;

    let payment_intent = db
        .find_payment_intent_by_id(key_manager_state, &req.id, &key_store, storage_scheme)
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
            key_manager_state,
            &key_store,
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
    } = req;
    let key_manager_state = &(&state).into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        .attach_printable("Error while fetching the key store by merchant_id")?;
    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(key_manager_state, &merchant_id, &key_store)
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

    let payment_intent = state
        .store
        .find_payment_intent_by_payment_id_merchant_id(
            key_manager_state,
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
                key_manager_state,
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
    fn get_payment_intent(&self) -> &storage::PaymentIntent;
    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod>;
    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds>;
    fn get_address(&self) -> &PaymentAddress;
    fn get_creds_identifier(&self) -> Option<&str>;
    fn get_token(&self) -> Option<&str>;
    fn get_multiple_capture_data(&self) -> Option<&types::MultipleCaptureData>;
    fn get_payment_link_data(&self) -> Option<api_models::payments::PaymentLinkResponse>;
    fn get_ephemeral_key(&self) -> Option<ephemeral_key::EphemeralKey>;
    fn get_setup_mandate(&self) -> Option<&MandateData>;
    fn get_poll_config(&self) -> Option<router_types::PollConfig>;
    fn get_authentication(&self) -> Option<&storage::Authentication>;
    fn get_frm_message(&self) -> Option<FraudCheck>;
    fn get_refunds(&self) -> Vec<storage::Refund>;
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
    fn get_capture_method(&self) -> Option<enums::CaptureMethod>;

    #[cfg(feature = "v2")]
    fn get_optional_payment_attempt(&self) -> Option<&storage::PaymentAttempt>;
}

pub trait OperationSessionSetters<F> {
    // Setter functions for PaymentData
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent);
    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt);
    fn set_payment_method_data(&mut self, payment_method_data: Option<domain::PaymentMethodData>);
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
    fn set_connector_in_payment_attempt(&mut self, connector: Option<String>);
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

    fn get_mandate_id(&self) -> Option<&payments_api::MandateIds> {
        self.mandate_id.as_ref()
    }

    // what is this address find out and not required remove this
    fn get_address(&self) -> &PaymentAddress {
        &self.address
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

    fn get_authentication(&self) -> Option<&storage::Authentication> {
        self.authentication.as_ref()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        self.frm_message.clone()
    }

    fn get_refunds(&self) -> Vec<storage::Refund> {
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

    #[cfg(feature = "v1")]
    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        self.payment_attempt.capture_method
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
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentIntentData<F> {
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        todo!()
    }

    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
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

    fn get_authentication(&self) -> Option<&storage::Authentication> {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<storage::Refund> {
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
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionSetters<F> for PaymentIntentData<F> {
    // Setters Implementation
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }

    fn set_payment_attempt(&mut self, _payment_attempt: storage::PaymentAttempt) {
        todo!()
    }

    fn set_payment_method_data(&mut self, _payment_method_data: Option<domain::PaymentMethodData>) {
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
        todo!()
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
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentConfirmData<F> {
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        &self.payment_attempt
    }

    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
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

    fn get_authentication(&self) -> Option<&storage::Authentication> {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<storage::Refund> {
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
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionSetters<F> for PaymentConfirmData<F> {
    // Setters Implementation
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }

    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt) {
        self.payment_attempt = payment_attempt;
    }

    fn set_payment_method_data(&mut self, _payment_method_data: Option<domain::PaymentMethodData>) {
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
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentStatusData<F> {
    #[track_caller]
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        todo!()
    }

    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
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

    fn get_authentication(&self) -> Option<&storage::Authentication> {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<storage::Refund> {
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
        self.payment_attempt.as_ref()
    }
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionSetters<F> for PaymentStatusData<F> {
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }

    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt) {
        self.payment_attempt = Some(payment_attempt);
    }

    fn set_payment_method_data(&mut self, _payment_method_data: Option<domain::PaymentMethodData>) {
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
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionGetters<F> for PaymentCaptureData<F> {
    #[track_caller]
    fn get_payment_attempt(&self) -> &storage::PaymentAttempt {
        &self.payment_attempt
    }

    fn get_payment_intent(&self) -> &storage::PaymentIntent {
        &self.payment_intent
    }

    fn get_payment_method_info(&self) -> Option<&domain::PaymentMethod> {
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

    fn get_authentication(&self) -> Option<&storage::Authentication> {
        todo!()
    }

    fn get_frm_message(&self) -> Option<FraudCheck> {
        todo!()
    }

    fn get_refunds(&self) -> Vec<storage::Refund> {
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
}

#[cfg(feature = "v2")]
impl<F: Clone> OperationSessionSetters<F> for PaymentCaptureData<F> {
    fn set_payment_intent(&mut self, payment_intent: storage::PaymentIntent) {
        self.payment_intent = payment_intent;
    }

    fn set_payment_attempt(&mut self, payment_attempt: storage::PaymentAttempt) {
        self.payment_attempt = payment_attempt;
    }

    fn set_payment_method_data(&mut self, _payment_method_data: Option<domain::PaymentMethodData>) {
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
}
