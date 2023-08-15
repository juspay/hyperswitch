pub mod types;
pub mod utils;

use common_utils::errors::ReportSwitchExt;
use error_stack::{report, IntoReport, ResultExt};
use masking::ExposeInterface;
use router_env::{instrument, tracing};

use super::{errors::StorageErrorExt, metrics};
use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, CustomResult, RouterResponse},
        payments, refunds,
    },
    logger,
    routes::{metrics::request::add_attributes, AppState},
    services,
    types::{
        self as router_types, api, domain,
        storage::{self, enums},
        transformers::{ForeignInto, ForeignTryInto},
    },
    utils::{generate_id, Encode, OptionExt, ValueExt},
};

const OUTGOING_WEBHOOK_TIMEOUT_SECS: u64 = 5;
const MERCHANT_ID: &str = "merchant_id";

#[instrument(skip_all)]
pub async fn payments_incoming_webhook_flow<W: types::OutgoingWebhookType>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let consume_or_trigger_flow = if source_verified {
        payments::CallConnectorAction::HandleResponse(webhook_details.resource_object)
    } else {
        payments::CallConnectorAction::Trigger
    };
    let payments_response = match webhook_details.object_reference_id {
        api_models::webhooks::ObjectReferenceId::PaymentId(id) => {
            let response = payments::payments_core::<api::PSync, api::PaymentsResponse, _, _, _>(
                &state,
                merchant_account.clone(),
                key_store,
                payments::operations::PaymentStatus,
                api::PaymentsRetrieveRequest {
                    resource_id: id,
                    merchant_id: Some(merchant_account.merchant_id.clone()),
                    force_sync: true,
                    connector: None,
                    param: None,
                    merchant_connector_details: None,
                    client_secret: None,
                    expand_attempts: None,
                },
                services::AuthFlow::Merchant,
                consume_or_trigger_flow,
            )
            .await;

            match response {
                Ok(value) => value,
                Err(err)
                    if matches!(
                        err.current_context(),
                        &errors::ApiErrorResponse::PaymentNotFound
                    ) && state
                        .conf
                        .webhooks
                        .ignore_error
                        .payment_not_found
                        .unwrap_or(true) =>
                {
                    metrics::WEBHOOK_PAYMENT_NOT_FOUND.add(
                        &metrics::CONTEXT,
                        1,
                        &[add_attributes("merchant_id", merchant_account.merchant_id)],
                    );
                    return Ok(());
                }
                error @ Err(_) => error?,
            }
        }
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .into_report()
            .attach_printable(
                "Did not get payment id as object reference id in webhook payments flow",
            )?,
    };

    match payments_response {
        services::ApplicationResponse::Json(payments_response) => {
            let payment_id = payments_response
                .payment_id
                .clone()
                .get_required_value("payment_id")
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("payment id not received from payments core")?;

            let event_type: enums::EventType = payments_response
                .status
                .foreign_try_into()
                .into_report()
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("payment event type mapping failed")?;

            create_event_and_trigger_outgoing_webhook::<W>(
                state,
                merchant_account,
                event_type,
                enums::EventClass::Payments,
                None,
                payment_id,
                enums::EventObjectType::PaymentDetails,
                api::OutgoingWebhookContent::PaymentDetails(payments_response),
            )
            .await?;
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .into_report()
            .attach_printable("received non-json response from payments core")?,
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn refunds_incoming_webhook_flow<W: types::OutgoingWebhookType>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    webhook_details: api::IncomingWebhookDetails,
    connector_name: &str,
    source_verified: bool,
    event_type: api_models::webhooks::IncomingWebhookEvent,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let db = &*state.store;
    //find refund by connector refund id
    let refund = match webhook_details.object_reference_id {
        api_models::webhooks::ObjectReferenceId::RefundId(refund_id_type) => match refund_id_type {
            api_models::webhooks::RefundIdType::RefundId(id) => db
                .find_refund_by_merchant_id_refund_id(
                    &merchant_account.merchant_id,
                    &id,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable_lazy(|| "Failed fetching the refund")?,
            api_models::webhooks::RefundIdType::ConnectorRefundId(id) => db
                .find_refund_by_merchant_id_connector_refund_id_connector(
                    &merchant_account.merchant_id,
                    &id,
                    connector_name,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable_lazy(|| "Failed fetching the refund")?,
        },
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .into_report()
            .attach_printable("received a non-refund id when processing refund webhooks")?,
    };
    let refund_id = refund.refund_id.to_owned();
    //if source verified then update refund status else trigger refund sync
    let updated_refund = if source_verified {
        let refund_update = storage::RefundUpdate::StatusUpdate {
            connector_refund_id: None,
            sent_to_gateway: true,
            refund_status: event_type
                .foreign_try_into()
                .into_report()
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("failed refund status mapping from event type")?,
        };
        state
            .store
            .update_refund(
                refund.to_owned(),
                refund_update,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
            .attach_printable_lazy(|| {
                format!(
                    "Failed while updating refund: refund_id: {}",
                    refund_id.to_owned()
                )
            })?
    } else {
        refunds::refund_retrieve_core(
            &state,
            merchant_account.clone(),
            key_store,
            api_models::refunds::RefundsRetrieveRequest {
                refund_id: refund_id.to_owned(),
                force_sync: Some(true),
                merchant_connector_details: None,
            },
        )
        .await
        .attach_printable_lazy(|| {
            format!(
                "Failed while updating refund: refund_id: {}",
                refund_id.to_owned()
            )
        })?
    };
    let event_type: enums::EventType = updated_refund
        .refund_status
        .foreign_try_into()
        .into_report()
        .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
        .attach_printable("refund status to event type mapping failed")?;
    let refund_response: api_models::refunds::RefundResponse = updated_refund.foreign_into();
    create_event_and_trigger_outgoing_webhook::<W>(
        state,
        merchant_account,
        event_type,
        enums::EventClass::Refunds,
        None,
        refund_id,
        enums::EventObjectType::RefundDetails,
        api::OutgoingWebhookContent::RefundDetails(refund_response),
    )
    .await?;
    Ok(())
}

pub async fn get_payment_attempt_from_object_reference_id(
    state: &AppState,
    object_reference_id: api_models::webhooks::ObjectReferenceId,
    merchant_account: &domain::MerchantAccount,
) -> CustomResult<diesel_models::payment_attempt::PaymentAttempt, errors::ApiErrorResponse> {
    let db = &*state.store;
    match object_reference_id {
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::ConnectorTransactionId(ref id)) => db
            .find_payment_attempt_by_merchant_id_connector_txn_id(
                &merchant_account.merchant_id,
                id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound),
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::PaymentAttemptId(ref id)) => db
            .find_payment_attempt_by_attempt_id_merchant_id(
                id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound),
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::PreprocessingId(ref id)) => db
            .find_payment_attempt_by_preprocessing_id_merchant_id(
                id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound),
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .into_report()
            .attach_printable("received a non-payment id for retrieving payment")?,
    }
}

pub async fn get_or_update_dispute_object(
    state: AppState,
    option_dispute: Option<diesel_models::dispute::Dispute>,
    dispute_details: api::disputes::DisputePayload,
    merchant_id: &str,
    payment_attempt: &diesel_models::payment_attempt::PaymentAttempt,
    event_type: api_models::webhooks::IncomingWebhookEvent,
    connector_name: &str,
) -> CustomResult<diesel_models::dispute::Dispute, errors::ApiErrorResponse> {
    let db = &*state.store;
    match option_dispute {
        None => {
            metrics::INCOMING_DISPUTE_WEBHOOK_NEW_RECORD_METRIC.add(&metrics::CONTEXT, 1, &[]);
            let dispute_id = generate_id(consts::ID_LENGTH, "dp");
            let new_dispute = diesel_models::dispute::DisputeNew {
                dispute_id,
                amount: dispute_details.amount,
                currency: dispute_details.currency,
                dispute_stage: dispute_details.dispute_stage,
                dispute_status: event_type
                    .foreign_try_into()
                    .into_report()
                    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("event type to dispute status mapping failed")?,
                payment_id: payment_attempt.payment_id.to_owned(),
                connector: connector_name.to_owned(),
                attempt_id: payment_attempt.attempt_id.to_owned(),
                merchant_id: merchant_id.to_owned(),
                connector_status: dispute_details.connector_status,
                connector_dispute_id: dispute_details.connector_dispute_id,
                connector_reason: dispute_details.connector_reason,
                connector_reason_code: dispute_details.connector_reason_code,
                challenge_required_by: dispute_details.challenge_required_by,
                connector_created_at: dispute_details.created_at,
                connector_updated_at: dispute_details.updated_at,
                evidence: None,
            };
            state
                .store
                .insert_dispute(new_dispute.clone())
                .await
                .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        }
        Some(dispute) => {
            logger::info!("Dispute Already exists, Updating the dispute details");
            metrics::INCOMING_DISPUTE_WEBHOOK_UPDATE_RECORD_METRIC.add(&metrics::CONTEXT, 1, &[]);
            let dispute_status: diesel_models::enums::DisputeStatus = event_type
                .foreign_try_into()
                .into_report()
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("event type to dispute state conversion failure")?;
            crate::core::utils::validate_dispute_stage_and_dispute_status(
                dispute.dispute_stage,
                dispute.dispute_status,
                dispute_details.dispute_stage,
                dispute_status,
            )
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
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
            db.update_dispute(dispute, update_dispute)
                .await
                .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)
        }
    }
}

