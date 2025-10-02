#[cfg(feature = "v1")]
use api_models::subscription as subscription_types;
use async_trait::async_trait;
use common_utils::{
    errors::CustomResult,
    ext_traits::{StringExt, ValueExt},
};
use diesel_models::{
    invoice::Invoice, process_tracker::business_status, subscription::Subscription,
};
use error_stack::ResultExt;
use router_env::logger;
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
    errors,
    types::process_data,
    utils as scheduler_utils,
};

use crate::core::subscription::{
    billing_processor_handler as billing, invoice_handler, payments_api_client,
};
use crate::{
    db::StorageInterface,
    errors as router_errors,
    routes::SessionState,
    types::{domain, storage},
};

const IVOICE_SYNC_WORKFLOW: &str = "INVOICE_SYNC";
const IVOICE_SYNC_WORKFLOW_TAG: &str = "INVOICE";
pub struct InvoiceSyncWorkflow;

pub struct InvoiceSyncHandler<'a> {
    pub state: &'a SessionState,
    pub tracking_data: storage::invoice_sync::InvoiceSyncTrackingData,
    pub key_store: domain::MerchantKeyStore,
    pub merchant_account: domain::MerchantAccount,
    pub customer: domain::Customer,
    pub profile: domain::Profile,
    pub subscription: Subscription,
    pub invoice: Invoice,
}

impl<'a> InvoiceSyncHandler<'a> {
    pub async fn create(
        state: &'a SessionState,
        tracking_data: storage::invoice_sync::InvoiceSyncTrackingData,
    ) -> Result<Self, errors::ProcessTrackerError> {
        let key_manager_state = &state.into();
        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &tracking_data.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .attach_printable("Failed to fetch Merchant key store from DB")?;

        let merchant_account = state
            .store
            .find_merchant_account_by_merchant_id(
                key_manager_state,
                &tracking_data.merchant_id,
                &key_store,
            )
            .await
            .attach_printable("Subscriptions: Failed to fetch Merchant Account from DB")?;

        let subscription = state
            .store
            .find_by_merchant_id_subscription_id(
                merchant_account.get_id(),
                tracking_data.subscription_id.get_string_repr().to_string(),
            )
            .await
            .attach_printable("Subscriptions: Failed to fetch subscription from DB")?;

        let invoice = state
            .store
            .get_latest_invoice_for_subscription(subscription.id.get_string_repr().to_string())
            .await
            .attach_printable("invoices: unable to get latest invoice from database")?;

        let customer = state
            .store
            .find_customer_by_customer_id_merchant_id(
                &(state).into(),
                &tracking_data.customer_id,
                merchant_account.get_id(),
                &key_store,
                merchant_account.storage_scheme,
            )
            .await
            .attach_printable("Subscriptions: Failed to fetch Customer from DB")?;

        let profile = state
            .store
            .find_business_profile_by_profile_id(
                &(state).into(),
                &key_store,
                &tracking_data.profile_id,
            )
            .await
            .attach_printable("Subscriptions: Failed to fetch Business Profile from DB")?;

        Ok(Self {
            state,
            tracking_data,
            key_store,
            merchant_account,
            customer,
            profile,
            subscription,
            invoice,
        })
    }

    pub async fn perform_payments_sync(
        &self,
    ) -> CustomResult<subscription_types::PaymentResponseData, router_errors::ApiErrorResponse>
    {
        let payment_id = self.invoice.payment_intent_id.clone().ok_or(
            router_errors::ApiErrorResponse::SubscriptionError {
                operation: "Invoice_sync: Missing Payment Intent ID in Invoice".to_string(),
            },
        )?;
        let payments_response = payments_api_client::PaymentsApiClient::sync_payment(
            self.state,
            payment_id.get_string_repr().to_string(),
            self.merchant_account.get_id().get_string_repr(),
            self.profile.get_id().get_string_repr(),
        )
        .await
        .change_context(router_errors::ApiErrorResponse::SubscriptionError {
            operation: "Invoice_sync: Failed to sync payment status from payments microservice"
                .to_string(),
        })
        .attach_printable("Failed to sync payment status from payments microservice")?;

        Ok(payments_response)
    }

