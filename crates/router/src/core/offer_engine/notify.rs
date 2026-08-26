use common_utils::{ext_traits::ValueExt, id_type};
use diesel_models::process_tracker::business_status;
use hyperswitch_domain_models::platform::ProcessorMerchantId;
use scheduler::{consumer::types::process_data, utils as pt_utils};

use super::{
    client::OfferEngineClient,
    config::resolve_offer_engine_credentials,
    types::{OfferNotifyOffer, OfferNotifyRequest, OfferNotifyStatus, OfferTxnStatus},
};
use crate::{
    core::{configs::dimension_state, errors},
    logger,
    routes::{metrics, SessionState},
    types::storage,
};

const OFFER_ENGINE_NOTIFY_NAME: &str = "OFFER_ENGINE_NOTIFY";
const OFFER_ENGINE_NOTIFY_TAG: &str = "OFFER_ENGINE";
const OFFER_ENGINE_NOTIFY_FLOW: &str = "OfferEngineNotify";
const OFFER_ENGINE_NOTIFY_RUNNER: diesel_models::ProcessTrackerRunner =
    diesel_models::ProcessTrackerRunner::OfferEngineNotifyWorkflow;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OfferEngineNotifyTrackingData {
    pub payment_id: id_type::PaymentId,
    pub payment_attempt_id: String,
    pub processor_merchant_id: id_type::MerchantId,
    pub refund_id: Option<String>,
    #[serde(default)]
    pub txn_status: OfferTxnStatus,
}

fn payment_task_id(attempt_id: &str) -> String {
    format!("offer_engine_notify_payment_{attempt_id}")
}

fn refund_task_id(refund_id: &str) -> String {
    format!("offer_engine_notify_refund_{refund_id}")
}

fn notify_attributes() -> [router_env::opentelemetry::KeyValue; 1] {
    [router_env::opentelemetry::KeyValue::new(
        "flow",
        OFFER_ENGINE_NOTIFY_FLOW,
    )]
}

/// Offer Engine txn status to report for an attempt, or `None` when it must not notify.
fn attempt_notify_status(status: common_enums::AttemptStatus) -> Option<OfferTxnStatus> {
    use common_enums::AttemptStatus;
    match status {
        AttemptStatus::AutoRefunded
        | AttemptStatus::Failure
        | AttemptStatus::AuthenticationFailed
        | AttemptStatus::AuthorizationFailed
        | AttemptStatus::RouterDeclined
        | AttemptStatus::Voided
        | AttemptStatus::VoidFailed
        | AttemptStatus::CaptureFailed
        | AttemptStatus::Expired => Some(OfferTxnStatus::Failure),
        AttemptStatus::Charged
        | AttemptStatus::PartialCharged
        | AttemptStatus::Started
        | AttemptStatus::AuthenticationPending
        | AttemptStatus::AuthenticationSuccessful
        | AttemptStatus::Authorized
        | AttemptStatus::Authorizing
        | AttemptStatus::CodInitiated
        | AttemptStatus::VoidedPostCharge
        | AttemptStatus::VoidInitiated
        | AttemptStatus::CaptureInitiated
        | AttemptStatus::PartiallyAuthorized
        | AttemptStatus::PartialChargedAndChargeable
        | AttemptStatus::Unresolved
        | AttemptStatus::Pending
        | AttemptStatus::PaymentMethodAwaited
        | AttemptStatus::ConfirmationAwaited
        | AttemptStatus::DeviceDataCollectionPending
        | AttemptStatus::IntegrityFailure
        | AttemptStatus::CaptureReview => None,
    }
}

