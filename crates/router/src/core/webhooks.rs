pub mod transformers;
pub mod utils;

use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use router_env::{instrument, tracing};

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
        storage::{self, enums},
        transformers::{ForeignInto, ForeignTryInto},
    },
    utils::{generate_id, Encode, OptionExt, ValueExt},
};

const OUTGOING_WEBHOOK_TIMEOUT_MS: u64 = 5000;

#[instrument(skip_all)]
async fn payments_incoming_webhook_flow<W: api::OutgoingWebhookType>(
    state: AppState,
    merchant_account: storage::MerchantAccount,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let consume_or_trigger_flow = if source_verified {
        payments::CallConnectorAction::HandleResponse(webhook_details.resource_object)
    } else {
        payments::CallConnectorAction::Trigger
    };

    let payments_response = payments::payments_core::<api::PSync, api::PaymentsResponse, _, _, _>(
        &state,
        merchant_account.clone(),
        payments::operations::PaymentStatus,
        api::PaymentsRetrieveRequest {
            resource_id: api::PaymentIdType::ConnectorTransactionId(
                webhook_details.object_reference_id,
            ),
            merchant_id: Some(merchant_account.merchant_id.clone()),
            force_sync: true,
            connector: None,
            param: None,
        },
        services::AuthFlow::Merchant,
        consume_or_trigger_flow,
    )
    .await
    .change_context(errors::WebhooksFlowError::PaymentsCoreFailed)?;

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
    merchant_account: storage::MerchantAccount,
    webhook_details: api::IncomingWebhookDetails,
    connector_name: &str,
    source_verified: bool,
    event_type: api_models::webhooks::IncomingWebhookEvent,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let db = &*state.store;
    //find refund by connector refund id
    let refund = db
        .find_refund_by_merchant_id_connector_refund_id_connector(
            &merchant_account.merchant_id,
            &webhook_details.object_reference_id,
            connector_name,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::WebhooksFlowError::ResourceNotFound)
        .attach_printable_lazy(|| "Failed fetching the refund")?;
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
        refunds::refund_retrieve_core(&state, merchant_account.clone(), refund_id.to_owned())
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

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn create_event_and_trigger_outgoing_webhook<W: api::OutgoingWebhookType>(
    state: AppState,
    merchant_account: storage::MerchantAccount,
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
    merchant_account: storage::MerchantAccount,
    webhook: api::OutgoingWebhook,
    _db: Box<dyn StorageInterface>,
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
            if !res.status().is_success() {
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
    merchant_account: storage::MerchantAccount,
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

    if !matches!(
        event_type,
        api_models::webhooks::IncomingWebhookEvent::EndpointVerification
    ) {
        let source_verified = connector
            .verify_webhook_source(
                &*state.store,
                &request_details,
                &merchant_account.merchant_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("There was an issue in incoming webhook source verification")?;

        let process_webhook_further = utils::lookup_webhook_event(
            &*state.store,
            connector_name,
            &merchant_account.merchant_id,
            &event_type,
        )
        .await;

        if process_webhook_further {
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

                api::WebhookFlow::ReturnResponse => {}

                _ => Err(errors::ApiErrorResponse::InternalServerError)
                    .into_report()
                    .attach_printable("Unsupported Flow Type received in incoming webhooks")?,
            }
        }
    }

    let response = connector
        .get_webhook_api_response(&request_details)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not get incoming webhook api response from connector")?;

    Ok(response)
}
