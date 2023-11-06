pub mod access_token;
pub mod customers;
pub mod flows;
pub mod helpers;
pub mod operations;
pub mod routing;
pub mod tokenization;
pub mod transformers;
pub mod types;

use std::{fmt::Debug, marker::PhantomData, ops::Deref, time::Instant, vec::IntoIter};

use api_models::{
    enums,
    payment_methods::{SurchargeDetailsResponse, SurchargeMetadata},
    payments::HeaderPayload,
};
use common_utils::{ext_traits::AsyncExt, pii};
use data_models::mandates::MandateData;
use diesel_models::{ephemeral_key, fraud_check::FraudCheck};
use error_stack::{IntoReport, ResultExt};
use futures::future::join_all;
use helpers::ApplePayData;
use masking::Secret;
use router_env::{instrument, tracing};
#[cfg(feature = "olap")]
use router_types::transformers::ForeignFrom;
use scheduler::{db::process_tracker::ProcessTrackerExt, errors as sch_errors, utils as pt_utils};
use time;

pub use self::operations::{
    PaymentApprove, PaymentCancel, PaymentCapture, PaymentConfirm, PaymentCreate,
    PaymentMethodValidate, PaymentReject, PaymentResponse, PaymentSession, PaymentStatus,
    PaymentUpdate,
};
use self::{
    flows::{ConstructFlowSpecificData, Feature},
    operations::{payment_complete_authorize, BoxedOperation, Operation},
    routing::{self as self_routing, SessionFlowRoutingInput},
};
use super::errors::StorageErrorExt;
use crate::{
    configs::settings::PaymentMethodTypeTokenFilter,
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
        storage::{self, enums as storage_enums},
        transformers::ForeignTryInto,
    },
    utils::{
        add_apple_pay_flow_metrics, add_connector_http_status_code_metrics, Encode, OptionExt,
        ValueExt,
    },
    workflows::payment_sync,
};

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_operation_core<F, Req, Op, FData, Ctx>(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    call_connector_action: CallConnectorAction,
    auth_flow: services::AuthFlow,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
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
    Req: Authenticate,
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

    let (operation, mut payment_data, customer_details) = operation
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

    let connector = get_connector_choice(
        &operation,
        state,
        &req,
        &merchant_account,
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
    )
    .await?;

    let mut connector_http_status_code = None;
    let mut external_latency = None;
    if let Some(connector_details) = connector {
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
                )
                .await?;
                let operation = Box::new(PaymentResponse);
                let db = &*state.store;
                connector_http_status_code = router_data.connector_http_status_code;
                external_latency = router_data.external_latency;
                //add connector http status code metrics
                add_connector_http_status_code_metrics(connector_http_status_code);
                operation
                    .to_post_update_tracker()?
                    .update_tracker(
                        db,
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
                    connector_data,
                    &operation,
                    &mut payment_data,
                    &customer,
                    call_connector_action,
                    &validate_result,
                    schedule_time,
                    header_payload,
                )
                .await?;

                let operation = Box::new(PaymentResponse);
                let db = &*state.store;
                connector_http_status_code = router_data.connector_http_status_code;
                external_latency = router_data.external_latency;
                //add connector http status code metrics
                add_connector_http_status_code_metrics(connector_http_status_code);
                operation
                    .to_post_update_tracker()?
                    .update_tracker(
                        db,
                        &validate_result.payment_id,
                        payment_data,
                        router_data,
                        merchant_account.storage_scheme,
                    )
                    .await?
            }

            api::ConnectorCallType::SessionMultiple(connectors) => {
                call_multiple_connectors_service(
                    state,
                    &merchant_account,
                    &key_store,
                    connectors,
                    &operation,
                    payment_data,
                    &customer,
                    None,
                )
                .await?
            }
        };
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
                &*state.store,
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

    Ok((
        payment_data,
        req,
        customer,
        connector_http_status_code,
        external_latency,
    ))
}

#[inline]
pub fn get_connector_data(
    connectors: &mut IntoIter<api::ConnectorData>,
) -> RouterResult<api::ConnectorData> {
    connectors
        .next()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Connector not found in connectors iterator")
}