pub async fn schedule_payment_notification_for_attempt(
    state: &SessionState,
    payment_attempt: &storage::PaymentAttempt,
) {
    if let Some(txn_status) = attempt_notify_status(payment_attempt.status)
        .filter(|_| payment_attempt.applied_offer_details.is_some())
    {
        let tracking_data = OfferEngineNotifyTrackingData {
            payment_id: payment_attempt.payment_id.clone(),
            payment_attempt_id: payment_attempt.attempt_id.clone(),
            processor_merchant_id: payment_attempt.processor_merchant_id.clone(),
            refund_id: None,
            txn_status,
        };
        let task_id = payment_task_id(&payment_attempt.attempt_id);
        insert_notification_task(state, task_id, tracking_data).await;
    }
}

pub async fn schedule_refund_notification(
    state: &SessionState,
    payment_attempt: &storage::PaymentAttempt,
    refund: &diesel_models::refund::Refund,
) {
    if payment_attempt.applied_offer_details.is_some() {
        let tracking_data = OfferEngineNotifyTrackingData {
            payment_id: refund.payment_id.clone(),
            payment_attempt_id: refund.attempt_id.clone(),
            processor_merchant_id: payment_attempt.processor_merchant_id.clone(),
            refund_id: Some(refund.refund_id.clone()),
            txn_status: OfferTxnStatus::Failure,
        };
        let task_id = refund_task_id(&refund.refund_id);
        insert_notification_task(state, task_id, tracking_data).await;
    }
}

async fn insert_notification_task(
    state: &SessionState,
    task_id: String,
    tracking_data: OfferEngineNotifyTrackingData,
) {
    let schedule_time = common_utils::date_time::now();
    match storage::ProcessTrackerNew::new(
        task_id.clone(),
        OFFER_ENGINE_NOTIFY_NAME,
        OFFER_ENGINE_NOTIFY_RUNNER,
        [OFFER_ENGINE_NOTIFY_TAG],
        tracking_data,
        None,
        schedule_time,
        common_types::consts::API_VERSION,
        common_enums::ApplicationSource::Main,
    ) {
        Err(err) => {
            logger::error!(?err, %task_id, "Failed to construct offer engine notify task");
            metrics::OFFER_ENGINE_NOTIFY_SCHEDULE_FAILURES.add(1, &notify_attributes());
        }
        Ok(entry) => match state.store.insert_process(entry).await {
            Ok(_) => {
                logger::info!(%task_id, "Scheduled offer engine notify task");
                metrics::OFFER_ENGINE_NOTIFY_TASKS_SCHEDULED.add(1, &notify_attributes());
            }
            Err(err) if err.current_context().is_db_unique_violation() => {
                logger::info!(%task_id, "Offer engine notify task already scheduled; suppressing duplicate");
            }
            Err(err) => {
                logger::error!(?err, %task_id, "Failed to schedule offer engine notify task");
                metrics::OFFER_ENGINE_NOTIFY_SCHEDULE_FAILURES.add(1, &notify_attributes());
            }
        },
    }
}

