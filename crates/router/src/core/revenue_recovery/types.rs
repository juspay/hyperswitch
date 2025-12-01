use std::{marker::PhantomData, str::FromStr};

use api_models::{
    enums as api_enums,
    payments::{
        AmountDetails, PaymentRevenueRecoveryMetadata, PaymentsUpdateIntentRequest,
        ProxyPaymentsRequest,
    },
};
use common_utils::{
    self,
    ext_traits::{AsyncExt, OptionExt, ValueExt},
    id_type,
};
use diesel_models::{
    enums, payment_intent, process_tracker::business_status, types as diesel_types,
};
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    business_profile, merchant_connector_account,
    payments::{
        self as domain_payments, payment_attempt::PaymentAttempt, PaymentConfirmData,
        PaymentIntent, PaymentIntentData, PaymentStatusData,
    },
    platform::Platform,
    router_data_v2::{self, flow_common_types},
    router_flow_types,
    router_request_types::revenue_recovery as revenue_recovery_request,
    router_response_types::revenue_recovery as revenue_recovery_response,
    ApiModelToDieselModelConvertor,
};
use time::PrimitiveDateTime;

use super::errors::StorageErrorExt;
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{self, helpers, operations::Operation, transformers::GenerateResponse},
        revenue_recovery::{self as revenue_recovery_core, pcr, perform_calculate_workflow},
        webhooks::{
            create_event_and_trigger_outgoing_webhook, recovery_incoming as recovery_incoming_flow,
        },
    },
    db::StorageInterface,
    logger,
    routes::SessionState,
    services::{self, connector_integration_interface::RouterDataConversion},
    types::{
        self, api as api_types, api::payments as payments_types, domain, storage,
        transformers::ForeignInto,
    },
    workflows::{
        payment_sync,
        revenue_recovery::{self, get_schedule_time_to_retry_mit_payments},
    },
};

type RecoveryResult<T> = error_stack::Result<T, errors::RecoveryError>;
pub const REVENUE_RECOVERY: &str = "revenue_recovery";
/// The status of Passive Churn Payments
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum RevenueRecoveryPaymentsAttemptStatus {
    Succeeded,
    Failed,
    Processing,
    InvalidStatus(String),
    //  Cancelled,
}