#[allow(clippy::too_many_arguments)]
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
    Req: Debug + Authenticate,
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
    async fn handle_payments_redirect_response(
        &self,
        state: AppState,
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

        let response = self
            .call_payment_flow(
                &state,
                merchant_account.clone(),
                key_store,
                req.clone(),
                flow_type,
            )
            .await;

        let payments_response = match response? {
            services::ApplicationResponse::Json(response) => Ok(response),
            services::ApplicationResponse::JsonWithHeaders((response, _)) => Ok(response),
            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .into_report()
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

        let result =
            self.generate_response(payments_response, business_profile, resource_id, connector)?;

        Ok(services::ApplicationResponse::JsonForRedirection(result))
    }
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectCompleteAuthorize;

#[async_trait::async_trait]
impl<Ctx: PaymentMethodRetrieve> PaymentRedirectFlow<Ctx> for PaymentRedirectCompleteAuthorize {
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
        payments_core::<api::CompleteAuthorize, api::PaymentsResponse, _, _, _, Ctx>(
            state.clone(),
            merchant_account,
            merchant_key_store,
            payment_complete_authorize::CompleteAuthorize,
            payment_confirm_req,
            services::api::AuthFlow::Merchant,
            connector_action,
            None,
            HeaderPayload::default(),
        )
        .await
    }

    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::CompleteAuthorize
    }

    fn generate_response(
        &self,
        payments_response: api_models::payments::PaymentsResponse,
        business_profile: diesel_models::business_profile::BusinessProfile,
        payment_id: String,
        connector: String,
    ) -> RouterResult<api::RedirectionResponse> {
        // There might be multiple redirections needed for some flows
        // If the status is requires customer action, then send the startpay url again
        // The redirection data must have been provided and updated by the connector
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
            // If the status is terminal status, then redirect to merchant return url to provide status
            api_models::enums::IntentStatus::Succeeded
            | api_models::enums::IntentStatus::Failed
            | api_models::enums::IntentStatus::Cancelled | api_models::enums::IntentStatus::RequiresCapture| api_models::enums::IntentStatus::Processing=> helpers::get_handle_response_url(
                payment_id,
                &business_profile,
                payments_response,
                connector,
            ),
            _ => Err(errors::ApiErrorResponse::InternalServerError).into_report().attach_printable_lazy(|| format!("Could not proceed with payment as payment status {} cannot be handled during redirection",payments_response.status))?
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectSync;

#[async_trait::async_trait]
impl<Ctx: PaymentMethodRetrieve> PaymentRedirectFlow<Ctx> for PaymentRedirectSync {
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
        payments_core::<api::PSync, api::PaymentsResponse, _, _, _, Ctx>(
            state.clone(),
            merchant_account,
            merchant_key_store,
            PaymentStatus,
            payment_sync_req,
            services::api::AuthFlow::Merchant,
            connector_action,
            None,
            HeaderPayload::default(),
        )
        .await
    }
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

    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::PSync
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
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

    let (pd, tokenization_action) = get_connector_tokenization_action_when_confirm_true(
        state,
        operation,
        payment_data,
        validate_result,
        &merchant_connector_account,
    )
    .await?;

    *payment_data = pd;

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
            Some(api_models::payments::PaymentMethodData::Wallet(
                api_models::payments::WalletData::ApplePay(wallet_data),
            )) => Some(
                ApplePayData::token_json(api_models::payments::WalletData::ApplePay(wallet_data))
                    .change_context(errors::ApiErrorResponse::InternalServerError)?
                    .decrypt(state)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
            ),
            _ => None,
        };

        let apple_pay_predecrypt = apple_pay_data
            .parse_value::<router_types::ApplePayPredecryptData>("ApplePayPredecryptData")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

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
            &*state.store,
            payment_data.clone(),
            customer.clone(),
            merchant_account.storage_scheme,
            updated_customer,
            key_store,
            None,
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
            .decide_flows(
                state,
                &connector,
                customer,
                call_connector_action,
                merchant_account,
                connector_request,
                key_store,
            )
            .await
    } else {
        Ok(router_data)
    };

    let etime_connector = Instant::now();
    let duration_connector = etime_connector.saturating_duration_since(stime_connector);
    tracing::info!(duration = format!("Duration taken: {}", duration_connector.as_millis()));

    router_data_res
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
    session_surcharge_metadata: Option<SurchargeMetadata>,
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

        payment_data.surcharge_details = session_surcharge_metadata
            .as_ref()
            .and_then(|surcharge_metadata| {
                surcharge_metadata.surcharge_results.get(
                    &SurchargeMetadata::get_key_for_surcharge_details_hash_map(
                        &session_connector_data.payment_method_type.into(),
                        &session_connector_data.payment_method_type,
                        None,
                    ),
                )
            })
            .cloned();

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
            customer,
            CallConnectorAction::Trigger,
            merchant_account,
            None,
            key_store,
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
        _ => (router_data, should_continue_payment),
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
        })
        .unwrap_or(false))
}

