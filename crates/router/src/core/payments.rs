pub mod access_token;
pub mod conditional_configs;
pub mod customers;
pub mod flows;
pub mod helpers;
pub mod operations;
#[cfg(feature = "retry")]
pub mod retry;
pub mod routing;
pub mod tokenization;
pub mod transformers;
pub mod types;

#[cfg(feature = "olap")]
use std::collections::{HashMap, HashSet};
use std::{fmt::Debug, marker::PhantomData, ops::Deref, time::Instant, vec::IntoIter};

#[cfg(feature = "olap")]
use api_models::admin::MerchantConnectorInfo;
use api_models::{
    self, enums,
    mandates::RecurringDetails,
    payments::{self as payments_api, HeaderPayload},
};
use common_utils::{ext_traits::AsyncExt, pii, types::Surcharge};
use data_models::mandates::{CustomerAcceptance, MandateData};
use diesel_models::{ephemeral_key, fraud_check::FraudCheck};
use error_stack::{report, ResultExt};
use events::EventInfo;
use futures::future::join_all;
use helpers::ApplePayData;
use masking::{ExposeInterface, Secret};
pub use payment_address::PaymentAddress;
use redis_interface::errors::RedisError;
use router_env::{instrument, tracing};
#[cfg(feature = "olap")]
use router_types::transformers::ForeignFrom;
use scheduler::utils as pt_utils;
#[cfg(feature = "olap")]
use strum::IntoEnumIterator;
use time;

