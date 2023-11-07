pub mod types;
pub mod utils;

use std::str::FromStr;

use api_models::{
    payments::HeaderPayload,
    webhooks::{self, WebhookResponseTracker},
};
use common_utils::errors::ReportSwitchExt;
use error_stack::{report, IntoReport, ResultExt};
use masking::ExposeInterface;
use router_env::{instrument, tracing};

use super::{errors::StorageErrorExt, metrics};
#[cfg(feature = "stripe")]
use crate::compatibility::stripe::webhooks as stripe_webhooks;
use crate::{
    consts,
    core::{
        api_locking,
        errors::{self, ConnectorErrorExt, CustomResult, RouterResponse},
        payment_methods::PaymentMethodRetrieve,
        payments, refunds,
    },
    db::StorageInterface,
    logger,
    routes::{lock_utils, metrics::request::add_attributes, AppState},
    services,
    types::{
        self as router_types,
        api::{self, mandates::MandateResponseExt},
        domain,
        storage::{self, enums},
        transformers::{ForeignInto, ForeignTryInto},
    },
    utils::{self as helper_utils, generate_id, Encode, OptionExt, ValueExt},
};

const OUTGOING_WEBHOOK_TIMEOUT_SECS: u64 = 5;
const MERCHANT_ID: &str = "merchant_id";

pub async fn payments_incoming_webhook_flow<
    W: types::OutgoingWebhookType,
    Ctx: PaymentMethodRetrieve,
>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    key_store: domain::MerchantKeyStore,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    let consume_or_trigger_flow = if source_verified {
        payments::CallConnectorAction::HandleResponse(webhook_details.resource_object)
    } else {
        payments::CallConnectorAction::Trigger
    };
    let payments_response = match webhook_details.object_reference_id {
        api_models::webhooks::ObjectReferenceId::PaymentId(id) => {
            let payment_id = get_payment_id(
                state.store.as_ref(),
                &id,
                merchant_account.merchant_id.as_str(),
                merchant_account.storage_scheme,
            )
            .await?;

            let lock_action = api_locking::LockAction::Hold {
                input: super::api_locking::LockingInput {
                    unique_locking_key: payment_id,
                    api_identifier: lock_utils::ApiIdentifier::Payments,
                    override_lock_retries: None,
                },
            };

            lock_action
                .clone()
                .perform_locking_action(&state, merchant_account.merchant_id.to_string())
                .await?;

            let response =
                payments::payments_core::<api::PSync, api::PaymentsResponse, _, _, _, Ctx>(
                    state.clone(),
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
                        expand_captures: None,
                    },
                    services::AuthFlow::Merchant,
                    consume_or_trigger_flow,
                    None,
                    HeaderPayload::default(),
                )
                .await;

            lock_action
                .free_lock_action(&state, merchant_account.merchant_id.to_owned())
                .await?;

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
                        &[add_attributes(
                            "merchant_id",
                            merchant_account.merchant_id.clone(),
                        )],
                    );
                    return Ok(WebhookResponseTracker::NoEffect);
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
        services::ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
            let payment_id = payments_response
                .payment_id
                .clone()
                .get_required_value("payment_id")
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("payment id not received from payments core")?;

            let status = payments_response.status;

            let event_type: Option<enums::EventType> = payments_response.status.foreign_into();

            // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
            if let Some(outgoing_event_type) = event_type {
                create_event_and_trigger_outgoing_webhook::<W>(
                    state,
                    merchant_account,
                    business_profile,
                    outgoing_event_type,
                    enums::EventClass::Payments,
                    None,
                    payment_id.clone(),
                    enums::EventObjectType::PaymentDetails,
                    api::OutgoingWebhookContent::PaymentDetails(payments_response),
                )
                .await?;
            };

            let response = WebhookResponseTracker::Payment { payment_id, status };

            Ok(response)
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .into_report()
            .attach_printable("received non-json response from payments core")?,
    }
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn refunds_incoming_webhook_flow<W: types::OutgoingWebhookType>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    key_store: domain::MerchantKeyStore,
    webhook_details: api::IncomingWebhookDetails,
    connector_name: &str,
    source_verified: bool,
    event_type: api_models::webhooks::IncomingWebhookEvent,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
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
            updated_by: merchant_account.storage_scheme.to_string(),
        };
        db.update_refund(
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
            state.clone(),
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
    let event_type: Option<enums::EventType> = updated_refund.refund_status.foreign_into();

    // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
    if let Some(outgoing_event_type) = event_type {
        let refund_response: api_models::refunds::RefundResponse =
            updated_refund.clone().foreign_into();
        create_event_and_trigger_outgoing_webhook::<W>(
            state,
            merchant_account,
            business_profile,
            outgoing_event_type,
            enums::EventClass::Refunds,
            None,
            refund_id,
            enums::EventObjectType::RefundDetails,
            api::OutgoingWebhookContent::RefundDetails(refund_response),
        )
        .await?;
    }

    Ok(WebhookResponseTracker::Refund {
        payment_id: updated_refund.payment_id,
        refund_id: updated_refund.refund_id,
        status: updated_refund.refund_status,
    })
}

