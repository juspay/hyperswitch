use std::{marker::PhantomData, str::FromStr};

use api_models::webhooks::{self, WebhookResponseTracker};
use common_utils::{
    errors::ReportSwitchExt,
    events::ApiEventsType,
    types::{keymanager::KeyManagerState, AmountConvertor, StringMinorUnitForConnector},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    api::{IncomingWebhookEventMetadata, WebhookResponse},
    payments::{payment_attempt::PaymentAttempt, HeaderPayload, PaymentStatusData},
    router_request_types::VerifyWebhookSourceRequestData,
    router_response_types::{VerifyWebhookSourceResponseData, VerifyWebhookStatus},
};
use hyperswitch_interfaces::webhooks::{
    IncomingWebhookRequestDetails, WebhookContext, WebhookResourceData,
};
use hyperswitch_masking::Secret;
use router_env::{instrument, tracing};

use super::{types, utils, MERCHANT_ID};
#[cfg(feature = "revenue_recovery")]
use crate::core::webhooks::recovery_incoming;
use crate::{
    consts,
    core::{
        api_locking,
        configs::dimension_state,
        errors::{self, ConnectorErrorExt, CustomResult, RouterResponse, StorageErrorExt},
        metrics,
        payments::{
            self,
            transformers::{GenerateResponse, ToResponse},
        },
        utils as core_utils,
        webhooks::{
            create_event_and_trigger_outgoing_webhook, utils::construct_webhook_router_data,
        },
    },
    db::StorageInterface,
    logger,
    routes::{app::ReqState, lock_utils, SessionState},
    services::{self, connector_integration_interface::ConnectorEnum, ConnectorValidation},
    types::{
        api::{self, ConnectorData, GetToken, IncomingWebhook},
        domain,
        storage::enums,
        transformers::{ForeignInto, ForeignTryFrom},
    },
    utils::generate_id,
};

