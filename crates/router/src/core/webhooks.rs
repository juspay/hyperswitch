pub mod types;
pub mod utils;
#[cfg(feature = "olap")]
pub mod webhook_events;

use std::{str::FromStr, time::Instant};

use actix_web::FromRequest;
use api_models::{
    payments::HeaderPayload,
    webhook_events::{OutgoingWebhookRequestContent, OutgoingWebhookResponseContent},
    webhooks::{self, WebhookResponseTracker},
};
use common_utils::{
    errors::ReportSwitchExt, events::ApiEventsType, ext_traits::Encode, request::RequestContent,
};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Mask, PeekInterface, Secret};
use router_env::{
    instrument,
    tracing::{self, Instrument},
    tracing_actix_web::RequestId,
};

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
    events::{
        api_logs::ApiEvent,
        outgoing_webhook_logs::{OutgoingWebhookEvent, OutgoingWebhookEventMetric},
    },
    logger,
    routes::{app::AppStateInfo, lock_utils, metrics::request::add_attributes, AppState},
    services::{self, authentication as auth},
    types::{
        api::{self, mandates::MandateResponseExt},
        domain::{self, types as domain_types},
        storage::{self, enums},
        transformers::{ForeignInto, ForeignTryFrom},
    },
    utils::{self as helper_utils, generate_id, OptionExt, ValueExt},
    workflows::outgoing_webhook_retry,
};

const OUTGOING_WEBHOOK_TIMEOUT_SECS: u64 = 5;
const MERCHANT_ID: &str = "merchant_id";

