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

use std::{fmt::Debug, marker::PhantomData, ops::Deref, time::Instant, vec::IntoIter};

use api_models::{self, enums, payments::HeaderPayload};
use common_utils::{ext_traits::AsyncExt, pii, types::Surcharge};
use data_models::mandates::MandateData;
use diesel_models::{ephemeral_key, fraud_check::FraudCheck};
use error_stack::{IntoReport, ResultExt};
use futures::future::join_all;
use helpers::ApplePayData;
use masking::Secret;
use redis_interface::errors::RedisError;
use router_env::{instrument, tracing};
#[cfg(feature = "olap")]
use router_types::transformers::ForeignFrom;
use scheduler::{db::process_tracker::ProcessTrackerExt, errors as sch_errors, utils as pt_utils};
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
use super::{errors::StorageErrorExt, payment_methods::surcharge_decision_configs};
#[cfg(feature = "frm")]
use crate::core::fraud_check as frm_core;
use crate::{
    configs::settings::{ApplePayPreDecryptFlow, PaymentMethodTypeTokenFilter},
    core::{
        errors::{self, CustomResult, RouterResponse, RouterResult},
        payment_methods::PaymentMethodRetrieve,
        utils,
    },
    db::StorageInterface,
    logger,
    routes::{metrics, payment_methods::ParentPaymentMethodToken, AppState},
    services::{self, api::Authenticate},
    types::{
        self as router_types,
        api::{self, ConnectorCallType},
        domain,
        storage::{self, enums as storage_enums, payment_attempt::PaymentAttemptExt},
        transformers::{ForeignInto, ForeignTryInto},
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
    FData: Send + Sync,
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
    } = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &validate_result.payment_id,
            &req,
            validate_result.mandate_type.to_owned(),
            &merchant_account,
            &key_store,
            auth_flow,
        )
        .await?;

    let (operation, customer) = operation
        .to_domain()?
        .get_or_create_customer_details(
            &*state.store,
            &mut payment_data,
            customer_details,
            &key_store,
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
    )
    .await?;

    let should_add_task_to_process_tracker = should_add_task_to_process_tracker(&payment_data);

    payment_data = tokenize_in_router_when_confirm_false(
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
            frm_core::call_frm_before_connector_call(
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
            )
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
                        .into_report()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while getting process schedule time")?
                    } else {
                        None
                    };
                    let router_data = call_connector_service(
                        state,
                        &merchant_account,
                        &key_store,
                        connector,
                        &operation,
                        &mut payment_data,
                        &customer,
                        call_connector_action,
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
                        .into_report()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed while getting process schedule time")?
                    } else {
                        None
                    };
                    let router_data = call_connector_service(
                        state,
                        &merchant_account,
                        &key_store,
                        connector_data.clone(),
                        &operation,
                        &mut payment_data,
                        &customer,
                        call_connector_action,
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
                                &mut payment_data,
                                connectors,
                                connector_data,
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
                    call_multiple_connectors_service(
                        state,
                        &merchant_account,
                        &key_store,
                        connectors,
                        &operation,
                        payment_data,
                        &customer,
                        session_surcharge_details,
                    )
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
                        .into_report()
                        .attach_printable("Frm configs label not found")?,
                    &customer,
                    key_store,
                ))
                .await?;
            }
        } else {
            (_, payment_data) = operation
                .to_update_tracker()?
                .update_trackers(
                    state,
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
    let cloned_request = req.clone();

    crate::utils::trigger_payments_webhook(
        merchant_account,
        business_profile,
        cloned_payment_data,
        Some(cloned_request),
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
/// Asynchronously calls the decision manager to perform decision management for a payment, based on the given state, merchant account, and payment data. It retrieves the routing algorithm reference from the merchant account, then uses it to perform decision management and update the payment data with the authentication type. 
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
/// Asynchronously populates the surcharge details for a payment attempt, based on the payment data and application state.
async fn populate_surcharge_details<F>(
    state: &AppState,
    payment_data: &mut PaymentData<F>,
) -> RouterResult<()>
where
    F: Send + Clone,
{
    // method implementation...
}

#[inline]
/// Retrieves the next connector data from the provided iterator and returns it as a result.
///
/// # Arguments
///
/// * `connectors` - A mutable reference to an iterator of `api::ConnectorData`
///
/// # Returns
///
/// * If there is a next connector data in the iterator, it returns `Ok(api::ConnectorData)`.
/// * If the iterator is empty, it returns `Err(errors::ApiErrorResponse::InternalServerError)`.
///
/// # Examples
///
///

#[instrument(skip_all)]
/// Asynchronously performs surcharge decision management for a session flow, based on the provided payment data and session connector data.
/// If a surcharge amount is present in the payment data, it calculates the final amount including the surcharge and tax, and returns the pre-determined surcharge details.
/// If no surcharge amount is present, it retrieves the payment method type list from the session connector data, and then performs surcharge decision management using the merchant's routing algorithm and the provided payment data, returning the calculated surcharge details if any.
pub async fn call_surcharge_decision_management_for_session_flow<O>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut PaymentData<O>,
    session_connector_data: &[api::SessionConnectorData],
) -> RouterResult<Option<api::SessionSurchargeDetails>>
where
    O: Send + Clone + Sync,
{
    // method implementation
}
#[allow(clippy::too_many_arguments)]
/// This method is responsible for handling payments core functionality. It takes various parameters such as state, merchant account, key store, operation, request, authentication flow, call connector action, eligible connectors, and header payload. It then performs core payment operations and generates a response using the provided parameters. The method also makes use of various traits and types to construct flow-specific data, interface data, and API integration. It returns a RouterResponse containing the generated response.
pub async fn payments_core<F, Res, Req, Op, FData, Ctx>(
    state: AppState,
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
    FData: Send + Sync,
    Op: Operation<F, Req, Ctx> + Send + Sync + Clone,
    Req: Debug + Authenticate + Clone,
    Res: transformers::ToResponse<Req, PaymentData<F>, Op>,
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
    let (payment_data, req, customer, connector_http_status_code, external_latency) =
        payments_operation_core::<_, _, _, _, Ctx>(
            &state,
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
        Some(req),
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

/// Checks if the provided operation is a "PaymentStart" operation.
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
    async fn call_payment_flow(
        &self,
        state: &AppState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
    ) -> RouterResponse<api::PaymentsResponse>;

    fn get_payment_action(&self) -> services::PaymentAction;

    fn generate_response(
        &self,
        payments_response: api_models::payments::PaymentsResponse,
        business_profile: diesel_models::business_profile::BusinessProfile,
        payment_id: String,
        connector: String,
    ) -> RouterResult<api::RedirectionResponse>;

    #[allow(clippy::too_many_arguments)]
        /// Handle the redirect response for payments. This method triggers redirection metrics, gets the connector, query parameters, and resource id, retrieves connector data, determines the flow type, makes a payment flow call, processes the response, retrieves the profile id, finds the business profile, generates the response, and returns the JSON for redirection.
    async fn handle_payments_redirect_response(
        &self,
        state: AppState,
        merchant_account: domain::MerchantAccount,
        key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
    ) -> RouterResponse<api::RedirectionResponse> {
        // implementation...
    }
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectCompleteAuthorize;

#[async_trait::async_trait]
impl<Ctx: PaymentMethodRetrieve> PaymentRedirectFlow<Ctx> for PaymentRedirectCompleteAuthorize {
        /// This method initiates the payment flow by creating a payment confirmation request and calling the payments_core function to complete the authorization process. It takes in the current application state, merchant account information, merchant key store, redirect response data, and connector action. It returns a RouterResponse containing the payment response from the payments_core function.
    async fn call_payment_flow(
            &self,
            state: &AppState,
            merchant_account: domain::MerchantAccount,
            merchant_key_store: domain::MerchantKeyStore,
            req: PaymentsRedirectResponseData,
            connector_action: CallConnectorAction,
        ) -> RouterResponse<api::PaymentsResponse> {
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
            Box::pin(payments_core::<
                api::CompleteAuthorize,
                api::PaymentsResponse,
                _,
                _,
                _,
                Ctx,
            >(
                state.clone(),
                merchant_account,
                merchant_key_store,
                payment_complete_authorize::CompleteAuthorize,
                payment_confirm_req,
                services::api::AuthFlow::Merchant,
                connector_action,
                None,
                HeaderPayload::default(),
            ))
            .await
        }

        /// This method returns the payment action associated with the current instance.
    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::CompleteAuthorize
    }

        /// Generates a redirection response based on the payments response, business profile, payment ID, and connector provided. 
    /// There might be multiple redirections needed for some flows. If the status is 'RequiresCustomerAction', then the startpay URL is sent again. 
    /// The redirection data must have been provided and updated by the connector. 
    fn generate_response(
        &self,
        payments_response: api_models::payments::PaymentsResponse,
        business_profile: diesel_models::business_profile::BusinessProfile,
        payment_id: String,
        connector: String,
    ) -> RouterResult<api::RedirectionResponse> {
        match payments_response.status {
            api_models::enums::IntentStatus::RequiresCustomerAction => {
                let startpay_url = payments_response
                    .next_action
                    .and_then(|next_action_data| match next_action_data {
                        api_models::payments::NextActionData::RedirectToUrl { redirect_to_url } => Some(redirect_to_url),
                        api_models::payments::NextActionData::DisplayBankTransferInformation { .. } => None,
                        api_models::payments::NextActionData::ThirdPartySdkSessionToken { .. } => None,
                        api_models::payments::NextActionData::QrCodeInformation{..} => None,
                        api_models::payments::NextActionData::DisplayVoucherInformation{ .. } => None,
                        api_models::payments::NextActionData::WaitScreenInformation{..} => None,
                    })
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .into_report()
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
            api_models::enums::IntentStatus::Succeeded
            | api_models::enums::IntentStatus::Failed
            | api_models::enums::IntentStatus::Cancelled 
            | api_models::enums::IntentStatus::RequiresCapture
            | api_models::enums::IntentStatus::Processing=> helpers::get_handle_response_url(
                payment_id,
                &business_profile,
                payments_response,
                connector,
            ),
            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .into_report()
                .attach_printable_lazy(|| format!("Could not proceed with payment as payment status {} cannot be handled during redirection",payments_response.status))?
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectSync;

#[async_trait::async_trait]
impl<Ctx: PaymentMethodRetrieve> PaymentRedirectFlow<Ctx> for PaymentRedirectSync {
        /// Asynchronously initiates the payment flow by making a call to the payments core, using the provided state, merchant account, merchant key store, redirect response data, and connector action. Returns a router response containing the payments response.
    async fn call_payment_flow(
        &self,
        state: &AppState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
    ) -> RouterResponse<api::PaymentsResponse> {
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
            merchant_account,
            merchant_key_store,
            PaymentStatus,
            payment_sync_req,
            services::api::AuthFlow::Merchant,
            connector_action,
            None,
            HeaderPayload::default(),
        ))
        .await
    }
        /// Generates a redirection response based on the provided payments response, business profile, payment ID, and connector. 
    /// The method uses the provided payment ID, business profile, payments response, and connector to get the handle response URL using the `get_handle_response_url` helper method.
    
    fn generate_response(
        &self,
        payments_response: api_models::payments::PaymentsResponse,
        business_profile: diesel_models::business_profile::BusinessProfile,
        payment_id: String,
        connector: String,
    ) -> RouterResult<api::RedirectionResponse> {
        helpers::get_handle_response_url(
            payment_id,
            &business_profile,
            payments_response,
            connector,
        )
    }

        /// This method returns the payment action associated with the payment service. In this case, it returns the PSync payment action.
    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::PSync
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
/// Calls the connector service to perform various operations related to payment processing, including constructing data, updating payment trackers, building requests, adding access tokens, and deciding payment flows. It handles the integration with different connectors and the processing of payment data based on the connector's capabilities and the specific payment method being used. Returns a result containing the router data for the payment flow and various related data types.
pub async fn call_connector_service<F, RouterDReq, ApiRequest, Ctx>(
    state: &AppState,
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
    // Method implementation...
}

#[allow(clippy::too_many_arguments)]
/// Calls multiple connector services to trigger payment flows for a given payment data and session details.
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
    // Method implementation omitted for brevity
}

/// Asynchronously calls the create connector customer if required. This method takes in various parameters including the application state, customer information, merchant account details, key store, connector account type, and payment data. It constructs flow specific data and constructs connector flow specific API. It then checks if the connector should be called to create a customer, and if so, creates the customer at the connector and updates the customer table to store this data. Finally, it updates the connector customer in the customers table and returns the customer update information.
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
    // method implementation...
}

/// This method completes the preprocessing steps required for different payment methods and connectors if necessary, based on various conditions and checks. It handles different scenarios for ACH transfers, wallets, cards, gift cards, bank debits, and PayPal cards, and determines whether the payment flow should continue or not. It also checks for specific connector and operation conditions to decide whether to continue the payment flow. The method returns a tuple containing the updated router data and a boolean value indicating whether the payment flow should continue.
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
            // ... (omitted for brevity)
        }
        Some(api_models::payments::PaymentMethodData::GiftCard(_)) => {
            // ... (omitted for brevity)
        }
        Some(api_models::payments::PaymentMethodData::BankDebit(_)) => {
            // ... (omitted for brevity)
        }
        _ => {
            // ... (omitted for brevity)
        }
    };

    Ok(router_data_and_should_continue_payment)
}

/// Checks if the given connector name requires preprocessing for wallets.
/// 
/// # Arguments
/// 
/// * `connector_name` - A String representing the name of the connector.
/// 
/// # Returns
/// 
/// A boolean value indicating if preprocessing is required for the given connector name.
pub fn is_preprocessing_required_for_wallets(connector_name: String) -> bool {
    connector_name == *"trustpay" || connector_name == *"payme"
}

#[instrument(skip_all)]
/// This method constructs a profile ID from the given payment data and merchant account, and then retrieves the merchant connector account using the constructed profile ID, connector name, and other parameters. It performs validation and error handling during the process.
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

/// Checks if tokenization is enabled for a specific payment method and connector.
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

/// Determines if the given payment method type, Apple Pay flow, and Apple Pay pre-decrypt flow filter
/// satisfy the condition for using Apple Pay pre-decrypt type connector tokenization.
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

/// Decides the Apple Pay flow based on the provided payment method type and merchant connector account.
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

/// Checks the Apple Pay metadata associated with a merchant connector account and returns the corresponding Apple Pay flow.
fn check_apple_pay_metadata(
    merchant_connector_account: Option<&helpers::MerchantConnectorAccountType>,
) -> Option<enums::ApplePayFlow> {
    // method implementation
}

/// Checks if the given payment method type is allowed for the given connector based on the given payment method type filter.
/// 
/// # Arguments
/// 
/// * `current_pm_type` - The current payment method type.
/// * `pm_type_filter` - The payment method type filter to apply.
/// 
/// # Returns
/// 
/// * `true` if the payment method type is allowed based on the filter, `false` otherwise.
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

/// Determines the appropriate tokenization action based on the provided parameters. This method takes into account the state of the application, the connector name, the payment method, the parent token (if any), the availability of connector tokenization, and the Apple Pay flow. It returns a `TokenizationAction` enum representing the action to be taken.
async fn decide_payment_method_tokenize_action(
    state: &AppState,
    connector_name: &str,
    payment_method: &storage::enums::PaymentMethod,
    pm_parent_token: Option<&String>,
    is_connector_tokenization_enabled: bool,
    apple_pay_flow: Option<enums::ApplePayFlow>,
) -> RouterResult<TokenizationAction> {
    // method implementation...
}

#[derive(Clone)]
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
/// Retrieves the tokenization action for a connector when the confirmation flag is true. This method takes in various parameters such as the application state, operation, payment data, validation result, merchant connector account, merchant key store, and customer information. It then determines the tokenization action based on the connector and payment method, and updates the payment data accordingly. Finally, it returns a tuple containing the updated payment data and the tokenization action.
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
    // method implementation
}