    pub async fn transition_workflow_state(
        &self,
        process: storage::ProcessTracker,
        payment_response: subscription_types::PaymentResponseData,
    ) -> CustomResult<(), router_errors::ApiErrorResponse> {
        let invoice_sync_status =
            storage::invoice_sync::InvoiceSyncPaymentStatus::from(payment_response.status);
        match invoice_sync_status {
            storage::invoice_sync::InvoiceSyncPaymentStatus::PaymentSucceeded => {
                perform_billing_processor_record_back(
                    self.state,
                    &self.merchant_account,
                    &self.key_store,
                    &self.tracking_data,
                    common_enums::AttemptStatus::Charged,
                    payment_response,
                    self.subscription.clone(),
                    self.customer.clone(),
                    self.profile.clone(),
                )
                .await
                .attach_printable("Failed to record back to billing processor")?;

                self.state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(
                        process.clone(),
                        business_status::COMPLETED_BY_PT,
                    )
                    .await
                    .change_context(router_errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to update process tracker status")
            }
            storage::invoice_sync::InvoiceSyncPaymentStatus::PaymentProcessing => {
                let db = &*self.state.store;
                let connector = self.tracking_data.connector_name.to_string().clone();
                let is_last_retry = retry_subscription_invoice_sync_task(
                    db,
                    connector,
                    self.merchant_account.get_id().to_owned(),
                    process.clone(),
                )
                .await
                .change_context(router_errors::ApiErrorResponse::SubscriptionError {
                    operation: "Invoice_sync process_tracker task retry".to_string(),
                })
                .attach_printable("Failed to update process tracker status")?;

                if is_last_retry {
                    self.state
                        .store
                        .as_scheduler()
                        .finish_process_with_business_status(
                            process,
                            business_status::COMPLETED_BY_PT,
                        )
                        .await
                        .change_context(router_errors::ApiErrorResponse::SubscriptionError {
                            operation: "Invoice_sync process_tracker task retry".to_string(),
                        })
                        .attach_printable("Failed to update process tracker status")?;
                }

                Ok(())
            }
            storage::invoice_sync::InvoiceSyncPaymentStatus::PaymentFailed => {
                perform_billing_processor_record_back(
                    self.state,
                    &self.merchant_account,
                    &self.key_store,
                    &self.tracking_data,
                    common_enums::AttemptStatus::Failure,
                    payment_response,
                    self.subscription.clone(),
                    self.customer.clone(),
                    self.profile.clone(),
                )
                .await
                .attach_printable("Failed to record back to billing processor")?;

                self.state
                    .store
                    .as_scheduler()
                    .finish_process_with_business_status(
                        process.clone(),
                        business_status::COMPLETED_BY_PT,
                    )
                    .await
                    .change_context(router_errors::ApiErrorResponse::SubscriptionError {
                        operation: "Invoice_sync process_tracker task retry".to_string(),
                    })
                    .attach_printable("Failed to update process tracker status")
            }
        }
    }
}

#[async_trait]
impl ProcessTrackerWorkflow<SessionState> for InvoiceSyncWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<storage::invoice_sync::InvoiceSyncTrackingData>(
            "InvoiceSyncTrackingData",
        )?;