#[allow(clippy::too_many_arguments)]
pub async fn incoming_webhooks_wrapper<W: types::OutgoingWebhookType>(
    state: SessionState,
    req_state: ReqState,
    req: &actix_web::HttpRequest,
    platform: domain::Platform,
    profile: domain::Profile,
    connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    body: actix_web::web::Bytes,
    is_relay_webhook: bool,
) -> RouterResponse<serde_json::Value> {
    let dimensions = dimension_state::Dimensions::new()
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
        .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id());

    let (webhook_response, webhooks_response_tracker, serialized_req) =
        Box::pin(incoming_webhooks_core::<W>(
            state.clone(),
            req_state,
            req,
            platform.clone(),
            profile,
            connector_id,
            body.clone(),
            is_relay_webhook,
        ))
        .await?;

    logger::info!(incoming_webhook_payload = ?serialized_req);

    let metadata = IncomingWebhookEventMetadata {
        event_type: ApiEventsType::Webhooks {
            connector: connector_id.clone(),
            payment_id: webhooks_response_tracker.get_payment_id(),
            refund_id: webhooks_response_tracker.get_refund_id(),
        },
        serialized_request: Secret::new(serialized_req),
        webhook_tracker_data: serde_json::to_value(&webhooks_response_tracker)
            .inspect_err(
                |err| logger::error!(error = ?err, "Could not convert webhook effect to string"),
            )
            .ok(),
    };

    Ok(services::ApplicationResponse::IncomingWebhookEvent {
        response: Box::new(webhook_response),
        metadata,
    })
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
async fn incoming_webhooks_core<W: types::OutgoingWebhookType>(
    state: SessionState,
    req_state: ReqState,
    req: &actix_web::HttpRequest,
    platform: domain::Platform,
    profile: domain::Profile,
    connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    body: actix_web::web::Bytes,
    _is_relay_webhook: bool,
) -> errors::RouterResult<(
    WebhookResponse<serde_json::Value>,
    WebhookResponseTracker,
    serde_json::Value,
)> {
    metrics::WEBHOOK_INCOMING_COUNT.add(
        1,
        router_env::metric_attributes!((
            MERCHANT_ID,
            platform.get_processor().get_account().get_id().clone()
        )),
    );
    let dimensions = dimension_state::Dimensions::new()
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
        .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id());

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
    let (merchant_connector_account, connector, connector_enum, connector_name) =
        fetch_mca_and_connector(
            &state,
            connector_id,
            platform.get_processor().get_key_store(),
        )
        .await?;

    let decoded_body = connector
        .decode_webhook_body(
            &request_details,
            platform.get_processor().get_account().get_id(),
            merchant_connector_account.connector_webhook_details.clone(),
            connector_name.as_str(),
        )
        .await
        .switch()
        .attach_printable("There was an error in incoming webhook body decoding")?;

    request_details.body = &decoded_body;

    let event_type = match connector
        .get_webhook_event_type(&request_details, None)
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
                    (
                        MERCHANT_ID,
                        platform.get_processor().get_account().get_id().clone()
                    ),
                    ("connector", connector_name)
                ),
            );

            let response = connector
                .get_webhook_api_response(
                    &request_details,
                    None,
                    Some(merchant_connector_account.connector_account_details.clone()),
                )
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

    // if it is a setup webhook event, return ok status
    if event_type == webhooks::IncomingWebhookEvent::SetupWebhook {
        return Ok((
            WebhookResponse::StatusOk,
            WebhookResponseTracker::NoEffect,
            serde_json::Value::default(),
        ));
    }

    let is_webhook_event_supported = !matches!(
        event_type,
        webhooks::IncomingWebhookEvent::EventNotSupported
    );
    let is_webhook_event_enabled =
        !utils::is_webhook_event_disabled(&state, connector_enum, &dimensions, &event_type).await;

    //process webhook further only if webhook event is enabled and is not event_not_supported
    let process_webhook_further = is_webhook_event_enabled && is_webhook_event_supported;

    logger::info!(process_webhook=?process_webhook_further);

    let flow_type: api::WebhookFlow = event_type.into();
    let mut event_object: Box<dyn hyperswitch_masking::ErasedMaskSerialize> =
        Box::new(serde_json::Value::Null);
    let webhook_effect = if process_webhook_further
        && !matches!(flow_type, api::WebhookFlow::ReturnResponse)
    {
        let object_ref_id = connector
            .get_webhook_object_reference_id(&request_details)
            .switch()
            .attach_printable("Could not find object reference id in incoming webhook body")?;
        let connectors_with_source_verification_call = &state.conf.webhook_source_verification_call;

        let source_verified = if connectors_with_source_verification_call
            .connectors_with_webhook_source_verification_call
            .contains(&connector_enum)
        {
            verify_webhook_source_verification_call(
                connector.clone(),
                &state,
                &platform,
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
                    platform.get_processor().get_account().get_id(),
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
                router_env::metric_attributes!((
                    MERCHANT_ID,
                    platform.get_processor().get_account().get_id().clone()
                )),
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
                        platform,
                        profile,
                        webhook_details,
                        source_verified,
                    ))
                    .await
                    .attach_printable("Incoming webhook flow for payments failed")?,

                    api::WebhookFlow::Refund => todo!(),

                    api::WebhookFlow::Dispute => Box::pin(disputes_incoming_webhook_flow(
                        state.clone(),
                        platform,
                        profile,
                        webhook_details,
                        source_verified,
                        &connector,
                        &request_details,
                        event_type,
                        &connector_name,
                    ))
                    .await
                    .attach_printable("Incoming webhook flow for disputes failed")?,

                    api::WebhookFlow::BankTransfer => todo!(),

                    api::WebhookFlow::ReturnResponse => WebhookResponseTracker::NoEffect,

                    api::WebhookFlow::Mandate => todo!(),

                    api::WebhookFlow::ExternalAuthentication => todo!(),
                    api::WebhookFlow::FraudCheck => todo!(),
                    api::WebhookFlow::Setup => WebhookResponseTracker::NoEffect,

                    #[cfg(feature = "payouts")]
                    api::WebhookFlow::Payout => todo!(),

                    api::WebhookFlow::Subscription => todo!(),
                    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
                    api::WebhookFlow::Recovery => {
                        Box::pin(recovery_incoming::recovery_incoming_webhook_flow(
                            state.clone(),
                            platform,
                            profile,
                            source_verified,
                            &connector,
                            merchant_connector_account.clone(),
                            &connector_name,
                            &request_details,
                            event_type,
                            req_state,
                            &object_ref_id,
                        ))
                        .await
                        .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                        .attach_printable("Failed to process recovery incoming webhook")?
                    }
                }
            }
        }
    } else {
        metrics::WEBHOOK_INCOMING_FILTERED_COUNT.add(
            1,
            router_env::metric_attributes!((
                MERCHANT_ID,
                platform.get_processor().get_account().get_id().clone()
            )),
        );
        WebhookResponseTracker::NoEffect
    };

    let response = connector
        .get_webhook_api_response(
            &request_details,
            None,
            Some(merchant_connector_account.connector_account_details.clone()),
        )
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
    platform: domain::Platform,
    profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let consume_or_trigger_flow = if source_verified {
        payments::CallConnectorAction::HandleResponse {
            resource_object: webhook_details.resource_object,
            event_type: None,
        }
    } else {
        payments::CallConnectorAction::Trigger
    };
    let key_manager_state = &(&state).into();
    let (payments_response, created_by) = match webhook_details.object_reference_id {
        webhooks::ObjectReferenceId::PaymentId(id) => {
            let get_trackers_response = get_trackers_response_for_payment_get_operation(
                state.store.as_ref(),
                &id,
                profile.get_id(),
                key_manager_state,
                platform.get_processor().get_key_store(),
                platform.get_processor().get_account().storage_scheme,
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
                .perform_locking_action(
                    &state,
                    platform.get_processor().get_account().get_id().to_owned(),
                )
                .await?;

            let (
                payment_data,
                _req,
                customer,
                connector_http_status_code,
                external_latency,
                connector_response_data,
            ) = Box::pin(payments::payments_operation_core::<
                api::PSync,
                _,
                _,
                _,
                PaymentStatusData<api::PSync>,
            >(
                &state,
                req_state,
                platform.clone(),
                &profile,
                payments::operations::PaymentGet,
                api::PaymentsRetrieveRequest {
                    force_sync: true,
                    expand_attempts: false,
                    param: None,
                    return_raw_connector_response: None,
                    merchant_connector_details: None,
                },
                get_trackers_response,
                consume_or_trigger_flow,
                HeaderPayload::default(),
            ))
            .await?;

            let created_by = payment_data.payment_attempt.created_by.clone();

            let response = payment_data.generate_response(
                &state,
                connector_http_status_code,
                external_latency,
                None,
                &platform,
                &profile,
                Some(connector_response_data),
            );

            lock_action
                .free_lock_action(
                    &state,
                    platform.get_processor().get_account().get_id().to_owned(),
                )
                .await?;

            match response {
                Ok(value) => (value, created_by),
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
                            platform.get_processor().get_account().get_id().clone()
                        )),
                    );
                    return Ok(WebhookResponseTracker::NoEffect);
                }
                Err(error) => Err(error)?,
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

            let event_type: Option<enums::EventType> = payments_response.status.into();

            // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
            if let Some(outgoing_event_type) = event_type {
                let primary_object_created_at = payments_response.created;
                let webhook_recipient = utils::resolve_webhook_recipient_from_created_by(
                    &state,
                    &platform,
                    &profile,
                    created_by.as_ref(),
                )
                .await?;
                Box::pin(create_event_and_trigger_outgoing_webhook(
                    state,
                    platform,
                    outgoing_event_type,
                    enums::EventClass::Payments,
                    payment_id.get_string_repr().to_owned(),
                    enums::EventObjectType::PaymentDetails,
                    api::OutgoingWebhookContent::PaymentDetails(Box::new(payments_response)),
                    primary_object_created_at,
                    webhook_recipient,
                ))
                .await?;
            };

            let response = WebhookResponseTracker::Payment { payment_id, status };

            Ok(response)
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received non-json response from payments core")?,
    }
}

