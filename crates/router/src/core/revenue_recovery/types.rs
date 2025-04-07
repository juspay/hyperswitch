use std::marker::PhantomData;

use api_models::{
    enums as api_enums,
    payments::{
        AmountDetails, PaymentRevenueRecoveryMetadata, PaymentsUpdateIntentRequest,
        ProxyPaymentsRequest,
    },
};
use common_utils::{
    self,
    ext_traits::{OptionExt, ValueExt},
    id_type,
};
use diesel_models::{enums, process_tracker::business_status, types as diesel_types};
use error_stack::{self, ResultExt};
use hyperswitch_domain_models::{
    business_profile, merchant_connector_account,
    payments::{
        self as domain_payments, payment_attempt, PaymentConfirmData, PaymentIntent,
        PaymentIntentData,
    },
    router_data_v2::{self, flow_common_types},
    router_flow_types,
    router_request_types::revenue_recovery as revenue_recovery_request,
    router_response_types::revenue_recovery as revenue_recovery_response,
    ApiModelToDieselModelConvertor,
};
use time::PrimitiveDateTime;

use crate::{
    core::{
        errors::{self, RouterResult},
        payments::{self, helpers, operations::Operation},
        revenue_recovery::{self as core_pcr},
    },
    db::StorageInterface,
    logger,
    routes::SessionState,
    services::{self, connector_integration_interface::RouterDataConversion},
    types::{
        self, api as api_types, api::payments as payments_types, storage, transformers::ForeignInto,
    },
    workflows::revenue_recovery::get_schedule_time_to_retry_mit_payments,
};

type RecoveryResult<T> = error_stack::Result<T, errors::RecoveryError>;

/// The status of Passive Churn Payments
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum PcrAttemptStatus {
    Succeeded,
    Failed,
    Processing,
    InvalidStatus(String),
    //  Cancelled,
}