impl RevenueRecoveryPaymentsAttemptStatus {
    pub(crate) async fn update_pt_status_based_on_attempt_status_for_execute_payment(
        &self,
        db: &dyn StorageInterface,
        execute_task_process: &storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        logger::info!("Entering update_pt_status_based_on_attempt_status_for_execute_payment");
        match &self {
            Self::Succeeded | Self::Failed | Self::Processing => {
                // finish the current execute task
                db.finish_process_with_business_status(
                    execute_task_process.clone(),
                    business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                )
                .await?;
            }

            Self::InvalidStatus(action) => {
                logger::debug!(
                    "Invalid Attempt Status for the Recovery Payment : {}",
                    action
                );
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Review,
                    business_status: Some(String::from(business_status::EXECUTE_WORKFLOW_COMPLETE)),
                };
                // update the process tracker status as Review
                db.update_process(execute_task_process.clone(), pt_update)
                    .await?;
            }
        };
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn update_pt_status_based_on_attempt_status_for_payments_sync(
        &self,
        state: &SessionState,
        payment_intent: &PaymentIntent,
        process_tracker: storage::ProcessTracker,
        profile: &domain::Profile,
        platform: domain::Platform,
        revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
        payment_attempt: PaymentAttempt,
        revenue_recovery_metadata: &mut PaymentRevenueRecoveryMetadata,
    ) -> Result<(), errors::ProcessTrackerError> {
        let connector_customer_id = payment_intent
            .extract_connector_customer_id_from_payment_intent()
            .change_context(errors::RecoveryError::ValueNotFound)
            .attach_printable("Failed to extract customer ID from payment intent")?;

        let db = &*state.store;

        let recovery_payment_intent =
            hyperswitch_domain_models::revenue_recovery::RecoveryPaymentIntent::from(
                payment_intent,
            );

        let recovery_payment_attempt =
            hyperswitch_domain_models::revenue_recovery::RecoveryPaymentAttempt::from(
                &payment_attempt,
            );

        let recovery_payment_tuple = recovery_incoming_flow::RecoveryPaymentTuple::new(
            &recovery_payment_intent,
            &recovery_payment_attempt,
        );

        let used_token = get_payment_processor_token_id_from_payment_attempt(&payment_attempt);

        let retry_count = process_tracker.retry_count;

        let psync_response = revenue_recovery_payment_data
            .psync_data
            .as_ref()
            .ok_or(errors::RecoveryError::ValueNotFound)
            .attach_printable("Psync data not found in revenue recovery payment data")?;

        match self {
            Self::Succeeded => {
                // finish psync task as the payment was a success
                db.as_scheduler()
                    .finish_process_with_business_status(
                        process_tracker,
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await?;

                let event_status = common_enums::EventType::PaymentSucceeded;

                // publish events to kafka
                if let Err(e) = recovery_incoming_flow::RecoveryPaymentTuple::publish_revenue_recovery_event_to_kafka(
                    state,
                    &recovery_payment_tuple,
                    Some(retry_count+1)
                )
                .await{
                    router_env::logger::error!(
                        "Failed to publish revenue recovery event to kafka: {:?}",
                        e
                    );
                };

                // update the status of token in redis
                let _update_error_code = storage::revenue_recovery_redis_operation::RedisTokenManager::update_payment_processor_token_error_code_from_process_tracker(
                    state,
                    &connector_customer_id,
                    &None,
                    &None,
                    used_token.as_deref(),
                )
                .await;

                // unlocking the token
                let _unlock_the_connector_customer_id = storage::revenue_recovery_redis_operation::RedisTokenManager::unlock_connector_customer_status(
                    state,
                    &connector_customer_id,
                    &payment_intent.id
                )
                .await;

                let payments_response = psync_response
                    .clone()
                    .generate_response(state, None, None, None, &platform, profile, None)
                    .change_context(errors::RecoveryError::PaymentsResponseGenerationFailed)
                    .attach_printable("Failed while generating response for payment")?;

                RevenueRecoveryOutgoingWebhook::send_outgoing_webhook_based_on_revenue_recovery_status(
                    state,
                    common_enums::EventClass::Payments,
                    event_status,
                    payment_intent,
                    &platform,
                    profile,
                    recovery_payment_attempt
                        .attempt_id
                        .get_string_repr()
                        .to_string(),
                    payments_response
                )
                .await?;

                // Record a successful transaction back to Billing Connector
                // TODO: Add support for retrying failed outgoing recordback webhooks
                record_back_to_billing_connector(
                    state,
                    &payment_attempt,
                    payment_intent,
                    &revenue_recovery_payment_data.billing_mca,
                )
                .await
                .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
                .attach_printable("Failed to update the process tracker")?;
            }
            Self::Failed => {
                // finish psync task
                db.as_scheduler()
                    .finish_process_with_business_status(
                        process_tracker.clone(),
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await?;
                // publish events to kafka
                if let Err(e) = recovery_incoming_flow::RecoveryPaymentTuple::publish_revenue_recovery_event_to_kafka(
                    state,
                    &recovery_payment_tuple,
                    Some(retry_count+1)
                )
                .await{
                    router_env::logger::error!(
                        "Failed to publish revenue recovery event to kafka : {:?}", e
                    );
                };

                let error_code = recovery_payment_attempt.error_code;

                let is_hard_decline = revenue_recovery::check_hard_decline(state, &payment_attempt)
                    .await
                    .ok();

                // update the status of token in redis
                let _update_error_code = storage::revenue_recovery_redis_operation::RedisTokenManager::update_payment_processor_token_error_code_from_process_tracker(
                    state,
                    &connector_customer_id,
                    &error_code,
                    &is_hard_decline,
                    used_token.as_deref(),
                )
                .await;

                // unlocking the token
                let _unlock_the_connector_customer_id = storage::revenue_recovery_redis_operation::RedisTokenManager::unlock_connector_customer_status(
                    state,
                    &connector_customer_id,
                    &payment_intent.id
                )
                .await;

                // Reopen calculate workflow on payment failure
                Box::pin(reopen_calculate_workflow_on_payment_failure(
                    state,
                    &process_tracker,
                    profile,
                    platform,
                    payment_intent,
                    revenue_recovery_payment_data,
                    psync_response.payment_attempt.get_id(),
                ))
                .await?;
            }
            Self::Processing => {
                // do a psync payment
                let action = Box::pin(Action::payment_sync_call(
                    state,
                    revenue_recovery_payment_data,
                    payment_intent,
                    &process_tracker,
                    profile,
                    platform,
                    payment_attempt,
                ))
                .await?;

                //handle the response
                Box::pin(action.psync_response_handler(
                    state,
                    payment_intent,
                    &process_tracker,
                    revenue_recovery_metadata,
                    revenue_recovery_payment_data,
                ))
                .await?;
            }
            Self::InvalidStatus(status) => logger::debug!(
                "Invalid Attempt Status for the Recovery Payment : {}",
                status
            ),
        }
        Ok(())
    }
}
pub enum Decision {
    Execute,
    Psync(enums::AttemptStatus, id_type::GlobalAttemptId),
    InvalidDecision,
    ReviewForSuccessfulPayment,
    ReviewForFailedPayment(enums::TriggeredBy),
}

impl Decision {
    pub async fn get_decision_based_on_params(
        state: &SessionState,
        intent_status: enums::IntentStatus,
        called_connector: enums::PaymentConnectorTransmission,
        active_attempt_id: Option<id_type::GlobalAttemptId>,
        revenue_recovery_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
        payment_id: &id_type::GlobalPaymentId,
    ) -> RecoveryResult<Self> {
        logger::info!("Entering get_decision_based_on_params");

        Ok(match (intent_status, called_connector, active_attempt_id) {
            (
                enums::IntentStatus::Failed,
                enums::PaymentConnectorTransmission::ConnectorCallUnsuccessful,
                None,
            ) => Self::Execute,
            (
                enums::IntentStatus::Processing,
                enums::PaymentConnectorTransmission::ConnectorCallSucceeded,
                Some(_),
            ) => {
                let psync_data = revenue_recovery_core::api::call_psync_api(
                    state,
                    payment_id,
                    revenue_recovery_data,
                    true,
                    true,
                )
                .await
                .change_context(errors::RecoveryError::PaymentCallFailed)
                .attach_printable("Error while executing the Psync call")?;
                let payment_attempt = psync_data.payment_attempt;
                Self::Psync(payment_attempt.status, payment_attempt.get_id().clone())
            }
            (
                enums::IntentStatus::Failed,
                enums::PaymentConnectorTransmission::ConnectorCallUnsuccessful,
                Some(_),
            ) => {
                let psync_data = revenue_recovery_core::api::call_psync_api(
                    state,
                    payment_id,
                    revenue_recovery_data,
                    true,
                    true,
                )
                .await
                .change_context(errors::RecoveryError::PaymentCallFailed)
                .attach_printable("Error while executing the Psync call")?;

                let payment_attempt = psync_data.payment_attempt;

                let attempt_triggered_by = payment_attempt
                    .feature_metadata
                    .and_then(|metadata| {
                        metadata.revenue_recovery.map(|revenue_recovery_metadata| {
                            revenue_recovery_metadata.attempt_triggered_by
                        })
                    })
                    .get_required_value("Attempt Triggered By")
                    .change_context(errors::RecoveryError::ValueNotFound)?;
                Self::ReviewForFailedPayment(attempt_triggered_by)
            }
            (enums::IntentStatus::Succeeded, _, _) => Self::ReviewForSuccessfulPayment,
            _ => Self::InvalidDecision,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    SyncPayment(PaymentAttempt),
    RetryPayment(PrimitiveDateTime),
    TerminalFailure(PaymentAttempt),
    SuccessfulPayment(PaymentAttempt),
    ReviewPayment,
    ManualReviewAction,
}
impl Action {
    #[allow(clippy::too_many_arguments)]
    pub async fn execute_payment(
        state: &SessionState,
        _merchant_id: &id_type::MerchantId,
        payment_intent: &PaymentIntent,
        process: &storage::ProcessTracker,
        profile: &domain::Profile,
        platform: domain::Platform,
        revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
        revenue_recovery_metadata: &PaymentRevenueRecoveryMetadata,
        latest_attempt_id: &id_type::GlobalAttemptId,
        scheduled_token: &storage::revenue_recovery_redis_operation::PaymentProcessorTokenStatus,
    ) -> RecoveryResult<Self> {
        let connector_customer_id = payment_intent
            .extract_connector_customer_id_from_payment_intent()
            .change_context(errors::RecoveryError::ValueNotFound)
            .attach_printable("Failed to extract customer ID from payment intent")?;

        let response = revenue_recovery_core::api::call_proxy_api(
            state,
            payment_intent,
            revenue_recovery_payment_data,
            revenue_recovery_metadata,
            &scheduled_token
                .payment_processor_token_details
                .payment_processor_token,
        )
        .await;

        let recovery_payment_intent =
            hyperswitch_domain_models::revenue_recovery::RecoveryPaymentIntent::from(
                payment_intent,
            );
        // handle proxy api's response
        match response {
            Ok(payment_data) => {
                let account_updater_action = storage::revenue_recovery_redis_operation::RedisTokenManager::handle_account_updater_token_update(
                    state,
                    &connector_customer_id,
                    scheduled_token,
                    payment_data.mandate_data.clone(),
                    &payment_data.payment_attempt.id
                ).await
                .inspect_err(|e| {
                    logger::error!(
                        "Failed to handle get valid action: {:?}",
                        e
                    );
                })
                .ok();

                let _account_updater_result = account_updater_action
                    .async_map(|action| {
                        let customer_id = connector_customer_id.clone();
                        let payment_attempt_id = payment_data.payment_attempt.id.clone();
                        async move {
                            action
                                .handle_account_updater_action(
                                    state,
                                    customer_id.as_str(),
                                    scheduled_token,
                                    &payment_attempt_id,
                                )
                                .await
                        }
                    })
                    .await
                    .transpose()
                    .inspect_err(|e| {
                        logger::error!("Failed to handle account updater action: {:?}", e);
                    })
                    .ok();

                match payment_data.payment_attempt.status.foreign_into() {
                    RevenueRecoveryPaymentsAttemptStatus::Succeeded => {
                        let recovery_payment_attempt =
                        hyperswitch_domain_models::revenue_recovery::RecoveryPaymentAttempt::from(
                            &payment_data.payment_attempt,
                        );

                        let recovery_payment_tuple =
                            recovery_incoming_flow::RecoveryPaymentTuple::new(
                                &recovery_payment_intent,
                                &recovery_payment_attempt,
                            );

                        // publish events to kafka
                        if let Err(e) = recovery_incoming_flow::RecoveryPaymentTuple::publish_revenue_recovery_event_to_kafka(
                        state,
                        &recovery_payment_tuple,
                        Some(process.retry_count+1)
                    )
                    .await{
                        router_env::logger::error!(
                            "Failed to publish revenue recovery event to kafka: {:?}",
                            e
                        );
                    };

                        // update the status of token in redis
                        let _update_error_code = storage::revenue_recovery_redis_operation::RedisTokenManager::update_payment_processor_token_error_code_from_process_tracker(
                        state,
                        &connector_customer_id,
                        &None,
                        &None,
                        Some(&scheduled_token.payment_processor_token_details.payment_processor_token),
                    )
                    .await;

                        // unlocking the token
                        let _unlock_the_connector_customer_id = storage::revenue_recovery_redis_operation::RedisTokenManager::unlock_connector_customer_status(
                        state,
                        &connector_customer_id,
                        &payment_intent.id
                    )
                    .await;

                        let event_status = common_enums::EventType::PaymentSucceeded;

                        let payments_response = payment_data
                            .clone()
                            .generate_response(state, None, None, None, &platform, profile, None)
                            .change_context(errors::RecoveryError::PaymentsResponseGenerationFailed)
                            .attach_printable("Failed while generating response for payment")?;

                        RevenueRecoveryOutgoingWebhook::send_outgoing_webhook_based_on_revenue_recovery_status(
                        state,
                        common_enums::EventClass::Payments,
                        event_status,
                        payment_intent,
                        &platform,
                        profile,
                        payment_data.payment_attempt.id.get_string_repr().to_string(),
                        payments_response
                    )
                    .await?;

                        Ok(Self::SuccessfulPayment(
                            payment_data.payment_attempt.clone(),
                        ))
                    }
                    RevenueRecoveryPaymentsAttemptStatus::Failed => {
                        let recovery_payment_attempt =
                        hyperswitch_domain_models::revenue_recovery::RecoveryPaymentAttempt::from(
                            &payment_data.payment_attempt,
                        );

                        let recovery_payment_tuple =
                            recovery_incoming_flow::RecoveryPaymentTuple::new(
                                &recovery_payment_intent,
                                &recovery_payment_attempt,
                            );

                        // publish events to kafka
                        if let Err(e) = recovery_incoming_flow::RecoveryPaymentTuple::publish_revenue_recovery_event_to_kafka(
                        state,
                        &recovery_payment_tuple,
                        Some(process.retry_count+1)
                    )
                    .await{
                        router_env::logger::error!(
                            "Failed to publish revenue recovery event to kafka: {:?}",
                            e
                        );
                    };

                        let error_code = payment_data
                            .payment_attempt
                            .clone()
                            .error
                            .map(|error| error.code);

                        let is_hard_decline = revenue_recovery::check_hard_decline(
                            state,
                            &payment_data.payment_attempt,
                        )
                        .await
                        .ok();

                        let _update_connector_customer_id = storage::revenue_recovery_redis_operation::RedisTokenManager::update_payment_processor_token_error_code_from_process_tracker(
                        state,
                        &connector_customer_id,
                        &error_code,
                        &is_hard_decline,
                        Some(&scheduled_token
                            .payment_processor_token_details
                            .payment_processor_token)
                            ,
                    )
                    .await;

                        // unlocking the token
                        let _unlock_connector_customer_id = storage::revenue_recovery_redis_operation::RedisTokenManager::unlock_connector_customer_status(
                        state,
                        &connector_customer_id,
                        &payment_intent.id
                    )
                    .await;

                        // Reopen calculate workflow on payment failure
                        Box::pin(reopen_calculate_workflow_on_payment_failure(
                            state,
                            process,
                            profile,
                            platform,
                            payment_intent,
                            revenue_recovery_payment_data,
                            latest_attempt_id,
                        ))
                        .await?;

                        // Return terminal failure to finish the current execute workflow
                        Ok(Self::TerminalFailure(payment_data.payment_attempt.clone()))
                    }

                    RevenueRecoveryPaymentsAttemptStatus::Processing => {
                        Ok(Self::SyncPayment(payment_data.payment_attempt.clone()))
                    }
                    RevenueRecoveryPaymentsAttemptStatus::InvalidStatus(action) => {
                        logger::info!(?action, "Invalid Payment Status For PCR Payment");
                        Ok(Self::ManualReviewAction)
                    }
                }
            }
            Err(err) =>
            // check for an active attempt being constructed or not
            {
                logger::error!(execute_payment_res=?err);
                Ok(Self::ReviewPayment)
            }
        }
    }

    pub async fn execute_payment_task_response_handler(
        &self,
        state: &SessionState,
        payment_intent: &PaymentIntent,
        execute_task_process: &storage::ProcessTracker,
        revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
        revenue_recovery_metadata: &mut PaymentRevenueRecoveryMetadata,
    ) -> Result<(), errors::ProcessTrackerError> {
        logger::info!("Entering execute_payment_task_response_handler");

        let db = &*state.store;
        match self {
            Self::SyncPayment(payment_attempt) => {
                revenue_recovery_core::insert_psync_pcr_task_to_pt(
                    revenue_recovery_payment_data.billing_mca.get_id().clone(),
                    db,
                    revenue_recovery_payment_data
                        .merchant_account
                        .get_id()
                        .to_owned(),
                    payment_intent.id.clone(),
                    revenue_recovery_payment_data.profile.get_id().to_owned(),
                    payment_attempt.id.clone(),
                    storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                    revenue_recovery_payment_data.retry_algorithm,
                )
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to create a psync workflow in the process tracker")?;

                db.as_scheduler()
                    .finish_process_with_business_status(
                        execute_task_process.clone(),
                        business_status::EXECUTE_WORKFLOW_COMPLETE_FOR_PSYNC,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }

            Self::RetryPayment(schedule_time) => {
                db.as_scheduler()
                    .retry_process(execute_task_process.clone(), *schedule_time)
                    .await?;

                // update the connector payment transmission field to Unsuccessful and unset active attempt id
                revenue_recovery_metadata.set_payment_transmission_field_for_api_request(
                    enums::PaymentConnectorTransmission::ConnectorCallUnsuccessful,
                );

                let payment_update_req =
                PaymentsUpdateIntentRequest::update_feature_metadata_and_active_attempt_with_api(
                    payment_intent
                        .feature_metadata
                        .clone()
                        .unwrap_or_default()
                        .convert_back()
                        .set_payment_revenue_recovery_metadata_using_api(
                            revenue_recovery_metadata.clone(),
                        ),
                    api_enums::UpdateActiveAttempt::Unset,
                );
                logger::info!(
                    "Call made to payments update intent api , with the request body {:?}",
                    payment_update_req
                );
                Box::pin(revenue_recovery_core::api::update_payment_intent_api(
                    state,
                    payment_intent.id.clone(),
                    revenue_recovery_payment_data,
                    payment_update_req,
                ))
                .await
                .change_context(errors::RecoveryError::PaymentCallFailed)?;
                Ok(())
            }
            Self::TerminalFailure(payment_attempt) => {
                db.as_scheduler()
                    .finish_process_with_business_status(
                        execute_task_process.clone(),
                        business_status::EXECUTE_WORKFLOW_FAILURE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                // TODO: Add support for retrying failed outgoing recordback webhooks
                Ok(())
            }
            Self::SuccessfulPayment(payment_attempt) => {
                db.as_scheduler()
                    .finish_process_with_business_status(
                        execute_task_process.clone(),
                        business_status::EXECUTE_WORKFLOW_COMPLETE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                // Record back to billing connector for terminal status
                // TODO: Add support for retrying failed outgoing recordback webhooks
                record_back_to_billing_connector(
                    state,
                    payment_attempt,
                    payment_intent,
                    &revenue_recovery_payment_data.billing_mca,
                )
                .await
                .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
                .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::ReviewPayment => {
                // requeue the process tracker in case of error response
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Pending,
                    business_status: Some(String::from(business_status::EXECUTE_WORKFLOW_REQUEUE)),
                };
                db.as_scheduler()
                    .update_process(execute_task_process.clone(), pt_update)
                    .await?;
                Ok(())
            }
            Self::ManualReviewAction => {
                logger::debug!("Invalid Payment Status For PCR Payment");
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Review,
                    business_status: Some(String::from(business_status::EXECUTE_WORKFLOW_COMPLETE)),
                };
                // update the process tracker status as Review
                db.as_scheduler()
                    .update_process(execute_task_process.clone(), pt_update)
                    .await?;
                Ok(())
            }
        }
    }

    pub async fn payment_sync_call(
        state: &SessionState,
        revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
        payment_intent: &PaymentIntent,
        process: &storage::ProcessTracker,
        profile: &domain::Profile,
        platform: domain::Platform,
        payment_attempt: PaymentAttempt,
    ) -> RecoveryResult<Self> {
        logger::info!("Entering payment_sync_call");

        let response = revenue_recovery_core::api::call_psync_api(
            state,
            payment_intent.get_id(),
            revenue_recovery_payment_data,
            true,
            true,
        )
        .await;
        let used_token = get_payment_processor_token_id_from_payment_attempt(&payment_attempt);

        match response {
            Ok(_payment_data) => match payment_attempt.status.foreign_into() {
                RevenueRecoveryPaymentsAttemptStatus::Succeeded => {
                    let connector_customer_id = payment_intent
                        .extract_connector_customer_id_from_payment_intent()
                        .change_context(errors::RecoveryError::ValueNotFound)
                        .attach_printable("Failed to extract customer ID from payment intent")?;

                    // update the status of token in redis
                    let _update_error_code = storage::revenue_recovery_redis_operation::RedisTokenManager::update_payment_processor_token_error_code_from_process_tracker(
                    state,
                    &connector_customer_id,
                    &None,
                    &None,
                    used_token.as_deref(),
                )
                .await;

                    // unlocking the token
                    let _unlock_the_connector_customer_id = storage::revenue_recovery_redis_operation::RedisTokenManager::unlock_connector_customer_status(
                    state,
                    &connector_customer_id,
                    &payment_intent.id
                )
                .await;

                    Ok(Self::SuccessfulPayment(payment_attempt))
                }
                RevenueRecoveryPaymentsAttemptStatus::Failed => {
                    let connector_customer_id = payment_intent
                        .extract_connector_customer_id_from_payment_intent()
                        .change_context(errors::RecoveryError::ValueNotFound)
                        .attach_printable("Failed to extract customer ID from payment intent")?;

                    let error_code = payment_attempt.clone().error.map(|error| error.code);

                    let is_hard_decline =
                        revenue_recovery::check_hard_decline(state, &payment_attempt)
                            .await
                            .ok();

                    let _update_error_code = storage::revenue_recovery_redis_operation::RedisTokenManager::update_payment_processor_token_error_code_from_process_tracker(
                            state,
                            &connector_customer_id,
                            &error_code,
                            &is_hard_decline,
                            used_token.as_deref(),
                        )
                        .await;

                    // unlocking the token
                    let _unlock_connector_customer_id = storage::revenue_recovery_redis_operation::RedisTokenManager::unlock_connector_customer_status(
                        state,
                        &connector_customer_id,
                        &payment_intent.id
                    )
                    .await;

                    // Reopen calculate workflow on payment failure
                    Box::pin(reopen_calculate_workflow_on_payment_failure(
                        state,
                        process,
                        profile,
                        platform,
                        payment_intent,
                        revenue_recovery_payment_data,
                        payment_attempt.get_id(),
                    ))
                    .await?;

                    Ok(Self::TerminalFailure(payment_attempt.clone()))
                }

                RevenueRecoveryPaymentsAttemptStatus::Processing => {
                    Ok(Self::SyncPayment(payment_attempt))
                }
                RevenueRecoveryPaymentsAttemptStatus::InvalidStatus(action) => {
                    logger::info!(?action, "Invalid Payment Status For PCR PSync Payment");
                    Ok(Self::ManualReviewAction)
                }
            },
            Err(err) =>
            // if there is an error while psync we create a new Review Task
            {
                logger::error!(sync_payment_response=?err);
                Ok(Self::ReviewPayment)
            }
        }
    }
    pub async fn psync_response_handler(
        &self,
        state: &SessionState,
        payment_intent: &PaymentIntent,
        psync_task_process: &storage::ProcessTracker,
        revenue_recovery_metadata: &mut PaymentRevenueRecoveryMetadata,
        revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
    ) -> Result<(), errors::ProcessTrackerError> {
        logger::info!("Entering psync_response_handler");

        let db = &*state.store;

        let connector_customer_id = payment_intent
            .feature_metadata
            .as_ref()
            .and_then(|fm| fm.payment_revenue_recovery_metadata.as_ref())
            .map(|rr| {
                rr.billing_connector_payment_details
                    .connector_customer_id
                    .clone()
            });

        match self {
            Self::SyncPayment(payment_attempt) => {
                //  get a schedule time for psync
                // and retry the process if there is a schedule time
                // if None mark the pt status as Retries Exceeded and finish the task
                payment_sync::recovery_retry_sync_task(
                    state,
                    connector_customer_id,
                    revenue_recovery_metadata.connector.to_string(),
                    revenue_recovery_payment_data
                        .merchant_account
                        .get_id()
                        .clone(),
                    psync_task_process.clone(),
                )
                .await?;
                Ok(())
            }

            Self::RetryPayment(schedule_time) => {
                // finish the psync task
                db.as_scheduler()
                    .finish_process_with_business_status(
                        psync_task_process.clone(),
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;

                // fetch the execute task
                let task = revenue_recovery_core::EXECUTE_WORKFLOW;
                let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
                let process_tracker_id = payment_intent
                    .get_id()
                    .get_execute_revenue_recovery_id(task, runner);
                let execute_task_process = db
                    .as_scheduler()
                    .find_process_by_id(&process_tracker_id)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)?
                    .get_required_value("Process Tracker")?;
                // retry the execute tasks
                db.as_scheduler()
                    .retry_process(execute_task_process, *schedule_time)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }

            Self::TerminalFailure(payment_attempt) => {
                // TODO: Add support for retrying failed outgoing recordback webhooks
                // finish the current psync task
                db.as_scheduler()
                    .finish_process_with_business_status(
                        psync_task_process.clone(),
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::SuccessfulPayment(payment_attempt) => {
                // finish the current psync task
                db.as_scheduler()
                    .finish_process_with_business_status(
                        psync_task_process.clone(),
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;

                // Record a successful transaction back to Billing Connector
                // TODO: Add support for retrying failed outgoing recordback webhooks
                record_back_to_billing_connector(
                    state,
                    payment_attempt,
                    payment_intent,
                    &revenue_recovery_payment_data.billing_mca,
                )
                .await
                .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
                .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::ReviewPayment => {
                // requeue the process tracker task in case of psync api error
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Pending,
                    business_status: Some(String::from(business_status::PSYNC_WORKFLOW_REQUEUE)),
                };
                db.as_scheduler()
                    .update_process(psync_task_process.clone(), pt_update)
                    .await?;
                Ok(())
            }
            Self::ManualReviewAction => {
                logger::debug!("Invalid Payment Status For PCR Payment");
                let pt_update = storage::ProcessTrackerUpdate::StatusUpdate {
                    status: enums::ProcessTrackerStatus::Review,
                    business_status: Some(String::from(business_status::PSYNC_WORKFLOW_COMPLETE)),
                };
                // update the process tracker status as Review
                db.as_scheduler()
                    .update_process(psync_task_process.clone(), pt_update)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;

                Ok(())
            }
        }
    }

    pub(crate) async fn decide_retry_failure_action(
        state: &SessionState,
        merchant_id: &id_type::MerchantId,
        pt: storage::ProcessTracker,
        revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
        payment_attempt: &PaymentAttempt,
        payment_intent: &PaymentIntent,
    ) -> RecoveryResult<Self> {
        let db = &*state.store;
        let next_retry_count = pt.retry_count + 1;
        let error_message = payment_attempt
            .error
            .as_ref()
            .map(|details| details.message.clone());
        let error_code = payment_attempt
            .error
            .as_ref()
            .map(|details| details.code.clone());
        let connector_name = payment_attempt
            .connector
            .clone()
            .ok_or(errors::RecoveryError::ValueNotFound)
            .attach_printable("unable to derive payment connector from payment attempt")?;
        let gsm_record = helpers::get_gsm_record(
            state,
            error_code,
            error_message,
            connector_name,
            REVENUE_RECOVERY.to_string(),
        )
        .await;
        let is_hard_decline = gsm_record
            .and_then(|gsm_record| gsm_record.error_category)
            .map(|gsm_error_category| {
                gsm_error_category == common_enums::ErrorCategory::HardDecline
            })
            .unwrap_or(false);
        let schedule_time = revenue_recovery_payment_data
            .get_schedule_time_based_on_retry_type(
                state,
                merchant_id,
                next_retry_count,
                payment_attempt,
                payment_intent,
                is_hard_decline,
            )
            .await;

        match schedule_time {
            Some(schedule_time) => Ok(Self::RetryPayment(schedule_time)),

            None => Ok(Self::TerminalFailure(payment_attempt.clone())),
        }
    }
}

/// Reopen calculate workflow when payment fails
pub async fn reopen_calculate_workflow_on_payment_failure(
    state: &SessionState,
    process: &storage::ProcessTracker,
    profile: &domain::Profile,
    platform: domain::Platform,
    payment_intent: &PaymentIntent,
    revenue_recovery_payment_data: &storage::revenue_recovery::RevenueRecoveryPaymentData,
    latest_attempt_id: &id_type::GlobalAttemptId,
) -> RecoveryResult<()> {
    let db = &*state.store;
    let id = payment_intent.id.clone();
    let task = revenue_recovery_core::CALCULATE_WORKFLOW;
    let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;

    let old_tracking_data: pcr::RevenueRecoveryWorkflowTrackingData =
        serde_json::from_value(process.tracking_data.clone())
            .change_context(errors::RecoveryError::ValueNotFound)
            .attach_printable("Failed to deserialize the tracking data from process tracker")?;

    let retry_algorithm_type = profile
        .revenue_recovery_retry_algorithm_type
        .filter(|retry_type| *retry_type != common_enums::RevenueRecoveryAlgorithmType::Monitoring) // ignore Monitoring
        .unwrap_or(old_tracking_data.revenue_recovery_retry);

    let new_tracking_data = pcr::RevenueRecoveryWorkflowTrackingData {
        payment_attempt_id: latest_attempt_id.clone(),
        revenue_recovery_retry: retry_algorithm_type,
        merchant_id: old_tracking_data.merchant_id.clone(),
        profile_id: old_tracking_data.profile_id.clone(),
        global_payment_id: old_tracking_data.global_payment_id.clone(),
        billing_mca_id: old_tracking_data.billing_mca_id.clone(),
        invoice_scheduled_time: old_tracking_data.invoice_scheduled_time,
    };

    let tracking_data = serde_json::to_value(new_tracking_data)
        .change_context(errors::RecoveryError::ValueNotFound)
        .attach_printable("Failed to serialize the tracking data for process tracker")?;

    // Construct the process tracker ID for CALCULATE_WORKFLOW
    let process_tracker_id = format!("{}_{}_{}", runner, task, id.get_string_repr());

    logger::info!(
        payment_id = %id.get_string_repr(),
        process_tracker_id = %process_tracker_id,
        "Attempting to reopen CALCULATE_WORKFLOW on payment failure"
    );

    // Find the existing CALCULATE_WORKFLOW process tracker
    let calculate_process = db
        .find_process_by_id(&process_tracker_id)
        .await
        .change_context(errors::RecoveryError::ProcessTrackerFailure)
        .attach_printable("Failed to find CALCULATE_WORKFLOW process tracker")?;

    match calculate_process {
        Some(process) => {
            logger::info!(
                payment_id = %id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                current_status = %process.business_status,
                current_retry_count = process.retry_count,
                "Found existing CALCULATE_WORKFLOW, updating status and retry count"
            );

            // Update the process tracker to reopen the calculate workflow
            // 1. Change status from "finish" to "pending"
            // 2. Increase retry count by 1
            // 3. Set business status to QUEUED
            // 4. Schedule for immediate execution
            let new_retry_count = process.retry_count + 1;
            let new_schedule_time = common_utils::date_time::now()
                + time::Duration::seconds(
                    state
                        .conf
                        .revenue_recovery
                        .recovery_timestamp
                        .reopen_workflow_buffer_time_in_seconds,
                );

            let pt_update = storage::ProcessTrackerUpdate::Update {
                name: Some(task.to_string()),
                retry_count: Some(new_retry_count),
                schedule_time: Some(new_schedule_time),
                tracking_data: Some(tracking_data),
                business_status: Some(String::from(business_status::PENDING)),
                status: Some(common_enums::ProcessTrackerStatus::Pending),
                updated_at: Some(common_utils::date_time::now()),
            };

            db.update_process(process.clone(), pt_update)
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to update CALCULATE_WORKFLOW process tracker")?;

            logger::info!(
                payment_id = %id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                new_retry_count = new_retry_count,
                new_schedule_time = %new_schedule_time,
                "Successfully reopened CALCULATE_WORKFLOW with increased retry count"
            );
        }
        None => {
            logger::info!(
                payment_id = %id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                "CALCULATE_WORKFLOW process tracker not found, creating new entry"
            );

            let task = "CALCULATE_WORKFLOW";

            let db = &*state.store;

            // Create process tracker ID in the format: CALCULATE_WORKFLOW_{payment_intent_id}
            let process_tracker_id = format!("{runner}_{task}_{}", id.get_string_repr());

            // Set scheduled time to current time + buffer time set in configuration
            let schedule_time = common_utils::date_time::now()
                + time::Duration::seconds(
                    state
                        .conf
                        .revenue_recovery
                        .recovery_timestamp
                        .reopen_workflow_buffer_time_in_seconds,
                );

            let new_retry_count = process.retry_count + 1;

            // Check if a process tracker entry already exists for this payment intent
            let existing_entry = db
                .as_scheduler()
                .find_process_by_id(&process_tracker_id)
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable(
                    "Failed to check for existing calculate workflow process tracker entry",
                )?;

            // No entry exists - create a new one
            router_env::logger::info!(
                    "No existing CALCULATE_WORKFLOW task found for payment_intent_id: {}, creating new entry... ",
                    id.get_string_repr()
                );

            let tag = ["PCR"];
            let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;

            let process_tracker_entry = storage::ProcessTrackerNew::new(
                &process_tracker_id,
                task,
                runner,
                tag,
                process.tracking_data.clone(),
                Some(new_retry_count),
                schedule_time,
                common_types::consts::API_VERSION,
            )
            .change_context(errors::RecoveryError::ProcessTrackerFailure)
            .attach_printable("Failed to construct calculate workflow process tracker entry")?;

            // Insert into process tracker with status New
            db.as_scheduler()
                .insert_process(process_tracker_entry)
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable(
                    "Failed to enter calculate workflow process_tracker_entry in DB",
                )?;

            router_env::logger::info!(
                "Successfully created new CALCULATE_WORKFLOW task for payment_intent_id: {}",
                id.get_string_repr()
            );

            logger::info!(
                payment_id = %id.get_string_repr(),
                process_tracker_id = %process_tracker_id,
                "Successfully created new CALCULATE_WORKFLOW entry using perform_calculate_workflow"
            );
        }
    }

    Ok(())
}

// TODO: Move these to impl based functions
async fn record_back_to_billing_connector(
    state: &SessionState,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
    billing_mca: &merchant_connector_account::MerchantConnectorAccount,
) -> RecoveryResult<()> {
    logger::info!("Entering record_back_to_billing_connector");

    let connector_name = billing_mca.connector_name.to_string();
    let connector_data = api_types::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api_types::GetToken::Connector,
        Some(billing_mca.get_id()),
    )
    .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
    .attach_printable("invalid connector name received in billing merchant connector account")?;

    let connector_integration: services::BoxedRevenueRecoveryRecordBackInterface<
        router_flow_types::InvoiceRecordBack,
        revenue_recovery_request::InvoiceRecordBackRequest,
        revenue_recovery_response::InvoiceRecordBackResponse,
    > = connector_data.connector.get_connector_integration();

    let router_data = construct_invoice_record_back_router_data(
        state,
        billing_mca,
        payment_attempt,
        payment_intent,
    )?;

    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
    .attach_printable("Failed while handling response of record back to billing connector")?;

    match response.response {
        Ok(response) => Ok(response),
        error @ Err(_) => {
            router_env::logger::error!(?error);
            Err(errors::RecoveryError::RecordBackToBillingConnectorFailed)
                .attach_printable("Failed while recording back to billing connector")
        }
    }?;
    Ok(())
}

pub fn construct_invoice_record_back_router_data(
    state: &SessionState,
    billing_mca: &merchant_connector_account::MerchantConnectorAccount,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
) -> RecoveryResult<hyperswitch_domain_models::types::InvoiceRecordBackRouterData> {
    logger::info!("Entering construct_invoice_record_back_router_data");

    let auth_type: types::ConnectorAuthType =
        helpers::MerchantConnectorAccountType::DbVal(Box::new(billing_mca.clone()))
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)?;

    let merchant_reference_id = payment_intent
        .merchant_reference_id
        .clone()
        .ok_or(errors::RecoveryError::RecordBackToBillingConnectorFailed)
        .attach_printable(
            "Merchant reference id not found while recording back to billing connector",
        )?;
    let connector_name = billing_mca.get_connector_name_as_string();
    let connector = common_enums::connector_enums::Connector::from_str(connector_name.as_str())
        .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
        .attach_printable("Cannot find connector from the connector_name")?;

    let connector_params =
        hyperswitch_domain_models::connector_endpoints::Connectors::get_connector_params(
            &state.conf.connectors,
            connector,
        )
        .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
        .attach_printable(format!(
            "cannot find connector params for this connector {connector} in this flow",
        ))?;

    let router_data = router_data_v2::RouterDataV2 {
        flow: PhantomData::<router_flow_types::InvoiceRecordBack>,
        tenant_id: state.tenant.tenant_id.clone(),
        resource_common_data: flow_common_types::InvoiceRecordBackData {
            connector_meta_data: None,
        },
        connector_auth_type: auth_type,
        request: revenue_recovery_request::InvoiceRecordBackRequest {
            merchant_reference_id,
            amount: payment_attempt.get_total_amount(),
            currency: payment_intent.amount_details.currency,
            payment_method_type: Some(payment_attempt.payment_method_subtype),
            attempt_status: payment_attempt.status,
            connector_transaction_id: payment_attempt
                .connector_payment_id
                .as_ref()
                .map(|id| common_utils::types::ConnectorTransactionId::TxnId(id.clone())),
            connector_params,
        },
        response: Err(types::ErrorResponse::default()),
    };
    let old_router_data = flow_common_types::InvoiceRecordBackData::to_old_router_data(router_data)
        .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
        .attach_printable("Cannot construct record back router data")?;
    Ok(old_router_data)
}

pub fn get_payment_processor_token_id_from_payment_attempt(
    payment_attempt: &PaymentAttempt,
) -> Option<String> {
    let used_token = payment_attempt
        .connector_token_details
        .as_ref()
        .and_then(|t| t.connector_mandate_id.clone());
    logger::info!("Used token in the payment attempt : {:?}", used_token);

    used_token
}

pub struct RevenueRecoveryOutgoingWebhook;

impl RevenueRecoveryOutgoingWebhook {
    #[allow(clippy::too_many_arguments)]
    pub async fn send_outgoing_webhook_based_on_revenue_recovery_status(
        state: &SessionState,
        event_class: common_enums::EventClass,
        event_status: common_enums::EventType,
        payment_intent: &PaymentIntent,
        platform: &domain::Platform,
        profile: &domain::Profile,
        payment_attempt_id: String,
        payments_response: ApplicationResponse<api_models::payments::PaymentsResponse>,
    ) -> RecoveryResult<()> {
        match payments_response {
            ApplicationResponse::JsonWithHeaders((response, _headers)) => {
                let outgoing_webhook_content =
                    api_models::webhooks::OutgoingWebhookContent::PaymentDetails(Box::new(
                        response,
                    ));
                create_event_and_trigger_outgoing_webhook(
                    state.clone(),
                    profile.clone(),
                    platform.get_processor().get_key_store(),
                    event_status,
                    event_class,
                    payment_attempt_id,
                    common_enums::EventObjectType::PaymentDetails,
                    outgoing_webhook_content,
                    payment_intent.created_at,
                )
                .await
                .change_context(errors::RecoveryError::InvalidTask)
                .attach_printable("Failed to send out going webhook")?;

                Ok(())
            }

            _other_variant => {
                // Handle other successful response types if needed
                logger::warn!("Unexpected application response variant for outgoing webhook");
                Err(errors::RecoveryError::RevenueRecoveryOutgoingWebhookFailed.into())
            }
        }
    }
}
