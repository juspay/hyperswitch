pub mod transformers;
pub mod utils;

use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use router_env::{instrument, tracing};

use super::metrics;
use crate::{
    consts,
    core::{
        errors::{self, CustomResult, RouterResponse},
        payments, refunds,
    },
    db::StorageInterface,
    logger,
    routes::AppState,
    services,
    types::{
        api,
        domain::merchant_account,
        storage::{self, enums},
        transformers::{ForeignInto, ForeignTryInto},
    },
    utils::{generate_id, Encode, OptionExt, ValueExt},
};

const OUTGOING_WEBHOOK_TIMEOUT_MS: u64 = 5000;

#[instrument(skip_all)]
async fn payments_incoming_webhook_flow<W: api::OutgoingWebhookType>(
    state: AppState,
    merchant_account: merchant_account::MerchantAccount,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let consume_or_trigger_flow = if source_verified {
        payments::CallConnectorAction::HandleResponse(webhook_details.resource_object)
    } else {
        payments::CallConnectorAction::Trigger
    };

    let payments_response = match webhook_details.object_reference_id {
        api_models::webhooks::ObjectReferenceId::PaymentId(id) => {
            payments::payments_core::<api::PSync, api::PaymentsResponse, _, _, _>(
                &state,
                merchant_account.clone(),
                payments::operations::PaymentStatus,
                api::PaymentsRetrieveRequest {
                    resource_id: id,
                    merchant_id: Some(merchant_account.merchant_id.clone()),
                    force_sync: true,
                    connector: None,
                    param: None,
                    merchant_connector_details: None,
                },
                services::AuthFlow::Merchant,
                consume_or_trigger_flow,
            )
            .await
            .change_context(errors::WebhooksFlowError::PaymentsCoreFailed)?
        }
        _ => Err(errors::WebhooksFlowError::PaymentsCoreFailed).into_report()?,
    };

    match payments_response {
        services::ApplicationResponse::Json(payments_response) => {
            let payment_id = payments_response
                .payment_id
                .clone()
                .get_required_value("payment_id")
                .change_context(errors::WebhooksFlowError::PaymentsCoreFailed)?;

            let event_type: enums::EventType = payments_response
                .status
                .foreign_try_into()
                .into_report()
                .change_context(errors::WebhooksFlowError::PaymentsCoreFailed)?;

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

        _ => Err(errors::WebhooksFlowError::PaymentsCoreFailed).into_report()?,
    }

    Ok(())
}

#[instrument(skip_all)]
async fn refunds_incoming_webhook_flow<W: api::OutgoingWebhookType>(
    state: AppState,
    merchant_account: merchant_account::MerchantAccount,
    webhook_details: api::IncomingWebhookDetails,
    connector_name: &str,
    source_verified: bool,
    event_type: api_models::webhooks::IncomingWebhookEvent,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let db = &*state.store;
    //find refund by connector refund id
    let refund = match webhook_details.object_reference_id {
        api_models::webhooks::ObjectReferenceId::RefundId(
            api_models::webhooks::RefundIdType::ConnectorRefundId(id),
        ) => db
            .find_refund_by_merchant_id_connector_refund_id_connector(
                &merchant_account.merchant_id,
                &id,
                connector_name,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::WebhooksFlowError::ResourceNotFound)
            .attach_printable_lazy(|| "Failed fetching the refund")?,
        api_models::webhooks::ObjectReferenceId::RefundId(
            api_models::webhooks::RefundIdType::RefundId(id),
        ) => db
            .find_refund_by_merchant_id_refund_id(
                &merchant_account.merchant_id,
                &id,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::WebhooksFlowError::ResourceNotFound)
            .attach_printable_lazy(|| "Failed fetching the refund")?,
        _ => Err(errors::WebhooksFlowError::RefundsCoreFailed).into_report()?,
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
                .change_context(errors::WebhooksFlowError::RefundsCoreFailed)?,
        };
        state
            .store
            .update_refund(
                refund.to_owned(),
                refund_update,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::WebhooksFlowError::RefundsCoreFailed)
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
            api_models::refunds::RefundsRetrieveRequest {
                refund_id: refund_id.to_owned(),
                merchant_connector_details: None,
            },
        )
        .await
        .change_context(errors::WebhooksFlowError::RefundsCoreFailed)
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
        .change_context(errors::WebhooksFlowError::RefundsCoreFailed)?;
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

async fn get_payment_attempt_from_object_reference_id(
    state: AppState,
    object_reference_id: api_models::webhooks::ObjectReferenceId,
    merchant_account: &merchant_account::MerchantAccount,
) -> CustomResult<storage_models::payment_attempt::PaymentAttempt, errors::WebhooksFlowError> {
    let db = &*state.store;
    match object_reference_id {
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::ConnectorTransactionId(ref id)) => db
            .find_payment_attempt_by_merchant_id_connector_txn_id(
                &merchant_account.merchant_id,
                id,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::WebhooksFlowError::ResourceNotFound),
        api::ObjectReferenceId::PaymentId(api::PaymentIdType::PaymentAttemptId(ref id)) => db
            .find_payment_attempt_by_attempt_id_merchant_id(
                id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::WebhooksFlowError::ResourceNotFound),
        _ => Err(errors::WebhooksFlowError::ResourceNotFound).into_report(),
    }
}

