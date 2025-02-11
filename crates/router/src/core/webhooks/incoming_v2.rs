use std::{marker::PhantomData, str::FromStr, time::Instant};

use actix_web::FromRequest;
use api_models::webhooks::{self, WebhookResponseTracker};
use common_utils::{
    errors::ReportSwitchExt, events::ApiEventsType, types::keymanager::KeyManagerState,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payments::{HeaderPayload, PaymentStatusData},
    router_request_types::VerifyWebhookSourceRequestData,
    router_response_types::{VerifyWebhookSourceResponseData, VerifyWebhookStatus},
};
use hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails;
use router_env::{instrument, tracing, tracing_actix_web::RequestId};

use super::{types, utils, MERCHANT_ID};
use crate::{
    core::{
        api_locking,
        errors::{self, ConnectorErrorExt, CustomResult, RouterResponse, StorageErrorExt},
        metrics,
        payments::{
            self,
            transformers::{GenerateResponse, ToResponse},
        },
        webhooks::utils::construct_webhook_router_data,
    },
    db::StorageInterface,
    events::api_logs::ApiEvent,
    logger,
    routes::{
        app::{ReqState, SessionStateInfo},
        lock_utils, SessionState,
    },
    services::{
        self, authentication as auth, connector_integration_interface::ConnectorEnum,
        ConnectorValidation,
    },
    types::{
        api::{self, ConnectorData, GetToken, IncomingWebhook},
        domain,
        storage::enums,
        transformers::ForeignInto,
    },
};