pub async fn payments_incoming_webhook_flow<Ctx: PaymentMethodRetrieve>(
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

            let response = Box::pin(payments::payments_core::<
                api::PSync,
                api::PaymentsResponse,
                _,
                _,
                _,
                Ctx,
            >(
                state.clone(),
                merchant_account.clone(),
                key_store.clone(),
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
            ))
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
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure).attach_printable(
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
                let primary_object_created_at = payments_response.created;
                create_event_and_trigger_outgoing_webhook(
                    state,
                    merchant_account,
                    business_profile,
                    &key_store,
                    outgoing_event_type,
                    enums::EventClass::Payments,
                    payment_id.clone(),
                    enums::EventObjectType::PaymentDetails,
                    api::OutgoingWebhookContent::PaymentDetails(payments_response),
                    primary_object_created_at,
                )
                .await?;
            };

            let response = WebhookResponseTracker::Payment { payment_id, status };

            Ok(response)
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received non-json response from payments core")?,
    }
}

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn refunds_incoming_webhook_flow(
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
                .attach_printable("Failed to fetch the refund")?,
            api_models::webhooks::RefundIdType::ConnectorRefundId(id) => db
                .find_refund_by_merchant_id_connector_refund_id_connector(
                    &merchant_account.merchant_id,
                    &id,
                    connector_name,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::WebhookResourceNotFound)
                .attach_printable("Failed to fetch the refund")?,
        },
        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received a non-refund id when processing refund webhooks")?,
    };
    let refund_id = refund.refund_id.to_owned();
    //if source verified then update refund status else trigger refund sync
    let updated_refund = if source_verified {
        let refund_update = storage::RefundUpdate::StatusUpdate {
            connector_refund_id: None,
            sent_to_gateway: true,
            refund_status: common_enums::RefundStatus::foreign_try_from(event_type)
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
        .attach_printable_lazy(|| format!("Failed while updating refund: refund_id: {refund_id}"))?
    } else {
        Box::pin(refunds::refund_retrieve_core(
            state.clone(),
            merchant_account.clone(),
            key_store.clone(),
            api_models::refunds::RefundsRetrieveRequest {
                refund_id: refund_id.to_owned(),
                force_sync: Some(true),
                merchant_connector_details: None,
            },
        ))
        .await
        .attach_printable_lazy(|| format!("Failed while updating refund: refund_id: {refund_id}"))?
    };
    let event_type: Option<enums::EventType> = updated_refund.refund_status.foreign_into();

    // If event is NOT an UnsupportedEvent, trigger Outgoing Webhook
    if let Some(outgoing_event_type) = event_type {
        let refund_response: api_models::refunds::RefundResponse =
            updated_refund.clone().foreign_into();
        create_event_and_trigger_outgoing_webhook(
            state,
            merchant_account,
            business_profile,
            &key_store,
            outgoing_event_type,
            enums::EventClass::Refunds,
            refund_id,
            enums::EventObjectType::RefundDetails,
            api::OutgoingWebhookContent::RefundDetails(refund_response),
            Some(updated_refund.created_at),
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
            .attach_printable("received a non-payment id for retrieving payment")?,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn get_or_update_dispute_object(
    state: AppState,
    option_dispute: Option<diesel_models::dispute::Dispute>,
    dispute_details: api::disputes::DisputePayload,
    merchant_id: &str,
    payment_attempt: &data_models::payments::payment_attempt::PaymentAttempt,
    event_type: api_models::webhooks::IncomingWebhookEvent,
    business_profile: &diesel_models::business_profile::BusinessProfile,
    connector_name: &str,
) -> CustomResult<diesel_models::dispute::Dispute, errors::ApiErrorResponse> {
    let db = &*state.store;
    match option_dispute {
        None => {
            metrics::INCOMING_DISPUTE_WEBHOOK_NEW_RECORD_METRIC.add(&metrics::CONTEXT, 1, &[]);
            let dispute_id = generate_id(consts::ID_LENGTH, "dp");
            let new_dispute = diesel_models::dispute::DisputeNew {
                dispute_id,
                amount: dispute_details.amount.clone(),
                currency: dispute_details.currency,
                dispute_stage: dispute_details.dispute_stage,
                dispute_status: common_enums::DisputeStatus::foreign_try_from(event_type)
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
                profile_id: Some(business_profile.profile_id.clone()),
                evidence: None,
                merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
                dispute_amount: dispute_details.amount.parse::<i64>().unwrap_or(0),
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
            let dispute_status = diesel_models::enums::DisputeStatus::foreign_try_from(event_type)
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

pub async fn mandates_incoming_webhook_flow(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    key_store: domain::MerchantKeyStore,
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
                .attach_printable("received a non-mandate id for retrieving mandate")?,
        };
        let mandate_status = common_enums::MandateStatus::foreign_try_from(event_type)
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
            api::mandates::MandateResponse::from_db_mandate(
                &state,
                key_store.clone(),
                updated_mandate.clone(),
            )
            .await?,
        );
        let event_type: Option<enums::EventType> = updated_mandate.mandate_status.foreign_into();
        if let Some(outgoing_event_type) = event_type {
            create_event_and_trigger_outgoing_webhook(
                state,
                merchant_account,
                business_profile,
                &key_store,
                outgoing_event_type,
                enums::EventClass::Mandates,
                updated_mandate.mandate_id.clone(),
                enums::EventObjectType::MandateDetails,
                api::OutgoingWebhookContent::MandateDetails(mandates_response),
                Some(updated_mandate.created_at),
            )
            .await?;
        }
        Ok(WebhookResponseTracker::Mandate {
            mandate_id: updated_mandate.mandate_id,
            status: updated_mandate.mandate_status,
        })
    } else {
        logger::error!("Webhook source verification failed for mandates webhook flow");
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn disputes_incoming_webhook_flow(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    key_store: domain::MerchantKeyStore,
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
            &business_profile,
            connector.id(),
        )
        .await?;
        let disputes_response = Box::new(dispute_object.clone().foreign_into());
        let event_type: enums::EventType = dispute_object.dispute_status.foreign_into();

        create_event_and_trigger_outgoing_webhook(
            state,
            merchant_account,
            business_profile,
            &key_store,
            event_type,
            enums::EventClass::Disputes,
            dispute_object.dispute_id.clone(),
            enums::EventObjectType::DisputeDetails,
            api::OutgoingWebhookContent::DisputeDetails(disputes_response),
            Some(dispute_object.created_at),
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
        Err(report!(
            errors::ApiErrorResponse::WebhookAuthenticationFailed
        ))
    }
}

async fn bank_transfer_webhook_flow<Ctx: PaymentMethodRetrieve>(
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
        Box::pin(payments::payments_core::<
            api::Authorize,
            api::PaymentsResponse,
            _,
            _,
            _,
            Ctx,
        >(
            state.clone(),
            merchant_account.to_owned(),
            key_store.clone(),
            payments::PaymentConfirm,
            request,
            services::api::AuthFlow::Merchant,
            payments::CallConnectorAction::Trigger,
            None,
            HeaderPayload::with_source(common_enums::PaymentSource::Webhook),
        ))
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
                let primary_object_created_at = payments_response.created;
                create_event_and_trigger_outgoing_webhook(
                    state,
                    merchant_account,
                    business_profile,
                    &key_store,
                    outgoing_event_type,
                    enums::EventClass::Payments,
                    payment_id.clone(),
                    enums::EventObjectType::PaymentDetails,
                    api::OutgoingWebhookContent::PaymentDetails(payments_response),
                    primary_object_created_at,
                )
                .await?;
            }

            Ok(WebhookResponseTracker::Payment { payment_id, status })
        }

        _ => Err(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("received non-json response from payments core")?,
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub(crate) async fn create_event_and_trigger_outgoing_webhook(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    merchant_key_store: &domain::MerchantKeyStore,
    event_type: enums::EventType,
    event_class: enums::EventClass,
    primary_object_id: String,
    primary_object_type: enums::EventObjectType,
    content: api::OutgoingWebhookContent,
    primary_object_created_at: Option<time::PrimitiveDateTime>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let delivery_attempt = enums::WebhookDeliveryAttempt::InitialAttempt;
    let idempotent_event_id =
        utils::get_idempotent_event_id(&primary_object_id, event_type, delivery_attempt);
    let webhook_url_result = get_webhook_url_from_business_profile(&business_profile);

    if !state.conf.webhooks.outgoing_enabled
        || webhook_url_result.is_err()
        || webhook_url_result.as_ref().is_ok_and(String::is_empty)
    {
        logger::debug!(
            business_profile_id=%business_profile.profile_id,
            %idempotent_event_id,
            "Outgoing webhooks are disabled in application configuration, or merchant webhook URL \
             could not be obtained; skipping outgoing webhooks for event"
        );
        return Ok(());
    }

    let event_id = utils::generate_event_id();
    let merchant_id = business_profile.merchant_id.clone();
    let now = common_utils::date_time::now();

    let outgoing_webhook = api::OutgoingWebhook {
        merchant_id: merchant_id.clone(),
        event_id: event_id.clone(),
        event_type,
        content: content.clone(),
        timestamp: now,
    };

    let request_content = get_outgoing_webhook_request(
        &merchant_account,
        outgoing_webhook,
        business_profile.payment_response_hash_key.as_deref(),
    )
    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
    .attach_printable("Failed to construct outgoing webhook request content")?;

    let new_event = domain::Event {
        event_id: event_id.clone(),
        event_type,
        event_class,
        is_webhook_notified: false,
        primary_object_id,
        primary_object_type,
        created_at: now,
        merchant_id: Some(business_profile.merchant_id.clone()),
        business_profile_id: Some(business_profile.profile_id.clone()),
        primary_object_created_at,
        idempotent_event_id: Some(idempotent_event_id.clone()),
        initial_attempt_id: Some(event_id.clone()),
        request: Some(
            domain_types::encrypt(
                request_content
                    .encode_to_string_of_json()
                    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("Failed to encode outgoing webhook request content")
                    .map(Secret::new)?,
                merchant_key_store.key.get_inner().peek(),
            )
            .await
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .attach_printable("Failed to encrypt outgoing webhook request content")?,
        ),
        response: None,
        delivery_attempt: Some(delivery_attempt),
    };

    let event_insert_result = state
        .store
        .insert_event(new_event, merchant_key_store)
        .await;

    let event = match event_insert_result {
        Ok(event) => Ok(event),
        Err(error) => {
            if error.current_context().is_db_unique_violation() {
                logger::debug!("Event with idempotent ID `{idempotent_event_id}` already exists in the database");
                return Ok(());
            } else {
                logger::error!(event_insertion_failure=?error);
                Err(error
                    .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
                    .attach_printable("Failed to insert event in events table"))
            }
        }
    }?;

    let process_tracker = add_outgoing_webhook_retry_task_to_process_tracker(
        &*state.store,
        &business_profile,
        &event,
    )
    .await
    .map_err(|error| {
        logger::error!(
            ?error,
            "Failed to add outgoing webhook retry task to process tracker"
        );
        error
    })
    .ok();

    let cloned_key_store = merchant_key_store.clone();
    // Using a tokio spawn here and not arbiter because not all caller of this function
    // may have an actix arbiter
    tokio::spawn(
        async move {
            trigger_webhook_and_raise_event(
                state,
                business_profile,
                &cloned_key_store,
                event,
                request_content,
                delivery_attempt,
                Some(content),
                process_tracker,
            )
            .await;
        }
        .in_current_span(),
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub(crate) async fn trigger_webhook_and_raise_event(
    state: AppState,
    business_profile: diesel_models::business_profile::BusinessProfile,
    merchant_key_store: &domain::MerchantKeyStore,
    event: domain::Event,
    request_content: OutgoingWebhookRequestContent,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    content: Option<api::OutgoingWebhookContent>,
    process_tracker: Option<storage::ProcessTracker>,
) {
    logger::debug!(
        event_id=%event.event_id,
        idempotent_event_id=?event.idempotent_event_id,
        initial_attempt_id=?event.initial_attempt_id,
        "Attempting to send webhook"
    );

    let merchant_id = business_profile.merchant_id.clone();
    let trigger_webhook_result = trigger_webhook_to_merchant(
        state.clone(),
        business_profile,
        merchant_key_store,
        event.clone(),
        request_content,
        delivery_attempt,
        process_tracker,
    )
    .await;

    raise_webhooks_analytics_event(state, trigger_webhook_result, content, merchant_id, event);
}

async fn trigger_webhook_to_merchant(
    state: AppState,
    business_profile: diesel_models::business_profile::BusinessProfile,
    merchant_key_store: &domain::MerchantKeyStore,
    event: domain::Event,
    request_content: OutgoingWebhookRequestContent,
    delivery_attempt: enums::WebhookDeliveryAttempt,
    process_tracker: Option<storage::ProcessTracker>,
) -> CustomResult<(), errors::WebhooksFlowError> {
    let webhook_url = match (
        get_webhook_url_from_business_profile(&business_profile),
        process_tracker.clone(),
    ) {
        (Ok(webhook_url), _) => Ok(webhook_url),
        (Err(error), Some(process_tracker)) => {
            if !error
                .current_context()
                .is_webhook_delivery_retryable_error()
            {
                logger::debug!("Failed to obtain merchant webhook URL, aborting retries");
                state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(process_tracker, "FAILURE".into())
                    .await
                    .change_context(
                        errors::WebhooksFlowError::OutgoingWebhookProcessTrackerTaskUpdateFailed,
                    )?;
            }
            Err(error)
        }
        (Err(error), None) => Err(error),
    }?;

    let event_id = event.event_id;

    let headers = request_content
        .headers
        .into_iter()
        .map(|(name, value)| (name, value.into_masked()))
        .collect();
    let request = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(&webhook_url)
        .attach_default_headers()
        .headers(headers)
        .set_body(RequestContent::RawBytes(
            request_content.body.expose().into_bytes(),
        ))
        .build();

    let response = state
        .api_client
        .send_request(&state, request, Some(OUTGOING_WEBHOOK_TIMEOUT_SECS), false)
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

    let api_client_error_handler =
        |client_error: error_stack::Report<errors::ApiClientError>,
         delivery_attempt: enums::WebhookDeliveryAttempt| {
            let error =
                client_error.change_context(errors::WebhooksFlowError::CallToMerchantFailed);
            logger::error!(
                ?error,
                ?delivery_attempt,
                "An error occurred when sending webhook to merchant"
            );
        };
    let update_event_in_storage = |state: AppState,
                                   merchant_key_store: domain::MerchantKeyStore,
                                   merchant_id: String,
                                   event_id: String,
                                   response: reqwest::Response| async move {
        let status_code = response.status();
        let is_webhook_notified = status_code.is_success();

        let response_headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    value
                        .to_str()
                        .map(|s| Secret::from(String::from(s)))
                        .unwrap_or_else(|error| {
                            logger::warn!(
                                "Response header {} contains non-UTF-8 characters: {error:?}",
                                name.as_str()
                            );
                            Secret::from(String::from("Non-UTF-8 header value"))
                        }),
                )
            })
            .collect::<Vec<_>>();
        let response_body = response
            .text()
            .await
            .map(Secret::from)
            .unwrap_or_else(|error| {
                logger::warn!("Response contains non-UTF-8 characters: {error:?}");
                Secret::from(String::from("Non-UTF-8 response body"))
            });
        let response_to_store = OutgoingWebhookResponseContent {
            body: response_body,
            headers: response_headers,
            status_code: status_code.as_u16(),
        };

        let event_update = domain::EventUpdate::UpdateResponse {
            is_webhook_notified,
            response: Some(
                domain_types::encrypt(
                    response_to_store
                        .encode_to_string_of_json()
                        .change_context(
                            errors::WebhooksFlowError::OutgoingWebhookResponseEncodingFailed,
                        )
                        .map(Secret::new)?,
                    merchant_key_store.key.get_inner().peek(),
                )
                .await
                .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)
                .attach_printable("Failed to encrypt outgoing webhook request content")?,
            ),
        };
        state
            .store
            .update_event_by_merchant_id_event_id(
                &merchant_id,
                &event_id,
                event_update,
                &merchant_key_store,
            )
            .await
            .change_context(errors::WebhooksFlowError::WebhookEventUpdationFailed)
    };
    let success_response_handler =
        |state: AppState,
         merchant_id: String,
         process_tracker: Option<storage::ProcessTracker>,
         business_status: &'static str| async move {
            metrics::WEBHOOK_OUTGOING_RECEIVED_COUNT.add(
                &metrics::CONTEXT,
                1,
                &[metrics::KeyValue::new(MERCHANT_ID, merchant_id)],
            );

            match process_tracker {
                Some(process_tracker) => state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(process_tracker, business_status.into())
                    .await
                    .change_context(
                        errors::WebhooksFlowError::OutgoingWebhookProcessTrackerTaskUpdateFailed,
                    ),
                None => Ok(()),
            }
        };
    let error_response_handler = |merchant_id: String,
                                  delivery_attempt: enums::WebhookDeliveryAttempt,
                                  status_code: u16,
                                  log_message: &'static str| {
        metrics::WEBHOOK_OUTGOING_NOT_RECEIVED_COUNT.add(
            &metrics::CONTEXT,
            1,
            &[metrics::KeyValue::new(MERCHANT_ID, merchant_id)],
        );

        let error = report!(errors::WebhooksFlowError::NotReceivedByMerchant);
        logger::warn!(?error, ?delivery_attempt, ?status_code, %log_message);
    };

    match delivery_attempt {
        enums::WebhookDeliveryAttempt::InitialAttempt => match response {
            Err(client_error) => api_client_error_handler(client_error, delivery_attempt),
            Ok(response) => {
                let status_code = response.status();
                let _updated_event = update_event_in_storage(
                    state.clone(),
                    merchant_key_store.clone(),
                    business_profile.merchant_id.clone(),
                    event_id.clone(),
                    response,
                )
                .await?;

                if status_code.is_success() {
                    success_response_handler(
                        state.clone(),
                        business_profile.merchant_id,
                        process_tracker,
                        "INITIAL_DELIVERY_ATTEMPT_SUCCESSFUL",
                    )
                    .await?;
                } else {
                    error_response_handler(
                        business_profile.merchant_id,
                        delivery_attempt,
                        status_code.as_u16(),
                        "Ignoring error when sending webhook to merchant",
                    );
                }
            }
        },
        enums::WebhookDeliveryAttempt::AutomaticRetry => {
            let process_tracker = process_tracker
                .get_required_value("process_tracker")
                .change_context(errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed)
                .attach_printable("`process_tracker` is unavailable in automatic retry flow")?;
            match response {
                Err(client_error) => {
                    api_client_error_handler(client_error, delivery_attempt);
                    // Schedule a retry attempt for webhook delivery
                    outgoing_webhook_retry::retry_webhook_delivery_task(
                        &*state.store,
                        &business_profile.merchant_id,
                        process_tracker,
                    )
                    .await
                    .change_context(
                        errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed,
                    )?;
                }
                Ok(response) => {
                    let status_code = response.status();
                    let _updated_event = update_event_in_storage(
                        state.clone(),
                        merchant_key_store.clone(),
                        business_profile.merchant_id.clone(),
                        event_id.clone(),
                        response,
                    )
                    .await?;

                    if status_code.is_success() {
                        success_response_handler(
                            state.clone(),
                            business_profile.merchant_id,
                            Some(process_tracker),
                            "COMPLETED_BY_PT",
                        )
                        .await?;
                    } else {
                        error_response_handler(
                            business_profile.merchant_id.clone(),
                            delivery_attempt,
                            status_code.as_u16(),
                            "An error occurred when sending webhook to merchant",
                        );
                        // Schedule a retry attempt for webhook delivery
                        outgoing_webhook_retry::retry_webhook_delivery_task(
                            &*state.store,
                            &business_profile.merchant_id,
                            process_tracker,
                        )
                        .await
                        .change_context(
                            errors::WebhooksFlowError::OutgoingWebhookRetrySchedulingFailed,
                        )?;
                    }
                }
            }
        }
        enums::WebhookDeliveryAttempt::ManualRetry => {
            // Will be updated when manual retry is implemented
            Err(errors::WebhooksFlowError::NotReceivedByMerchant)?
        }
    }

    Ok(())
}

fn raise_webhooks_analytics_event(
    state: AppState,
    trigger_webhook_result: CustomResult<(), errors::WebhooksFlowError>,
    content: Option<api::OutgoingWebhookContent>,
    merchant_id: String,
    event: domain::Event,
) {
    let error = if let Err(error) = trigger_webhook_result {
        logger::error!(?error, "Failed to send webhook to merchant");

        serde_json::to_value(error.current_context())
            .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)
            .map_err(|error| {
                logger::error!(?error, "Failed to serialize outgoing webhook error as JSON");
                error
            })
            .ok()
    } else {
        None
    };

    let outgoing_webhook_event_content = content
        .as_ref()
        .and_then(api::OutgoingWebhookContent::get_outgoing_webhook_event_content);
    let webhook_event = OutgoingWebhookEvent::new(
        merchant_id,
        event.event_id,
        event.event_type,
        outgoing_webhook_event_content,
        error,
        event.initial_attempt_id,
    );
    state.event_handler().log_event(&webhook_event);
}