/// Pull the connector transaction id out of a dispute webhook's object reference.
///
/// v2 can only look a payment attempt up by connector transaction id, so every other variant is
/// rejected here rather than silently mishandled.
fn connector_transaction_id_from_object_reference_id(
    object_reference_id: &webhooks::ObjectReferenceId,
) -> CustomResult<&str, errors::ApiErrorResponse> {
    match object_reference_id {
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::ConnectorTransactionId(id)) => {
            Ok(id.as_str())
        }
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure).attach_printable(
            "received an unsupported object reference id for retrieving payment attempt",
        ),
    }
}

/// Resolve the payment attempt that a dispute webhook refers to.
///
/// The lookup is keyed on the profile resolved by the dispatcher, not on the merchant id.
async fn get_payment_attempt_from_object_reference_id(
    state: &SessionState,
    object_reference_id: &webhooks::ObjectReferenceId,
    platform: &domain::Platform,
    profile: &domain::Profile,
) -> CustomResult<PaymentAttempt, errors::ApiErrorResponse> {
    let connector_transaction_id =
        connector_transaction_id_from_object_reference_id(object_reference_id)?;
    state
        .store
        .find_payment_attempt_by_profile_id_connector_transaction_id(
            platform.get_processor().get_key_store(),
            profile.get_id(),
            connector_transaction_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
}

#[allow(clippy::too_many_arguments)]
async fn get_or_update_dispute_object(
    state: &SessionState,
    option_dispute: Option<diesel_models::dispute::Dispute>,
    dispute_details: api::disputes::DisputePayload,
    platform: &domain::Platform,
    payment_attempt: &PaymentAttempt,
    dispute_status: common_enums::DisputeStatus,
    business_profile: &domain::Profile,
    connector_name: &str,
) -> CustomResult<diesel_models::dispute::Dispute, errors::ApiErrorResponse> {
    let db = &*state.store;
    match option_dispute {
        None => {
            metrics::INCOMING_DISPUTE_WEBHOOK_NEW_RECORD_METRIC.add(1, &[]);
            let dispute_id = generate_id(consts::ID_LENGTH, "dp");
            let new_dispute = diesel_models::dispute::DisputeNew {
                dispute_id,
                amount: dispute_details.amount.clone(),
                currency: dispute_details.currency.to_string(),
                dispute_stage: dispute_details.dispute_stage,
                dispute_status,
                payment_id: payment_attempt.payment_id.to_owned(),
                attempt_id: payment_attempt.get_id().to_owned(),
                merchant_id: platform.get_provider().get_account().get_id().to_owned(),
                connector_status: dispute_details.connector_status,
                connector_dispute_id: dispute_details.connector_dispute_id,
                connector_reason: dispute_details.connector_reason,
                connector_reason_code: dispute_details.connector_reason_code,
                challenge_required_by: dispute_details.challenge_required_by,
                connector_created_at: dispute_details.created_at,
                connector_updated_at: dispute_details.updated_at,
                connector: connector_name.to_owned(),
                evidence: Secret::new(serde_json::json!({})),
                profile_id: Some(business_profile.get_id().to_owned()),
                merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
                dispute_amount: StringMinorUnitForConnector::convert_back(
                    &StringMinorUnitForConnector,
                    dispute_details.amount,
                    dispute_details.currency,
                )
                .change_context(errors::ApiErrorResponse::AmountConversionFailed {
                    amount_type: "MinorUnit",
                })?,
                organization_id: platform
                    .get_processor()
                    .get_account()
                    .organization_id
                    .clone(),
                dispute_currency: Some(dispute_details.currency),
                processor_merchant_id: Some(
                    platform.get_processor().get_account().get_id().to_owned(),
                ),
                created_by: payment_attempt
                    .created_by
                    .as_ref()
                    .map(|created_by| created_by.to_string()),
                created_at: common_utils::date_time::now(),
                modified_at: common_utils::date_time::now(),
            };
            db.insert_dispute(
                new_dispute,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        }
        Some(dispute) => {
            logger::info!("Dispute Already exists, Updating the dispute details");
            metrics::INCOMING_DISPUTE_WEBHOOK_UPDATE_RECORD_METRIC.add(1, &[]);
            core_utils::validate_dispute_stage_and_dispute_status(
                dispute.dispute_stage,
                dispute.dispute_status,
                dispute_details.dispute_stage,
                dispute_status,
            )
            .change_context(errors::ApiErrorResponse::WebhookBadRequest)
            .attach_printable("dispute stage and status validation failed")?;
            let update_dispute = diesel_models::dispute::DisputeUpdate::Update {
                dispute_stage: dispute_details.dispute_stage,
                dispute_status,
                connector_status: dispute_details.connector_status,
                connector_reason: dispute_details.connector_reason,
                connector_reason_code: dispute_details.connector_reason_code,
                challenge_required_by: dispute_details.challenge_required_by,
                connector_updated_at: dispute_details.updated_at,
            };
            db.update_dispute(
                dispute,
                update_dispute,
                platform.get_processor().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        }
    }
}

#[instrument(skip_all)]
async fn disputes_incoming_webhook_flow(
    state: SessionState,
    platform: domain::Platform,
    business_profile: domain::Profile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector: &ConnectorEnum,
    request_details: &IncomingWebhookRequestDetails<'_>,
    event_type: webhooks::IncomingWebhookEvent,
    connector_name: &str,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    metrics::INCOMING_DISPUTE_WEBHOOK_METRIC.add(1, &[]);
    if !source_verified {
        metrics::INCOMING_DISPUTE_WEBHOOK_SIGNATURE_FAILURE_METRIC.add(1, &[]);
        return Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ));
    }
    let db = &*state.store;
    let payment_attempt = get_payment_attempt_from_object_reference_id(
        &state,
        &webhook_details.object_reference_id,
        &platform,
        &business_profile,
    )
    .await?;
    let resource_data = WebhookResourceData::Payment {
        payment_attempt: payment_attempt.clone(),
    };
    let dispute_details = connector
        .get_dispute_details(request_details, Some(&WebhookContext::from(&resource_data)))
        .switch()?;

    let option_dispute = db
        .find_by_processor_merchant_id_payment_id_connector_dispute_id(
            platform.get_processor().get_account().get_id(),
            &payment_attempt.payment_id,
            &dispute_details.connector_dispute_id,
            platform.get_processor().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)?;

    let dispute_status = common_enums::DisputeStatus::foreign_try_from(event_type)
        .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
        .attach_printable("event type to dispute status mapping failed")?;

    let dispute_object = get_or_update_dispute_object(
        &state,
        option_dispute,
        dispute_details,
        &platform,
        &payment_attempt,
        dispute_status,
        &business_profile,
        connector_name,
    )
    .await?;

    Ok(WebhookResponseTracker::Dispute {
        dispute_id: dispute_object.dispute_id,
        payment_id: dispute_object.payment_id,
        status: dispute_object.dispute_status,
    })
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
                .find_payment_intent_by_id(id, merchant_key_store, storage_scheme)
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let payment_attempt = db
                .find_payment_attempt_by_id(
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
                    merchant_key_store,
                    profile_id,
                    id,
                    storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let payment_intent = db
                .find_payment_intent_by_id(
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
                .find_payment_attempt_by_id(merchant_key_store, &global_attempt_id, storage_scheme)
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            let payment_intent = db
                .find_payment_intent_by_id(
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
            payment_attempt,
            attempts: None,
            should_sync_with_connector: true,
            payment_address,
            merchant_connector_details: None,
        },
    })
}

#[inline]
async fn verify_webhook_source_verification_call(
    connector: ConnectorEnum,
    state: &SessionState,
    platform: &domain::Platform,
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
            platform.get_processor().get_account().get_id(),
            connector_name,
            merchant_connector_account.connector_webhook_details.clone(),
        )
        .await
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

    let router_data = construct_webhook_router_data(
        state,
        connector_name,
        merchant_connector_account,
        platform,
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
) -> CustomResult<
    (
        domain::MerchantConnectorAccount,
        ConnectorEnum,
        common_enums::connector_enums::Connector,
        String,
    ),
    errors::ApiErrorResponse,
> {
    let db = &state.store;
    let mca = db
        .find_merchant_connector_account_by_id(connector_id, key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: connector_id.get_string_repr().to_owned(),
        })
        .attach_printable("error while fetching merchant_connector_account from connector_id")?;

    let connector_enum = mca.connector_name;
    let (connector, connector_name) =
        get_connector_by_connector_name(state, &connector_enum.to_string(), Some(mca.get_id()))?;

    Ok((mca, connector, connector_enum, connector_name))
}