async fn get_or_update_dispute_object(
    state: AppState,
    option_dispute: Option<storage_models::dispute::Dispute>,
    dispute_details: api::disputes::DisputePayload,
    merchant_id: &str,
    payment_attempt: &storage_models::payment_attempt::PaymentAttempt,
    event_type: api_models::webhooks::IncomingWebhookEvent,
    connector_name: &str,
) -> CustomResult<storage_models::dispute::Dispute, errors::WebhooksFlowError> {
    let db = &*state.store;
    match option_dispute {
        None => {
            metrics::INCOMING_DISPUTE_WEBHOOK_NEW_RECORD_METRIC.add(&metrics::CONTEXT, 1, &[]);
            let dispute_id = generate_id(consts::ID_LENGTH, "dp");
            let new_dispute = storage_models::dispute::DisputeNew {
                dispute_id,
                amount: dispute_details.amount,
                currency: dispute_details.currency,
                dispute_stage: dispute_details.dispute_stage.foreign_into(),
                dispute_status: event_type
                    .foreign_try_into()
                    .into_report()
                    .change_context(errors::WebhooksFlowError::DisputeCoreFailed)?,
                payment_id: payment_attempt.payment_id.to_owned(),
                connector: connector_name.to_owned(),
                attempt_id: payment_attempt.attempt_id.to_owned(),
                merchant_id: merchant_id.to_owned(),
                connector_status: dispute_details.connector_status,
                connector_dispute_id: dispute_details.connector_dispute_id,
                connector_reason: dispute_details.connector_reason,
                connector_reason_code: dispute_details.connector_reason_code,
                challenge_required_by: dispute_details.challenge_required_by,
                dispute_created_at: dispute_details.created_at,
                updated_at: dispute_details.updated_at,
            };
            state
                .store
                .insert_dispute(new_dispute.clone())
                .await
                .change_context(errors::WebhooksFlowError::WebhookEventCreationFailed)
        }
        Some(dispute) => {
            logger::info!("Dispute Already exists, Updating the dispute details");
            metrics::INCOMING_DISPUTE_WEBHOOK_UPDATE_RECORD_METRIC.add(&metrics::CONTEXT, 1, &[]);
            let dispute_status: storage_models::enums::DisputeStatus = event_type
                .foreign_try_into()
                .into_report()
                .change_context(errors::WebhooksFlowError::DisputeCoreFailed)?;
            crate::core::utils::validate_dispute_stage_and_dispute_status(
                dispute.dispute_stage.foreign_into(),
                dispute.dispute_status.foreign_into(),
                dispute_details.dispute_stage.clone(),
                dispute_status.foreign_into(),
            )?;
            let update_dispute = storage_models::dispute::DisputeUpdate::Update {
                dispute_stage: dispute_details.dispute_stage.foreign_into(),
                dispute_status,
                connector_status: dispute_details.connector_status,
                connector_reason: dispute_details.connector_reason,
                connector_reason_code: dispute_details.connector_reason_code,
                challenge_required_by: dispute_details.challenge_required_by,
                updated_at: dispute_details.updated_at,
            };
            db.update_dispute(dispute, update_dispute)
                .await
                .change_context(errors::WebhooksFlowError::ResourceNotFound)
        }
    }
}

