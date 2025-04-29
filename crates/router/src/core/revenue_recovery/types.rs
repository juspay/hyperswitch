use std::{marker::PhantomData,str::FromStr};
use api_models::{
    enums as api_enums,
    mandates::RecurringDetails,
    payments::{
        AmountDetails, FeatureMetadata, PaymentRevenueRecoveryMetadata,
        PaymentsUpdateIntentRequest, ProxyPaymentsRequest,
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
}
#[derive(Debug, Clone)]
pub enum Decision {
    Execute,
    Psync(enums::AttemptStatus, id_type::GlobalAttemptId),
    InvalidDecision,
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
                let psync_data = core_pcr::call_psync_api(state, payment_id, pcr_data)
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
        let response =
            call_proxy_api(state, payment_intent, pcr_data, revenue_recovery_metadata).await;
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
                    billing_mca.get_id().clone(),
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
                update_payment_intent_api(
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

            Self::ReviewPayment => Ok(()),
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
        let connector_name = billing_mca.get_connector_name_as_string();
        let connector = common_enums::connector_enums::Connector::from_str(connector_name.as_str())
            .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
            .attach_printable("Cannot find connector from the connector_name")?;

        let connector_params = hyperswitch_domain_models::configs::Connectors::get_connector_params(
            &state.conf.connectors,
            connector
        )
        .change_context(errors::RecoveryError::RecordBackToBillingConnectorFailed)
        .attach_printable(format!("cannot find connector params for this connector {} in this flow",connector))?;
    
        let router_data = router_data_v2::RouterDataV2 {
            flow: PhantomData::<router_flow_types::RecoveryRecordBack>,
            tenant_id: state.tenant.tenant_id.clone(),
            resource_common_data: flow_common_types::RevenueRecoveryRecordBackData,
            connector_auth_type: auth_type,
            request: revenue_recovery_request::RevenueRecoveryRecordBackRequest {
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

async fn call_proxy_api(
    state: &SessionState,
    payment_intent: &PaymentIntent,
    pcr_data: &storage::revenue_recovery::PcrPaymentData,
    revenue_recovery: &PaymentRevenueRecoveryMetadata,
) -> RouterResult<PaymentConfirmData<payments_types::Authorize>> {
    let operation = payments::operations::proxy_payments_intent::PaymentProxyIntent;
    let req = ProxyPaymentsRequest {
        return_url: None,
        amount: AmountDetails::new(payment_intent.amount_details.clone().into()),
        recurring_details: revenue_recovery.get_payment_token_for_api_request(),
        shipping: None,
        browser_info: None,
        connector: revenue_recovery.connector.to_string(),
        merchant_connector_id: revenue_recovery.get_merchant_connector_id_for_api_request(),
    };
    logger::info!(
        "Call made to payments proxy api , with the request body {:?}",
        req
    );

    // TODO : Use api handler instead of calling get_tracker and payments_operation_core
    // Get the tracker related information. This includes payment intent and payment attempt
    let get_tracker_response = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            payment_intent.get_id(),
            &req,
            &pcr_data.merchant_account,
            &pcr_data.profile,
            &pcr_data.key_store,
            &hyperswitch_domain_models::payments::HeaderPayload::default(),
            None,
        )
        .await?;

    let (payment_data, _req, _, _) = Box::pin(payments::proxy_for_payments_operation_core::<
        payments_types::Authorize,
        _,
        _,
        _,
        PaymentConfirmData<payments_types::Authorize>,
    >(
        state,
        state.get_req_state(),
        pcr_data.merchant_account.clone(),
        pcr_data.key_store.clone(),
        pcr_data.profile.clone(),
        operation,
        req,
        get_tracker_response,
        payments::CallConnectorAction::Trigger,
        hyperswitch_domain_models::payments::HeaderPayload::default(),
    ))
    .await?;
    Ok(payment_data)
}

pub async fn update_payment_intent_api(
    state: &SessionState,
    global_payment_id: id_type::GlobalPaymentId,
    pcr_data: &storage::revenue_recovery::PcrPaymentData,
    update_req: PaymentsUpdateIntentRequest,
) -> RouterResult<PaymentIntentData<payments_types::PaymentUpdateIntent>> {
    // TODO : Use api handler instead of calling payments_intent_operation_core
    let operation = payments::operations::PaymentUpdateIntent;
    let (payment_data, _req, customer) = payments::payments_intent_operation_core::<
        payments_types::PaymentUpdateIntent,
        _,
        _,
        PaymentIntentData<payments_types::PaymentUpdateIntent>,
    >(
        state,
        state.get_req_state(),
        pcr_data.merchant_account.clone(),
        pcr_data.profile.clone(),
        pcr_data.key_store.clone(),
        operation,
        update_req,
        global_payment_id,
        hyperswitch_domain_models::payments::HeaderPayload::default(),
        None,
    )
    .await?;
    Ok(payment_data)
}