#[allow(clippy::too_many_arguments)]
pub async fn incoming_webhooks_wrapper<W: types::OutgoingWebhookType>(
    flow: &impl router_env::types::FlowMetric,
    state: SessionState,
    req_state: ReqState,
    req: &actix_web::HttpRequest,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    body: actix_web::web::Bytes,
    is_relay_webhook: bool,
) -> RouterResponse<serde_json::Value> {
    let start_instant = Instant::now();
    let (application_response, webhooks_response_tracker, serialized_req) =
        Box::pin(incoming_webhooks_core::<W>(
            state.clone(),
            req_state,
            req,
            merchant_account.clone(),
            profile,
            key_store,
            connector_id,
            body.clone(),
            is_relay_webhook,
        ))
        .await?;

    logger::info!(incoming_webhook_payload = ?serialized_req);

    let request_duration = Instant::now()
        .saturating_duration_since(start_instant)
        .as_millis();

    let request_id = RequestId::extract(req)
        .await
        .attach_printable("Unable to extract request id from request")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let auth_type = auth::AuthenticationType::WebhookAuth {
        merchant_id: merchant_account.get_id().clone(),
    };
    let status_code = 200;
    let api_event = ApiEventsType::Webhooks {
        connector: connector_id.clone(),
        payment_id: webhooks_response_tracker.get_payment_id(),
    };
    let response_value = serde_json::to_value(&webhooks_response_tracker)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not convert webhook effect to string")?;

    let api_event = ApiEvent::new(
        state.tenant.tenant_id.clone(),
        Some(merchant_account.get_id().clone()),
        flow,
        &request_id,
        request_duration,
        status_code,
        serialized_req,
        Some(response_value),
        None,
        auth_type,
        None,
        api_event,
        req,
        req.method(),
    );
    state.event_handler().log_event(&api_event);
    Ok(application_response)
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn incoming_webhooks_core<W: types::OutgoingWebhookType>(
    state: SessionState,
    req_state: ReqState,
    req: &actix_web::HttpRequest,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    body: actix_web::web::Bytes,
    _is_relay_webhook: bool,
) -> errors::RouterResult<(
    services::ApplicationResponse<serde_json::Value>,
    WebhookResponseTracker,
    serde_json::Value,
)> {
    metrics::WEBHOOK_INCOMING_COUNT.add(
        1,
        router_env::metric_attributes!((MERCHANT_ID, merchant_account.get_id().clone())),
    );
    let mut request_details = IncomingWebhookRequestDetails {
        method: req.method().clone(),
        uri: req.uri().clone(),
        headers: req.headers(),
        query_params: req.query_string().to_string(),
        body: &body,
    };

    // Fetch the merchant connector account to get the webhooks source secret
    // `webhooks source secret` is a secret shared between the merchant and connector
    // This is used for source verification and webhooks integrity
    let (merchant_connector_account, connector, connector_name) =
        fetch_mca_and_connector(&state, connector_id, &key_store).await?;

    let decoded_body = connector
        .decode_webhook_body(
            &request_details,
            merchant_account.get_id(),
            merchant_connector_account.connector_webhook_details.clone(),
            connector_name.as_str(),
        )
        .await
        .switch()
        .attach_printable("There was an error in incoming webhook body decoding")?;

    request_details.body = &decoded_body;

    let event_type = match connector
        .get_webhook_event_type(&request_details)
        .allow_webhook_event_type_not_found(
            state
                .clone()
                .conf
                .webhooks
                .ignore_error
                .event_type
                .unwrap_or(true),
        )
        .switch()
        .attach_printable("Could not find event type in incoming webhook body")?
    {
        Some(event_type) => event_type,
        // Early return allows us to acknowledge the webhooks that we do not support
        None => {
            logger::error!(
                webhook_payload =? request_details.body,
                "Failed while identifying the event type",
            );

            metrics::WEBHOOK_EVENT_TYPE_IDENTIFICATION_FAILURE_COUNT.add(
                1,
                router_env::metric_attributes!(
                    (MERCHANT_ID, merchant_account.get_id().clone()),
                    ("connector", connector_name)
                ),
            );

            let response = connector
                .get_webhook_api_response(&request_details, None)
                .switch()
                .attach_printable("Failed while early return in case of event type parsing")?;

            return Ok((
                response,
                WebhookResponseTracker::NoEffect,
                serde_json::Value::Null,
            ));
        }
    };
    logger::info!(event_type=?event_type);

    let is_webhook_event_supported = !matches!(
        event_type,
        webhooks::IncomingWebhookEvent::EventNotSupported
    );
    let is_webhook_event_enabled = !utils::is_webhook_event_disabled(
        &*state.clone().store,
        connector_name.as_str(),
        merchant_account.get_id(),
        &event_type,
    )
    .await;

    //process webhook further only if webhook event is enabled and is not event_not_supported
    let process_webhook_further = is_webhook_event_enabled && is_webhook_event_supported;

    logger::info!(process_webhook=?process_webhook_further);

    let flow_type: api::WebhookFlow = event_type.into();
    let mut event_object: Box<dyn masking::ErasedMaskSerialize> = Box::new(serde_json::Value::Null);
    let webhook_effect = if process_webhook_further
        && !matches!(flow_type, api::WebhookFlow::ReturnResponse)
    {
        let object_ref_id = connector
            .get_webhook_object_reference_id(&request_details)
            .switch()
            .attach_printable("Could not find object reference id in incoming webhook body")?;
        let connector_enum = api_models::enums::Connector::from_str(&connector_name)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector",
            })
            .attach_printable_lazy(|| {
                format!("unable to parse connector name {connector_name:?}")
            })?;
        let connectors_with_source_verification_call = &state.conf.webhook_source_verification_call;

        let source_verified = if connectors_with_source_verification_call
            .connectors_with_webhook_source_verification_call
            .contains(&connector_enum)
        {
            verify_webhook_source_verification_call(
                connector.clone(),
                &state,
                &merchant_account,
                merchant_connector_account.clone(),
                &connector_name,
                &request_details,
            )
            .await
            .or_else(|error| match error.current_context() {
                errors::ConnectorError::WebhookSourceVerificationFailed => {
                    logger::error!(?error, "Source Verification Failed");
                    Ok(false)
                }
                _ => Err(error),
            })
            .switch()
            .attach_printable("There was an issue in incoming webhook source verification")?
        } else {
            connector
                .clone()
                .verify_webhook_source(
                    &request_details,
                    merchant_account.get_id(),
                    merchant_connector_account.connector_webhook_details.clone(),
                    merchant_connector_account.connector_account_details.clone(),
                    connector_name.as_str(),
                )
                .await
                .or_else(|error| match error.current_context() {
                    errors::ConnectorError::WebhookSourceVerificationFailed => {
                        logger::error!(?error, "Source Verification Failed");
                        Ok(false)
                    }
                    _ => Err(error),
                })
                .switch()
                .attach_printable("There was an issue in incoming webhook source verification")?
        };

        logger::info!(source_verified=?source_verified);

        if source_verified {
            metrics::WEBHOOK_SOURCE_VERIFIED_COUNT.add(
                1,
                router_env::metric_attributes!((MERCHANT_ID, merchant_account.get_id().clone())),
            );
        }

        // If source verification is mandatory and source is not verified, fail with webhook authentication error
        // else continue the flow
        match (
            connector.is_webhook_source_verification_mandatory(),
            source_verified,
        ) {
            (true, false) => Err(errors::ApiErrorResponse::WebhookAuthenticationFailed)?,
            _ => {
                event_object = connector
                    .get_webhook_resource_object(&request_details)
                    .switch()
                    .attach_printable("Could not find resource object in incoming webhook body")?;

                let webhook_details = api::IncomingWebhookDetails {
                    object_reference_id: object_ref_id.clone(),
                    resource_object: serde_json::to_vec(&event_object)
                        .change_context(errors::ParsingError::EncodeError("byte-vec"))
                        .attach_printable("Unable to convert webhook payload to a value")
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "There was an issue when encoding the incoming webhook body to bytes",
                        )?,
                };

                match flow_type {
                    api::WebhookFlow::Payment => Box::pin(payments_incoming_webhook_flow(
                        state.clone(),
                        req_state,
                        merchant_account,
                        profile,
                        key_store,
                        webhook_details,
                        source_verified,
                    ))
                    .await
                    .attach_printable("Incoming webhook flow for payments failed")?,

                    api::WebhookFlow::Refund => todo!(),

                    api::WebhookFlow::Dispute => todo!(),

                    api::WebhookFlow::BankTransfer => todo!(),

                    api::WebhookFlow::ReturnResponse => WebhookResponseTracker::NoEffect,

                    api::WebhookFlow::Mandate => todo!(),

                    api::WebhookFlow::ExternalAuthentication => todo!(),
                    api::WebhookFlow::FraudCheck => todo!(),

                    #[cfg(feature = "payouts")]
                    api::WebhookFlow::Payout => todo!(),

                    api::WebhookFlow::Subscription => todo!(),
                }
            }
        }
    } else {
        metrics::WEBHOOK_INCOMING_FILTERED_COUNT.add(
            1,
            router_env::metric_attributes!((MERCHANT_ID, merchant_account.get_id().clone())),
        );
        WebhookResponseTracker::NoEffect
    };

    let response = connector
        .get_webhook_api_response(&request_details, None)
        .switch()
        .attach_printable("Could not get incoming webhook api response from connector")?;

    let serialized_request = event_object
        .masked_serialize()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not convert webhook effect to string")?;
    Ok((response, webhook_effect, serialized_request))
}