#[instrument(skip_all)]
async fn disputes_incoming_webhook_flow<W: api::OutgoingWebhookType>(
    state: AppState,
    merchant_account: merchant_account::MerchantAccount,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector: &(dyn api::Connector + Sync),
    request_details: &api::IncomingWebhookRequestDetails<'_>,
    event_type: api_models::webhooks::IncomingWebhookEvent,
) -> CustomResult<(), errors::WebhooksFlowError> {
    metrics::INCOMING_DISPUTE_WEBHOOK_METRIC.add(&metrics::CONTEXT, 1, &[]);
    if source_verified {
        let db = &*state.store;
        let dispute_details = connector
            .get_dispute_details(request_details)
            .change_context(errors::WebhooksFlowError::WebhookEventObjectCreationFailed)?;
        let payment_attempt = get_payment_attempt_from_object_reference_id(
            state.clone(),
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
            .change_context(errors::WebhooksFlowError::ResourceNotFound)?;
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
            .change_context(errors::WebhooksFlowError::DisputeCoreFailed)?;
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
        Err(errors::WebhooksFlowError::WebhookSourceVerificationFailed).into_report()
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn create_event_and_trigger_outgoing_webhook<W: api::OutgoingWebhookType>(
    state: AppState,
    merchant_account: merchant_account::MerchantAccount,
    event_type: enums::EventType,
    event_class: enums::EventClass,
    intent_reference_id: Option<String>,
    primary_object_id: String,
    primary_object_type: enums::EventObjectType,
    content: api::OutgoingWebhookContent,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let new_event = storage::EventNew {
        event_id: generate_id(consts::ID_LENGTH, "evt"),
        event_type,
        event_class,
        is_webhook_notified: false,
        intent_reference_id,
        primary_object_id,
        primary_object_type,
    };

    let event = state
        .store
        .insert_event(new_event)
        .await
        .change_context(errors::WebhooksFlowError::WebhookEventCreationFailed)?;

    if state.conf.webhooks.outgoing_enabled {
        let arbiter = actix::Arbiter::try_current()
            .ok_or(errors::WebhooksFlowError::ForkFlowFailed)
            .into_report()?;

        let outgoing_webhook = api::OutgoingWebhook {
            merchant_id: merchant_account.merchant_id.clone(),
            event_id: event.event_id,
            event_type: event.event_type.foreign_into(),
            content,
            timestamp: event.created_at,
        };

        arbiter.spawn(async move {
            let result =
                trigger_webhook_to_merchant::<W>(merchant_account, outgoing_webhook, state.store)
                    .await;

            if let Err(e) = result {
                logger::error!(?e);
            }
        });
    }

    Ok(())
}

async fn trigger_webhook_to_merchant<W: api::OutgoingWebhookType>(
    merchant_account: merchant_account::MerchantAccount,
    webhook: api::OutgoingWebhook,
    db: Box<dyn StorageInterface>,
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

    let response = reqwest::Client::new()
        .post(&webhook_url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&transformed_outgoing_webhook)
        .timeout(core::time::Duration::from_millis(
            OUTGOING_WEBHOOK_TIMEOUT_MS,
        ))
        .send()
        .await;

    match response {
        Err(e) => {
            // [#217]: Schedule webhook for retry.
            Err(e)
                .into_report()
                .change_context(errors::WebhooksFlowError::CallToMerchantFailed)?;
        }
        Ok(res) => {
            if res.status().is_success() {
                let update_event = storage::EventUpdate::UpdateWebhookNotified {
                    is_webhook_notified: Some(true),
                };
                db.update_event(outgoing_webhook_event_id, update_event)
                    .await
                    .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)?;
            } else {
                // [#217]: Schedule webhook for retry.
                Err(errors::WebhooksFlowError::NotReceivedByMerchant).into_report()?;
            }
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn webhooks_core<W: api::OutgoingWebhookType>(
    state: &AppState,
    req: &actix_web::HttpRequest,
    merchant_account: merchant_account::MerchantAccount,
    connector_name: &str,
    body: actix_web::web::Bytes,
) -> RouterResponse<serde_json::Value> {
    let connector = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("There was an error in incoming webhook body decoding")?;

    request_details.body = &decoded_body;

    let event_type = connector
        .get_webhook_event_type(&request_details)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not find event type in incoming webhook body")?;

    let process_webhook_further = utils::lookup_webhook_event(
        &*state.store,
        connector_name,
        &merchant_account.merchant_id,
        &event_type,
    )
    .await;

    logger::info!(process_webhook=?process_webhook_further);
    logger::info!(event_type=?event_type);

    if process_webhook_further {
        let source_verified = connector
            .verify_webhook_source(
                &*state.store,
                &request_details,
                &merchant_account.merchant_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("There was an issue in incoming webhook source verification")?;

        let object_ref_id = connector
            .get_webhook_object_reference_id(&request_details)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not find object reference id in incoming webhook body")?;

        let event_object = connector
            .get_webhook_resource_object(&request_details)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not find resource object in incoming webhook body")?;

        let webhook_details = api::IncomingWebhookDetails {
            object_reference_id: object_ref_id,
            resource_object: Encode::<serde_json::Value>::encode_to_vec(&event_object)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "There was an issue when encoding the incoming webhook body to bytes",
                )?,
        };

        let flow_type: api::WebhookFlow = event_type.to_owned().into();
        match flow_type {
            api::WebhookFlow::Payment => payments_incoming_webhook_flow::<W>(
                state.clone(),
                merchant_account,
                webhook_details,
                source_verified,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Incoming webhook flow for payments failed")?,

            api::WebhookFlow::Refund => refunds_incoming_webhook_flow::<W>(
                state.clone(),
                merchant_account,
                webhook_details,
                connector_name,
                source_verified,
                event_type,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
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
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Incoming webhook flow for disputes failed")?,

            api::WebhookFlow::ReturnResponse => {}

            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .into_report()
                .attach_printable("Unsupported Flow Type received in incoming webhooks")?,
        }
    }

    let response = connector
        .get_webhook_api_response(&request_details)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not get incoming webhook api response from connector")?;

    Ok(response)
}