pub use self::operations::{
    PaymentApprove, PaymentCancel, PaymentCapture, PaymentConfirm, PaymentCreate,
    PaymentIncrementalAuthorization, PaymentReject, PaymentResponse, PaymentSession, PaymentStatus,
    PaymentUpdate,
};
use self::{
    conditional_configs::perform_decision_management,
    flows::{ConstructFlowSpecificData, Feature},
    helpers::get_key_params_for_surcharge_details,
    operations::{payment_complete_authorize, BoxedOperation, Operation},
    routing::{self as self_routing, SessionFlowRoutingInput},
};
use super::{
    errors::StorageErrorExt, payment_methods::surcharge_decision_configs, routing::TransactionData,
};
#[cfg(feature = "frm")]
use crate::core::fraud_check as frm_core;
use crate::{
    configs::settings::{ApplePayPreDecryptFlow, PaymentMethodTypeTokenFilter},
    core::{
        authentication as authentication_core,
        errors::{self, CustomResult, RouterResponse, RouterResult},
        payment_methods::PaymentMethodRetrieve,
        utils,
    },
    db::StorageInterface,
    logger,
    routes::{app::ReqState, metrics, payment_methods::ParentPaymentMethodToken, AppState},
    services::{self, api::Authenticate},
    types::{
        self as router_types,
        api::{self, authentication, ConnectorCallType},
        domain,
        storage::{self, enums as storage_enums, payment_attempt::PaymentAttemptExt},
        transformers::{ForeignInto, ForeignTryInto},
        BrowserInformation,
    },
    utils::{
        add_apple_pay_flow_metrics, add_connector_http_status_code_metrics, Encode, OptionExt,
        ValueExt,
    },
    workflows::payment_sync,
};

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_operation_core<F, Req, Op, FData, Ctx>(
    state: &AppState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    call_connector_action: CallConnectorAction,
    auth_flow: services::AuthFlow,
    eligible_connectors: Option<Vec<common_enums::RoutableConnectors>>,
    header_payload: HeaderPayload,
) -> RouterResult<(
    PaymentData<F>,
    Req,
    Option<domain::Customer>,
    Option<u16>,
    Option<u128>,
)>
where
    F: Send + Clone + Sync,
    Req: Authenticate + Clone,
    Op: Operation<F, Req, Ctx> + Send + Sync,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    router_types::RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn router_types::api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Ctx>,
    FData: Send + Sync + Clone,
    Ctx: PaymentMethodRetrieve,
{
    let operation: BoxedOperation<'_, F, Req, Ctx> = Box::new(operation);

    tracing::Span::current().record("merchant_id", merchant_account.merchant_id.as_str());
    let (operation, validate_result) = operation
        .to_validate_request()?
        .validate_request(&req, &merchant_account)?;

    tracing::Span::current().record("payment_id", &format!("{}", validate_result.payment_id));

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
            header_payload.payment_confirm_source,
        )
        .await?;

    let (operation, customer) = operation
        .to_domain()?
        .get_or_create_customer_details(
            &*state.store,
            &mut payment_data,
            customer_details,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Failed while fetching/creating customer")?;

    call_decision_manager(state, &merchant_account, &mut payment_data).await?;

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

    payment_data = tokenize_in_router_when_confirm_false_or_external_authentication(
        state,
        &operation,
        &mut payment_data,
        &validate_result,
        &key_store,
        &customer,
    )
    .await?;

    let mut connector_http_status_code = None;
    let mut external_latency = None;
    if let Some(connector_details) = connector {
        // Fetch and check FRM configs
        #[cfg(feature = "frm")]
        let mut frm_info = None;
        #[cfg(feature = "frm")]
        let db = &*state.store;
        #[allow(unused_variables, unused_mut)]
        let mut should_continue_transaction: bool = true;
        #[cfg(feature = "frm")]
        let mut should_continue_capture: bool = true;
        #[cfg(feature = "frm")]
        let frm_configs = if state.conf.frm.enabled {
            Box::pin(frm_core::call_frm_before_connector_call(
                db,
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
            "frm_configs: {:?}\nshould_cancel_transaction: {:?}\nshould_continue_capture: {:?}",
            frm_configs,
            should_continue_transaction,
            should_continue_capture,
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
            )
            .await?;

        if should_continue_transaction {
            #[cfg(feature = "frm")]
            match (
                should_continue_capture,
                payment_data.payment_attempt.capture_method,
            ) {
                (false, Some(storage_enums::CaptureMethod::Automatic))
                | (false, Some(storage_enums::CaptureMethod::Scheduled)) => {
                    payment_data.payment_attempt.capture_method =
                        Some(storage_enums::CaptureMethod::Manual);
                }
                _ => (),
            };
            payment_data = match connector_details {
                api::ConnectorCallType::PreDetermined(connector) => {
                    let schedule_time = if should_add_task_to_process_tracker {
                        payment_sync::get_sync_process_schedule_time(
                            &*state.store,
                            connector.connector.id(),
                            &merchant_account.merchant_id,
                            0,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while getting process schedule time")?
                    } else {
                        None
                    };
                    let router_data = call_connector_service(
                        state,
                        req_state,
                        &merchant_account,
                        &key_store,
                        connector.clone(),
                        &operation,
                        &mut payment_data,
                        &customer,
                        call_connector_action.clone(),
                        &validate_result,
                        schedule_time,
                        header_payload,
                        #[cfg(feature = "frm")]
                        frm_info.as_ref().and_then(|fi| fi.suggested_action),
                        #[cfg(not(feature = "frm"))]
                        None,
                    )
                    .await?;

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
                        )
                        .await?;

                    operation
                        .to_post_update_tracker()?
                        .update_tracker(
                            state,
                            &validate_result.payment_id,
                            payment_data,
                            router_data,
                            merchant_account.storage_scheme,
                        )
                        .await?
                }

                api::ConnectorCallType::Retryable(connectors) => {
                    let mut connectors = connectors.into_iter();

                    let connector_data = get_connector_data(&mut connectors)?;

                    let schedule_time = if should_add_task_to_process_tracker {
                        payment_sync::get_sync_process_schedule_time(
                            &*state.store,
                            connector_data.connector.id(),
                            &merchant_account.merchant_id,
                            0,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while getting process schedule time")?
                    } else {
                        None
                    };
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
                        &validate_result,
                        schedule_time,
                        header_payload,
                        #[cfg(feature = "frm")]
                        frm_info.as_ref().and_then(|fi| fi.suggested_action),
                        #[cfg(not(feature = "frm"))]
                        None,
                    )
                    .await?;

                    #[cfg(feature = "retry")]
                    let mut router_data = router_data;
                    #[cfg(feature = "retry")]
                    {
                        use crate::core::payments::retry::{self, GsmValidation};
                        let config_bool = retry::config_should_call_gsm(
                            &*state.store,
                            &merchant_account.merchant_id,
                        )
                        .await;

                        if config_bool && router_data.should_call_gsm() {
                            router_data = retry::do_gsm_actions(
                                state,
                                req_state,
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
                            )
                            .await?;
                        };
                    }

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
                        )
                        .await?;

                    operation
                        .to_post_update_tracker()?
                        .update_tracker(
                            state,
                            &validate_result.payment_id,
                            payment_data,
                            router_data,
                            merchant_account.storage_scheme,
                        )
                        .await?
                }

                api::ConnectorCallType::SessionMultiple(connectors) => {
                    let session_surcharge_details =
                        call_surcharge_decision_management_for_session_flow(
                            state,
                            &merchant_account,
                            &mut payment_data,
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
                    ))
                    .await?
                }
            };

            #[cfg(feature = "frm")]
            if let Some(fraud_info) = &mut frm_info {
                Box::pin(frm_core::post_payment_frm_core(
                    state,
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
                    header_payload,
                )
                .await?;
        }

        payment_data
            .payment_attempt
            .payment_token
            .as_ref()
            .zip(payment_data.payment_attempt.payment_method)
            .map(ParentPaymentMethodToken::create_key_for_token)
            .async_map(|key_for_hyperswitch_token| async move {
                if key_for_hyperswitch_token
                    .should_delete_payment_method_token(payment_data.payment_intent.status)
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
                header_payload,
            )
            .await?;
    }

    let cloned_payment_data = payment_data.clone();
    let cloned_customer = customer.clone();

    crate::utils::trigger_payments_webhook(
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

#[instrument(skip_all)]
pub async fn call_decision_manager<O>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut PaymentData<O>,
) -> RouterResult<()>
where
    O: Send + Clone,
{
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
        merchant_account.merchant_id.as_str(),
        payment_data,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Could not decode the conditional config")?;
    payment_data.payment_attempt.authentication_type = payment_data
        .payment_attempt
        .authentication_type
        .or(output.override_3ds.map(ForeignInto::foreign_into))
        .or(Some(storage_enums::AuthenticationType::NoThreeDs));
    Ok(())
}

#[instrument(skip_all)]
async fn populate_surcharge_details<F>(
    state: &AppState,
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
            .and_then(get_key_params_for_surcharge_details)
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

        payment_data.surcharge_details = calculated_surcharge_details;
    } else {
        let surcharge_details =
            payment_data
                .payment_attempt
                .get_surcharge_details()
                .map(|surcharge_details| {
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

#[instrument(skip_all)]
pub async fn call_surcharge_decision_management_for_session_flow<O>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut PaymentData<O>,
    session_connector_data: &[api::SessionConnectorData],
) -> RouterResult<Option<api::SessionSurchargeDetails>>
where
    O: Send + Clone + Sync,
{
    if let Some(surcharge_amount) = payment_data.payment_attempt.surcharge_amount {
        let tax_on_surcharge_amount = payment_data.payment_attempt.tax_amount.unwrap_or(0);
        let final_amount =
            payment_data.payment_attempt.amount + surcharge_amount + tax_on_surcharge_amount;
        Ok(Some(api::SessionSurchargeDetails::PreDetermined(
            types::SurchargeDetails {
                original_amount: payment_data.payment_attempt.amount,
                surcharge: Surcharge::Fixed(surcharge_amount),
                tax_on_surcharge: None,
                surcharge_amount,
                tax_on_surcharge_amount,
                final_amount,
            },
        )))
    } else {
        let payment_method_type_list = session_connector_data
            .iter()
            .map(|session_connector_data| session_connector_data.payment_method_type)
            .collect();
        let algorithm_ref: api::routing::RoutingAlgorithmRef = merchant_account
            .routing_algorithm
            .clone()
            .map(|val| val.parse_value("routing algorithm"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not decode the routing algorithm")?
            .unwrap_or_default();
        let surcharge_results =
            surcharge_decision_configs::perform_surcharge_decision_management_for_session_flow(
                state,
                algorithm_ref,
                payment_data,
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
#[allow(clippy::too_many_arguments)]
pub async fn payments_core<F, Res, Req, Op, FData, Ctx>(
    state: AppState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    auth_flow: services::AuthFlow,
    call_connector_action: CallConnectorAction,
    eligible_connectors: Option<Vec<api_models::enums::Connector>>,
    header_payload: HeaderPayload,
) -> RouterResponse<Res>
where
    F: Send + Clone + Sync,
    FData: Send + Sync + Clone,
    Op: Operation<F, Req, Ctx> + Send + Sync + Clone,
    Req: Debug + Authenticate + Clone,
    Res: transformers::ToResponse<PaymentData<F>, Op>,
    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, FData, router_types::PaymentsResponseData>,
    router_types::RouterData<F, FData, router_types::PaymentsResponseData>: Feature<F, FData>,
    Ctx: PaymentMethodRetrieve,

    // To construct connector flow specific api
    dyn router_types::api::Connector:
        services::api::ConnectorIntegration<F, FData, router_types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData, Ctx>,
{
    let eligible_routable_connectors = eligible_connectors.map(|connectors| {
        connectors
            .into_iter()
            .flat_map(|c| c.foreign_try_into())
            .collect()
    });
    let (payment_data, _req, customer, connector_http_status_code, external_latency) =
        payments_operation_core::<_, _, _, _, Ctx>(
            &state,
            req_state,
            merchant_account,
            key_store,
            operation.clone(),
            req,
            call_connector_action,
            auth_flow,
            eligible_routable_connectors,
            header_payload,
        )
        .await?;

    Res::generate_response(
        payment_data,
        customer,
        auth_flow,
        &state.conf.server,
        operation,
        &state.conf.connector_request_reference_id_config,
        connector_http_status_code,
        external_latency,
        header_payload.x_hs_latency,
    )
}

fn is_start_pay<Op: Debug>(operation: &Op) -> bool {
    format!("{operation:?}").eq("PaymentStart")
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PaymentsRedirectResponseData {
    pub connector: Option<String>,
    pub param: Option<String>,
    pub merchant_id: Option<String>,
    pub json_payload: Option<serde_json::Value>,
    pub resource_id: api::PaymentIdType,
    pub force_sync: bool,
    pub creds_identifier: Option<String>,
}

#[async_trait::async_trait]
pub trait PaymentRedirectFlow<Ctx: PaymentMethodRetrieve>: Sync {
    // Associated type for call_payment_flow response
    type PaymentFlowResponse;

    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &AppState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        connector: String,
        payment_id: String,
    ) -> RouterResult<Self::PaymentFlowResponse>;

    fn get_payment_action(&self) -> services::PaymentAction;

    fn generate_response(
        &self,
        payment_flow_response: &Self::PaymentFlowResponse,
        payment_id: String,
        connector: String,
    ) -> RouterResult<services::ApplicationResponse<api::RedirectionResponse>>;

    #[allow(clippy::too_many_arguments)]
    async fn handle_payments_redirect_response(
        &self,
        state: AppState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
    ) -> RouterResponse<api::RedirectionResponse> {
        metrics::REDIRECTION_TRIGGERED.add(
            &metrics::CONTEXT,
            1,
            &[
                metrics::request::add_attributes(
                    "connector",
                    req.connector.to_owned().unwrap_or("null".to_string()),
                ),
                metrics::request::add_attributes(
                    "merchant_id",
                    merchant_account.merchant_id.to_owned(),
                ),
            ],
        );
        let connector = req.connector.clone().get_required_value("connector")?;

        let query_params = req.param.clone().get_required_value("param")?;

        let resource_id = api::PaymentIdTypeExt::get_payment_intent_id(&req.resource_id)
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_id",
            })?;

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
            )
            .await?;

        self.generate_response(&payment_flow_response, resource_id, connector)
    }
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectCompleteAuthorize;

#[async_trait::async_trait]
impl<Ctx: PaymentMethodRetrieve> PaymentRedirectFlow<Ctx> for PaymentRedirectCompleteAuthorize {
    type PaymentFlowResponse = router_types::RedirectPaymentFlowResponse;

    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &AppState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        _connector: String,
        _payment_id: String,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let payment_confirm_req = api::PaymentsRequest {
            payment_id: Some(req.resource_id.clone()),
            merchant_id: req.merchant_id.clone(),
            feature_metadata: Some(api_models::payments::FeatureMetadata {
                redirect_response: Some(api_models::payments::RedirectResponse {
                    param: req.param.map(Secret::new),
                    json_payload: Some(req.json_payload.unwrap_or(serde_json::json!({})).into()),
                }),
            }),
            ..Default::default()
        };
        let response = Box::pin(payments_core::<
            api::CompleteAuthorize,
            api::PaymentsResponse,
            _,
            _,
            _,
            Ctx,
        >(
            state.clone(),
            req_state,
            merchant_account,
            merchant_key_store,
            payment_complete_authorize::CompleteAuthorize,
            payment_confirm_req,
            services::api::AuthFlow::Merchant,
            connector_action,
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
            .find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
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
        payment_id: String,
        connector: String,
    ) -> RouterResult<services::ApplicationResponse<api::RedirectionResponse>> {
        let payments_response = &payment_flow_response.payments_response;
        // There might be multiple redirections needed for some flows
        // If the status is requires customer action, then send the startpay url again
        // The redirection data must have been provided and updated by the connector
        let redirection_response = match payments_response.status {
            api_models::enums::IntentStatus::RequiresCustomerAction => {
                let startpay_url = payments_response
                    .next_action
                    .clone()
                    .and_then(|next_action_data| match next_action_data {
                        api_models::payments::NextActionData::RedirectToUrl { redirect_to_url } => Some(redirect_to_url),
                        api_models::payments::NextActionData::DisplayBankTransferInformation { .. } => None,
                        api_models::payments::NextActionData::ThirdPartySdkSessionToken { .. } => None,
                        api_models::payments::NextActionData::QrCodeInformation{..} => None,
                        api_models::payments::NextActionData::DisplayVoucherInformation{ .. } => None,
                        api_models::payments::NextActionData::WaitScreenInformation{..} => None,
                        api_models::payments::NextActionData::ThreeDsInvoke{..} => None,
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
            api_models::enums::IntentStatus::Succeeded
            | api_models::enums::IntentStatus::Failed
            | api_models::enums::IntentStatus::Cancelled | api_models::enums::IntentStatus::RequiresCapture| api_models::enums::IntentStatus::Processing=> helpers::get_handle_response_url(
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

#[async_trait::async_trait]
impl<Ctx: PaymentMethodRetrieve> PaymentRedirectFlow<Ctx> for PaymentRedirectSync {
    type PaymentFlowResponse = router_types::RedirectPaymentFlowResponse;

    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &AppState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        _connector: String,
        _payment_id: String,
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
        };
        let response = Box::pin(payments_core::<
            api::PSync,
            api::PaymentsResponse,
            _,
            _,
            _,
            Ctx,
        >(
            state.clone(),
            req_state,
            merchant_account,
            merchant_key_store,
            PaymentStatus,
            payment_sync_req,
            services::api::AuthFlow::Merchant,
            connector_action,
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
            .find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
            })?;
        Ok(router_types::RedirectPaymentFlowResponse {
            payments_response,
            business_profile,
        })
    }
    fn generate_response(
        &self,
        payment_flow_response: &Self::PaymentFlowResponse,
        payment_id: String,
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

#[derive(Clone, Debug)]
pub struct PaymentAuthenticateCompleteAuthorize;

#[async_trait::async_trait]
impl<Ctx: PaymentMethodRetrieve> PaymentRedirectFlow<Ctx> for PaymentAuthenticateCompleteAuthorize {
    type PaymentFlowResponse = router_types::AuthenticatePaymentFlowResponse;

    #[allow(clippy::too_many_arguments)]
    async fn call_payment_flow(
        &self,
        state: &AppState,
        req_state: ReqState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
        connector: String,
        payment_id: String,
    ) -> RouterResult<Self::PaymentFlowResponse> {
        let merchant_id = merchant_account.merchant_id.clone();
        let payment_intent = state
            .store
            .find_payment_intent_by_payment_id_merchant_id(
                &payment_id,
                &merchant_id,
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
            crate::utils::check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata(
                authentication_merchant_connector_account
                    .get_metadata()
                    .map(|metadata| metadata.expose()),
            );
        let response = if is_pull_mechanism_enabled {
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
                }),
                ..Default::default()
            };
            Box::pin(payments_core::<
                api::Authorize,
                api::PaymentsResponse,
                _,
                _,
                _,
                Ctx,
            >(
                state.clone(),
                req_state,
                merchant_account,
                merchant_key_store,
                PaymentConfirm,
                payment_confirm_req,
                services::api::AuthFlow::Merchant,
                connector_action,
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
            };
            Box::pin(payments_core::<
                api::PSync,
                api::PaymentsResponse,
                _,
                _,
                _,
                Ctx,
            >(
                state.clone(),
                req_state,
                merchant_account.clone(),
                merchant_key_store,
                PaymentStatus,
                payment_sync_req,
                services::api::AuthFlow::Merchant,
                connector_action,
                None,
                HeaderPayload::default(),
            ))
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
            let req_poll_id =
                super::utils::get_external_authentication_request_poll_id(&payment_id);
            let poll_id = super::utils::get_poll_id(merchant_id.clone(), req_poll_id.clone());
            let redis_conn = state
                .store
                .get_redis_conn()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get redis connection")?;
            redis_conn
                .set_key_with_expiry(
                    &poll_id,
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
                &format!("poll_config_external_three_ds_{connector}"),
                Some(default_config_str),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("The poll config was not found in the DB")?;
        let poll_config =
            serde_json::from_str::<Option<router_types::PollConfig>>(&poll_config.config)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while parsing PollConfig")?
                .unwrap_or(default_poll_config);
        let profile_id = payments_response
            .profile_id
            .as_ref()
            .get_required_value("profile_id")?;
        let business_profile = state
            .store
            .find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
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
        payment_id: String,
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
        let html = utils::get_html_redirect_response_for_external_authentication(
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

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn call_connector_service<F, RouterDReq, ApiRequest, Ctx>(
    state: &AppState,
    req_state: ReqState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest, Ctx>,
    payment_data: &mut PaymentData<F>,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    validate_result: &operations::ValidateResult<'_>,
    schedule_time: Option<time::PrimitiveDateTime>,
    header_payload: HeaderPayload,
    frm_suggestion: Option<storage_enums::FrmSuggestion>,
) -> RouterResult<router_types::RouterData<F, RouterDReq, router_types::PaymentsResponseData>>
where
    F: Send + Clone + Sync,
    RouterDReq: Send + Sync,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, RouterDReq, router_types::PaymentsResponseData>,
    router_types::RouterData<F, RouterDReq, router_types::PaymentsResponseData>:
        Feature<F, RouterDReq> + Send,
    Ctx: PaymentMethodRetrieve,

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

    if payment_data.payment_attempt.merchant_connector_id.is_none() {
        payment_data.payment_attempt.merchant_connector_id =
            merchant_connector_account.get_mca_id();
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
    )
    .await?;
    *payment_data = pd;

    // Validating the blocklist guard and generate the fingerprint
    blocklist_guard(state, merchant_account, operation, payment_data).await?;

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
        )
        .await?;

    let add_access_token_result = router_data
        .add_access_token(state, &connector, merchant_account)
        .await?;

    let mut should_continue_further = access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &call_connector_action,
    );

    // Tokenization Action will be DecryptApplePayToken, only when payment method type is Apple Pay
    // and the connector supports Apple Pay predecrypt
    if matches!(
        tokenization_action,
        TokenizationAction::DecryptApplePayToken
            | TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt
    ) {
        let apple_pay_data = match payment_data.payment_method_data.clone() {
            Some(payment_data) => {
                let domain_data = domain::PaymentMethodData::from(payment_data);
                match domain_data {
                    domain::PaymentMethodData::Wallet(domain::WalletData::ApplePay(
                        wallet_data,
                    )) => Some(
                        ApplePayData::token_json(domain::WalletData::ApplePay(wallet_data))
                            .change_context(errors::ApiErrorResponse::InternalServerError)?
                            .decrypt(state)
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)?,
                    ),
                    _ => None,
                }
            }
            _ => None,
        };

        let apple_pay_predecrypt = apple_pay_data
            .parse_value::<router_types::ApplePayPredecryptData>("ApplePayPredecryptData")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        logger::debug!(?apple_pay_predecrypt);

        router_data.payment_method_token = Some(router_types::PaymentMethodToken::ApplePayDecrypt(
            Box::new(apple_pay_predecrypt),
        ));
    }

    let pm_token = router_data
        .add_payment_method_token(state, &connector, &tokenization_action)
        .await?;
    if let Some(payment_method_token) = pm_token.clone() {
        router_data.payment_method_token = Some(router_types::PaymentMethodToken::Token(
            payment_method_token,
        ));
    };

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
        payment_data.sessions_token.push(session_token);
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
                &payment_data.payment_attempt,
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
            header_payload,
        )
        .await?;

    let router_data_res = if should_continue_further {
        // The status of payment_attempt and intent will be updated in the previous step
        // update this in router_data.
        // This is added because few connector integrations do not update the status,
        // and rely on previous status set in router_data
        router_data.status = payment_data.payment_attempt.status;
        router_data
            .decide_flows(state, &connector, call_connector_action, connector_request)
            .await
    } else {
        Ok(router_data)
    };

    let etime_connector = Instant::now();
    let duration_connector = etime_connector.saturating_duration_since(stime_connector);
    tracing::info!(duration = format!("Duration taken: {}", duration_connector.as_millis()));

    router_data_res
}

async fn blocklist_guard<F, ApiRequest, Ctx>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    operation: &BoxedOperation<'_, F, ApiRequest, Ctx>,
    payment_data: &mut PaymentData<F>,
) -> CustomResult<bool, errors::ApiErrorResponse>
where
    F: Send + Clone + Sync,
    Ctx: PaymentMethodRetrieve,
{
    let merchant_id = &payment_data.payment_attempt.merchant_id;
    let blocklist_enabled_key = format!("guard_blocklist_for_{merchant_id}");
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
            .guard_payment_against_blocklist(state, merchant_account, payment_data)
            .await?)
    } else {
        Ok(false)
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn call_multiple_connectors_service<F, Op, Req, Ctx>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connectors: Vec<api::SessionConnectorData>,
    _operation: &Op,
    mut payment_data: PaymentData<F>,
    customer: &Option<domain::Customer>,
    session_surcharge_details: Option<api::SessionSurchargeDetails>,
) -> RouterResult<PaymentData<F>>
where
    Op: Debug,
    F: Send + Clone,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, Req, router_types::PaymentsResponseData>,
    router_types::RouterData<F, Req, router_types::PaymentsResponseData>: Feature<F, Req>,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, Req, router_types::PaymentsResponseData>,
    Ctx: PaymentMethodRetrieve,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, Req, Ctx>,
{
    let call_connectors_start_time = Instant::now();
    let mut join_handlers = Vec::with_capacity(connectors.len());
    for session_connector_data in connectors.iter() {
        let connector_id = session_connector_data.connector.connector.id();

        let merchant_connector_account = construct_profile_id_and_get_mca(
            state,
            merchant_account,
            &mut payment_data,
            &session_connector_data.connector.connector_name.to_string(),
            session_connector_data
                .connector
                .merchant_connector_id
                .as_ref(),
            key_store,
            false,
        )
        .await?;

        payment_data.surcharge_details =
            session_surcharge_details
                .as_ref()
                .and_then(|session_surcharge_details| {
                    session_surcharge_details.fetch_surcharge_details(
                        &session_connector_data.payment_method_type.into(),
                        &session_connector_data.payment_method_type,
                        None,
                    )
                });

        let router_data = payment_data
            .construct_router_data(
                state,
                connector_id,
                merchant_account,
                key_store,
                customer,
                &merchant_connector_account,
            )
            .await?;

        let res = router_data.decide_flows(
            state,
            &session_connector_data.connector,
            CallConnectorAction::Trigger,
            None,
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
                }) = connector_response.response
                {
                    // If session token is NoSessionTokenReceived, it is not pushed into the sessions_token as there is no response or there can be some error
                    // In case of error, that error is already logged
                    if !matches!(
                        session_token,
                        api_models::payments::SessionToken::NoSessionTokenReceived,
                    ) {
                        payment_data.sessions_token.push(session_token);
                    }
                }
            }
            Err(connector_error) => {
                logger::error!(
                    "sessions_connector_error {} {:?}",
                    connector_name,
                    connector_error
                );
            }
        }
    }

    let call_connectors_end_time = Instant::now();
    let call_connectors_duration =
        call_connectors_end_time.saturating_duration_since(call_connectors_start_time);
    tracing::info!(duration = format!("Duration taken: {}", call_connectors_duration.as_millis()));

    Ok(payment_data)
}

pub async fn call_create_connector_customer_if_required<F, Req>(
    state: &AppState,
    customer: &Option<domain::Customer>,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    payment_data: &mut PaymentData<F>,
) -> RouterResult<Option<storage::CustomerUpdate>>
where
    F: Send + Clone + Sync,
    Req: Send + Sync,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, Req, router_types::PaymentsResponseData>,
    router_types::RouterData<F, Req, router_types::PaymentsResponseData>: Feature<F, Req> + Send,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, Req, router_types::PaymentsResponseData>,
{
    let connector_name = payment_data.payment_attempt.connector.clone();

    match connector_name {
        Some(connector_name) => {
            let connector = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &connector_name,
                api::GetToken::Connector,
                merchant_connector_account.get_mca_id(),
            )?;

            let connector_label = super::utils::get_connector_label(
                payment_data.payment_intent.business_country,
                payment_data.payment_intent.business_label.as_ref(),
                payment_data.payment_attempt.business_sub_label.as_ref(),
                &connector_name,
            );

            let connector_label = if let Some(connector_label) =
                merchant_connector_account.get_mca_id().or(connector_label)
            {
                connector_label
            } else {
                let profile_id = utils::get_profile_id_from_business_details(
                    payment_data.payment_intent.business_country,
                    payment_data.payment_intent.business_label.as_ref(),
                    merchant_account,
                    payment_data.payment_intent.profile_id.as_ref(),
                    &*state.store,
                    false,
                )
                .await
                .attach_printable("Could not find profile id from business details")?;

                format!("{connector_name}_{profile_id}")
            };

            let (should_call_connector, existing_connector_customer_id) =
                customers::should_call_connector_create_customer(
                    state,
                    &connector,
                    customer,
                    &connector_label,
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
                    )
                    .await?;

                let connector_customer_id = router_data
                    .create_connector_customer(state, &connector)
                    .await?;

                let customer_update = customers::update_connector_customer_in_customers(
                    &connector_label,
                    customer.as_ref(),
                    &connector_customer_id,
                )
                .await;

                payment_data.connector_customer_id = connector_customer_id;
                Ok(customer_update)
            } else {
                // Customer already created in previous calls use the same value, no need to update
                payment_data.connector_customer_id =
                    existing_connector_customer_id.map(ToOwned::to_owned);
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

async fn complete_preprocessing_steps_if_required<F, Req, Q, Ctx>(
    state: &AppState,
    connector: &api::ConnectorData,
    payment_data: &PaymentData<F>,
    mut router_data: router_types::RouterData<F, Req, router_types::PaymentsResponseData>,
    operation: &BoxedOperation<'_, F, Q, Ctx>,
    should_continue_payment: bool,
) -> RouterResult<(
    router_types::RouterData<F, Req, router_types::PaymentsResponseData>,
    bool,
)>
where
    F: Send + Clone + Sync,
    Req: Send + Sync,
    router_types::RouterData<F, Req, router_types::PaymentsResponseData>: Feature<F, Req> + Send,
    dyn api::Connector:
        services::api::ConnectorIntegration<F, Req, router_types::PaymentsResponseData>,
{
    //TODO: For ACH transfers, if preprocessing_step is not required for connectors encountered in future, add the check
    let router_data_and_should_continue_payment = match payment_data.payment_method_data.clone() {
        Some(api_models::payments::PaymentMethodData::BankTransfer(data)) => match data.deref() {
            api_models::payments::BankTransferData::AchBankTransfer { .. }
            | api_models::payments::BankTransferData::MultibancoBankTransfer { .. }
                if connector.connector_name == router_types::Connector::Stripe =>
            {
                if payment_data.payment_attempt.preprocessing_step_id.is_none() {
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
        Some(api_models::payments::PaymentMethodData::Wallet(_)) => {
            if is_preprocessing_required_for_wallets(connector.connector_name.to_string()) {
                (
                    router_data.preprocessing_steps(state, connector).await?,
                    false,
                )
            } else {
                (router_data, should_continue_payment)
            }
        }
        Some(api_models::payments::PaymentMethodData::Card(_)) => {
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
                        .payment_attempt
                        .external_three_ds_authentication_attempted,
                    Some(true)
                )
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                (router_data, false)
            } else if (connector.connector_name == router_types::Connector::Cybersource
                || connector.connector_name == router_types::Connector::Bankofamerica)
                && is_operation_complete_authorize(&operation)
                && router_data.auth_type == storage_enums::AuthenticationType::ThreeDs
            {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                // Should continue the flow only if no redirection_data is returned else a response with redirection form shall be returned
                let should_continue = matches!(
                    router_data.response,
                    Ok(router_types::PaymentsResponseData::TransactionResponse {
                        redirection_data: None,
                        ..
                    })
                ) && router_data.status
                    != common_enums::AttemptStatus::AuthenticationFailed;
                (router_data, should_continue)
            } else {
                (router_data, should_continue_payment)
            }
        }
        Some(api_models::payments::PaymentMethodData::GiftCard(_)) => {
            if connector.connector_name == router_types::Connector::Adyen {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                let is_error_in_response = router_data.response.is_err();
                // If is_error_in_response is true, should_continue_payment should be false, we should throw the error
                (router_data, !is_error_in_response)
            } else {
                (router_data, should_continue_payment)
            }
        }
        Some(api_models::payments::PaymentMethodData::BankDebit(_)) => {
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
                && payment_data.payment_attempt.payment_method
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

pub fn is_preprocessing_required_for_wallets(connector_name: String) -> bool {
    connector_name == *"trustpay" || connector_name == *"payme"
}

#[instrument(skip_all)]
pub async fn construct_profile_id_and_get_mca<'a, F>(
    state: &'a AppState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut PaymentData<F>,
    connector_name: &str,
    merchant_connector_id: Option<&String>,
    key_store: &domain::MerchantKeyStore,
    should_validate: bool,
) -> RouterResult<helpers::MerchantConnectorAccountType>
where
    F: Clone,
{
    let profile_id = utils::get_profile_id_from_business_details(
        payment_data.payment_intent.business_country,
        payment_data.payment_intent.business_label.as_ref(),
        merchant_account,
        payment_data.payment_intent.profile_id.as_ref(),
        &*state.store,
        should_validate,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("profile_id is not set in payment_intent")?;

    let merchant_connector_account = helpers::get_merchant_connector_account(
        state,
        merchant_account.merchant_id.as_str(),
        payment_data.creds_identifier.to_owned(),
        key_store,
        &profile_id,
        connector_name,
        merchant_connector_id,
    )
    .await?;

    Ok(merchant_connector_account)
}

fn is_payment_method_tokenization_enabled_for_connector(
    state: &AppState,
    connector_name: &str,
    payment_method: &storage::enums::PaymentMethod,
    payment_method_type: &Option<storage::enums::PaymentMethodType>,
    apple_pay_flow: &Option<enums::ApplePayFlow>,
) -> RouterResult<bool> {
    let connector_tokenization_filter = state.conf.tokenization.0.get(connector_name);

    Ok(connector_tokenization_filter
        .map(|connector_filter| {
            connector_filter
                .payment_method
                .clone()
                .contains(payment_method)
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
    payment_method_type: &Option<storage::enums::PaymentMethodType>,
    apple_pay_flow: &Option<enums::ApplePayFlow>,
    apple_pay_pre_decrypt_flow_filter: Option<ApplePayPreDecryptFlow>,
) -> bool {
    match (payment_method_type, apple_pay_flow) {
        (
            Some(storage::enums::PaymentMethodType::ApplePay),
            Some(enums::ApplePayFlow::Simplified),
        ) => !matches!(
            apple_pay_pre_decrypt_flow_filter,
            Some(ApplePayPreDecryptFlow::NetworkTokenization)
        ),
        _ => true,
    }
}

fn decide_apple_pay_flow(
    payment_method_type: &Option<api_models::enums::PaymentMethodType>,
    merchant_connector_account: Option<&helpers::MerchantConnectorAccountType>,
) -> Option<enums::ApplePayFlow> {
    payment_method_type.and_then(|pmt| match pmt {
        api_models::enums::PaymentMethodType::ApplePay => {
            check_apple_pay_metadata(merchant_connector_account)
        }
        _ => None,
    })
}

fn check_apple_pay_metadata(
    merchant_connector_account: Option<&helpers::MerchantConnectorAccountType>,
) -> Option<enums::ApplePayFlow> {
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
                .map_err(
                    |error| logger::warn!(%error, "Failed to Parse Value to ApplepaySessionTokenData"),
                );

            parsed_metadata.ok().map(|metadata| match metadata {
                api_models::payments::ApplepaySessionTokenMetadata::ApplePayCombined(
                    apple_pay_combined,
                ) => match apple_pay_combined {
                    api_models::payments::ApplePayCombinedMetadata::Simplified { .. } => {
                        enums::ApplePayFlow::Simplified
                    }
                    api_models::payments::ApplePayCombinedMetadata::Manual { .. } => {
                        enums::ApplePayFlow::Manual
                    }
                },
                api_models::payments::ApplepaySessionTokenMetadata::ApplePay(_) => {
                    enums::ApplePayFlow::Manual
                }
            })
        })
    })
}

fn is_payment_method_type_allowed_for_connector(
    current_pm_type: &Option<storage::enums::PaymentMethodType>,
    pm_type_filter: Option<PaymentMethodTypeTokenFilter>,
) -> bool {
    match (*current_pm_type).zip(pm_type_filter) {
        Some((pm_type, type_filter)) => match type_filter {
            PaymentMethodTypeTokenFilter::AllAccepted => true,
            PaymentMethodTypeTokenFilter::EnableOnly(enabled) => enabled.contains(&pm_type),
            PaymentMethodTypeTokenFilter::DisableOnly(disabled) => !disabled.contains(&pm_type),
        },
        None => true, // Allow all types if payment_method_type is not present
    }
}

async fn decide_payment_method_tokenize_action(
    state: &AppState,
    connector_name: &str,
    payment_method: &storage::enums::PaymentMethod,
    pm_parent_token: Option<&String>,
    is_connector_tokenization_enabled: bool,
    apple_pay_flow: Option<enums::ApplePayFlow>,
) -> RouterResult<TokenizationAction> {
    let is_apple_pay_predecrypt_supported =
        matches!(apple_pay_flow, Some(enums::ApplePayFlow::Simplified));

    match pm_parent_token {
        None => {
            if is_connector_tokenization_enabled && is_apple_pay_predecrypt_supported {
                Ok(TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt)
            } else if is_connector_tokenization_enabled {
                Ok(TokenizationAction::TokenizeInConnectorAndRouter)
            } else if is_apple_pay_predecrypt_supported {
                Ok(TokenizationAction::DecryptApplePayToken)
            } else {
                Ok(TokenizationAction::TokenizeInRouter)
            }
        }
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
                .get_key::<Option<String>>(&key)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to fetch the token from redis")?;

            match connector_token_option {
                Some(connector_token) => Ok(TokenizationAction::ConnectorToken(connector_token)),
                None => {
                    if is_connector_tokenization_enabled && is_apple_pay_predecrypt_supported {
                        Ok(TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt)
                    } else if is_connector_tokenization_enabled {
                        Ok(TokenizationAction::TokenizeInConnectorAndRouter)
                    } else if is_apple_pay_predecrypt_supported {
                        Ok(TokenizationAction::DecryptApplePayToken)
                    } else {
                        Ok(TokenizationAction::TokenizeInRouter)
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum TokenizationAction {
    TokenizeInRouter,
    TokenizeInConnector,
    TokenizeInConnectorAndRouter,
    ConnectorToken(String),
    SkipConnectorTokenization,
    DecryptApplePayToken,
    TokenizeInConnectorAndApplepayPreDecrypt,
}

#[allow(clippy::too_many_arguments)]
pub async fn get_connector_tokenization_action_when_confirm_true<F, Req, Ctx>(
    state: &AppState,
    operation: &BoxedOperation<'_, F, Req, Ctx>,
    payment_data: &mut PaymentData<F>,
    validate_result: &operations::ValidateResult<'_>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
    merchant_key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
) -> RouterResult<(PaymentData<F>, TokenizationAction)>
where
    F: Send + Clone,
    Ctx: PaymentMethodRetrieve,
{
    let connector = payment_data.payment_attempt.connector.to_owned();

    let is_mandate = payment_data
        .mandate_id
        .as_ref()
        .and_then(|inner| inner.mandate_reference_id.as_ref())
        .map(|mandate_reference| match mandate_reference {
            api_models::payments::MandateReferenceId::ConnectorMandateId(_) => true,
            api_models::payments::MandateReferenceId::NetworkMandateId(_) => false,
        })
        .unwrap_or(false);

    let payment_data_and_tokenization_action = match connector {
        Some(_) if is_mandate => (
            payment_data.to_owned(),
            TokenizationAction::SkipConnectorTokenization,
        ),
        Some(connector) if is_operation_confirm(&operation) => {
            let payment_method = &payment_data
                .payment_attempt
                .payment_method
                .get_required_value("payment_method")?;
            let payment_method_type = &payment_data.payment_attempt.payment_method_type;

            let apple_pay_flow =
                decide_apple_pay_flow(payment_method_type, Some(merchant_connector_account));

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
                payment_data.payment_attempt.connector.clone(),
                payment_data.payment_attempt.merchant_id.clone(),
            );

            let payment_method_action = decide_payment_method_tokenize_action(
                state,
                &connector,
                payment_method,
                payment_data.token.as_ref(),
                is_connector_tokenization_enabled,
                apple_pay_flow,
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
                        )
                        .await?;
                    payment_data.payment_method_data = payment_method_data;
                    payment_data.payment_attempt.payment_method_id = pm_id;

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
                        )
                        .await?;

                    payment_data.payment_method_data = payment_method_data;
                    payment_data.payment_attempt.payment_method_id = pm_id;
                    TokenizationAction::TokenizeInConnector
                }
                TokenizationAction::ConnectorToken(token) => {
                    payment_data.pm_token = Some(token);
                    TokenizationAction::SkipConnectorTokenization
                }
                TokenizationAction::SkipConnectorTokenization => {
                    TokenizationAction::SkipConnectorTokenization
                }
                TokenizationAction::DecryptApplePayToken => {
                    TokenizationAction::DecryptApplePayToken
                }
                TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt => {
                    TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt
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

pub async fn tokenize_in_router_when_confirm_false_or_external_authentication<F, Req, Ctx>(
    state: &AppState,
    operation: &BoxedOperation<'_, F, Req, Ctx>,
    payment_data: &mut PaymentData<F>,
    validate_result: &operations::ValidateResult<'_>,
    merchant_key_store: &domain::MerchantKeyStore,
    customer: &Option<domain::Customer>,
) -> RouterResult<PaymentData<F>>
where
    F: Send + Clone,
    Ctx: PaymentMethodRetrieve,
{
    // On confirm is false and only router related
    let is_external_authentication_requested = payment_data
        .payment_intent
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
                )
                .await?;
            payment_data.payment_method_data = payment_method_data;
            if let Some(payment_method_id) = pm_id {
                payment_data.payment_attempt.payment_method_id = Some(payment_method_id);
            }
            payment_data
        } else {
            payment_data
        };
    Ok(payment_data.to_owned())
}

#[derive(Clone, PartialEq)]
pub enum CallConnectorAction {
    Trigger,
    Avoid,
    StatusUpdate {
        status: storage_enums::AttemptStatus,
        error_code: Option<String>,
        error_message: Option<String>,
    },
    HandleResponse(Vec<u8>),
}

pub mod payment_address {
    use super::*;

    #[derive(Clone, Default, Debug)]
    pub struct PaymentAddress {
        shipping: Option<api::Address>,
        billing: Option<api::Address>,
        unified_payment_method_billing: Option<api::Address>,
        payment_method_billing: Option<api::Address>,
    }

    impl PaymentAddress {
        pub fn new(
            shipping: Option<api::Address>,
            billing: Option<api::Address>,
            payment_method_billing: Option<api::Address>,
        ) -> Self {
            // billing -> .billing, this is the billing details passed in the root of payments request
            // payment_method_billing -> .payment_method_data.billing

            // Merge the billing details field from both `payment.billing` and `payment.payment_method_data.billing`
            // The unified payment_method_billing will be used as billing address and passed to the connector module
            // This unification is required in order to provide backwards compatibility
            // so that if `payment.billing` is passed it should be sent to the connector module
            // Unify the billing details with `payment_method_data.billing`
            let unified_payment_method_billing = payment_method_billing
                .as_ref()
                .map(|payment_method_billing| {
                    payment_method_billing
                        .clone()
                        .unify_address(billing.as_ref())
                })
                .or(billing.clone());

            Self {
                shipping,
                billing,
                unified_payment_method_billing,
                payment_method_billing,
            }
        }

        pub fn get_shipping(&self) -> Option<&api::Address> {
            self.shipping.as_ref()
        }

        pub fn get_payment_method_billing(&self) -> Option<&api::Address> {
            self.unified_payment_method_billing.as_ref()
        }

        /// Unify the billing details from `payment_method_data.[payment_method_data].billing details`.
        pub fn unify_with_payment_method_data_billing(
            self,
            payment_method_data_billing: Option<api::Address>,
        ) -> Self {
            // Unify the billing details with `payment_method_data.billing_details`
            let unified_payment_method_billing = payment_method_data_billing
                .map(|payment_method_data_billing| {
                    payment_method_data_billing.unify_address(self.get_payment_method_billing())
                })
                .or(self.get_payment_method_billing().cloned());

            Self {
                shipping: self.shipping,
                billing: self.billing,
                unified_payment_method_billing,
                payment_method_billing: self.payment_method_billing,
            }
        }

        pub fn get_request_payment_method_billing(&self) -> Option<&api::Address> {
            self.payment_method_billing.as_ref()
        }

        pub fn get_payment_billing(&self) -> Option<&api::Address> {
            self.billing.as_ref()
        }
    }
}

#[derive(Clone)]
pub struct MandateConnectorDetails {
    pub connector: String,
    pub merchant_connector_id: Option<String>,
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
    pub payment_method_data: Option<api::PaymentMethodData>,
    pub payment_method_info: Option<storage::PaymentMethod>,
    pub refunds: Vec<storage::Refund>,
    pub disputes: Vec<storage::Dispute>,
    pub attempts: Option<Vec<storage::PaymentAttempt>>,
    pub sessions_token: Vec<api::SessionToken>,
    pub card_cvc: Option<Secret<String>>,
    pub email: Option<pii::Email>,
    pub creds_identifier: Option<String>,
    pub pm_token: Option<String>,
    pub connector_customer_id: Option<String>,
    pub recurring_mandate_payment_data: Option<RecurringMandatePaymentData>,
    pub ephemeral_key: Option<ephemeral_key::EphemeralKey>,
    pub redirect_response: Option<api_models::payments::RedirectResponse>,
    pub surcharge_details: Option<types::SurchargeDetails>,
    pub frm_message: Option<FraudCheck>,
    pub payment_link_data: Option<api_models::payments::PaymentLinkResponse>,
    pub incremental_authorization_details: Option<IncrementalAuthorizationDetails>,
    pub authorizations: Vec<diesel_models::authorization::Authorization>,
    pub authentication: Option<storage::Authentication>,
    pub frm_metadata: Option<serde_json::Value>,
    pub recurring_details: Option<RecurringDetails>,
}

#[derive(Clone, serde::Serialize, Debug)]
pub struct PaymentEvent {
    payment_intent: storage::PaymentIntent,
    payment_attempt: storage::PaymentAttempt,
}

impl<F: Clone> PaymentData<F> {
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
    pub additional_amount: i64,
    pub total_amount: i64,
    pub reason: Option<String>,
    pub authorization_id: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct RecurringMandatePaymentData {
    pub payment_method_type: Option<storage_enums::PaymentMethodType>, //required for making recurring payment using saved payment method through stripe
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<storage_enums::Currency>,
}

#[derive(Debug, Default, Clone)]
pub struct CustomerDetails {
    pub customer_id: Option<String>,
    pub name: Option<Secret<String, masking::WithType>>,
    pub email: Option<pii::Email>,
    pub phone: Option<Secret<String, masking::WithType>>,
    pub phone_country_code: Option<String>,
}

pub fn if_not_create_change_operation<'a, Op, F, Ctx>(
    status: storage_enums::IntentStatus,
    confirm: Option<bool>,
    current: &'a Op,
) -> BoxedOperation<'_, F, api::PaymentsRequest, Ctx>
where
    F: Send + Clone,
    Op: Operation<F, api::PaymentsRequest, Ctx> + Send + Sync,
    &'a Op: Operation<F, api::PaymentsRequest, Ctx>,
    Ctx: PaymentMethodRetrieve,
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

pub fn is_confirm<'a, F: Clone + Send, R, Op, Ctx>(
    operation: &'a Op,
    confirm: Option<bool>,
) -> BoxedOperation<'_, F, R, Ctx>
where
    PaymentConfirm: Operation<F, R, Ctx>,
    &'a PaymentConfirm: Operation<F, R, Ctx>,
    Op: Operation<F, R, Ctx> + Send + Sync,
    &'a Op: Operation<F, R, Ctx>,
    Ctx: PaymentMethodRetrieve,
{
    if confirm.unwrap_or(false) {
        Box::new(&PaymentConfirm)
    } else {
        Box::new(operation)
    }
}

pub fn should_call_connector<Op: Debug, F: Clone>(
    operation: &Op,
    payment_data: &PaymentData<F>,
) -> bool {
    match format!("{operation:?}").as_str() {
        "PaymentConfirm" => true,
        "PaymentStart" => {
            !matches!(
                payment_data.payment_intent.status,
                storage_enums::IntentStatus::Failed | storage_enums::IntentStatus::Succeeded
            ) && payment_data.payment_attempt.authentication_data.is_none()
        }
        "PaymentStatus" => {
            matches!(
                payment_data.payment_intent.status,
                storage_enums::IntentStatus::Processing
                    | storage_enums::IntentStatus::RequiresCustomerAction
                    | storage_enums::IntentStatus::RequiresMerchantAction
                    | storage_enums::IntentStatus::RequiresCapture
                    | storage_enums::IntentStatus::PartiallyCapturedAndCapturable
            ) && payment_data.force_sync.unwrap_or(false)
        }
        "PaymentCancel" => matches!(
            payment_data.payment_intent.status,
            storage_enums::IntentStatus::RequiresCapture
                | storage_enums::IntentStatus::PartiallyCapturedAndCapturable
        ),
        "PaymentCapture" => {
            matches!(
                payment_data.payment_intent.status,
                storage_enums::IntentStatus::RequiresCapture
                    | storage_enums::IntentStatus::PartiallyCapturedAndCapturable
            ) || (matches!(
                payment_data.payment_intent.status,
                storage_enums::IntentStatus::Processing
            ) && matches!(
                payment_data.payment_attempt.capture_method,
                Some(storage_enums::CaptureMethod::ManualMultiple)
            ))
        }
        "CompleteAuthorize" => true,
        "PaymentApprove" => true,
        "PaymentReject" => true,
        "PaymentSession" => true,
        "PaymentIncrementalAuthorization" => matches!(
            payment_data.payment_intent.status,
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

#[cfg(feature = "olap")]
pub async fn list_payments(
    state: AppState,
    merchant: domain::MerchantAccount,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<api::PaymentListResponse> {
    use data_models::errors::StorageError;
    helpers::validate_payment_list_request(&constraints)?;
    let merchant_id = &merchant.merchant_id;
    let db = state.store.as_ref();
    let payment_intents =
        helpers::filter_by_constraints(db, &constraints, merchant_id, merchant.storage_scheme)
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
                            "payment_attempts missing for payment_id : {}",
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
#[cfg(feature = "olap")]
pub async fn apply_filters_on_payments(
    state: AppState,
    merchant: domain::MerchantAccount,
    constraints: api::PaymentListFilterConstraints,
) -> RouterResponse<api::PaymentListResponseV2> {
    let limit = &constraints.limit;
    helpers::validate_payment_list_request_for_joins(*limit)?;
    let db = state.store.as_ref();
    let list: Vec<(storage::PaymentIntent, storage::PaymentAttempt)> = db
        .get_filtered_payment_intents_attempt(
            &merchant.merchant_id,
            &constraints.clone().into(),
            merchant.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let data: Vec<api::PaymentsResponse> =
        list.into_iter().map(ForeignFrom::foreign_from).collect();

    let active_attempt_ids = db
        .get_filtered_active_attempt_ids_for_total_count(
            &merchant.merchant_id,
            &constraints.clone().into(),
            merchant.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    let total_count = db
        .get_total_count_of_filtered_payment_attempts(
            &merchant.merchant_id,
            &active_attempt_ids,
            constraints.connector,
            constraints.payment_method,
            constraints.payment_method_type,
            constraints.authentication_type,
            constraints.merchant_connector_id,
            merchant.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(services::ApplicationResponse::Json(
        api::PaymentListResponseV2 {
            count: data.len(),
            total_count,
            data,
        },
    ))
}

#[cfg(feature = "olap")]
pub async fn get_filters_for_payments(
    state: AppState,
    merchant: domain::MerchantAccount,
    time_range: api::TimeRange,
) -> RouterResponse<api::PaymentListFilters> {
    let db = state.store.as_ref();
    let pi = db
        .filter_payment_intents_by_time_range_constraints(
            &merchant.merchant_id,
            &time_range,
            merchant.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let filters = db
        .get_filters_for_payments(
            pi.as_slice(),
            &merchant.merchant_id,
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
    state: AppState,
    merchant: domain::MerchantAccount,
) -> RouterResponse<api::PaymentListFiltersV2> {
    let merchant_connector_accounts = if let services::ApplicationResponse::Json(data) =
        super::admin::list_payment_connectors(state, merchant.merchant_id).await?
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
                    let info = MerchantConnectorInfo {
                        connector_label: label.clone(),
                        merchant_connector_id: merchant_connector_account
                            .merchant_connector_id
                            .clone(),
                    };
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
        },
    ))
}

pub async fn add_process_sync_task(
    db: &dyn StorageInterface,
    payment_attempt: &storage::PaymentAttempt,
    schedule_time: time::PrimitiveDateTime,
) -> CustomResult<(), errors::StorageError> {
    let tracking_data = api::PaymentsRetrieveRequest {
        force_sync: true,
        merchant_id: Some(payment_attempt.merchant_id.clone()),
        resource_id: api::PaymentIdType::PaymentAttemptId(payment_attempt.attempt_id.clone()),
        ..Default::default()
    };
    let runner = storage::ProcessTrackerRunner::PaymentsSyncWorkflow;
    let task = "PAYMENTS_SYNC";
    let tag = ["SYNC", "PAYMENT"];
    let process_tracker_id = pt_utils::get_process_tracker_id(
        runner,
        task,
        &payment_attempt.attempt_id,
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
        &payment_attempt.attempt_id,
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

pub fn update_straight_through_routing<F>(
    payment_data: &mut PaymentData<F>,
    request_straight_through: serde_json::Value,
) -> CustomResult<(), errors::ParsingError>
where
    F: Send + Clone,
{
    let _: api_models::routing::RoutingAlgorithm = request_straight_through
        .clone()
        .parse_value("RoutingAlgorithm")
        .attach_printable("Invalid straight through routing rules format")?;

    payment_data.payment_attempt.straight_through_algorithm = Some(request_straight_through);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn get_connector_choice<F, Req, Ctx>(
    operation: &BoxedOperation<'_, F, Req, Ctx>,
    state: &AppState,
    req: &Req,
    merchant_account: &domain::MerchantAccount,
    business_profile: &storage::business_profile::BusinessProfile,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<Option<ConnectorCallType>>
where
    F: Send + Clone,
    Ctx: PaymentMethodRetrieve,
{
    let connector_choice = operation
        .to_domain()?
        .get_connector(
            merchant_account,
            &state.clone(),
            req,
            &payment_data.payment_intent,
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
                api::ConnectorCallType::SessionMultiple(routing_output)
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

#[allow(clippy::too_many_arguments)]
pub async fn connector_selection<F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &storage::business_profile::BusinessProfile,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    request_straight_through: Option<serde_json::Value>,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
{
    let request_straight_through: Option<api::routing::StraightThroughAlgorithm> =
        request_straight_through
            .map(|val| val.parse_value("RoutingAlgorithm"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid straight through routing rules format")?;

    let mut routing_data = storage::RoutingData {
        routed_through: payment_data.payment_attempt.connector.clone(),
        #[cfg(feature = "connector_choice_mca_id")]
        merchant_connector_id: payment_data.payment_attempt.merchant_connector_id.clone(),
        #[cfg(not(feature = "connector_choice_mca_id"))]
        business_sub_label: payment_data.payment_attempt.business_sub_label.clone(),
        algorithm: request_straight_through.clone(),
        routing_info: payment_data
            .payment_attempt
            .straight_through_algorithm
            .clone()
            .map(|val| val.parse_value("PaymentRoutingInfo"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid straight through algorithm format found in payment attempt")?
            .unwrap_or_else(|| storage::PaymentRoutingInfo {
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

    payment_data.payment_attempt.connector = routing_data.routed_through;
    #[cfg(feature = "connector_choice_mca_id")]
    {
        payment_data.payment_attempt.merchant_connector_id = routing_data.merchant_connector_id;
    }
    #[cfg(not(feature = "connector_choice_mca_id"))]
    {
        payment_data.payment_attempt.business_sub_label = routing_data.business_sub_label;
    }
    payment_data.payment_attempt.straight_through_algorithm = Some(encoded_info);

    Ok(decided_connector)
}

#[allow(clippy::too_many_arguments)]
pub async fn decide_connector<F>(
    state: AppState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &storage::business_profile::BusinessProfile,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    request_straight_through: Option<api::routing::StraightThroughAlgorithm>,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
{
    // If the connector was already decided previously, use the same connector
    // This is in case of flows like payments_sync, payments_cancel where the successive operations
    // with the connector have to be made using the same connector account.
    if let Some(ref connector_name) = payment_data.payment_attempt.connector {
        // Connector was already decided previously, use the same connector
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            api::GetToken::Connector,
            payment_data.payment_attempt.merchant_connector_id.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in 'routed_through'")?;

        routing_data.routed_through = Some(connector_name.clone());
        return Ok(api::ConnectorCallType::PreDetermined(connector_data));
    }

    if let Some(mandate_connector_details) = payment_data.mandate_connector.as_ref() {
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &mandate_connector_details.connector,
            api::GetToken::Connector,
            #[cfg(feature = "connector_choice_mca_id")]
            mandate_connector_details.merchant_connector_id.clone(),
            #[cfg(not(feature = "connector_choice_mca_id"))]
            None,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in 'routed_through'")?;

        routing_data.routed_through = Some(mandate_connector_details.connector.clone());
        #[cfg(feature = "connector_choice_mca_id")]
        {
            routing_data.merchant_connector_id =
                mandate_connector_details.merchant_connector_id.clone();
        }
        return Ok(api::ConnectorCallType::PreDetermined(connector_data));
    }

    if let Some((pre_routing_results, storage_pm_type)) = routing_data
        .routing_info
        .pre_routing_results
        .as_ref()
        .zip(payment_data.payment_attempt.payment_method_type.as_ref())
    {
        if let (Some(choice), None) = (
            pre_routing_results.get(storage_pm_type),
            &payment_data.token_data,
        ) {
            let connector_data = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &choice.connector.to_string(),
                api::GetToken::Connector,
                #[cfg(feature = "connector_choice_mca_id")]
                choice.merchant_connector_id.clone(),
                #[cfg(not(feature = "connector_choice_mca_id"))]
                None,
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid connector name received")?;

            routing_data.routed_through = Some(choice.connector.to_string());
            #[cfg(feature = "connector_choice_mca_id")]
            {
                routing_data.merchant_connector_id = choice.merchant_connector_id.clone();
            }
            #[cfg(not(feature = "connector_choice_mca_id"))]
            {
                routing_data.business_sub_label = choice.sub_label.clone();
            }
            return Ok(api::ConnectorCallType::PreDetermined(connector_data));
        }
    }

    if let Some(routing_algorithm) = request_straight_through {
        let (mut connectors, check_eligibility) = routing::perform_straight_through_routing(
            &routing_algorithm,
            payment_data.creds_identifier.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed execution of straight through routing")?;

        if check_eligibility {
            #[cfg(feature = "business_profile_routing")]
            let profile_id = payment_data.payment_intent.profile_id.clone();

            #[cfg(not(feature = "business_profile_routing"))]
            let _profile_id: Option<String> = None;

            connectors = routing::perform_eligibility_analysis_with_fallback(
                &state.clone(),
                key_store,
                merchant_account.modified_at.assume_utc().unix_timestamp(),
                connectors,
                &TransactionData::Payment(payment_data),
                eligible_connectors,
                #[cfg(feature = "business_profile_routing")]
                profile_id,
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
                    #[cfg(feature = "connector_choice_mca_id")]
                    conn.merchant_connector_id.clone(),
                    #[cfg(not(feature = "connector_choice_mca_id"))]
                    None,
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
        )
        .await;
    }

    if let Some(ref routing_algorithm) = routing_data.routing_info.algorithm {
        let (mut connectors, check_eligibility) = routing::perform_straight_through_routing(
            routing_algorithm,
            payment_data.creds_identifier.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed execution of straight through routing")?;

        if check_eligibility {
            #[cfg(feature = "business_profile_routing")]
            let profile_id = payment_data.payment_intent.profile_id.clone();

            #[cfg(not(feature = "business_profile_routing"))]
            let _profile_id: Option<String> = None;

            connectors = routing::perform_eligibility_analysis_with_fallback(
                &state,
                key_store,
                merchant_account.modified_at.assume_utc().unix_timestamp(),
                connectors,
                &TransactionData::Payment(payment_data),
                eligible_connectors,
                #[cfg(feature = "business_profile_routing")]
                profile_id,
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
                    #[cfg(feature = "connector_choice_mca_id")]
                    conn.merchant_connector_id,
                    #[cfg(not(feature = "connector_choice_mca_id"))]
                    None,
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
        )
        .await;
    }

    route_connector_v1(
        &state,
        merchant_account,
        business_profile,
        key_store,
        TransactionData::Payment(payment_data),
        routing_data,
        eligible_connectors,
        mandate_type,
    )
    .await
}

pub async fn decide_multiplex_connector_for_normal_or_recurring_payment<F: Clone>(
    state: &AppState,
    payment_data: &mut PaymentData<F>,
    routing_data: &mut storage::RoutingData,
    connectors: Vec<api::ConnectorData>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType> {
    match (
        payment_data.payment_intent.setup_future_usage,
        payment_data.token_data.as_ref(),
        payment_data.recurring_details.as_ref(),
        payment_data.payment_intent.off_session,
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
                .payment_method_info
                .as_ref()
                .get_required_value("payment_method_info")?;

            let connector_mandate_details = &payment_method_info
                .connector_mandate_details
                .clone()
                .map(|details| {
                    details.parse_value::<storage::PaymentsMandateReference>(
                        "connector_mandate_details",
                    )
                })
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("unable to deserialize connector mandate details")?;

            let profile_id = payment_data
                .payment_intent
                .profile_id
                .as_ref()
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)?;

            let pg_agnostic = state
                .store
                .find_config_by_key_unwrap_or(
                    &format!("pg_agnostic_mandate_{}", profile_id),
                    Some("false".to_string()),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("The pg_agnostic config was not found in the DB")?;

            let mut connector_choice = None;

            for connector_data in connectors {
                let merchant_connector_id = connector_data
                    .merchant_connector_id
                    .as_ref()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)?;

                if is_network_transaction_id_flow(
                    state,
                    &pg_agnostic.config,
                    connector_data.connector_name,
                    payment_method_info,
                ) {
                    logger::info!("using network_transaction_id for MIT flow");
                    let network_transaction_id = payment_method_info
                        .network_transaction_id
                        .as_ref()
                        .ok_or(errors::ApiErrorResponse::InternalServerError)?;

                    let mandate_reference_id =
                        Some(payments_api::MandateReferenceId::NetworkMandateId(
                            network_transaction_id.to_string(),
                        ));

                    connector_choice = Some((connector_data, mandate_reference_id.clone()));
                    break;
                } else if connector_mandate_details
                    .clone()
                    .map(|connector_mandate_details| {
                        connector_mandate_details.contains_key(merchant_connector_id)
                    })
                    .unwrap_or(false)
                {
                    if let Some(merchant_connector_id) =
                        connector_data.merchant_connector_id.as_ref()
                    {
                        if let Some(mandate_reference_record) = connector_mandate_details.clone()
                        .get_required_value("connector_mandate_details")
                            .change_context(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                            .attach_printable("no eligible connector found for token-based MIT flow since there were no connector mandate details")?
                            .get(merchant_connector_id)
                        {
                            common_utils::fp_utils::when(
                                mandate_reference_record
                                    .original_payment_authorized_currency
                                    .map(|mandate_currency| mandate_currency != payment_data.currency)
                                    .unwrap_or(false),
                                || {
                                    Err(report!(errors::ApiErrorResponse::MandateValidationFailed {
                                        reason: "cross currency mandates not supported".into()
                                    }))
                                },
                            )?;
                            let mandate_reference_id =
                                Some(payments_api::MandateReferenceId::ConnectorMandateId(
                                    payments_api::ConnectorMandateReferenceId {
                                        connector_mandate_id: Some(
                                            mandate_reference_record.connector_mandate_id.clone(),
                                        ),
                                        payment_method_id: Some(
                                            payment_method_info.payment_method_id.clone(),
                                        ),
                                        update_history: None,
                                    },
                                ));
                            payment_data.recurring_mandate_payment_data =
                                Some(RecurringMandatePaymentData {
                                    payment_method_type: mandate_reference_record
                                        .payment_method_type,
                                    original_payment_authorized_amount: mandate_reference_record
                                        .original_payment_authorized_amount,
                                    original_payment_authorized_currency: mandate_reference_record
                                        .original_payment_authorized_currency,
                                });

                            connector_choice = Some((connector_data, mandate_reference_id.clone()));
                            break;
                        }
                    }
                } else {
                    continue;
                }
            }

            let (chosen_connector_data, mandate_reference_id) = connector_choice
                .get_required_value("connector_choice")
                .change_context(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                .attach_printable("no eligible connector found for token-based MIT payment")?;

            routing_data.routed_through = Some(chosen_connector_data.connector_name.to_string());
            #[cfg(feature = "connector_choice_mca_id")]
            {
                routing_data.merchant_connector_id =
                    chosen_connector_data.merchant_connector_id.clone();
            }
            routing_data.routed_through = Some(chosen_connector_data.connector_name.to_string());
            #[cfg(feature = "connector_choice_mca_id")]
            {
                routing_data.merchant_connector_id =
                    chosen_connector_data.merchant_connector_id.clone();
            }

            payment_data.mandate_id = Some(payments_api::MandateIds {
                mandate_id: None,
                mandate_reference_id,
            });

            Ok(api::ConnectorCallType::PreDetermined(chosen_connector_data))
        }
        _ => {
            let first_choice = connectors
                .first()
                .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
                .attach_printable("no eligible connector found for payment")?
                .clone();

            routing_data.routed_through = Some(first_choice.connector_name.to_string());
            #[cfg(feature = "connector_choice_mca_id")]
            {
                routing_data.merchant_connector_id = first_choice.merchant_connector_id;
            }

            Ok(api::ConnectorCallType::Retryable(connectors))
        }
    }
}

pub fn is_network_transaction_id_flow(
    state: &AppState,
    pg_agnostic: &String,
    connector: enums::Connector,
    payment_method_info: &storage::PaymentMethod,
) -> bool {
    let ntid_supported_connectors = &state
        .conf
        .network_transaction_id_supported_connectors
        .connector_list;

    pg_agnostic == "true"
        && payment_method_info.payment_method == Some(storage_enums::PaymentMethod::Card)
        && ntid_supported_connectors.contains(&connector)
        && payment_method_info.network_transaction_id.is_some()
}

pub fn should_add_task_to_process_tracker<F: Clone>(payment_data: &PaymentData<F>) -> bool {
    let connector = payment_data.payment_attempt.connector.as_deref();

    !matches!(
        (payment_data.payment_attempt.payment_method, connector),
        (
            Some(storage_enums::PaymentMethod::BankTransfer),
            Some("stripe")
        )
    )
}

pub async fn perform_session_token_routing<F>(
    state: AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    connectors: Vec<api::SessionConnectorData>,
) -> RouterResult<Vec<api::SessionConnectorData>>
where
    F: Clone,
{
    let routing_info: Option<storage::PaymentRoutingInfo> = payment_data
        .payment_attempt
        .straight_through_algorithm
        .clone()
        .map(|val| val.parse_value("PaymentRoutingInfo"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("invalid payment routing info format found in payment attempt")?;

    if let Some(storage::PaymentRoutingInfo {
        pre_routing_results: Some(pre_routing_results),
        ..
    }) = routing_info
    {
        let mut payment_methods: rustc_hash::FxHashMap<
            (String, enums::PaymentMethodType),
            api::SessionConnectorData,
        > = rustc_hash::FxHashMap::from_iter(connectors.iter().map(|c| {
            (
                (
                    c.connector.connector_name.to_string(),
                    c.payment_method_type,
                ),
                c.clone(),
            )
        }));

        let mut final_list: Vec<api::SessionConnectorData> = Vec::new();
        for (routed_pm_type, choice) in pre_routing_results.into_iter() {
            if let Some(session_connector_data) =
                payment_methods.remove(&(choice.to_string(), routed_pm_type))
            {
                final_list.push(session_connector_data);
            }
        }

        if !final_list.is_empty() {
            return Ok(final_list);
        }
    }

    let routing_enabled_pms = std::collections::HashSet::from([
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
            .address
            .get_payment_method_billing()
            .and_then(|address| address.address.as_ref())
            .and_then(|details| details.country),
        key_store,
        merchant_account,
        payment_attempt: &payment_data.payment_attempt,
        payment_intent: &payment_data.payment_intent,

        chosen,
    };
    let result = self_routing::perform_session_flow_routing(sfr, &enums::TransactionType::Payment)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error performing session flow routing")?;

    let mut final_list: Vec<api::SessionConnectorData> = Vec::new();

    #[cfg(not(feature = "connector_choice_mca_id"))]
    for mut connector_data in connectors {
        if !routing_enabled_pms.contains(&connector_data.payment_method_type) {
            final_list.push(connector_data);
        } else if let Some(choice) = result.get(&connector_data.payment_method_type) {
            if connector_data.connector.connector_name == choice.connector.connector_name {
                connector_data.business_sub_label = choice.sub_label.clone();
                final_list.push(connector_data);
            }
        }
    }

    #[cfg(feature = "connector_choice_mca_id")]
    for connector_data in connectors {
        if !routing_enabled_pms.contains(&connector_data.payment_method_type) {
            final_list.push(connector_data);
        } else if let Some(choice) = result.get(&connector_data.payment_method_type) {
            if connector_data.connector.connector_name == choice.connector.connector_name {
                final_list.push(connector_data);
            }
        }
    }

    Ok(final_list)
}

#[allow(clippy::too_many_arguments)]
pub async fn route_connector_v1<F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &storage::business_profile::BusinessProfile,
    key_store: &domain::MerchantKeyStore,
    transaction_data: TransactionData<'_, F>,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
    mandate_type: Option<api::MandateTransactionType>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
{
    #[allow(unused_variables)]
    let (profile_id, routing_algorithm) = match &transaction_data {
        TransactionData::Payment(payment_data) => {
            if cfg!(feature = "business_profile_routing") {
                (
                    payment_data.payment_intent.profile_id.clone(),
                    business_profile.routing_algorithm.clone(),
                )
            } else {
                (None, merchant_account.routing_algorithm.clone())
            }
        }
        #[cfg(feature = "payouts")]
        TransactionData::Payout(payout_data) => {
            if cfg!(feature = "business_profile_routing") {
                (
                    Some(payout_data.payout_attempt.profile_id.clone()),
                    business_profile.payout_routing_algorithm.clone(),
                )
            } else {
                (None, merchant_account.payout_routing_algorithm.clone())
            }
        }
    };

    let algorithm_ref = routing_algorithm
        .map(|ra| ra.parse_value::<api::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode merchant routing algorithm ref")?
        .unwrap_or_default();

    let connectors = routing::perform_static_routing_v1(
        state,
        &merchant_account.merchant_id,
        algorithm_ref,
        &transaction_data,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let connectors = routing::perform_eligibility_analysis_with_fallback(
        &state.clone(),
        key_store,
        merchant_account.modified_at.assume_utc().unix_timestamp(),
        connectors,
        &transaction_data,
        eligible_connectors,
        #[cfg(feature = "business_profile_routing")]
        profile_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("failed eligibility analysis and fallback")?;

    #[cfg(feature = "payouts")]
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
                #[cfg(feature = "connector_choice_mca_id")]
                conn.merchant_connector_id,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                None,
            )
        })
        .collect::<CustomResult<Vec<_>, _>>()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received")?;

    match transaction_data {
        TransactionData::Payment(payment_data) => {
            decide_multiplex_connector_for_normal_or_recurring_payment(
                state,
                payment_data,
                routing_data,
                connector_data,
                mandate_type,
            )
            .await
        }

        #[cfg(feature = "payouts")]
        TransactionData::Payout(_) => {
            routing_data.routed_through = Some(first_connector_choice.connector.to_string());

            #[cfg(feature = "connector_choice_mca_id")]
            {
                routing_data.merchant_connector_id = first_connector_choice.merchant_connector_id;
            }
            #[cfg(not(feature = "connector_choice_mca_id"))]
            {
                routing_data.business_sub_label = first_connector_choice.sub_label;
            }

            Ok(ConnectorCallType::Retryable(connector_data))
        }
    }
}

#[instrument(skip_all)]
pub async fn payment_external_authentication(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: api_models::payments::PaymentsExternalAuthenticationRequest,
) -> RouterResponse<api_models::payments::PaymentsExternalAuthenticationResponse> {
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let storage_scheme = merchant_account.storage_scheme;
    let payment_id = req.payment_id;
    let payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
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
        &payment_intent.status,
        &[storage_enums::IntentStatus::RequiresCustomerAction],
        "authenticate",
    )?;
    let optional_customer = match &payment_intent.customer_id {
        Some(customer_id) => Some(
            state
                .store
                .find_customer_by_customer_id_merchant_id(
                    customer_id,
                    &merchant_account.merchant_id,
                    &key_store,
                    storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable_lazy(|| {
                    format!("error while finding customer with customer_id {customer_id}")
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
    let amount = payment_attempt.get_total_amount().into();
    let shipping_address = helpers::create_or_find_address_for_payment_by_request(
        db,
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
        db,
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
            merchant_id.to_string(),
            payment_attempt
                .authentication_id
                .clone()
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("missing authentication_id in payment_attempt")?,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while fetching authentication record")?;
    let payment_method_details = helpers::get_payment_method_details_from_payment_token(
        &state,
        &payment_attempt,
        &payment_intent,
        &key_store,
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
        &state.conf.server.base_url,
        &payment_attempt.clone(),
        payment_connector_name,
    ));
    let webhook_url = helpers::create_webhook_url(
        &state.conf.server.base_url,
        merchant_id,
        &authentication_connector,
    );

    let business_profile = state
        .store
        .find_business_profile_by_profile_id(profile_id)
        .await
        .change_context(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: profile_id.to_string(),
        })?;

    let authentication_response = Box::pin(authentication_core::perform_authentication(
        &state,
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
        business_profile,
        merchant_connector_account,
        amount,
        Some(currency),
        authentication::MessageCategory::Payment,
        req.device_channel,
        authentication,
        return_url,
        req.sdk_information,
        req.threeds_method_comp_ind,
        optional_customer.and_then(|customer| customer.email.map(common_utils::pii::Email::from)),
        webhook_url,
    ))
    .await?;
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
        },
    ))
}