#[instrument(skip_all)]
async fn payments_incoming_webhook_flow(
    state: SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let consume_or_trigger_flow = if source_verified {
        payments::CallConnectorAction::HandleResponse(webhook_details.resource_object)
    } else {
        payments::CallConnectorAction::Trigger
    };
    let key_manager_state = &(&state).into();
    let payments_response = match webhook_details.object_reference_id {
        webhooks::ObjectReferenceId::PaymentId(id) => {
            let get_trackers_response = get_trackers_response_for_payment_get_operation(
                state.store.as_ref(),
                &id,
                profile.get_id(),
                key_manager_state,
                &key_store,
                merchant_account.storage_scheme,
            )
            .await?;

            let payment_id = get_trackers_response.payment_data.get_payment_id();

            let lock_action = api_locking::LockAction::Hold {
                input: api_locking::LockingInput {
                    unique_locking_key: payment_id.get_string_repr().to_owned(),
                    api_identifier: lock_utils::ApiIdentifier::Payments,
                    override_lock_retries: None,
                },
            };

            lock_action
                .clone()
                .perform_locking_action(&state, merchant_account.get_id().to_owned())
                .await?;

            let (payment_data, _req, customer, connector_http_status_code, external_latency) =
                Box::pin(payments::payments_operation_core::<
                    api::PSync,
                    _,
                    _,
                    _,
                    PaymentStatusData<api::PSync>,
                >(
                    &state,
                    req_state,
                    merchant_account.clone(),
                    key_store.clone(),
                    profile,
                    payments::operations::PaymentGet,
                    api::PaymentsRetrieveRequest {
                        force_sync: true,
                        expand_attempts: false,
                        param: None,
                    },
                    get_trackers_response,
                    consume_or_trigger_flow,
                    HeaderPayload::default(),
                ))
                .await?;

            let response = payment_data.generate_response(
                &state,
                connector_http_status_code,
                external_latency,
                None,
                &merchant_account,
            );

            lock_action
                .free_lock_action(&state, merchant_account.get_id().to_owned())
                .await?;

            match response {
                Ok(value) => value,
                Err(err)
                    if matches!(
                        err.current_context(),
                        &errors::ApiErrorResponse::PaymentNotFound
                    ) && state
                        .clone()
                        .conf
                        .webhooks
                        .ignore_error
                        .payment_not_found
                        .unwrap_or(true) =>
                {
                    metrics::WEBHOOK_PAYMENT_NOT_FOUND.add(
                        1,
                        router_env::metric_attributes!((
                            "merchant_id",
                            merchant_account.get_id().clone()
                        )),
                    );
                    return Ok(WebhookResponseTracker::NoEffect);
                }
                error @ Err(_) => error?,
            }
        }
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure).attach_printable(
            "Did not get payment id as object reference id in webhook payments flow",
        )?,
    };

    match payments_response {
        services::ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
            let payment_id = payments_response.id.clone();

            let status = payments_response.status;

            let event_type: Option<enums::EventType> = payments_response.status.foreign_into();

            // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
            if let Some(_outgoing_event_type) = event_type {
                let _primary_object_created_at = payments_response.created;
                // TODO: trigger an outgoing webhook to merchant
                // Box::pin(super::create_event_and_trigger_outgoing_webhook(
                //     state,
                //     merchant_account,
                //     profile,
                //     &key_store,
                //     outgoing_event_type,
                //     enums::EventClass::Payments,
                //     payment_id.get_string_repr().to_owned(),
                //     enums::EventObjectType::PaymentDetails,
                //     api::OutgoingWebhookContent::PaymentDetails(Box::new(payments_response)),
                //     Some(primary_object_created_at),
                // ))
                // .await?;
            };

            let response = WebhookResponseTracker::Payment { payment_id, status };

            Ok(response)
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received non-json response from payments core")?,
    }
}