/// Tokenizes payment method data in the router when the confirm flag is set to false. 
/// This method takes in the application state, operation, payment data, validation result, 
/// merchant key store, and customer information. It then checks if the operation confirm 
/// flag is false, and if so, retrieves the payment method data from the operation, 
/// updates the payment data, and returns the updated payment data. If the confirm flag is 
/// true, it simply returns the payment data as is. 
pub async fn tokenize_in_router_when_confirm_false<F, Req, Ctx>(
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
    let payment_data = if !is_operation_confirm(operation) {
        let (_operation, payment_method_data) = operation
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

#[derive(Clone, Default, Debug)]
pub struct PaymentAddress {
    pub shipping: Option<api::Address>,
    pub billing: Option<api::Address>,
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
    pub address: PaymentAddress,
    pub token: Option<String>,
    pub confirm: Option<bool>,
    pub force_sync: Option<bool>,
    pub payment_method_data: Option<api::PaymentMethodData>,
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
    pub frm_metadata: Option<serde_json::Value>,
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

/// Determines the appropriate change operation based on the intent status and confirmation status.
///
/// # Arguments
///
/// * `status` - The status of the intent
/// * `confirm` - Whether the change operation requires confirmation
/// * `current` - The current operation
///
/// # Generic Parameters
///
/// * `Op` - The type of operation
/// * `F` - The type of function
/// * `Ctx` - The payment method retrieve context
///
/// # Returns
///
/// A boxed operation based on the intent status and confirmation status
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

/// This method takes an operation and a confirmation flag and returns a boxed operation based on the confirmation flag. If the confirmation flag is true or not provided, it returns a boxed PaymentConfirm operation, otherwise it returns a boxed operation. The returned operation is boxed to hide the concrete type of the operation.
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

/// Determines whether to call the connector based on the provided operation and payment data.
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

/// Checks if the provided operation is a "PaymentConfirm" operation.
pub fn is_operation_confirm<Op: Debug>(operation: &Op) -> bool {
    matches!(format!("{operation:?}").as_str(), "PaymentConfirm")
}

/// Checks if the given operation is a complete authorization. 
/// 
/// # Arguments
/// 
/// * `operation` - The operation to be checked for completeness.
/// 
/// # Returns
/// 
/// A boolean value indicating if the operation is a complete authorization.
pub fn is_operation_complete_authorize<Op: Debug>(operation: &Op) -> bool {
    matches!(format!("{operation:?}").as_str(), "CompleteAuthorize")
}

#[cfg(feature = "olap")]
/// Retrieves a list of payments based on the provided constraints for the given merchant account.
pub async fn list_payments(
    state: AppState,
    merchant: domain::MerchantAccount,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<api::PaymentListResponse> {
    // Method implementation goes here
}
#[cfg(feature = "olap")]
/// Apply filters on payments based on the given constraints and return a list of filtered payment intents and attempts along with the total count of filtered payment attempts. 
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
/// Asynchronously retrieves filters for payments based on the provided merchant account and time range constraints.
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

/// Asynchronously adds a process sync task to the database for tracking payment attempts.
///
/// # Arguments
///
/// * `db` - A reference to a storage interface for database operations.
/// * `payment_attempt` - A reference to the payment attempt data to be synchronized.
/// * `schedule_time` - The time at which the sync task should be scheduled.
///
/// # Returns
///
/// * `Result<(), sch_errors::ProcessTrackerError>` - A result indicating success or an error of type ProcessTrackerError.
///
/// # Examples
///
///

/// Asynchronously resets a process sync task by updating its schedule time in the database.
///
/// # Arguments
///
/// * `db` - A reference to a dyn StorageInterface trait object.
/// * `payment_attempt` - A reference to a storage::PaymentAttempt object representing the payment attempt.
/// * `schedule_time` - A time::PrimitiveDateTime object representing the new schedule time for the task.
///
/// # Returns
///
/// * `Result<(), errors::ProcessTrackerError>` - A result indicating success or an error of type errors::ProcessTrackerError.
///
/// # Examples
///
///

/// Updates the straight through routing algorithm for a given payment data based on the provided request.
///
/// # Arguments
///
/// * `payment_data` - A mutable reference to the payment data to be updated
/// * `request_straight_through` - The request for the straight through routing algorithm in JSON format
///
/// # Returns
///
/// A `CustomResult` indicating the result of the update operation, with a `ParsingError` if there was an issue parsing the request
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
/// This method is responsible for getting the connector choice for a given operation and payment data. It takes in various parameters including the operation, application state, request, merchant account, business profile, key store, payment data, and eligible connectors. It then determines the appropriate connector choice based on the operation and payment data, and returns an optional connector call type based on the connector choice.
pub async fn get_connector_choice<F, Req, Ctx>(
    operation: &BoxedOperation<'_, F, Req, Ctx>,
    state: &AppState,
    req: &Req,
    merchant_account: &domain::MerchantAccount,
    business_profile: &storage::business_profile::BusinessProfile,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
) -> RouterResult<Option<ConnectorCallType>>
where
    F: Send + Clone,
    Ctx: PaymentMethodRetrieve,
{
    // method implementation
}

/// This method is used for selecting a connector for payment routing based on certain criteria. It takes in various parameters such as the application state, merchant account, business profile, key store, payment data, straight through request, and eligible connectors. It then processes the data and makes a decision on the connector to be used for payment routing. The method also handles encoding and updating the payment data with the selected connector information. Finally, it returns the selected connector for further processing.
pub async fn connector_selection<F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &storage::business_profile::BusinessProfile,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    request_straight_through: Option<serde_json::Value>,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
{
    // method implementation goes here
}

#[allow(clippy::too_many_arguments)]
/// This method is responsible for deciding the connector to be used for a payment transaction based on various criteria such as previously decided connector, mandate connector details, pre-routing results, routing algorithm, and eligibility analysis. It also handles the execution of straight through routing and fallback in case of eligibility checks. If none of the criteria match, it falls back to the default routing method. The method returns the type of connector call to be made (pre-determined or retryable) along with the connector data.
pub async fn decide_connector<F>(
    state: AppState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &storage::business_profile::BusinessProfile,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    request_straight_through: Option<api::routing::StraightThroughAlgorithm>,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
{
    // method implementation goes here
}

/// Checks if a task should be added to the process tracker based on the payment data.
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

/// Asynchronously performs session token routing based on the provided payment data, merchant information, and available connectors. 
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
    // Method implementation goes here
}

/// Perform routing logic to determine the appropriate connector for the given payment data. 
/// This method takes in various input parameters such as the application state, merchant account, 
/// business profile, key store, payment data, routing data, and eligible connectors, and uses 
/// them to perform routing algorithm, static routing, eligibility analysis, and connector selection 
/// to determine the appropriate connector for the payment. It returns the selected connector 
/// along with its data in a `RouterResult`.
pub async fn route_connector_v1<F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &storage::business_profile::BusinessProfile,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
{
    // Method implementation...
}