fn decide_apple_pay_flow(
    payment_method_type: &Option<api_models::enums::PaymentMethodType>,
    merchant_connector_account: &Option<helpers::MerchantConnectorAccountType>,
) -> Option<enums::ApplePayFlow> {
    payment_method_type.and_then(|pmt| match pmt {
        api_models::enums::PaymentMethodType::ApplePay => {
            check_apple_pay_metadata(merchant_connector_account)
        }
        _ => None,
    })
}

fn check_apple_pay_metadata(
    merchant_connector_account: &Option<helpers::MerchantConnectorAccountType>,
) -> Option<enums::ApplePayFlow> {
    merchant_connector_account.clone().and_then(|mca| {
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
                    |error| logger::error!(%error, "Failed to Parse Value to ApplepaySessionTokenData"),
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
pub async fn get_connector_tokenization_action_when_confirm_true<F, Req, Ctx>(
    state: &AppState,
    operation: &BoxedOperation<'_, F, Req, Ctx>,
    payment_data: &mut PaymentData<F>,
    validate_result: &operations::ValidateResult<'_>,
    merchant_connector_account: &helpers::MerchantConnectorAccountType,
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

            let is_connector_tokenization_enabled =
                is_payment_method_tokenization_enabled_for_connector(
                    state,
                    &connector,
                    payment_method,
                    payment_method_type,
                )?;

            let apple_pay_flow = decide_apple_pay_flow(
                payment_method_type,
                &Some(merchant_connector_account.clone()),
            );

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
                    let (_operation, payment_method_data) = operation
                        .to_domain()?
                        .make_pm_data(state, payment_data, validate_result.storage_scheme)
                        .await?;
                    payment_data.payment_method_data = payment_method_data;
                    TokenizationAction::SkipConnectorTokenization
                }

                TokenizationAction::TokenizeInConnector => TokenizationAction::TokenizeInConnector,
                TokenizationAction::TokenizeInConnectorAndRouter => {
                    let (_operation, payment_method_data) = operation
                        .to_domain()?
                        .make_pm_data(state, payment_data, validate_result.storage_scheme)
                        .await?;

                    payment_data.payment_method_data = payment_method_data;
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

pub async fn tokenize_in_router_when_confirm_false<F, Req, Ctx>(
    state: &AppState,
    operation: &BoxedOperation<'_, F, Req, Ctx>,
    payment_data: &mut PaymentData<F>,
    validate_result: &operations::ValidateResult<'_>,
) -> RouterResult<PaymentData<F>>
where
    F: Send + Clone,
    Ctx: PaymentMethodRetrieve,
{
    // On confirm is false and only router related
    let payment_data = if !is_operation_confirm(operation) {
        let (_operation, payment_method_data) = operation
            .to_domain()?
            .make_pm_data(state, payment_data, validate_result.storage_scheme)
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
    pub connector_response: storage::ConnectorResponse,
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
    pub surcharge_details: Option<SurchargeDetailsResponse>,
    pub frm_message: Option<FraudCheck>,
    pub payment_link_data: Option<api_models::payments::PaymentLinkResponse>,
}

#[derive(Debug, Default, Clone)]
pub struct RecurringMandatePaymentData {
    pub payment_method_type: Option<storage_enums::PaymentMethodType>, //required for making recurring payment using saved payment method through stripe
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
            ) && payment_data
                .connector_response
                .authentication_data
                .is_none()
        }
        "PaymentStatus" => {
            matches!(
                payment_data.payment_intent.status,
                storage_enums::IntentStatus::Processing
                    | storage_enums::IntentStatus::RequiresCustomerAction
                    | storage_enums::IntentStatus::RequiresMerchantAction
                    | storage_enums::IntentStatus::RequiresCapture
                    | storage_enums::IntentStatus::PartiallyCaptured
            ) && payment_data.force_sync.unwrap_or(false)
        }
        "PaymentCancel" => matches!(
            payment_data.payment_intent.status,
            storage_enums::IntentStatus::RequiresCapture
                | storage_enums::IntentStatus::PartiallyCaptured
        ),
        "PaymentCapture" => {
            matches!(
                payment_data.payment_intent.status,
                storage_enums::IntentStatus::RequiresCapture
                    | storage_enums::IntentStatus::PartiallyCaptured
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
        "PaymentSession" => true,
        _ => false,
    }
}

pub fn is_operation_confirm<Op: Debug>(operation: &Op) -> bool {
    matches!(format!("{operation:?}").as_str(), "PaymentConfirm")
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
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?
        .into_iter()
        .map(|(pi, pa)| (pi, pa))
        .collect();

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

pub async fn add_process_sync_task(
    db: &dyn StorageInterface,
    payment_attempt: &storage::PaymentAttempt,
    schedule_time: time::PrimitiveDateTime,
) -> Result<(), sch_errors::ProcessTrackerError> {
    let tracking_data = api::PaymentsRetrieveRequest {
        force_sync: true,
        merchant_id: Some(payment_attempt.merchant_id.clone()),
        resource_id: api::PaymentIdType::PaymentAttemptId(payment_attempt.attempt_id.clone()),
        ..Default::default()
    };
    let runner = "PAYMENTS_SYNC_WORKFLOW";
    let task = "PAYMENTS_SYNC";
    let process_tracker_id = pt_utils::get_process_tracker_id(
        runner,
        task,
        &payment_attempt.attempt_id,
        &payment_attempt.merchant_id,
    );
    let process_tracker_entry = <storage::ProcessTracker>::make_process_tracker_new(
        process_tracker_id,
        task,
        runner,
        tracking_data,
        schedule_time,
    )?;

    db.insert_process(process_tracker_entry).await?;
    Ok(())
}

pub async fn reset_process_sync_task(
    db: &dyn StorageInterface,
    payment_attempt: &storage::PaymentAttempt,
    schedule_time: time::PrimitiveDateTime,
) -> Result<(), errors::ProcessTrackerError> {
    let runner = "PAYMENTS_SYNC_WORKFLOW";
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
    psync_process
        .reset(db.as_scheduler(), schedule_time)
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

pub async fn get_connector_choice<F, Req, Ctx>(
    operation: &BoxedOperation<'_, F, Req, Ctx>,
    state: &AppState,
    req: &Req,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
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
                    key_store,
                    payment_data,
                    Some(straight_through),
                    eligible_connectors,
                )
                .await?
            }

            api::ConnectorChoice::Decide => {
                connector_selection(
                    state,
                    merchant_account,
                    key_store,
                    payment_data,
                    None,
                    eligible_connectors,
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

pub async fn connector_selection<F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    request_straight_through: Option<serde_json::Value>,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
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
        key_store,
        payment_data,
        request_straight_through,
        &mut routing_data,
        eligible_connectors,
    )
    .await?;

    let encoded_info =
        Encode::<storage::PaymentRoutingInfo>::encode_to_value(&routing_data.routing_info)
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

pub async fn decide_connector<F>(
    state: AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    request_straight_through: Option<api::routing::StraightThroughAlgorithm>,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
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
        if let Some(choice) = pre_routing_results.get(storage_pm_type) {
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
        let (mut connectors, check_eligibility) =
            routing::perform_straight_through_routing(&routing_algorithm, payment_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed execution of straight through routing")?;

        if check_eligibility {
            connectors = routing::perform_eligibility_analysis_with_fallback(
                &state.clone(),
                key_store,
                merchant_account.modified_at.assume_utc().unix_timestamp(),
                connectors,
                payment_data,
                eligible_connectors,
                #[cfg(feature = "business_profile_routing")]
                payment_data.payment_intent.profile_id.clone(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed eligibility analysis and fallback")?;
        }

        let first_connector_choice = connectors
            .first()
            .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
            .into_report()
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
                    conn.merchant_connector_id.clone(),
                    #[cfg(not(feature = "connector_choice_mca_id"))]
                    None,
                )
            })
            .collect::<CustomResult<Vec<_>, _>>()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid connector name received")?;

        routing_data.routed_through = Some(first_connector_choice.connector.to_string());
        #[cfg(feature = "connector_choice_mca_id")]
        {
            routing_data.merchant_connector_id = first_connector_choice.merchant_connector_id;
        }
        #[cfg(not(feature = "connector_choice_mca_id"))]
        {
            routing_data.business_sub_label = first_connector_choice.sub_label.clone();
        }
        routing_data.routing_info.algorithm = Some(routing_algorithm);
        return Ok(api::ConnectorCallType::Retryable(connector_data));
    }

    if let Some(ref routing_algorithm) = routing_data.routing_info.algorithm {
        let (mut connectors, check_eligibility) =
            routing::perform_straight_through_routing(routing_algorithm, payment_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed execution of straight through routing")?;

        if check_eligibility {
            connectors = routing::perform_eligibility_analysis_with_fallback(
                &state,
                key_store,
                merchant_account.modified_at.assume_utc().unix_timestamp(),
                connectors,
                payment_data,
                eligible_connectors,
                #[cfg(feature = "business_profile_routing")]
                payment_data.payment_intent.profile_id.clone(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed eligibility analysis and fallback")?;
        }

        let first_connector_choice = connectors
            .first()
            .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
            .into_report()
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

        routing_data.routed_through = Some(first_connector_choice.connector.to_string());
        #[cfg(feature = "connector_choice_mca_id")]
        {
            routing_data.merchant_connector_id = first_connector_choice.merchant_connector_id;
        }
        #[cfg(not(feature = "connector_choice_mca_id"))]
        {
            routing_data.business_sub_label = first_connector_choice.sub_label;
        }
        return Ok(api::ConnectorCallType::Retryable(connector_data));
    }

    route_connector_v1(
        &state,
        merchant_account,
        key_store,
        payment_data,
        routing_data,
        eligible_connectors,
    )
    .await
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
            .billing
            .as_ref()
            .and_then(|address| address.address.as_ref())
            .and_then(|details| details.country),
        key_store,
        merchant_account,
        payment_attempt: &payment_data.payment_attempt,
        payment_intent: &payment_data.payment_intent,

        chosen,
    };
    let result = self_routing::perform_session_flow_routing(sfr)
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

pub async fn route_connector_v1<F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
    routing_data: &mut storage::RoutingData,
    eligible_connectors: Option<Vec<api_models::enums::RoutableConnectors>>,
) -> RouterResult<ConnectorCallType>
where
    F: Send + Clone,
{
    #[cfg(not(feature = "business_profile_routing"))]
    let algorithm_ref: api::routing::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|ra| ra.parse_value("RoutingAlgorithmRef"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode merchant routing algorithm ref")?
        .unwrap_or_default();

    #[cfg(feature = "business_profile_routing")]
    let algorithm_ref: api::routing::RoutingAlgorithmRef = {
        let profile_id = payment_data
            .payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let business_profile = state
            .store
            .find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
            })?;

        business_profile
            .routing_algorithm
            .clone()
            .map(|ra| ra.parse_value("RoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not decode merchant routing algorithm ref")?
            .unwrap_or_default()
    };

    let connectors = routing::perform_static_routing_v1(
        state,
        &merchant_account.merchant_id,
        algorithm_ref,
        payment_data,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let connectors = routing::perform_eligibility_analysis_with_fallback(
        &state.clone(),
        key_store,
        merchant_account.modified_at.assume_utc().unix_timestamp(),
        connectors,
        payment_data,
        eligible_connectors,
        #[cfg(feature = "business_profile_routing")]
        payment_data.payment_intent.profile_id.clone(),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("failed eligibility analysis and fallback")?;

    let first_connector_choice = connectors
        .first()
        .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
        .into_report()
        .attach_printable("Empty connector list returned")?
        .clone();

    routing_data.routed_through = Some(first_connector_choice.connector.to_string());

    #[cfg(feature = "connector_choice_mca_id")]
    {
        routing_data.merchant_connector_id = first_connector_choice.merchant_connector_id;
    }
    #[cfg(not(feature = "connector_choice_mca_id"))]
    {
        routing_data.business_sub_label = first_connector_choice.sub_label;
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

    Ok(ConnectorCallType::Retryable(connector_data))
}