async fn get_trackers_response_for_payment_get_operation<F>(
    db: &dyn StorageInterface,
    payment_id: &api::PaymentIdType,
    profile_id: &common_utils::id_type::ProfileId,
    key_manager_state: &KeyManagerState,
    merchant_key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
) -> errors::RouterResult<payments::operations::GetTrackerResponse<PaymentStatusData<F>>>
where
    F: Clone,
{
    let (payment_intent, payment_attempt) = match payment_id {
        api_models::payments::PaymentIdType::PaymentIntentId(ref id) => {
            let payment_intent = db
                .find_payment_intent_by_id(
                    key_manager_state,
                    id,
                    merchant_key_store,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let payment_attempt = db
                .find_payment_attempt_by_id(
                    key_manager_state,
                    merchant_key_store,
                    &payment_intent
                        .active_attempt_id
                        .clone()
                        .ok_or(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("active_attempt_id not present in payment_attempt")?,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            (payment_intent, payment_attempt)
        }
        api_models::payments::PaymentIdType::ConnectorTransactionId(ref id) => {
            let payment_attempt = db
                .find_payment_attempt_by_profile_id_connector_transaction_id(
                    key_manager_state,
                    merchant_key_store,
                    profile_id,
                    id,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let payment_intent = db
                .find_payment_intent_by_id(
                    key_manager_state,
                    &payment_attempt.payment_id,
                    merchant_key_store,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            (payment_intent, payment_attempt)
        }
        api_models::payments::PaymentIdType::PaymentAttemptId(ref id) => {
            let global_attempt_id = common_utils::id_type::GlobalAttemptId::try_from(
                std::borrow::Cow::Owned(id.to_owned()),
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while getting GlobalAttemptId")?;
            let payment_attempt = db
                .find_payment_attempt_by_id(
                    key_manager_state,
                    merchant_key_store,
                    &global_attempt_id,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let payment_intent = db
                .find_payment_intent_by_id(
                    key_manager_state,
                    &payment_attempt.payment_id,
                    merchant_key_store,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            (payment_intent, payment_attempt)
        }
        api_models::payments::PaymentIdType::PreprocessingId(ref _id) => todo!(),
    };

    // We need the address here to send it in the response
    // In case we need to send an outgoing webhook, we might have to send the billing address and shipping address
    let payment_address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
        payment_intent
            .shipping_address
            .clone()
            .map(|address| address.into_inner()),
        payment_intent
            .billing_address
            .clone()
            .map(|address| address.into_inner()),
        payment_attempt
            .payment_method_billing_address
            .clone()
            .map(|address| address.into_inner()),
        Some(true),
    );

    Ok(payments::operations::GetTrackerResponse {
        payment_data: PaymentStatusData {
            flow: PhantomData,
            payment_intent,
            payment_attempt: Some(payment_attempt),
            attempts: None,
            should_sync_with_connector: true,
            payment_address,
        },
    })
}

#[inline]
async fn verify_webhook_source_verification_call(
    connector: ConnectorEnum,
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: domain::MerchantConnectorAccount,
    connector_name: &str,
    request_details: &IncomingWebhookRequestDetails<'_>,
) -> CustomResult<bool, errors::ConnectorError> {
    let connector_data = ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector_name,
        GetToken::Connector,
        None,
    )
    .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    .attach_printable("invalid connector name received in payment attempt")?;
    let connector_integration: services::BoxedWebhookSourceVerificationConnectorIntegrationInterface<
        hyperswitch_domain_models::router_flow_types::VerifyWebhookSource,
        VerifyWebhookSourceRequestData,
        VerifyWebhookSourceResponseData,
    > = connector_data.connector.get_connector_integration();
    let connector_webhook_secrets = connector
        .get_webhook_source_verification_merchant_secret(
            merchant_account.get_id(),
            connector_name,
            merchant_connector_account.connector_webhook_details.clone(),
        )
        .await
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

    let router_data = construct_webhook_router_data(
        state,
        connector_name,
        merchant_connector_account,
        merchant_account,
        &connector_webhook_secrets,
        request_details,
    )
    .await
    .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    .attach_printable("Failed while constructing webhook router data")?;

    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await?;

    let verification_result = response
        .response
        .map(|response| response.verify_webhook_status);
    match verification_result {
        Ok(VerifyWebhookStatus::SourceVerified) => Ok(true),
        _ => Ok(false),
    }
}

fn get_connector_by_connector_name(
    state: &SessionState,
    connector_name: &str,
    merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
) -> CustomResult<(ConnectorEnum, String), errors::ApiErrorResponse> {
    let authentication_connector =
        api_models::enums::convert_authentication_connector(connector_name);
    #[cfg(feature = "frm")]
    {
        let frm_connector = api_models::enums::convert_frm_connector(connector_name);
        if frm_connector.is_some() {
            let frm_connector_data =
                api::FraudCheckConnectorData::get_connector_by_name(connector_name)?;
            return Ok((
                frm_connector_data.connector,
                frm_connector_data.connector_name.to_string(),
            ));
        }
    }

    let (connector, connector_name) = if authentication_connector.is_some() {
        let authentication_connector_data =
            api::AuthenticationConnectorData::get_connector_by_name(connector_name)?;
        (
            authentication_connector_data.connector,
            authentication_connector_data.connector_name.to_string(),
        )
    } else {
        let connector_data = ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            GetToken::Connector,
            merchant_connector_id,
        )
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "invalid connector name received".to_string(),
        })
        .attach_printable("Failed construction of ConnectorData")?;
        (
            connector_data.connector,
            connector_data.connector_name.to_string(),
        )
    };
    Ok((connector, connector_name))
}

/// This function fetches the merchant connector account and connector details
async fn fetch_mca_and_connector(
    state: &SessionState,
    connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<(domain::MerchantConnectorAccount, ConnectorEnum, String), errors::ApiErrorResponse>
{
    let db = &state.store;
    let mca = db
        .find_merchant_connector_account_by_id(&state.into(), connector_id, key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: connector_id.get_string_repr().to_owned(),
        })
        .attach_printable("error while fetching merchant_connector_account from connector_id")?;

    let (connector, connector_name) = get_connector_by_connector_name(
        state,
        &mca.connector_name.to_string(),
        Some(mca.get_id()),
    )?;

    Ok((mca, connector, connector_name))
}