#[cfg(test)]
mod dispute_webhook_tests {
    use super::*;

    #[test]
    fn connector_transaction_id_is_extracted() {
        let object_reference_id = api::ObjectReferenceId::PaymentId(
            api::PaymentIdType::ConnectorTransactionId("txn_1234".to_string()),
        );

        assert_eq!(
            connector_transaction_id_from_object_reference_id(&object_reference_id).unwrap(),
            "txn_1234"
        );
    }

    #[test]
    fn unsupported_object_reference_is_rejected() {
        let unsupported = [
            api::ObjectReferenceId::PaymentId(api::PaymentIdType::PaymentAttemptId(
                "attempt_1234".to_string(),
            )),
            api::ObjectReferenceId::PaymentId(api::PaymentIdType::PreprocessingId(
                "pre_1234".to_string(),
            )),
            api::ObjectReferenceId::RefundId(webhooks::RefundIdType::ConnectorRefundId(
                "ref_1234".to_string(),
            )),
        ];

        for object_reference_id in unsupported {
            assert!(connector_transaction_id_from_object_reference_id(&object_reference_id).is_err());
        }
    }

    #[test]
    fn every_dispute_event_maps_to_a_status() {
        let expected = [
            (
                webhooks::IncomingWebhookEvent::DisputeOpened,
                common_enums::DisputeStatus::DisputeOpened,
            ),
            (
                webhooks::IncomingWebhookEvent::DisputeExpired,
                common_enums::DisputeStatus::DisputeExpired,
            ),
            (
                webhooks::IncomingWebhookEvent::DisputeAccepted,
                common_enums::DisputeStatus::DisputeAccepted,
            ),
            (
                webhooks::IncomingWebhookEvent::DisputeCancelled,
                common_enums::DisputeStatus::DisputeCancelled,
            ),
            (
                webhooks::IncomingWebhookEvent::DisputeChallenged,
                common_enums::DisputeStatus::DisputeChallenged,
            ),
            (
                webhooks::IncomingWebhookEvent::DisputeWon,
                common_enums::DisputeStatus::DisputeWon,
            ),
            (
                webhooks::IncomingWebhookEvent::DisputeLost,
                common_enums::DisputeStatus::DisputeLost,
            ),
        ];

        for (event, status) in expected {
            // Every event routed to the dispute flow must have a status, otherwise the flow
            // rejects a webhook the dispatcher already committed to handling.
            assert_eq!(api::WebhookFlow::from(event), api::WebhookFlow::Dispute);
            assert_eq!(
                common_enums::DisputeStatus::foreign_try_from(event).unwrap(),
                status
            );
        }
    }
}