impl PcrAttemptStatus {
    pub(crate) async fn update_pt_status_based_on_attempt_status_for_execute_payment(
        &self,
        db: &dyn StorageInterface,
        execute_task_process: &storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
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

    pub(crate) async fn update_pt_status_based_on_attempt_status_for_payments_sync(
        &self,
        state: &SessionState,
        process_tracker: storage::ProcessTracker,
        pcr_data: &storage::revenue_recovery::PcrPaymentData,
        tracking_data: &storage::revenue_recovery::PcrWorkflowTrackingData,
        payment_attempt: payment_attempt::PaymentAttempt,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;

        match self {
            Self::Succeeded => {
                // finish psync task as the payment was a success
                db.as_scheduler()
                    .finish_process_with_business_status(
                        process_tracker,
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await?;
                // TODO: send back the successful webhook
            }
            Self::Failed => {
                // finish psync task
                db.as_scheduler()
                    .finish_process_with_business_status(
                        process_tracker.clone(),
                        business_status::PSYNC_WORKFLOW_COMPLETE,
                    )
                    .await?;

                // get a reschedule time
                let schedule_time = get_schedule_time_to_retry_mit_payments(
                    db,
                    &pcr_data.merchant_account.get_id().clone(),
                    process_tracker.retry_count + 1,
                )
                .await;

                // check if retry is possible
                if let Some(schedule_time) = schedule_time {
                    // schedule a retry
                    db.as_scheduler()
                        .retry_process(process_tracker.clone(), schedule_time)
                        .await?;
                } else {
                    // TODO: Record a failure back to the billing connector
                }

                // TODO: Update connecter called field and active attempt
            }
            Self::Processing => {
                // do a psync payment
                let action = Box::pin(Action::payment_sync_call(
                    state,
                    pcr_data,
                    &tracking_data.global_payment_id,
                    &process_tracker,
                    payment_attempt,
                ))
                .await?;

                //handle the response
                action
                    .psync_response_handler(db, &process_tracker, tracking_data)
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
    ReviewForFailedPayment,
}

impl Decision {
    pub async fn get_decision_based_on_params(
        state: &SessionState,
        intent_status: enums::IntentStatus,
        called_connector: enums::PaymentConnectorTransmission,
        active_attempt_id: Option<id_type::GlobalAttemptId>,
        pcr_data: &storage::revenue_recovery::PcrPaymentData,
        payment_id: &id_type::GlobalPaymentId,
    ) -> RecoveryResult<Self> {
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
                let psync_data = core_pcr::api::call_psync_api(state, payment_id, pcr_data)
                    .await
                    .change_context(errors::RecoveryError::PaymentCallFailed)
                    .attach_printable("Error while executing the Psync call")?;
                let payment_attempt = psync_data
                    .payment_attempt
                    .get_required_value("Payment Attempt")
                    .change_context(errors::RecoveryError::ValueNotFound)
                    .attach_printable("Error while executing the Psync call")?;
                Self::Psync(payment_attempt.status, payment_attempt.get_id().clone())
            }
            (
                enums::IntentStatus::Failed,
                enums::PaymentConnectorTransmission::ConnectorCallSucceeded,
                Some(_),
            ) => Self::ReviewForFailedPayment,
            (
                enums::IntentStatus::Succeeded,
                enums::PaymentConnectorTransmission::ConnectorCallSucceeded,
                Some(_),
            ) => Self::ReviewForSuccessfulPayment,
            _ => Self::InvalidDecision,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    SyncPayment(id_type::GlobalAttemptId),
    RetryPayment(PrimitiveDateTime),
    TerminalFailure(payment_attempt::PaymentAttempt),
    SuccessfulPayment(payment_attempt::PaymentAttempt),
    ReviewPayment,
    ManualReviewAction,
}
impl Action {
    pub async fn execute_payment(
        state: &SessionState,
        merchant_id: &id_type::MerchantId,
        payment_intent: &PaymentIntent,
        process: &storage::ProcessTracker,
        pcr_data: &storage::revenue_recovery::PcrPaymentData,
        revenue_recovery_metadata: &PaymentRevenueRecoveryMetadata,
    ) -> RecoveryResult<Self> {
        let db = &*state.store;
        let response = core_pcr::api::call_proxy_api(
            state,
            payment_intent,
            pcr_data,
            revenue_recovery_metadata,
        )
        .await;
        // handle proxy api's response
        match response {
            Ok(payment_data) => match payment_data.payment_attempt.status.foreign_into() {
                PcrAttemptStatus::Succeeded => Ok(Self::SuccessfulPayment(
                    payment_data.payment_attempt.clone(),
                )),
                PcrAttemptStatus::Failed => {
                    Self::decide_retry_failure_action(
                        db,
                        merchant_id,
                        process.clone(),
                        &payment_data.payment_attempt,
                    )
                    .await
                }

                PcrAttemptStatus::Processing => {
                    Ok(Self::SyncPayment(payment_data.payment_attempt.id.clone()))
                }
                PcrAttemptStatus::InvalidStatus(action) => {
                    logger::info!(?action, "Invalid Payment Status For PCR Payment");
                    Ok(Self::ManualReviewAction)
                }
            },
            Err(err) =>
            // check for an active attempt being constructed or not
            {
                logger::error!(execute_payment_res=?err);
                match payment_intent.active_attempt_id.clone() {
                    Some(attempt_id) => Ok(Self::SyncPayment(attempt_id)),
                    None => Ok(Self::ReviewPayment),
                }
            }
        }
    }

    pub async fn execute_payment_task_response_handler(
        &self,
        state: &SessionState,
        payment_intent: &PaymentIntent,
        execute_task_process: &storage::ProcessTracker,
        pcr_data: &storage::revenue_recovery::PcrPaymentData,
        revenue_recovery_metadata: &mut PaymentRevenueRecoveryMetadata,
        billing_mca: &merchant_connector_account::MerchantConnectorAccount,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        match self {
            Self::SyncPayment(attempt_id) => {
                core_pcr::insert_psync_pcr_task(
                    db,
                    pcr_data.merchant_account.get_id().to_owned(),
                    payment_intent.id.clone(),
                    pcr_data.profile.get_id().to_owned(),
                    attempt_id.clone(),
                    storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
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

                let payment_update_req = PaymentsUpdateIntentRequest::update_feature_metadata_and_active_attempt_with_api(
        payment_intent.feature_metadata.clone().unwrap_or_default().convert_back().set_payment_revenue_recovery_metadata_using_api(
                revenue_recovery_metadata.clone()
            ),
            api_enums::UpdateActiveAttempt::Unset,
        );
                logger::info!(
                    "Call made to payments update intent api , with the request body {:?}",
                    payment_update_req
                );
                core_pcr::api::update_payment_intent_api(
                    state,
                    payment_intent.id.clone(),
                    pcr_data,
                    payment_update_req,
                )
                .await
                .change_context(errors::RecoveryError::PaymentCallFailed)?;
                Ok(())
            }
            Self::TerminalFailure(payment_attempt) => {
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
                self.record_back_to_billing_connector(
                    state,
                    payment_attempt,
                    payment_intent,
                    billing_mca,
                )
                .await
                .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
                .attach_printable("Failed to record back the billing connector")?;
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
                self.record_back_to_billing_connector(
                    state,
                    payment_attempt,
                    payment_intent,
                    billing_mca,
                )
                .await
                .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
                .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }
            Self::ReviewPayment => {
                let tracking_data_for_review = execute_task_process
                    .tracking_data
                    .clone()
                    .parse_value::<storage::revenue_recovery::PcrWorkflowTrackingData>(
                    "PCRWorkflowTrackingData",
                )?;
                core_pcr::insert_review_task(
                    db,
                    &tracking_data_for_review,
                    storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                )
                .await?;
                db.finish_process_with_business_status(
                    execute_task_process.clone(),
                    business_status::PSYNC_WORKFLOW_COMPLETE_FOR_REVIEW,
                )
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
        pcr_data: &storage::revenue_recovery::PcrPaymentData,
        global_payment_id: &id_type::GlobalPaymentId,
        process: &storage::ProcessTracker,
        payment_attempt: payment_attempt::PaymentAttempt,
    ) -> RecoveryResult<Self> {
        let response = core_pcr::api::call_psync_api(state, global_payment_id, pcr_data).await;
        let db = &*state.store;
        let active_attempt_id = payment_attempt.get_id().clone();
        match response {
            Ok(_payment_data) => match payment_attempt.status.foreign_into() {
                PcrAttemptStatus::Succeeded => Ok(Self::SuccessfulPayment(payment_attempt)),
                PcrAttemptStatus::Failed => {
                    Self::decide_retry_failure_action(
                        db,
                        pcr_data.merchant_account.get_id(),
                        process.clone(),
                        &payment_attempt,
                    )
                    .await
                }

                PcrAttemptStatus::Processing => Ok(Self::SyncPayment(active_attempt_id)),
                PcrAttemptStatus::InvalidStatus(action) => {
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
        db: &dyn StorageInterface,
        psync_task_process: &storage::ProcessTracker,
        tracking_data: &storage::revenue_recovery::PcrWorkflowTrackingData,
    ) -> Result<(), errors::ProcessTrackerError> {
        match self {
            Self::SyncPayment(_active_attempt_id) => {
                // retry the Psync Tasks
                let pt_task_update = diesel_models::ProcessTrackerUpdate::StatusUpdate {
                    status: storage::enums::ProcessTrackerStatus::Pending,
                    business_status: Some(business_status::PENDING.to_owned()),
                };
                // ? get a schedule time for psync and retry the process
                db.as_scheduler()
                    .update_process(psync_task_process.clone(), pt_task_update)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
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

                // TODO: Update connecter called field and active attempt

                // retry the execute task
                let task = "EXECUTE_WORKFLOW";
                let runner = storage::ProcessTrackerRunner::PassiveRecoveryWorkflow;
                let process_tracker_id = format!(
                    "{runner}_{task}_{}",
                    tracking_data.global_payment_id.get_string_repr()
                );
                let execute_task_process = db
                    .as_scheduler()
                    .find_process_by_id(&process_tracker_id)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)?
                    .get_required_value("Process Tracker")?;

                db.as_scheduler()
                    .retry_process(execute_task_process, *schedule_time)
                    .await
                    .change_context(errors::RecoveryError::ProcessTrackerFailure)
                    .attach_printable("Failed to update the process tracker")?;
                Ok(())
            }

            Self::TerminalFailure(payment_attempt) => {
                // TODO: Record a failure transaction back to Billing Connector
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
                // TODO: Record a successful transaction back to Billing Connector
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
            Self::ReviewPayment => {
                core_pcr::insert_review_task(
                    db,
                    tracking_data,
                    storage::ProcessTrackerRunner::PassiveRecoveryWorkflow,
                )
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to insert the task to process tracker")?;

                db.finish_process_with_business_status(
                    psync_task_process.clone(),
                    business_status::PSYNC_WORKFLOW_COMPLETE_FOR_REVIEW,
                )
                .await
                .change_context(errors::RecoveryError::ProcessTrackerFailure)
                .attach_printable("Failed to update the process tracker")?;
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
    async fn record_back_to_billing_connector(
        &self,
        state: &SessionState,
        payment_attempt: &payment_attempt::PaymentAttempt,
        payment_intent: &PaymentIntent,
        billing_mca: &merchant_connector_account::MerchantConnectorAccount,
    ) -> RecoveryResult<()> {
        let connector_name = billing_mca.connector_name.to_string();
        let connector_data = api_types::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector_name,
            api_types::GetToken::Connector,
            Some(billing_mca.get_id()),
        )
        .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
        .attach_printable(
            "invalid connector name received in billing merchant connector account",
        )?;

        let connector_integration: services::BoxedRevenueRecoveryRecordBackInterface<
            router_flow_types::RecoveryRecordBack,
            revenue_recovery_request::RevenueRecoveryRecordBackRequest,
            revenue_recovery_response::RevenueRecoveryRecordBackResponse,
        > = connector_data.connector.get_connector_integration();

        let router_data = self.construct_recovery_record_back_router_data(
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
        )
        .await
        .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
        .attach_printable("Failed while handling response of record back to billing connector")?;

        let record_back_response = match response.response {
            Ok(response) => Ok(response),
            error @ Err(_) => {
                router_env::logger::error!(?error);
                Err(errors::RecoveryError::RecordBackToBillingConnectorFailed)
                    .attach_printable("Failed while recording back to billing connector")
            }
        }?;
        Ok(())
    }

    pub fn construct_recovery_record_back_router_data(
        &self,
        state: &SessionState,
        billing_mca: &merchant_connector_account::MerchantConnectorAccount,
        payment_attempt: &payment_attempt::PaymentAttempt,
        payment_intent: &PaymentIntent,
    ) -> RecoveryResult<hyperswitch_domain_models::types::RevenueRecoveryRecordBackRouterData> {
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

        let router_data = router_data_v2::RouterDataV2 {
            flow: PhantomData::<router_flow_types::RecoveryRecordBack>,
            tenant_id: state.tenant.tenant_id.clone(),
            resource_common_data: flow_common_types::RevenueRecoveryRecordBackData,
            connector_auth_type: auth_type,
            request: revenue_recovery_request::RevenueRecoveryRecordBackRequest {
                merchant_reference_id,
                amount: payment_attempt.get_total_amount(),
                currency: payment_intent.amount_details.currency,
                payment_method_type: payment_attempt.payment_method_subtype,
                attempt_status: payment_attempt.status,
                connector_transaction_id: payment_attempt
                    .connector_payment_id
                    .as_ref()
                    .map(|id| common_utils::types::ConnectorTransactionId::TxnId(id.clone())),
            },
            response: Err(types::ErrorResponse::default()),
        };
        let old_router_data =
            flow_common_types::RevenueRecoveryRecordBackData::to_old_router_data(router_data)
                .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
                .attach_printable("Cannot construct record back router data")?;
        Ok(old_router_data)
    }

    pub(crate) async fn decide_retry_failure_action(
        db: &dyn StorageInterface,
        merchant_id: &id_type::MerchantId,
        pt: storage::ProcessTracker,
        payment_attempt: &payment_attempt::PaymentAttempt,
    ) -> RecoveryResult<Self> {
        let schedule_time =
            get_schedule_time_to_retry_mit_payments(db, merchant_id, pt.retry_count + 1).await;
        match schedule_time {
            Some(schedule_time) => Ok(Self::RetryPayment(schedule_time)),

            None => Ok(Self::TerminalFailure(payment_attempt.clone())),
        }
    }
}