pub async fn webhooks_wrapper<W: types::OutgoingWebhookType, Ctx: PaymentMethodRetrieve>(
    flow: &impl router_env::types::FlowMetric,
    state: AppState,
    req: &actix_web::HttpRequest,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    connector_name_or_mca_id: &str,
    body: actix_web::web::Bytes,
) -> RouterResponse<serde_json::Value> {
    let start_instant = Instant::now();
    let (application_response, webhooks_response_tracker, serialized_req) =
        Box::pin(webhooks_core::<W, Ctx>(
            state.clone(),
            req,
            merchant_account.clone(),
            key_store,
            connector_name_or_mca_id,
            body.clone(),
        ))
        .await?;

    let request_duration = Instant::now()
        .saturating_duration_since(start_instant)
        .as_millis();

    let request_id = RequestId::extract(req)
        .await
        .attach_printable("Unable to extract request id from request")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let auth_type = auth::AuthenticationType::WebhookAuth {
        merchant_id: merchant_account.merchant_id.clone(),
    };
    let status_code = 200;
    let api_event = ApiEventsType::Webhooks {
        connector: connector_name_or_mca_id.to_string(),
        payment_id: webhooks_response_tracker.get_payment_id(),
    };
    let response_value = serde_json::to_value(&webhooks_response_tracker)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not convert webhook effect to string")?;

    let api_event = ApiEvent::new(
        Some(merchant_account.merchant_id.clone()),
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

/*
This is a temporary fix for converting http::HeaderMap from actix_web to reqwest
Once actix_web upgrades the http version from v0.2.9 to 1.x, this can be removed
*/
fn convert_headers(
    actix_headers: &actix_web::http::header::HeaderMap,
) -> Result<reqwest::header::HeaderMap, errors::ApiErrorResponse> {
    let mut reqwest_headers = reqwest::header::HeaderMap::new();
    for (name, value) in actix_headers.iter() {
        let name_result = reqwest::header::HeaderName::from_str(name.as_str())
            .map_err(|_err| errors::ApiErrorResponse::InternalServerError)?;
        let value_result = reqwest::header::HeaderValue::from_bytes(value.as_bytes())
            .map_err(|_err| errors::ApiErrorResponse::InternalServerError)?;

        reqwest_headers.insert(name_result, value_result);
    }
    Ok(reqwest_headers)
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
    serde_json::Value,
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
        headers: &convert_headers(req.headers())?,
        query_params: req.query_string().to_string(),
        body: &body,
    };

    // Fetch the merchant connector account to get the webhooks source secret
    // `webhooks source secret` is a secret shared between the merchant and connector
    // This is used for source verification and webhooks integrity
    let (merchant_connector_account, connector) = fetch_optional_mca_and_connector(
        &state,
        &merchant_account,
        connector_name_or_mca_id,
        &key_store,
    )
    .await?;

    let connector_name = connector.connector_name.to_string();

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

        let merchant_connector_account = match merchant_connector_account {
            Some(merchant_connector_account) => merchant_connector_account,
            None => {
                helper_utils::get_mca_from_object_reference_id(
                    &*state.clone().store,
                    object_ref_id.clone(),
                    &merchant_account,
                    &connector_name,
                    &key_store,
                )
                .await?
            }
        };

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

        event_object = connector
            .get_webhook_resource_object(&request_details)
            .switch()
            .attach_printable("Could not find resource object in incoming webhook body")?;

        let webhook_details = api::IncomingWebhookDetails {
            object_reference_id: object_ref_id,
            resource_object: serde_json::to_vec(&event_object)
                .change_context(errors::ParsingError::EncodeError("byte-vec"))
                .attach_printable("Unable to convert webhook payload to a value")
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
            api::WebhookFlow::Payment => Box::pin(payments_incoming_webhook_flow::<Ctx>(
                state.clone(),
                merchant_account,
                business_profile,
                key_store,
                webhook_details,
                source_verified,
            ))
            .await
            .attach_printable("Incoming webhook flow for payments failed")?,

            api::WebhookFlow::Refund => Box::pin(refunds_incoming_webhook_flow(
                state.clone(),
                merchant_account,
                business_profile,
                key_store,
                webhook_details,
                connector_name.as_str(),
                source_verified,
                event_type,
            ))
            .await
            .attach_printable("Incoming webhook flow for refunds failed")?,

            api::WebhookFlow::Dispute => Box::pin(disputes_incoming_webhook_flow(
                state.clone(),
                merchant_account,
                business_profile,
                key_store,
                webhook_details,
                source_verified,
                *connector,
                &request_details,
                event_type,
            ))
            .await
            .attach_printable("Incoming webhook flow for disputes failed")?,

            api::WebhookFlow::BankTransfer => Box::pin(bank_transfer_webhook_flow::<Ctx>(
                state.clone(),
                merchant_account,
                business_profile,
                key_store,
                webhook_details,
                source_verified,
            ))
            .await
            .attach_printable("Incoming bank-transfer webhook flow failed")?,

            api::WebhookFlow::ReturnResponse => WebhookResponseTracker::NoEffect,

            api::WebhookFlow::Mandate => Box::pin(mandates_incoming_webhook_flow(
                state.clone(),
                merchant_account,
                business_profile,
                key_store,
                webhook_details,
                source_verified,
                event_type,
            ))
            .await
            .attach_printable("Incoming webhook flow for mandates failed")?,

            _ => Err(errors::ApiErrorResponse::InternalServerError)
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

    let serialized_request = event_object
        .masked_serialize()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not convert webhook effect to string")?;
    Ok((response, webhook_effect, serialized_request))
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

/// This function fetches the merchant connector account ( if the url used is /{merchant_connector_id})
/// if merchant connector id is not passed in the request, then this will return None for mca
async fn fetch_optional_mca_and_connector(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    connector_name_or_mca_id: &str,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<
    (Option<domain::MerchantConnectorAccount>, api::ConnectorData),
    errors::ApiErrorResponse,
> {
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

        Ok((Some(mca), connector))
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

        Ok((None, connector))
    }
}

pub async fn add_outgoing_webhook_retry_task_to_process_tracker(
    db: &dyn StorageInterface,
    business_profile: &diesel_models::business_profile::BusinessProfile,
    event: &domain::Event,
) -> CustomResult<storage::ProcessTracker, errors::StorageError> {
    let schedule_time = outgoing_webhook_retry::get_webhook_delivery_retry_schedule_time(
        db,
        &business_profile.merchant_id,
        0,
    )
    .await
    .ok_or(errors::StorageError::ValueNotFound(
        "Process tracker schedule time".into(), // Can raise a better error here
    ))
    .attach_printable("Failed to obtain initial process tracker schedule time")?;

    let tracking_data = types::OutgoingWebhookTrackingData {
        merchant_id: business_profile.merchant_id.clone(),
        business_profile_id: business_profile.profile_id.clone(),
        event_type: event.event_type,
        event_class: event.event_class,
        primary_object_id: event.primary_object_id.clone(),
        primary_object_type: event.primary_object_type,
        initial_attempt_id: event.initial_attempt_id.clone(),
    };

    let runner = storage::ProcessTrackerRunner::OutgoingWebhookRetryWorkflow;
    let task = "OUTGOING_WEBHOOK_RETRY";
    let tag = ["OUTGOING_WEBHOOKS"];
    let process_tracker_id = scheduler::utils::get_process_tracker_id(
        runner,
        task,
        &event.event_id,
        &business_profile.merchant_id,
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

    match db.insert_process(process_tracker_entry).await {
        Ok(process_tracker) => {
            crate::routes::metrics::TASKS_ADDED_COUNT.add(
                &metrics::CONTEXT,
                1,
                &[add_attributes("flow", "OutgoingWebhookRetry")],
            );
            Ok(process_tracker)
        }
        Err(error) => {
            crate::routes::metrics::TASK_ADDITION_FAILURES_COUNT.add(
                &metrics::CONTEXT,
                1,
                &[add_attributes("flow", "OutgoingWebhookRetry")],
            );
            Err(error)
        }
    }
}

fn get_webhook_url_from_business_profile(
    business_profile: &diesel_models::business_profile::BusinessProfile,
) -> CustomResult<String, errors::WebhooksFlowError> {
    let webhook_details_json = business_profile
        .webhook_details
        .clone()
        .get_required_value("webhook_details")
        .change_context(errors::WebhooksFlowError::MerchantWebhookDetailsNotFound)?;

    let webhook_details: api::WebhookDetails =
        webhook_details_json
            .parse_value("WebhookDetails")
            .change_context(errors::WebhooksFlowError::MerchantWebhookDetailsNotFound)?;

    webhook_details
        .webhook_url
        .get_required_value("webhook_url")
        .change_context(errors::WebhooksFlowError::MerchantWebhookUrlNotConfigured)
        .map(ExposeInterface::expose)
}

pub(crate) fn get_outgoing_webhook_request(
    merchant_account: &domain::MerchantAccount,
    outgoing_webhook: api::OutgoingWebhook,
    payment_response_hash_key: Option<&str>,
) -> CustomResult<OutgoingWebhookRequestContent, errors::WebhooksFlowError> {
    #[inline]
    fn get_outgoing_webhook_request_inner<WebhookType: types::OutgoingWebhookType>(
        outgoing_webhook: api::OutgoingWebhook,
        payment_response_hash_key: Option<&str>,
    ) -> CustomResult<OutgoingWebhookRequestContent, errors::WebhooksFlowError> {
        let mut headers = vec![(
            reqwest::header::CONTENT_TYPE.to_string(),
            mime::APPLICATION_JSON.essence_str().into(),
        )];

        let transformed_outgoing_webhook = WebhookType::from(outgoing_webhook);

        let outgoing_webhooks_signature = transformed_outgoing_webhook
            .get_outgoing_webhooks_signature(payment_response_hash_key)?;

        if let Some(signature) = outgoing_webhooks_signature.signature {
            WebhookType::add_webhook_header(&mut headers, signature)
        }

        Ok(OutgoingWebhookRequestContent {
            body: outgoing_webhooks_signature.payload,
            headers: headers
                .into_iter()
                .map(|(name, value)| (name, Secret::new(value.into_inner())))
                .collect(),
        })
    }

    match merchant_account.get_compatible_connector() {
        #[cfg(feature = "stripe")]
        Some(api_models::enums::Connector::Stripe) => get_outgoing_webhook_request_inner::<
            stripe_webhooks::StripeOutgoingWebhook,
        >(
            outgoing_webhook, payment_response_hash_key
        ),
        _ => get_outgoing_webhook_request_inner::<api_models::webhooks::OutgoingWebhook>(
            outgoing_webhook,
            payment_response_hash_key,
        ),
    }
}