        match process.name.as_deref() {
            Some(IVOICE_SYNC_WORKFLOW) => {
                Box::pin(perform_subscription_invoice_sync(
                    state,
                    process,
                    tracking_data,
                ))
                .await
            }
            _ => Err(errors::ProcessTrackerError::JobNotFound),
        }
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> CustomResult<(), errors::ProcessTrackerError> {
        logger::error!("Encountered error");
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

async fn perform_subscription_invoice_sync(
    state: &SessionState,
    process: storage::ProcessTracker,
    tracking_data: storage::invoice_sync::InvoiceSyncTrackingData,
) -> Result<(), errors::ProcessTrackerError> {
    let handler = InvoiceSyncHandler::create(state, tracking_data).await?;

    let payment_status = handler.perform_payments_sync().await?;

    Box::pin(handler.transition_workflow_state(process, payment_status)).await?;

    Ok(())
}

pub async fn perform_billing_processor_record_back(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    tracking_data: &storage::invoice_sync::InvoiceSyncTrackingData,
    payment_status: common_enums::AttemptStatus,
    payment_response: subscription_types::PaymentResponseData,
    subscription: Subscription,
    customer: domain::Customer,
    profile: domain::Profile,
) -> CustomResult<(), router_errors::ApiErrorResponse> {
    logger::info!("perform_billing_processor_record_back");

    let billing_handler = billing::BillingHandler::create(
        state,
        merchant_account,
        key_store,
        customer,
        profile.clone(),
    )
    .await
    .attach_printable("Failed to create billing handler")?;

    let invoice_handler = invoice_handler::InvoiceHandler::new(
        subscription.clone(),
        merchant_account.clone(),
        profile,
    );

    // Should we fetch latest invoice or use the one from tracking data?
    let invoice = invoice_handler
        .get_latest_invoice(state)
        .await
        .attach_printable("Unable to fetch Invoice from DB")?;

    // TODO: Handle retries here on failure
    billing_handler
        .record_back_to_billing_processor(
            state,
            tracking_data.connector_invoice_id.clone(),
            payment_response.payment_id.to_owned(),
            payment_status,
            payment_response.amount,
            payment_response.currency,
            payment_response.payment_method_type,
        )
        .await
        .attach_printable("Failed to record back to billing processor")?;

    invoice_handler
        .update_invoice(
            state,
            invoice.id.to_owned(),
            None,
            common_enums::connector_enums::InvoiceStatus::InvoicePaid,
        )
        .await
        .attach_printable("Failed to update invoice in DB")?;

    Ok(())
}

pub async fn create_invoice_sync_job(
    state: &SessionState,
    request: storage::invoice_sync::InvoiceSyncRequest,
) -> CustomResult<(), router_errors::ApiErrorResponse> {
    let tracking_data = storage::invoice_sync::InvoiceSyncTrackingData::from(request);

    let process_tracker_entry = diesel_models::ProcessTrackerNew::new(
        common_utils::generate_id(crate::consts::ID_LENGTH, "proc"),
        IVOICE_SYNC_WORKFLOW.to_string(),
        common_enums::ProcessTrackerRunner::InvoiceSyncflow,
        vec![IVOICE_SYNC_WORKFLOW_TAG.to_string()],
        tracking_data,
        Some(0),
        common_utils::date_time::now(),
        common_types::consts::API_VERSION,
    )
    .change_context(router_errors::ApiErrorResponse::InternalServerError)
    .attach_printable("subscriptions: unable to form process_tracker type")?;

    state
        .store
        .insert_process(process_tracker_entry)
        .await
        .change_context(router_errors::ApiErrorResponse::InternalServerError)
        .attach_printable("subscriptions: unable to insert process_tracker entry in DB")?;

    Ok(())
}

pub async fn get_subscription_invoice_sync_process_schedule_time(
    db: &dyn StorageInterface,
    connector: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    // Can have config based mapping as well
    let mapping: CustomResult<
        process_data::SubscriptionInvoiceSyncPTMapping,
        router_errors::StorageError,
    > = db
        .find_config_by_key(&format!("invoice_sync_pt_mapping_{connector}"))
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("SubscriptionInvoiceSyncPTMapping")
                .change_context(router_errors::StorageError::DeserializationFailed)
        });
    let mapping = match mapping {
        Ok(x) => x,
        Err(error) => {
            logger::info!(?error, "Redis Mapping Error");
            process_data::SubscriptionInvoiceSyncPTMapping::default()
        }
    };

    let time_delta = scheduler_utils::get_subscription_invoice_sync_retry_schedule_time(
        mapping,
        merchant_id,
        retry_count,
    );

    Ok(scheduler_utils::get_time_from_delta(time_delta))
}

pub async fn retry_subscription_invoice_sync_task(
    db: &dyn StorageInterface,
    connector: String,
    merchant_id: common_utils::id_type::MerchantId,
    pt: storage::ProcessTracker,
) -> Result<bool, errors::ProcessTrackerError> {
    let schedule_time = get_subscription_invoice_sync_process_schedule_time(
        db,
        connector.as_str(),
        &merchant_id,
        pt.retry_count + 1,
    )
    .await?;

    match schedule_time {
        Some(s_time) => {
            db.as_scheduler().retry_process(pt, s_time).await?;
            Ok(false)
        }
        None => {
            db.as_scheduler()
                .finish_process_with_business_status(pt, business_status::RETRIES_EXCEEDED)
                .await?;
            Ok(true)
        }
    }
}