#[instrument(skip_all)]
pub async fn disputes_incoming_webhook_flow<W: types::OutgoingWebhookType>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector: &(dyn api::Connector + Sync),
    request_details: &api::IncomingWebhookRequestDetails<'_>,
    event_type: api_models::webhooks::IncomingWebhookEvent,
) -> CustomResult<(), errors::ApiErrorResponse> {
    metrics::INCOMING_DISPUTE_WEBHOOK_METRIC.add(&metrics::CONTEXT, 1, &[]);
    if source_verified {
        let db = &*state.store;
        let dispute_details = connector.get_dispute_details(request_details).switch()?;
        let payment_attempt = get_payment_attempt_from_object_reference_id(
            &state,
            webhook_details.object_reference_id,
            &merchant_account,
        )
        .await?;
        let option_dispute = db
            .find_by_merchant_id_payment_id_connector_dispute_id(
                &merchant_account.merchant_id,
                &payment_attempt.payment_id,
                &dispute_details.connector_dispute_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::WebhookResourceNotFound)?;
        let dispute_object = get_or_update_dispute_object(
            state.clone(),
            option_dispute,
            dispute_details,
            &merchant_account.merchant_id,
            &payment_attempt,
            event_type.clone(),
            connector.id(),
        )
        .await?;
        let disputes_response = Box::new(dispute_object.clone().foreign_into());
        let event_type: enums::EventType = dispute_object
            .dispute_status
            .foreign_try_into()
            .into_report()
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("failed to map dispute status to event type")?;
        create_event_and_trigger_outgoing_webhook::<W>(
            state,
            merchant_account,
            event_type,
            enums::EventClass::Disputes,
            None,
            dispute_object.dispute_id,
            enums::EventObjectType::DisputeDetails,
            api::OutgoingWebhookContent::DisputeDetails(disputes_response),
        )
        .await?;
        metrics::INCOMING_DISPUTE_WEBHOOK_MERCHANT_NOTIFIED_METRIC.add(&metrics::CONTEXT, 1, &[]);
        Ok(())
    } else {
        metrics::INCOMING_DISPUTE_WEBHOOK_SIGNATURE_FAILURE_METRIC.add(&metrics::CONTEXT, 1, &[]);
        Err(errors::ApiErrorResponse::WebhookAuthenticationFailed).into_report()
    }
}