pub async fn get_payment_attempt_from_object_reference_id(
    state: &AppState,
    object_reference_id: api_models::webhooks::ObjectReferenceId,
    merchant_account: &domain::MerchantAccount,
) -> CustomResult<data_models::payments::payment_attempt::PaymentAttempt, errors::ApiErrorResponse>
{
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
    payment_attempt: &data_models::payments::payment_attempt::PaymentAttempt,
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
                profile_id: None,
                evidence: None,
                merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
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

pub async fn mandates_incoming_webhook_flow<W: types::OutgoingWebhookType>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    event_type: api_models::webhooks::IncomingWebhookEvent,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
    if source_verified {
        let db = &*state.store;
        let mandate = match webhook_details.object_reference_id {
            webhooks::ObjectReferenceId::MandateId(webhooks::MandateIdType::MandateId(
                mandate_id,
            )) => db
                .find_mandate_by_merchant_id_mandate_id(
                    &merchant_account.merchant_id,
                    mandate_id.as_str(),
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?,
            webhooks::ObjectReferenceId::MandateId(
                webhooks::MandateIdType::ConnectorMandateId(connector_mandate_id),
            ) => db
                .find_mandate_by_merchant_id_connector_mandate_id(
                    &merchant_account.merchant_id,
                    connector_mandate_id.as_str(),
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?,
            _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
                .into_report()
                .attach_printable("received a non-mandate id for retrieving mandate")?,
        };
        let mandate_status = event_type
            .foreign_try_into()
            .into_report()
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("event type to mandate status mapping failed")?;
        let updated_mandate = db
            .update_mandate_by_merchant_id_mandate_id(
                &merchant_account.merchant_id,
                &mandate.mandate_id,
                storage::MandateUpdate::StatusUpdate { mandate_status },
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?;
        let mandates_response = Box::new(
            api::mandates::MandateResponse::from_db_mandate(&state, updated_mandate.clone())
                .await?,
        );
        let event_type: Option<enums::EventType> = updated_mandate.mandate_status.foreign_into();
        if let Some(outgoing_event_type) = event_type {
            create_event_and_trigger_outgoing_webhook::<W>(
                state,
                merchant_account,
                business_profile,
                outgoing_event_type,
                enums::EventClass::Mandates,
                None,
                updated_mandate.mandate_id.clone(),
                enums::EventObjectType::MandateDetails,
                api::OutgoingWebhookContent::MandateDetails(mandates_response),
            )
            .await?;
        }
        Ok(WebhookResponseTracker::Mandate {
            mandate_id: updated_mandate.mandate_id,
            status: updated_mandate.mandate_status,
        })
    } else {
        logger::error!("Webhook source verification failed for mandates webhook flow");
        Err(errors::ApiErrorResponse::WebhookAuthenticationFailed).into_report()
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn disputes_incoming_webhook_flow<W: types::OutgoingWebhookType>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
    connector: &(dyn api::Connector + Sync),
    request_details: &api::IncomingWebhookRequestDetails<'_>,
    event_type: api_models::webhooks::IncomingWebhookEvent,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
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
            event_type,
            connector.id(),
        )
        .await?;
        let disputes_response = Box::new(dispute_object.clone().foreign_into());
        let event_type: enums::EventType = dispute_object.dispute_status.foreign_into();

        create_event_and_trigger_outgoing_webhook::<W>(
            state,
            merchant_account,
            business_profile,
            event_type,
            enums::EventClass::Disputes,
            None,
            dispute_object.dispute_id.clone(),
            enums::EventObjectType::DisputeDetails,
            api::OutgoingWebhookContent::DisputeDetails(disputes_response),
        )
        .await?;
        metrics::INCOMING_DISPUTE_WEBHOOK_MERCHANT_NOTIFIED_METRIC.add(&metrics::CONTEXT, 1, &[]);
        Ok(WebhookResponseTracker::Dispute {
            dispute_id: dispute_object.dispute_id,
            payment_id: dispute_object.payment_id,
            status: dispute_object.dispute_status,
        })
    } else {
        metrics::INCOMING_DISPUTE_WEBHOOK_SIGNATURE_FAILURE_METRIC.add(&metrics::CONTEXT, 1, &[]);
        Err(errors::ApiErrorResponse::WebhookAuthenticationFailed).into_report()
    }
}

async fn bank_transfer_webhook_flow<W: types::OutgoingWebhookType, Ctx: PaymentMethodRetrieve>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    key_store: domain::MerchantKeyStore,
    webhook_details: api::IncomingWebhookDetails,
    source_verified: bool,
) -> CustomResult<WebhookResponseTracker, errors::ApiErrorResponse> {
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
        payments::payments_core::<api::Authorize, api::PaymentsResponse, _, _, _, Ctx>(
            state.clone(),
            merchant_account.to_owned(),
            key_store,
            payments::PaymentConfirm,
            request,
            services::api::AuthFlow::Merchant,
            payments::CallConnectorAction::Trigger,
            None,
            HeaderPayload::default(),
        )
        .await
    } else {
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    };

    match response? {
        services::ApplicationResponse::JsonWithHeaders((payments_response, _)) => {
            let payment_id = payments_response
                .payment_id
                .clone()
                .get_required_value("payment_id")
                .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                .attach_printable("did not receive payment id from payments core response")?;

            let event_type: Option<enums::EventType> = payments_response.status.foreign_into();
            let status = payments_response.status;

            // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
            if let Some(outgoing_event_type) = event_type {
                create_event_and_trigger_outgoing_webhook::<W>(
                    state,
                    merchant_account,
                    business_profile,
                    outgoing_event_type,
                    enums::EventClass::Payments,
                    None,
                    payment_id.clone(),
                    enums::EventObjectType::PaymentDetails,
                    api::OutgoingWebhookContent::PaymentDetails(payments_response),
                )
                .await?;
            }

            Ok(WebhookResponseTracker::Payment { payment_id, status })
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .into_report()
            .attach_printable("received non-json response from payments core")?,
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn create_event_and_trigger_appropriate_outgoing_webhook(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    event_type: enums::EventType,
    event_class: enums::EventClass,
    intent_reference_id: Option<String>,
    primary_object_id: String,
    primary_object_type: enums::EventObjectType,
    content: api::OutgoingWebhookContent,
) -> CustomResult<(), errors::ApiErrorResponse> {
    match merchant_account.get_compatible_connector() {
        #[cfg(feature = "stripe")]
        Some(api_models::enums::Connector::Stripe) => {
            create_event_and_trigger_outgoing_webhook::<stripe_webhooks::StripeOutgoingWebhook>(
                state.clone(),
                merchant_account,
                business_profile,
                event_type,
                event_class,
                intent_reference_id,
                primary_object_id,
                primary_object_type,
                content,
            )
            .await
        }
        _ => {
            create_event_and_trigger_outgoing_webhook::<api_models::webhooks::OutgoingWebhook>(
                state.clone(),
                merchant_account,
                business_profile,
                event_type,
                event_class,
                intent_reference_id,
                primary_object_id,
                primary_object_type,
                content,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn create_event_and_trigger_outgoing_webhook<W: types::OutgoingWebhookType>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
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
        let outgoing_webhook = api::OutgoingWebhook {
            merchant_id: merchant_account.merchant_id.clone(),
            event_id: event.event_id,
            event_type: event.event_type,
            content,
            timestamp: event.created_at,
        };

        // Using a tokio spawn here and not arbiter because not all caller of this function
        // may have an actix arbiter
        tokio::spawn(async move {
            let result =
                trigger_webhook_to_merchant::<W>(business_profile, outgoing_webhook, &state).await;

            if let Err(e) = result {
                logger::error!(?e);
            }
        });
    }

    Ok(())
}

pub async fn trigger_webhook_to_merchant<W: types::OutgoingWebhookType>(
    business_profile: diesel_models::business_profile::BusinessProfile,
    webhook: api::OutgoingWebhook,
    state: &AppState,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let webhook_details_json = business_profile
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
        .get_outgoing_webhooks_signature(business_profile.payment_response_hash_key.clone())?;

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

    let response = state
        .api_client
        .send_request(state, request, Some(OUTGOING_WEBHOOK_TIMEOUT_SECS), false)
        .await;

    metrics::WEBHOOK_OUTGOING_COUNT.add(
        &metrics::CONTEXT,
        1,
        &[metrics::KeyValue::new(
            MERCHANT_ID,
            business_profile.merchant_id.clone(),
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
                        business_profile.merchant_id.clone(),
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
                        business_profile.merchant_id.clone(),
                    )],
                );
                // [#217]: Schedule webhook for retry.
                Err(errors::WebhooksFlowError::NotReceivedByMerchant).into_report()?;
            }
        }
    }

    Ok(())
}

pub async fn webhooks_wrapper<W: types::OutgoingWebhookType, Ctx: PaymentMethodRetrieve>(
    state: AppState,
    req: &actix_web::HttpRequest,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    connector_name_or_mca_id: &str,
    body: actix_web::web::Bytes,
) -> RouterResponse<serde_json::Value> {
    let (application_response, _webhooks_response_tracker) = webhooks_core::<W, Ctx>(
        state,
        req,
        merchant_account,
        key_store,
        connector_name_or_mca_id,
        body,
    )
    .await?;

    Ok(application_response)
}

#[instrument(skip_all)]

pub async fn webhooks_core<W: types::OutgoingWebhookType, Ctx: PaymentMethodRetrieve>(
    state: AppState,
    req: &actix_web::HttpRequest,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    connector_name_or_mca_id: &str,
    body: actix_web::web::Bytes,
) -> errors::RouterResult<(
    services::ApplicationResponse<serde_json::Value>,
    WebhookResponseTracker,
)> {
    metrics::WEBHOOK_INCOMING_COUNT.add(
        &metrics::CONTEXT,
        1,
        &[metrics::KeyValue::new(
            MERCHANT_ID,
            merchant_account.merchant_id.clone(),
        )],
    );
    let mut request_details = api::IncomingWebhookRequestDetails {
        method: req.method().clone(),
        uri: req.uri().clone(),
        headers: req.headers(),
        query_params: req.query_string().to_string(),
        body: &body,
    };

    // Fetch the merchant connector account to get the webhooks source secret
    // `webhooks source secret` is a secret shared between the merchant and connector
    // This is used for source verification and webhooks integrity
    let (merchant_connector_account, connector) = fetch_mca_and_connector(
        &state,
        &merchant_account,
        connector_name_or_mca_id,
        &key_store,
        &request_details,
    )
    .await?;

    let connector_name = merchant_connector_account.clone().connector_name;

    let connector = connector.connector;

    let decoded_body = connector
        .decode_webhook_body(
            &*state.clone().store,
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
                &metrics::CONTEXT,
                1,
                &[
                    metrics::KeyValue::new(MERCHANT_ID, merchant_account.merchant_id.clone()),
                    metrics::KeyValue::new("connector", connector_name.to_string()),
                ],
            );

            let response = connector
                .get_webhook_api_response(&request_details)
                .switch()
                .attach_printable("Failed while early return in case of event type parsing")?;

            return Ok((response, WebhookResponseTracker::NoEffect));
        }
    };
    logger::info!(event_type=?event_type);

    let is_webhook_event_supported = !matches!(
        event_type,
        api_models::webhooks::IncomingWebhookEvent::EventNotSupported
    );
    let is_webhook_event_enabled = !utils::is_webhook_event_disabled(
        &*state.clone().store,
        connector_name.as_str(),
        &merchant_account.merchant_id,
        &event_type,
    )
    .await;

    //process webhook further only if webhook event is enabled and is not event_not_supported
    let process_webhook_further = is_webhook_event_enabled && is_webhook_event_supported;

    logger::info!(process_webhook=?process_webhook_further);

    let flow_type: api::WebhookFlow = event_type.to_owned().into();
    let webhook_effect = if process_webhook_further
        && !matches!(flow_type, api::WebhookFlow::ReturnResponse)
    {
        let object_ref_id = connector
            .get_webhook_object_reference_id(&request_details)
            .switch()
            .attach_printable("Could not find object reference id in incoming webhook body")?;
        let connector_enum = api_models::enums::Connector::from_str(&connector_name)
            .into_report()
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
            connector
                .verify_webhook_source_verification_call(
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
                .verify_webhook_source(
                    &request_details,
                    &merchant_account,
                    merchant_connector_account.clone(),
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

        if source_verified {
            metrics::WEBHOOK_SOURCE_VERIFIED_COUNT.add(
                &metrics::CONTEXT,
                1,
                &[metrics::KeyValue::new(
                    MERCHANT_ID,
                    merchant_account.merchant_id.clone(),
                )],
            );
        } else if connector.is_webhook_source_verification_mandatory() {
            // if webhook consumption is mandatory for connector, fail webhook
            // so that merchant can retrigger it after updating merchant_secret
            return Err(errors::ApiErrorResponse::WebhookAuthenticationFailed.into());
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

        let profile_id = merchant_connector_account
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not find profile_id in merchant connector account")?;

        let business_profile = state
            .store
            .find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
            })?;

        match flow_type {
            api::WebhookFlow::Payment => payments_incoming_webhook_flow::<W, Ctx>(
                state.clone(),
                merchant_account,
                business_profile,
                key_store,
                webhook_details,
                source_verified,
            )
            .await
            .attach_printable("Incoming webhook flow for payments failed")?,

            api::WebhookFlow::Refund => refunds_incoming_webhook_flow::<W>(
                state.clone(),
                merchant_account,
                business_profile,
                key_store,
                webhook_details,
                connector_name.as_str(),
                source_verified,
                event_type,
            )
            .await
            .attach_printable("Incoming webhook flow for refunds failed")?,

            api::WebhookFlow::Dispute => disputes_incoming_webhook_flow::<W>(
                state.clone(),
                merchant_account,
                business_profile,
                webhook_details,
                source_verified,
                *connector,
                &request_details,
                event_type,
            )
            .await
            .attach_printable("Incoming webhook flow for disputes failed")?,

            api::WebhookFlow::BankTransfer => bank_transfer_webhook_flow::<W, Ctx>(
                state.clone(),
                merchant_account,
                business_profile,
                key_store,
                webhook_details,
                source_verified,
            )
            .await
            .attach_printable("Incoming bank-transfer webhook flow failed")?,

            api::WebhookFlow::ReturnResponse => WebhookResponseTracker::NoEffect,

            api::WebhookFlow::Mandate => mandates_incoming_webhook_flow::<W>(
                state.clone(),
                merchant_account,
                business_profile,
                webhook_details,
                source_verified,
                event_type,
            )
            .await
            .attach_printable("Incoming webhook flow for mandates failed")?,

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
        WebhookResponseTracker::NoEffect
    };

    let response = connector
        .get_webhook_api_response(&request_details)
        .switch()
        .attach_printable("Could not get incoming webhook api response from connector")?;

    Ok((response, webhook_effect))
}

#[inline]
pub async fn get_payment_id(
    db: &dyn StorageInterface,
    payment_id: &api::PaymentIdType,
    merchant_id: &str,
    storage_scheme: enums::MerchantStorageScheme,
) -> errors::RouterResult<String> {
    let pay_id = || async {
        match payment_id {
            api_models::payments::PaymentIdType::PaymentIntentId(ref id) => Ok(id.to_string()),
            api_models::payments::PaymentIdType::ConnectorTransactionId(ref id) => db
                .find_payment_attempt_by_merchant_id_connector_txn_id(
                    merchant_id,
                    id,
                    storage_scheme,
                )
                .await
                .map(|p| p.payment_id),
            api_models::payments::PaymentIdType::PaymentAttemptId(ref id) => db
                .find_payment_attempt_by_attempt_id_merchant_id(id, merchant_id, storage_scheme)
                .await
                .map(|p| p.payment_id),
            api_models::payments::PaymentIdType::PreprocessingId(ref id) => db
                .find_payment_attempt_by_preprocessing_id_merchant_id(
                    id,
                    merchant_id,
                    storage_scheme,
                )
                .await
                .map(|p| p.payment_id),
        }
    };

    pay_id()
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
}

async fn fetch_mca_and_connector(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    connector_name_or_mca_id: &str,
    key_store: &domain::MerchantKeyStore,
    request_details: &api::IncomingWebhookRequestDetails<'_>,
) -> CustomResult<(domain::MerchantConnectorAccount, api::ConnectorData), errors::ApiErrorResponse>
{
    let db = &state.store;
    if connector_name_or_mca_id.starts_with("mca_") {
        let mca = db
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                &merchant_account.merchant_id,
                connector_name_or_mca_id,
                key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: connector_name_or_mca_id.to_string(),
            })
            .attach_printable(
                "error while fetching merchant_connector_account from connector_id",
            )?;

        let connector = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &mca.connector_name,
            api::GetToken::Connector,
            Some(mca.merchant_connector_id.clone()),
        )
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "invalid connector name received".to_string(),
        })
        .attach_printable("Failed construction of ConnectorData")?;

        Ok((mca, connector))
    } else {
        // Merchant connector account is already being queried, it is safe to set connector id as None
        let connector = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name_or_mca_id,
            api::GetToken::Connector,
            None,
        )
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "invalid connector name received".to_string(),
        })
        .attach_printable("Failed construction of ConnectorData")?;

        let object_ref_id = connector
            .connector
            .get_webhook_object_reference_id(request_details)
            .switch()
            .attach_printable("Could not find object reference id in incoming webhook body")?;

        let profile_id = helper_utils::get_profile_id_using_object_reference_id(
            &*state.store,
            object_ref_id,
            merchant_account,
            connector_name_or_mca_id,
        )
        .await
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "object reference id",
        })
        .attach_printable("Could not find profile id from object reference id")?;

        let mca = db
            .find_merchant_connector_account_by_profile_id_connector_name(
                &profile_id,
                connector_name_or_mca_id,
                key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: format!(
                    "profile_id {profile_id} and connector name {connector_name_or_mca_id}"
                ),
            })
            .attach_printable("error while fetching merchant_connector_account from profile_id")?;

        Ok((mca, connector))
    }
}