pub async fn execute_notification(
    state: &SessionState,
    process: storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let tracking_data: OfferEngineNotifyTrackingData = process
        .tracking_data
        .clone()
        .parse_value("OfferEngineNotifyTrackingData")?;
    let attributes = notify_attributes();

    let db = &*state.store;
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            &tracking_data.processor_merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await?;
    let merchant_account = db
        .find_merchant_account_by_merchant_id(&tracking_data.processor_merchant_id, &key_store)
        .await?;
    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_processor_merchant_id(
            &tracking_data.payment_attempt_id,
            &tracking_data.processor_merchant_id,
            merchant_account.storage_scheme,
            &key_store,
        )
        .await?;

    match payment_attempt.applied_offer_details.as_ref() {
        None => {
            logger::warn!(
                payment_id = %tracking_data.payment_id.get_string_repr(),
                payment_attempt_id = %tracking_data.payment_attempt_id,
                refund_id = ?tracking_data.refund_id,
                process_tracker_id = %process.id,
                "applied_offer_details missing; terminating offer engine notify task"
            );
            metrics::OFFER_ENGINE_NOTIFY_TERMINAL_FAILURES.add(1, &attributes);
            db.as_scheduler()
                .finish_process_with_business_status(
                    process,
                    business_status::RESOURCE_STATUS_MISMATCH,
                )
                .await
                .map_err(Into::<errors::ProcessTrackerError>::into)
        }
        Some(applied) => {
            let dimensions = dimension_state::Dimensions::new()
                .with_processor_merchant_id(ProcessorMerchantId::from(
                    payment_attempt.processor_merchant_id.clone(),
                ))
                .with_organization_id(payment_attempt.organization_id.clone())
                .with_profile_id(payment_attempt.profile_id.clone());
            match resolve_offer_engine_credentials(state, &dimensions).await {
                Ok(Some(config)) => {
                    let applied = applied.inner();
                    let request = OfferNotifyRequest {
                        order_id: tracking_data.payment_id.get_string_repr().to_string(),
                        txn_id: applied.offer_engine_txn_id.clone(),
                        txn_status: tracking_data.txn_status,
                        merchant_id: applied.offer_engine_merchant_id.clone(),
                        offers: vec![OfferNotifyOffer {
                            offer_id: applied.offer_id.clone(),
                            status: OfferNotifyStatus::Revoked,
                            error_code: None,
                            error_message: None,
                        }],
                        refund_id: tracking_data.refund_id.clone(),
                    };

                    let client =
                        OfferEngineClient::new(config, &state.conf.trace_header.header_name);
                    match client.notify(state, request).await {
                        Ok(()) => {
                            logger::info!(
                                payment_id = %tracking_data.payment_id.get_string_repr(),
                                payment_attempt_id = %tracking_data.payment_attempt_id,
                                refund_id = ?tracking_data.refund_id,
                                offer_id = %applied.offer_id,
                                process_tracker_id = %process.id,
                                attempt = process.retry_count,
                                "Offer engine notification delivered"
                            );
                            metrics::OFFER_ENGINE_NOTIFY_SUCCESS.add(1, &attributes);
                            db.as_scheduler()
                                .finish_process_with_business_status(
                                    process,
                                    business_status::COMPLETED_BY_PT,
                                )
                                .await
                                .map_err(Into::<errors::ProcessTrackerError>::into)
                        }
                        Err(err) => {
                            logger::warn!(
                                ?err,
                                payment_attempt_id = %tracking_data.payment_attempt_id,
                                refund_id = ?tracking_data.refund_id,
                                process_tracker_id = %process.id,
                                attempt = process.retry_count,
                                "Offer engine notification delivery failed"
                            );
                            let mapping = process_data::RetryMapping::default();
                            let time_delta = if process.retry_count == 0 {
                                Some(mapping.start_after)
                            } else {
                                pt_utils::get_delay(process.retry_count + 1, &mapping.frequencies)
                            };
                            match pt_utils::get_time_from_delta(time_delta) {
                                Some(schedule_time) => db
                                    .as_scheduler()
                                    .retry_process(process, schedule_time)
                                    .await
                                    .map_err(Into::<errors::ProcessTrackerError>::into),
                                None => {
                                    metrics::OFFER_ENGINE_NOTIFY_TERMINAL_FAILURES
                                        .add(1, &attributes);
                                    db.as_scheduler()
                                        .finish_process_with_business_status(
                                            process,
                                            business_status::RETRIES_EXCEEDED,
                                        )
                                        .await
                                        .map_err(Into::<errors::ProcessTrackerError>::into)
                                }
                            }
                        }
                    }
                }
                other => {
                    logger::error!(
                        ?other,
                        process_tracker_id = %process.id,
                        "Offer Engine credentials unavailable; terminating notify task"
                    );
                    metrics::OFFER_ENGINE_NOTIFY_TERMINAL_FAILURES.add(1, &attributes);
                    db.as_scheduler()
                        .finish_process_with_business_status(process, business_status::FAILURE)
                        .await
                        .map_err(Into::<errors::ProcessTrackerError>::into)
                }
            }
        }
    }
}