async fn bank_transfer_webhook_flow<W: types::OutgoingWebhookType>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let response = if source_verified {
        let payment_attempt = get_payment_attempt_from_object_reference_id(
            &state,
            webhook_details.object_reference_id,
            &merchant_account,
        )
        .await?;
        let payment_id = payment_attempt.payment_id;
        let request = api::PaymentsRequest {
            payment_id: Some(api_models::payments::PaymentIdType::PaymentIntentId(
                payment_id,
            )),
            payment_token: payment_attempt.payment_token,
            ..Default::default()
        };
        payments::payments_core::<api::Authorize, api::PaymentsResponse, _, _, _>(
            &state,
            merchant_account.to_owned(),
            key_store,
            payments::PaymentConfirm,
            request,
            services::api::AuthFlow::Merchant,
            payments::CallConnectorAction::Trigger,
        )
        .await
    } else {
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    };

    match response? {
        services::ApplicationResponse::Json(payments_response) => {
            let payment_id = payments_response
                .payment_id
                .clone()
                .get_required_value("payment_id")
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("did not receive payment id from payments core response")?;

            let event_type: enums::EventType = payments_response
                .status
                .foreign_try_into()
                .into_report()
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("error mapping payments response status to event type")?;

            create_event_and_trigger_outgoing_webhook::<W>(
                state,
                merchant_account,
                event_type,
                enums::EventClass::Payments,
                None,
                payment_id,
                enums::EventObjectType::PaymentDetails,
                api::OutgoingWebhookContent::PaymentDetails(payments_response),
            )
            .await?;
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .into_report()
            .attach_printable("received non-json response from payments core")?,
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn create_event_and_trigger_outgoing_webhook<W: types::OutgoingWebhookType>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    event_type: enums::EventType,
    event_class: enums::EventClass,
    intent_reference_id: Option<String>,
    primary_object_id: String,
    primary_object_type: enums::EventObjectType,
    content: api::OutgoingWebhookContent,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let event_id = format!("{primary_object_id}_{}", event_type);
    let new_event = storage::EventNew {
        event_id: event_id.clone(),
        event_type,
        event_class,
        is_webhook_notified: false,
        intent_reference_id,
        primary_object_id,
        primary_object_type,
    };

    let event_insert_result = state.store.insert_event(new_event).await;

    let event = match event_insert_result {
        Ok(event) => Ok(event),
        Err(error) => {
            if error.current_context().is_db_unique_violation() {
                logger::info!("Merchant already notified about the event {event_id}");
                return Ok(());
            } else {
                logger::error!(event_insertion_failure=?error);
                Err(error
                    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("Failed to insert event in events table"))
            }
        }
    }?;

    if state.conf.webhooks.outgoing_enabled {
        let arbiter = actix::Arbiter::try_current()
            .ok_or(errors::ApiErrorResponse::WebhookProcessingFailure)
            .into_report()
            .attach_printable("arbiter retrieval failure")?;

        let outgoing_webhook = api::OutgoingWebhook {
            merchant_id: merchant_account.merchant_id.clone(),
            event_id: event.event_id,
            event_type: event.event_type,
            content,
            timestamp: event.created_at,
        };

        arbiter.spawn(async move {
            let result =
                trigger_webhook_to_merchant::<W>(merchant_account, outgoing_webhook, &state).await;

            if let Err(e) = result {
                logger::error!(?e);
            }
        });
    }

    Ok(())
}

pub async fn trigger_webhook_to_merchant<W: types::OutgoingWebhookType>(
    merchant_account: domain::MerchantAccount,
    webhook: api::OutgoingWebhook,
    state: &AppState,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let webhook_details_json = merchant_account
        .webhook_details
        .get_required_value("webhook_details")
        .change_context(errors::WebhooksFlowError::MerchantWebhookDetailsNotFound)?;

    let webhook_details: api::WebhookDetails =
        webhook_details_json
            .parse_value("WebhookDetails")
            .change_context(errors::WebhooksFlowError::MerchantWebhookDetailsNotFound)?;

    let webhook_url = webhook_details
        .webhook_url
        .get_required_value("webhook_url")
        .change_context(errors::WebhooksFlowError::MerchantWebhookURLNotConfigured)
        .map(ExposeInterface::expose)?;

    let outgoing_webhook_event_id = webhook.event_id.clone();

    let transformed_outgoing_webhook = W::from(webhook);

    let outgoing_webhooks_signature = transformed_outgoing_webhook
        .get_outgoing_webhooks_signature(merchant_account.payment_response_hash_key.clone())?;

    let transformed_outgoing_webhook_string = router_types::RequestBody::log_and_get_request_body(
        &transformed_outgoing_webhook,
        Encode::<serde_json::Value>::encode_to_string_of_json,
    )
    .change_context(errors::WebhooksFlowError::OutgoingWebhookEncodingFailed)
    .attach_printable("There was an issue when encoding the outgoing webhook body")?;

    let mut header = vec![(
        reqwest::header::CONTENT_TYPE.to_string(),
        "application/json".into(),
    )];

    if let Some(signature) = outgoing_webhooks_signature {
        W::add_webhook_header(&mut header, signature)
    }

    let request = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(&webhook_url)
        .attach_default_headers()
        .headers(header)
        .body(Some(transformed_outgoing_webhook_string))
        .build();

    let response =
        services::api::send_request(state, request, Some(OUTGOING_WEBHOOK_TIMEOUT_SECS)).await;

    metrics::WEBHOOK_OUTGOING_COUNT.add(
        &metrics::CONTEXT,
        1,
        &[metrics::KeyValue::new(
            MERCHANT_ID,
            merchant_account.merchant_id.clone(),
        )],
    );
    logger::debug!(outgoing_webhook_response=?response);

    match response {
        Err(e) => {
            // [#217]: Schedule webhook for retry.
            Err(e).change_context(errors::WebhooksFlowError::CallToMerchantFailed)?;
        }
        Ok(res) => {
            if res.status().is_success() {
                metrics::WEBHOOK_OUTGOING_RECEIVED_COUNT.add(
                    &metrics::CONTEXT,
                    1,
                    &[metrics::KeyValue::new(
                        MERCHANT_ID,
                        merchant_account.merchant_id.clone(),
                    )],
                );
                let update_event = storage::EventUpdate::UpdateWebhookNotified {
                    is_webhook_notified: Some(true),
                };
                state
                    .store
                    .update_event(outgoing_webhook_event_id, update_event)
                    .await
                    .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)?;
            } else {
                metrics::WEBHOOK_OUTGOING_NOT_RECEIVED_COUNT.add(
                    &metrics::CONTEXT,
                    1,
                    &[metrics::KeyValue::new(
                        MERCHANT_ID,
                        merchant_account.merchant_id.clone(),
                    )],
                );
                // [#217]: Schedule webhook for retry.
                Err(errors::WebhooksFlowError::NotReceivedByMerchant).into_report()?;
            }
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn webhooks_core<W: types::OutgoingWebhookType>(
    state: &AppState,
    req: &actix_web::HttpRequest,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    connector_name: &str,
    body: actix_web::web::Bytes,
) -> RouterResponse<serde_json::Value> {
    metrics::WEBHOOK_INCOMING_COUNT.add(
        &metrics::CONTEXT,
        1,
        &[metrics::KeyValue::new(
            MERCHANT_ID,
            merchant_account.merchant_id.clone(),
        )],
    );

    let connector = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InvalidRequestData {
        message: "invalid connnector name received".to_string(),
    })
    .attach_printable("Failed construction of ConnectorData")?;

    let connector = connector.connector;
    let mut request_details = api::IncomingWebhookRequestDetails {
        method: req.method().clone(),
        headers: req.headers(),
        query_params: req.query_string().to_string(),
        body: &body,
    };

    let decoded_body = connector
        .decode_webhook_body(
            &*state.store,
            &request_details,
            &merchant_account.merchant_id,
        )
        .await
        .switch()
        .attach_printable("There was an error in incoming webhook body decoding")?;

    request_details.body = &decoded_body;

    let event_type = match connector
        .get_webhook_event_type(&request_details)
        .allow_webhook_event_type_not_found(
            state.conf.webhooks.ignore_error.event_type.unwrap_or(true),
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
                &metrics::CONTEXT,
                1,
                &[
                    metrics::KeyValue::new(MERCHANT_ID, merchant_account.merchant_id.clone()),
                    metrics::KeyValue::new("connector", connector_name.to_string()),
                ],
            );

            return connector
                .get_webhook_api_response(&request_details)
                .switch()
                .attach_printable("Failed while early return in case of event type parsing");
        }
    };

    let process_webhook_further = utils::lookup_webhook_event(
        &*state.store,
        connector_name,
        &merchant_account.merchant_id,
        &event_type,
    )
    .await;

    logger::info!(process_webhook=?process_webhook_further);
    logger::info!(event_type=?event_type);

    let flow_type: api::WebhookFlow = event_type.to_owned().into();
    if process_webhook_further && !matches!(flow_type, api::WebhookFlow::ReturnResponse) {
        let object_ref_id = connector
            .get_webhook_object_reference_id(&request_details)
            .switch()
            .attach_printable("Could not find object reference id in incoming webhook body")?;

        let source_verified = connector
            .verify_webhook_source(
                &*state.store,
                &request_details,
                &merchant_account,
                connector_name,
                &key_store,
                object_ref_id.clone(),
            )
            .await
            .switch()
            .attach_printable("There was an issue in incoming webhook source verification")?;

        if source_verified {
            metrics::WEBHOOK_SOURCE_VERIFIED_COUNT.add(
                &metrics::CONTEXT,
                1,
                &[metrics::KeyValue::new(
                    MERCHANT_ID,
                    merchant_account.merchant_id.clone(),
                )],
            );
        }

        logger::info!(source_verified=?source_verified);

        let event_object = connector
            .get_webhook_resource_object(&request_details)
            .switch()
            .attach_printable("Could not find resource object in incoming webhook body")?;

        let webhook_details = api::IncomingWebhookDetails {
            object_reference_id: object_ref_id,
            resource_object: Encode::<serde_json::Value>::encode_to_vec(&event_object)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "There was an issue when encoding the incoming webhook body to bytes",
                )?,
        };

        match flow_type {
            api::WebhookFlow::Payment => payments_incoming_webhook_flow::<W>(
                state.clone(),
                merchant_account,
                key_store,
                webhook_details,
                source_verified,
            )
            .await
            .attach_printable("Incoming webhook flow for payments failed")?,

            api::WebhookFlow::Refund => refunds_incoming_webhook_flow::<W>(
                state.clone(),
                merchant_account,
                key_store,
                webhook_details,
                connector_name,
                source_verified,
                event_type,
            )
            .await
            .attach_printable("Incoming webhook flow for refunds failed")?,

            api::WebhookFlow::Dispute => disputes_incoming_webhook_flow::<W>(
                state.clone(),
                merchant_account,
                webhook_details,
                source_verified,
                *connector,
                &request_details,
                event_type,
            )
            .await
            .attach_printable("Incoming webhook flow for disputes failed")?,

            api::WebhookFlow::BankTransfer => bank_transfer_webhook_flow::<W>(
                state.clone(),
                merchant_account,
                key_store,
                webhook_details,
                source_verified,
            )
            .await
            .attach_printable("Incoming bank-transfer webhook flow failed")?,

            api::WebhookFlow::ReturnResponse => {}

            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .into_report()
                .attach_printable("Unsupported Flow Type received in incoming webhooks")?,
        }
    } else {
        metrics::WEBHOOK_INCOMING_FILTERED_COUNT.add(
            &metrics::CONTEXT,
            1,
            &[metrics::KeyValue::new(
                MERCHANT_ID,
                merchant_account.merchant_id.clone(),
            )],
        );
    }

    let response = connector
        .get_webhook_api_response(&request_details)
        .switch()
        .attach_printable("Could not get incoming webhook api response from connector")?;

    Ok(response)
}
